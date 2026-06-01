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
    const frame = buildPuzzleStudioThreeFrame(snapshot, view);
    const scene = new THREE.Scene();
    scene.background = threeBackground(THREE, view.background);
    addLights(THREE, scene, frame);
    addGrid(THREE, scene, frame);
    addMeshes(THREE, scene, frame);
    frame.rendererViewTarget = this.viewTarget;
    frame.rendererViewDistance = this.viewDistance;
    const camera = buildCamera(THREE, frame, this.canvas);
    this.renderer.setSize(this.canvas.clientWidth || this.canvas.width || 1, this.canvas.clientHeight || this.canvas.height || 1, false);
    this.renderer.setClearColor(0x000000, scene.background ? 1 : 0);
    disposeScene(this.scene);
    this.renderer.render(scene, camera);
    this.scene = scene;
    this.camera = camera;
    this.viewPayload = threeViewPayload(frame, camera, this.canvas);
    this.updateViewportMotion(frame);
    return {
      rendered: true,
      objectCount: frame.objectCount,
      animating: frame.viewport?.follow === "smooth" && frame.viewportAnimating === true,
      view: this.viewPayload,
    };
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

function disposeScene(scene) {
  if (!scene) {
    return;
  }
  scene.traverse((object) => {
    object.geometry?.dispose?.();
    const materials = Array.isArray(object.material) ? object.material : [object.material];
    for (const material of materials) {
      material?.dispose?.();
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
  const sprites = snapshot.sprites || {};
  const cells = (snapshot.cells || []).map((cell) => ({
    position: normalizePosition(cell.position),
    objects: (cell.objects || [])
      .map((object) => resolveObject(object, objectCatalog, sprites))
      .filter(Boolean),
  }));
  const frame = {
    size,
    cells,
    objectCatalog,
    objectCount: cells.reduce((count, cell) => count + cell.objects.length, 0),
    camera: snapshot.camera || {},
    editorView: view.editorView || snapshot.view || {},
    settings: snapshot.settings || {},
    viewport: normalizeViewport(snapshot.viewport),
    viewportSnapNext: view.viewportSnapNext === true,
  };
  frame.focusCell = frame.viewport ? viewportFocusCell(frame) : null;
  frame.viewportRanges = viewportRanges(frame);
  frame.renderRanges = renderRanges(frame);
  frame.renderCells = frame.renderRanges ? cells.filter((cell) => cellInRanges(cell, frame.renderRanges)) : cells;
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

function resolveObject(object, catalog, sprites) {
  const id = Number(object?.id);
  const base = Number.isFinite(id) ? catalog.get(id) || {} : {};
  const merged = { ...base, ...object, id: Number.isFinite(id) ? id : base.id };
  const spriteName = merged.sprite || merged.name;
  const name = merged.name || spriteName || (Number.isFinite(Number(merged.id)) ? `object_${merged.id}` : "");
  const visual = spriteVisual(sprites[spriteName]);
  if ((!Number.isFinite(Number(merged.id)) && !name && !spriteName) || !visual) {
    return null;
  }
  return {
    id: Number.isFinite(Number(merged.id)) ? Number(merged.id) : name,
    name,
    sprite: spriteName,
    layer: Number(merged.layer ?? base.layer ?? 0) || 0,
    visual,
  };
}

function spriteVisual(sprite) {
  if (!sprite) {
    return null;
  }
  const palette = sprite.palette || {};
  const bitmap = sprite.bitmap || [];
  const blocks = splitBitmapSlices(bitmap);
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
        const color = palette[token];
        if (!color || color === "transparent" || token === "." || token === " ") {
          continue;
        }
        voxels.push({
          x,
          y: Math.max(0, depth - 1 - rowIndex),
          z: Math.max(0, height - 1 - slice),
          color,
        });
      }
    }
  }
  if (!voxels.length) {
    return null;
  }
  return { kind: "voxels", size: { width, depth, height }, voxels };
}

function splitBitmapSlices(bitmap) {
  const slices = [[]];
  for (const row of bitmap || []) {
    if (String(row).length === 0) {
      slices.push([]);
    } else {
      slices[slices.length - 1].push(String(row));
    }
  }
  return slices.filter((slice) => slice.length > 0);
}

function addLights(THREE, scene, frame) {
  const size = Math.max(frame.size.width, frame.size.depth, frame.size.height, 1);
  scene.add(new THREE.AmbientLight("#ffffff", 1.35));
  const key = new THREE.DirectionalLight("#ffffff", 0.72);
  key.position.set(size * 1.2, size * 2.2, size * 0.9);
  scene.add(key);
  const fill = new THREE.DirectionalLight("#dbeafe", 0.42);
  fill.position.set(-size * 1.5, size * 1.0, -size * 1.1);
  scene.add(fill);
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

function addMeshes(THREE, scene, frame) {
  const groups = new Map();
  for (const cell of frame.renderCells || frame.cells) {
    for (const object of cell.objects) {
      for (const instance of objectInstances(frame, cell.position, object)) {
        const key = `${instance.color}:${instance.alpha}:${instance.scale.x}:${instance.scale.y}:${instance.scale.z}`;
        if (!groups.has(key)) {
          groups.set(key, { ...instance, items: [] });
        }
        groups.get(key).items.push(instance);
      }
    }
  }
  for (const group of groups.values()) {
    const geometry = new THREE.BoxGeometry(group.scale.x, group.scale.y, group.scale.z);
    const material = new THREE.MeshLambertMaterial({
      color: group.color,
      transparent: group.alpha < 1,
      opacity: group.alpha,
      depthWrite: group.alpha >= 1,
    });
    const mesh = new THREE.InstancedMesh(geometry, material, group.items.length);
    group.items.forEach((item, index) => {
      const matrix = new THREE.Matrix4();
      matrix.makeTranslation(item.position.x, item.position.y, item.position.z);
      mesh.setMatrixAt(index, matrix);
    });
    mesh.instanceMatrix.needsUpdate = true;
    scene.add(mesh);
  }
}

function objectInstances(frame, position, object) {
  if (object.visual?.kind === "voxels") {
    return voxelInstances(frame, position, object);
  }
  return [];
}

function voxelInstances(frame, position, object) {
  const visual = object.visual;
  const size = visual.size;
  const step = 1 / Math.max(size.width, size.height, size.depth, 1);
  const base = renderPositionForCell(frame, position);
  return visual.voxels.map((voxel) => {
    const local = spriteVoxelLocalPosition(voxel, step);
    return {
      position: {
        x: base.x + local.x,
        y: base.y + object.layer * 0.08 + local.z,
        z: base.z - local.y,
      },
      color: voxel.color,
      alpha: 1,
      scale: { x: step, y: step, z: step },
    };
  });
}

function spriteVoxelLocalPosition(voxel, step) {
  return {
    x: (voxel.x + 0.5) * step - 0.5,
    y: (voxel.y + 0.5) * step - 0.5,
    z: (voxel.z + 0.5) * step - 0.5,
  };
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
    || object.sprite === viewport.focus
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

function cellInRanges(cell, ranges) {
  const position = cell.position || {};
  return Number(position.x) >= ranges.x.min
    && Number(position.x) < ranges.x.max
    && Number(position.y) >= ranges.y.min
    && Number(position.y) < ranges.y.max
    && Number(position.z) >= ranges.z.min
    && Number(position.z) < ranges.z.max;
}

function renderRanges(frame) {
  if (!frame.viewportRanges) {
    return null;
  }
  const viewport = frame.viewport || {};
  const box = viewport.framingBox || {};
  const marginX = Math.max(2, Math.ceil(Number(box.width) || 1));
  const marginY = Math.max(2, Math.ceil(Number(box.depth) || 1));
  const marginZ = box.height === "full" ? 0 : Math.max(1, Math.ceil(Number(box.height) || 1));
  return expandRanges(frame.viewportRanges, { x: marginX, y: marginY, z: marginZ });
}

function expandRanges(ranges, margin) {
  return {
    x: { min: ranges.x.min - margin.x, max: ranges.x.max + margin.x },
    y: { min: ranges.y.min - margin.y, max: ranges.y.max + margin.y },
    z: { min: ranges.z.min - margin.z, max: ranges.z.max + margin.z },
  };
}

function buildCamera(THREE, frame, canvas) {
  const width = canvas.clientWidth || canvas.width || 1;
  const height = canvas.clientHeight || canvas.height || 1;
  const aspect = width / Math.max(1, height);
  const cameraSettings = frame.camera || {};
  const zoom = Math.max(0.1, Number(cameraSettings.zoom ?? frame.editorView?.zoom ?? 1) || 1);
  const view = cameraViewForFrame(frame, aspect, zoom);
  const targetPoint = new THREE.Vector3(view.target.x, view.target.y, view.target.z);
  const distance = view.distance;
  const yaw = degreesToRadians(cameraSettings.yawDegrees ?? 0);
  const pitch = degreesToRadians(clamp(Number(cameraSettings.pitchDegrees ?? 35) || 35, -90, 90));
  const horizontal = Math.cos(pitch);
  const projection = String(cameraSettings.projection || "").toLowerCase();
  const near = 0.1;
  const far = Math.max(1000, distance * 4);
  const camera = projection === "orthographic"
    ? buildOrthographicCamera(THREE, aspect, view.visibleHeight, near, far)
    : new THREE.PerspectiveCamera(34, aspect, near, far);
  camera.up.set(0, 1, 0);
  if (Math.abs(Math.cos(pitch)) < 0.001) {
    camera.up.set(0, 0, -Math.sign(Math.sin(pitch)) || -1);
  }
  camera.position.set(
    targetPoint.x - Math.sin(yaw) * horizontal * distance,
    targetPoint.y + Math.sin(pitch) * distance,
    targetPoint.z + Math.cos(yaw) * horizontal * distance,
  );
  camera.lookAt(targetPoint);
  return camera;
}

function threeViewPayload(frame, camera, canvas) {
  const width = Math.max(1, Number(canvas.clientWidth) || Number(canvas.width) || 1);
  const height = Math.max(1, Number(canvas.clientHeight) || Number(canvas.height) || 1);
  const rect = canvas.getBoundingClientRect();
  const cameraView = frame.cameraView || cameraViewForFrame(frame, width / height, Number(frame.camera?.zoom ?? frame.editorView?.zoom ?? 1) || 1);
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
  const logicalTarget = frame.viewportRanges ? center : {
    ...center,
    ...(frame.editorView?.target || {}),
  };
  const target = renderPositionForCell(frame, logicalTarget);
  const visibleWidth = Math.max(1, ranges.x.max - ranges.x.min);
  const visibleDepth = Math.max(1, ranges.y.max - ranges.y.min);
  const fov = 34 * Math.PI / 180;
  const visibleHeight = Math.max(visibleDepth, visibleWidth / Math.max(0.01, aspect)) * 1.08;
  const fittedVisibleHeight = visibleHeight * 1.12 / zoom;
  const targetDistance = Math.max(4, fittedVisibleHeight / (2 * Math.tan(fov / 2)));
  const snap = frame.viewport?.follow !== "smooth" || frame.viewportSnapNext || !thisLikeHasView(frame);
  const previousTarget = frame.rendererViewTarget || null;
  const previousDistance = Number(frame.rendererViewDistance);
  const cameraTarget = snap || !previousTarget
    ? target
    : {
        x: lerp(previousTarget.x, target.x, 0.12),
        y: lerp(previousTarget.y, target.y, 0.12),
        z: lerp(previousTarget.z, target.z, 0.12),
      };
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
};
})();
