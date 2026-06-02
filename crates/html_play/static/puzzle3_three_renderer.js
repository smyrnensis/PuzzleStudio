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
    frame.rendererViewTarget = this.viewTarget;
    frame.rendererViewDistance = this.viewDistance;
    const camera = buildCamera(THREE, frame, this.canvas);
    addMeshes(THREE, scene, frame);
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
  const { voxels, occupied } = frameVisibleVoxels(frame);
  const faces = mergedVoxelFaces(voxels, occupied);
  const opaqueGroups = new Map();
  const materialCache = new Map();
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
    scene.add(mesh);
  }
}

function frameVisibleVoxels(frame) {
  const voxels = [];
  const occupied = emptyVoxelOccupancy();
  for (const cell of frame.renderCells || frame.cells) {
    const visible = cellVisibleVoxels(frame, cell);
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

function cellVisibleVoxels(frame, cell) {
  const stacks = new Map();
  for (const [objectIndex, object] of (cell.objects || []).entries()) {
    const sourceKey = `${cellKey(cell.position)}:${objectIndex}`;
    const objectOrder = objectRenderOrder(object, objectIndex);
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
  for (const voxel of ordered) {
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

function objectRenderOrder(object, fallbackIndex = 0) {
  const layer = Number(object?.layer);
  return Number.isFinite(layer) ? layer : Number(fallbackIndex) || 0;
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
  return visual.voxels.map((voxel) => {
    const local = spriteVoxelLocalPosition(voxel, step);
    const layerY = object.layer * 0.08;
    const renderPosition = {
      x: base.x + local.x,
      y: base.y + layerY + local.z,
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
      grid: { x: voxel.x, y: voxel.z, z: -voxel.y },
      position: renderPosition,
      stackPosition,
      bounds: voxelBounds(renderPosition, step),
      sourceKey,
      objectOrder,
    };
  });
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

function spriteVoxelLocalPosition(voxel, step) {
  return {
    x: (voxel.x + 0.5) * step - 0.5,
    y: (voxel.y + 0.5) * step - 0.5,
    z: (voxel.z + 0.5) * step - 0.5,
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
