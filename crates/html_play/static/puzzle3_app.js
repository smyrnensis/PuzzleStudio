const screenView = document.querySelector("#screenView") || document.body;
const componentEmbedMode = new URLSearchParams(window.location.search).get("component") === "1"
  || window.Puzzle3DComponentEmbed === true;
let editorComponentEmbedMode = false;
document.documentElement.classList.toggle("is-component-embed", componentEmbedMode);
document.body.classList.add("theme-clean");
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
  settings: {
    interactiveLook: false,
    interactiveZoom: false,
    grid: { visibility: 0 },
    shade: true,
  },
  directions: {
    left: { dx: -1, dy: 0, dz: 0 },
    right: { dx: 1, dy: 0, dz: 0 },
    forward: { dx: 0, dy: 1, dz: 0 },
    backward: { dx: 0, dy: -1, dz: 0 },
    up: { dx: 0, dy: 0, dz: 1 },
    down: { dx: 0, dy: 0, dz: -1 },
  },
  directionSets: {
    horizontal: ["left", "right", "forward", "backward"],
    vertical: ["up", "down"],
  },
  controls: {
    keys: {
      ArrowLeft: "left",
      ArrowRight: "right",
      ArrowUp: "forward",
      ArrowDown: "backward",
    },
  },
  sprites: {
    bumpy: {
      palette: { 0: "#000000" },
      bitmap: [
        " 00 ",
        "0000",
        "0000",
        " 00 ",
        "",
        "0000",
        "0000",
        "0000",
        "000 ",
        "",
        " 000",
        "0000",
        "0000",
        " 00 ",
        "",
        " 00 ",
        "000 ",
        " 00 ",
        " 0  ",
      ],
    },
    red_cube: {
      palette: { r: "#d94132" },
      bitmap: [
        "rrrr",
        "rrrr",
        "rrrr",
        "rrrr",
        "",
        "rrrr",
        "rrrr",
        "rrrr",
        "rrrr",
        "",
        "rrrr",
        "rrrr",
        "rrrr",
        "rrrr",
        "",
        "rrrr",
        "rrrr",
        "rrrr",
        "rrrr",
      ],
    },
  },
  cells: [
    {
      position: { x: 1, y: 1, z: 1 },
      objects: [{ id: 1, name: "Bumpy", sprite: "bumpy" }],
    },
    {
      position: { x: 2, y: 1, z: 1 },
      objects: [{ id: 2, name: "Red Cube", sprite: "red_cube" }],
    },
    {
      position: { x: 0, y: 2, z: 1 },
      objects: [{ id: 2, name: "Red Cube", sprite: "red_cube" }],
    },
  ],
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
};
let snapshot = fallbackSnapshot;
let runtime = window.Puzzle3DTestRuntime.create(fallbackSnapshot);
let initialCamera = cloneCamera(fallbackSnapshot.camera);
let currentSceneName = initialSceneName(fallbackSnapshot);
let editorModelComponentPreview = null;
let levelMenuCursor = 0;
let sceneButtonCursor = 0;
let mountedPuzzle3Component = null;
let pendingResizeFrame = 0;
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
  loadSnapshotData(nextSnapshot);
  if (window.PuzzleStudioInitialModelComponentPreview?.type === "PuzzleStudioRenderPuzzle3ModelComponent") {
    applyPuzzle3ModelComponentPreviewUpdate(window.PuzzleStudioInitialModelComponentPreview);
  }
}

function loadSnapshotData(source, options = {}) {
  snapshot = normalizeSnapshot(source || fallbackSnapshot);
  runtime = window.Puzzle3DTestRuntime.create(snapshot);
  snapshot = runtime.snapshot();
  editorModelComponentPreview = options.modelComponentPreview || null;
  document.title = snapshot.title || "Puzzle3";
  currentSceneName = editorModelComponentPreview?.sceneName
    || options.scene
    || (options.preferPuzzleScene ? puzzleSceneName(snapshot) : "")
    || initialSceneName(snapshot);
  levelMenuCursor = snapshot.levelIndex || 0;
  initialCamera = cloneCamera(snapshot.camera || fallbackSnapshot.camera);
  renderScene();
}

function applyPuzzle3PreviewUpdate(update = {}) {
  const next = puzzle3PreviewSnapshot(update);
  loadSnapshotData(next, {
    scene: update.scene,
    preferPuzzleScene: update.preferPuzzleScene !== false,
  });
}

function applyPuzzle3ModelComponentPreviewUpdate(update = {}) {
  const next = puzzle3PreviewSnapshot(update);
  loadSnapshotData(next, {
    modelComponentPreview: {
      sceneName: update.scene || "__editor_model_preview__",
      component: puzzle3ModelPreviewComponent(update),
    },
  });
}

function puzzle3PreviewSnapshot(update = {}) {
  const next = JSON.parse(JSON.stringify(snapshot || fallbackSnapshot));
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

function puzzle3ComponentFor(scene) {
  return findSceneComponent(scene?.components || [], (component) => component.kind === "puzzle3");
}

function renderScene() {
  if (editorModelComponentPreview) {
    const sceneName = editorModelComponentPreview.sceneName || "__editor_model_preview__";
    screenView.className = `scene ${sceneName}`;
    puzzle3Component.mount(editorModelComponentPreview.component || puzzle3ModelPreviewComponent(), sceneName);
    return;
  }
  const scene = currentScene();
  screenView.className = `scene ${scene?.name || currentSceneName || "default"}`;
  const embeddedPuzzle3Component = puzzle3ComponentFor(scene);
  if (effectiveComponentEmbedMode() && embeddedPuzzle3Component) {
    puzzle3Component.mount(embeddedPuzzle3Component, scene?.name || currentSceneName || "default");
    return;
  }
  const components = scene?.components?.length
    ? scene.components
    : [{ kind: "title", text: snapshot.title || "Puzzle3" }];
  sceneButtonCursor = clamp(sceneButtonCursor, 0, Math.max(0, sceneButtonComponents(scene).length - 1));
  const rootComponent = {
    kind: "column",
    children: components,
    layout: scene?.layout || { gap: 0, align: { x: "center", y: "center" } },
  };
  const measured = measureSceneNode(rootComponent);
  const viewport = {
    width: Math.max(1, screenView.clientWidth || window.innerWidth || 1),
    height: Math.max(1, screenView.clientHeight || window.innerHeight || 1),
  };
  const scale = Math.max(0.0001, Math.min(viewport.width / measured.width, viewport.height / measured.height));
  const root = document.createElement("section");
  root.className = "scene-layout-root";
  root.style.width = `${measured.width * scale}px`;
  root.style.height = `${measured.height * scale}px`;
  renderSceneNode(rootComponent, root, { x: 0, y: 0, width: measured.width, height: measured.height }, scale, scene?.name || currentSceneName || "default");
  screenView.replaceChildren(root);
  if (puzzle3ComponentFor(scene)) {
    requestAnimationFrame(() => puzzle3Component.handleResize());
  }
}

function renderSceneNode(component, parent, rect, scale, sceneName) {
  if (component.kind === "row" || component.kind === "column" || component.kind === "box") {
    renderSceneContainer(component, parent, rect, scale, sceneName);
    return;
  }
  const node = sceneNodeElement(component, rect, scale);
  if (component.kind === "title") {
    const title = document.createElement("h1");
    title.className = "view-title";
    title.textContent = component.text;
    node.append(title);
  } else if (component.kind === "button") {
    const buttons = sceneButtonComponents();
    const currentIndex = buttons.indexOf(component);
    const levelMenuOwnsCursor = Boolean(activeLevelMenuComponent());
    node.append(sceneButton(component.label, () => applySceneAction(component.action), {
      selected: !levelMenuOwnsCursor && currentIndex === sceneButtonCursor,
    }));
  } else if (component.kind === "level_menu") {
    node.append(renderLevelMenu(component));
  } else if (component.kind === "puzzle3") {
    node.classList.add("scene-puzzle3-node");
    const inner = containRect(
      { x: 0, y: 0, width: rect.width, height: rect.height },
      currentPuzzle3IntrinsicSize(),
      sceneLayout(component),
    );
    puzzle3Frame.style.position = "absolute";
    puzzle3Frame.style.left = `${inner.x * scale}px`;
    puzzle3Frame.style.top = `${inner.y * scale}px`;
    puzzle3Frame.style.width = `${inner.width * scale}px`;
    puzzle3Frame.style.height = `${inner.height * scale}px`;
    node.append(puzzle3Frame);
    applySceneComponentMetadata(component, sceneName);
    updateCameraInteractionState();
  }
  parent.append(node);
}

function renderSceneContainer(component, parent, rect, scale, sceneName) {
  const node = sceneNodeElement(component, rect, scale);
  node.classList.add(`scene-${component.kind}-node`);
  parent.append(node);
  const children = component.children || [];
  if (children.length === 0) {
    return;
  }
  const layout = sceneLayout(component);
  const gap = sceneGap(component);
  if (component.kind === "row") {
    const measured = children.map(measureSceneNode);
    const totalWidth = measured.reduce((total, child) => total + child.width, 0) + gap * Math.max(0, children.length - 1);
    let x = alignOffset(rect.width, totalWidth, layout.align?.x);
    for (let index = 0; index < children.length; index += 1) {
      const child = measured[index];
      const y = alignOffset(rect.height, child.height, layout.align?.y);
      renderSceneNode(children[index], node, { x, y, width: child.width, height: child.height }, scale, sceneName);
      x += child.width + gap;
    }
    return;
  }
  const measured = children.map(measureSceneNode);
  const totalHeight = measured.reduce((total, child) => total + child.height, 0) + gap * Math.max(0, children.length - 1);
  let y = alignOffset(rect.height, totalHeight, layout.align?.y);
  for (let index = 0; index < children.length; index += 1) {
    const child = measured[index];
    const x = alignOffset(rect.width, child.width, layout.align?.x);
    renderSceneNode(children[index], node, { x, y, width: child.width, height: child.height }, scale, sceneName);
    y += child.height + gap;
  }
}

function sceneNodeElement(component, rect, scale) {
  const node = document.createElement("div");
  node.className = `scene-layout-node scene-component-${component.kind}`;
  node.style.left = `${rect.x * scale}px`;
  node.style.top = `${rect.y * scale}px`;
  node.style.width = `${rect.width * scale}px`;
  node.style.height = `${rect.height * scale}px`;
  return node;
}

function measureSceneNode(component) {
  const explicit = component.layout?.size;
  if (explicit) {
    return {
      width: Math.max(1, Number(explicit.width) || 1),
      height: Math.max(1, Number(explicit.height) || 1),
    };
  }
  if (component.kind === "row") {
    const children = (component.children || []).map(measureSceneNode);
    return {
      width: Math.max(1, children.reduce((total, child) => total + child.width, 0) + sceneGap(component) * Math.max(0, children.length - 1)),
      height: Math.max(1, children.reduce((height, child) => Math.max(height, child.height), 0)),
    };
  }
  if (component.kind === "column" || component.kind === "box") {
    const children = (component.children || []).map(measureSceneNode);
    return {
      width: Math.max(1, children.reduce((width, child) => Math.max(width, child.width), 0)),
      height: Math.max(1, children.reduce((total, child) => total + child.height, 0) + sceneGap(component) * Math.max(0, children.length - 1)),
    };
  }
  if (component.kind === "puzzle3") {
    return currentPuzzle3IntrinsicSize();
  }
  if (component.kind === "level_menu") {
    return { width: 5, height: Math.max(1, puzzle3Component.levelEntries(component.levels).length) };
  }
  if (component.kind === "button") {
    return { width: Math.max(4, Math.ceil(String(component.label || "").length / 3)), height: 1 };
  }
  if (component.kind === "title") {
    return { width: Math.max(6, Math.ceil(String(component.text || "").length / 3)), height: 1 };
  }
  return { width: 1, height: 1 };
}

function currentPuzzle3IntrinsicSize() {
  const size = snapshot.size || fallbackSnapshot.size;
  return {
    width: Math.max(1, Number(size.width) || 1),
    height: Math.max(1, Number(size.depth) || Number(size.height) || 1),
  };
}

function sceneLayout(component) {
  return component.layout || { align: { x: "center", y: "center" } };
}

function sceneGap(component) {
  return Math.max(0, Number(component.layout?.gap || 0));
}

function alignOffset(outer, inner, align) {
  const space = Math.max(0, outer - inner);
  if (align === "left" || align === "top") {
    return 0;
  }
  if (align === "right" || align === "bottom") {
    return space;
  }
  return space / 2;
}

function containRect(box, intrinsic, layout) {
  const scale = Math.min(box.width / intrinsic.width, box.height / intrinsic.height);
  const width = intrinsic.width * scale;
  const height = intrinsic.height * scale;
  return {
    x: box.x + alignOffset(box.width, width, layout.align?.x),
    y: box.y + alignOffset(box.height, height, layout.align?.y),
    width,
    height,
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

function collectSceneComponents(components, predicate, out = []) {
  for (const component of components || []) {
    if (predicate(component)) {
      out.push(component);
    }
    if (component.children) {
      collectSceneComponents(component.children, predicate, out);
    }
  }
  return out;
}

function renderMenuScene(scene) {
  const root = document.createElement("section");
  root.className = "scene-menu is-menu-scene";
  let buttonIndex = 0;
  for (const component of scene?.components || []) {
    if (component.kind === "title") {
      const title = document.createElement("h1");
      title.className = "view-title";
      title.textContent = component.text;
      root.append(title);
    } else if (component.kind === "button") {
      const currentIndex = buttonIndex;
      root.append(sceneButton(component.label, () => applySceneAction(component.action), {
        selected: !levelMenuOwnsCursor && currentIndex === sceneButtonCursor,
      }));
      buttonIndex += 1;
    } else if (component.kind === "level_menu") {
      root.append(renderLevelMenu(component));
    }
  }
  if (!root.childElementCount) {
    const title = document.createElement("h1");
    title.textContent = snapshot.title || "Puzzle3";
    root.append(title);
  }
  return root;
}

function sceneButton(label, onClick, options = {}) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "scene-button";
  button.classList.toggle("is-selected", Boolean(options.selected));
  button.textContent = label;
  button.addEventListener("click", onClick);
  return button;
}

function renderLevelMenu(component) {
  const menu = document.createElement("div");
  menu.className = "level-menu";
  menu.dataset.levels = component.levels || "levels";
  const levels = puzzle3Component.levelEntries(component.levels);
  levelMenuCursor = clamp(levelMenuCursor, 0, Math.max(0, levels.length - 1));
  levels.forEach((level, index) => {
    const label = level.label || level.name || `Level ${index + 1}`;
    const button = sceneButton(label, () => applyStartLevels(component.action, index));
    button.classList.toggle("is-selected", index === levelMenuCursor);
    button.setAttribute("aria-current", index === levelMenuCursor ? "true" : "false");
    menu.append(button);
  });
  return menu;
}

function activeLevelMenuComponent() {
  return findSceneComponent(currentScene()?.components || [], (component) => component.kind === "level_menu");
}

function sceneButtonComponents(scene = currentScene()) {
  return collectSceneComponents(scene?.components || [], (component) => component.kind === "button");
}

function applySelectedSceneButton() {
  const button = sceneButtonComponents()[sceneButtonCursor];
  if (button) {
    applySceneAction(button.action);
  }
}

function applySceneAction(action) {
  if (action?.kind === "goto" && action.scene) {
    gotoScene(action.scene);
  } else if (action?.kind === "start_levels" && action.scene) {
    applyStartLevels(action, 0);
  }
}

function applyStartLevels(action, levelIndex) {
  puzzle3Component.startLevel(action?.levels, levelIndex);
  currentSceneName = action?.scene || puzzleSceneName() || currentSceneName;
  renderScene();
}

function gotoScene(sceneName) {
  currentSceneName = sceneName;
  sceneButtonCursor = 0;
  const menu = activeLevelMenuComponent();
  if (menu) {
    levelMenuCursor = puzzle3Component.currentLevelIndex(menu.levels);
  }
  renderScene();
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
    startLevel(levelsName, levelIndex) {
      runtime.loadLevel(globalLevelIndexForBundle(levelsName, levelIndex));
      snapshot = runtime.snapshot();
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
      if (input === "undo") {
        if (!runtime.undo()) {
          return false;
        }
      } else if (input === "restart") {
        if (!runtime.restart()) {
          return false;
        }
      } else if (!runtime.applyInput(input)) {
        return false;
      }
      snapshot = runtime.snapshot();
      draw();
      return true;
    },
    nextLevel() {
      if (!runtime.nextLevel()) {
        return false;
      }
      snapshot = runtime.snapshot();
      draw();
      return true;
    },
    previousLevel() {
      if (!runtime.previousLevel()) {
        return false;
      }
      snapshot = runtime.snapshot();
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
      this.applyInput(input);
      return true;
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

function schedulePuzzle3Resize() {
  if (pendingResizeFrame) {
    return;
  }
  pendingResizeFrame = requestAnimationFrame(() => {
    pendingResizeFrame = 0;
    if (effectiveComponentEmbedMode() && puzzle3ComponentFor(currentScene())) {
      puzzle3Component.handleResize();
    } else {
      renderScene();
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
  if (!shouldAutoFitFiniteStage(size)) {
    return;
  }
  const width = Math.max(1, Number(rect.width) || 1);
  const height = Math.max(1, Number(rect.height) || 1);
  const zoom = projectionZoom(camera);
  const bounds = projectedSceneBoundsUnit(size, camera);
  const boundsWidth = Math.max(0.001, bounds.maxX - bounds.minX);
  const boundsHeight = Math.max(0.001, bounds.maxY - bounds.minY);
  const padding = 0.72;
  const scale = Math.min(width / boundsWidth, height / boundsHeight) * padding;
  view.cellScale = Math.max(0.0001, scale / zoom);
  const effectiveScale = view.cellScale * zoom;
  view.originX = width / 2 - ((bounds.minX + bounds.maxX) / 2) * effectiveScale;
  view.originY = height / 2 - ((bounds.minY + bounds.maxY) / 2) * effectiveScale;
  view.projectionWidth = width;
  view.projectionHeight = height;
  view.projectionFitKey = projectionFitKey(size, camera);
}

function ensureProjectionFit() {
  const rect = canvas.getBoundingClientRect();
  const size = snapshot.size || fallbackSnapshot.size;
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

function projectionZoom(camera) {
  return Math.max(0.1, Number(camera?.zoom ?? 1) || 1);
}

function projectedSceneBoundsUnit(size, camera) {
  const width = Math.max(1, Number(size.width) || 1);
  const depth = Math.max(1, Number(size.depth) || 1);
  const height = Math.max(1, Number(size.height) || 1);
  const corners = [];
  for (const x of [-0.5, width - 0.5]) {
    for (const y of [-0.5, depth - 0.5]) {
      for (const z of [-0.55, height - 0.5]) {
        corners.push(projectScenePointUnit({ x, y, z }, { width, depth, height }, camera));
      }
    }
  }
  return corners.reduce(
    (bounds, point) => ({
      minX: Math.min(bounds.minX, point.x),
      maxX: Math.max(bounds.maxX, point.x),
      minY: Math.min(bounds.minY, point.y),
      maxY: Math.max(bounds.maxY, point.y),
    }),
    { minX: Infinity, maxX: -Infinity, minY: Infinity, maxY: -Infinity },
  );
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
  return Puzzle3VisualCore.projectOrthographic(position, {
    camera,
    center: {
      x: (snapshot.size.width - 1) / 2,
      y: (snapshot.size.depth - 1) / 2,
      z: (snapshot.size.height - 1) / 2,
    },
    origin: { x: view.originX, y: view.originY },
    scale: view.cellScale,
  });
}

function degreesToRadians(value) {
  return (value * Math.PI) / 180;
}

function draw() {
  syncCanvasSize();
  ensureProjectionFit();
  const width = canvas.clientWidth;
  const height = canvas.clientHeight;
  ctx.clearRect(0, 0, width, height);

  let primitives = scenePrimitives();
  if (fitProjectionToContent(primitives, width, height)) {
    primitives = scenePrimitives();
  }
  primitives.sort(comparePrimitiveOrder);
  for (const primitive of primitives) {
    if (primitive.kind === "line") {
      lineSegment(primitive.from, primitive.to, primitive.stroke, primitive.width, primitive.alpha);
    } else {
      polygonPoints(primitive.points, primitive.fill);
    }
  }
  notifyPuzzle3View(width, height);
}

function scenePrimitives() {
  const primitives = [...sceneFaces(), ...gridLines(gridSettings())];
  if (view.shadowsEnabled) {
    primitives.push(...shadowFaces());
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
  const minX = Math.min(...points.map((point) => point.x));
  const maxX = Math.max(...points.map((point) => point.x));
  const minY = Math.min(...points.map((point) => point.y));
  const maxY = Math.max(...points.map((point) => point.y));
  const contentWidth = Math.max(0.001, maxX - minX);
  const contentHeight = Math.max(0.001, maxY - minY);
  const margin = Math.max(0, Number(fit.margin) || 0);
  const availableWidth = Math.max(1, width - margin * 2);
  const availableHeight = Math.max(1, height - margin * 2);
  const multiplier = Math.min(availableWidth / contentWidth, availableHeight / contentHeight);
  if (!Number.isFinite(multiplier) || multiplier <= 0 || Math.abs(multiplier - 1) < 0.0001) {
    return false;
  }
  const centerX = (minX + maxX) / 2;
  const centerY = (minY + maxY) / 2;
  view.originX = width / 2 + (view.originX - centerX) * multiplier;
  view.originY = height / 2 + (view.originY - centerY) * multiplier;
  view.cellScale *= multiplier;
  return true;
}

function fitProjectionToXYStageBounds(width, height, fit) {
  const size = snapshot.size || fallbackSnapshot.size;
  const camera = snapshot.camera || fallbackSnapshot.camera;
  const margin = Math.max(0, Number(fit.margin) || 0);
  const availableWidth = Math.max(1, width - margin * 2);
  const availableHeight = Math.max(1, height - margin * 2);
  const bounds = stageProjectionUnitBounds(size, camera, "xy");
  const boundsWidth = Math.max(0.001, bounds.maxX - bounds.minX);
  const boundsHeight = Math.max(0.001, bounds.maxY - bounds.minY);
  const effectiveScale = Math.max(0.0001, Math.min(availableWidth / boundsWidth, availableHeight / boundsHeight));
  const nextCellScale = effectiveScale / projectionZoom(camera);
  const nextOriginX = width / 2 - ((bounds.minX + bounds.maxX) / 2) * effectiveScale;
  const nextOriginY = height / 2 - ((bounds.minY + bounds.maxY) / 2) * effectiveScale;
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
  const margin = Math.max(0, Number(fit.margin) || 0);
  const availableWidth = Math.max(1, width - margin * 2);
  const availableHeight = Math.max(1, height - margin * 2);
  const bounds = stageProjectionUnitBounds(size, camera, fit.mode);
  const boundsWidth = Math.max(0.001, bounds.maxX - bounds.minX);
  const boundsHeight = Math.max(0.001, bounds.maxY - bounds.minY);
  const effectiveScale = Math.max(0.0001, Math.min(availableWidth / boundsWidth, availableHeight / boundsHeight));
  const nextCellScale = effectiveScale / projectionZoom(camera);
  const nextOriginX = width / 2 - ((bounds.minX + bounds.maxX) / 2) * effectiveScale;
  const nextOriginY = height / 2 - ((bounds.minY + bounds.maxY) / 2) * effectiveScale;
  const changed = Math.abs(view.cellScale - nextCellScale) > 0.0001
    || Math.abs(view.originX - nextOriginX) > 0.0001
    || Math.abs(view.originY - nextOriginY) > 0.0001;
  view.cellScale = nextCellScale;
  view.originX = nextOriginX;
  view.originY = nextOriginY;
  return changed;
}

function stageProjectionUnitBounds(size, camera, mode = "stage") {
  const width = Math.max(1, Number(size.width) || 1);
  const depth = Math.max(1, Number(size.depth) || 1);
  const height = Math.max(1, Number(size.height) || 1);
  const zValues = mode === "xy" ? [0] : [-0.5, height - 0.5];
  const points = [];
  for (const x of [-0.5, width - 0.5]) {
    for (const y of [-0.5, depth - 0.5]) {
      for (const z of zValues) {
        points.push(projectScenePointUnit({ x, y, z }, { width, depth, height }, camera));
      }
    }
  }
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
  const camera = snapshot.camera || fallbackSnapshot.camera;
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
      scale: view.cellScale * projectionZoom(camera),
      center: {
        x: (Math.max(1, Number(size.width) || 1) - 1) / 2,
        y: (Math.max(1, Number(size.depth) || 1) - 1) / 2,
        z: (Math.max(1, Number(size.height) || 1) - 1) / 2,
      },
      camera: cloneCamera(camera),
      size: {
        width: Math.max(1, Number(size.width) || 1),
        depth: Math.max(1, Number(size.depth) || 1),
        height: Math.max(1, Number(size.height) || 1),
      },
      cellFootprints: projectedStageCellFootprints(size),
    },
  }, "*");
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

function gridLines(grid) {
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
    for (const cell of snapshot.cells || []) {
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
  return {
    kind: "line",
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

function puzzle3VisualView() {
  return { camera: snapshot.camera || fallbackSnapshot.camera };
}

function gridStroke(kind, grid) {
  if (kind === "stageFrame") {
    return grid.frameColor || "rgba(29, 37, 44, 0.82)";
  }
  return grid.color || "rgba(31, 36, 40, 0.62)";
}

function sceneFaces() {
  const faces = [];
  for (const cell of snapshot.cells) {
    const { voxels, occupied } = cellVisibleVoxels(cell);
    faces.push(...mergedVoxelFaces(voxels, occupied, cellRenderOwner(cell.position)));
  }
  return faces;
}

function shadowFaces() {
  const faces = [];
  for (const cell of snapshot.cells) {
    const point = projectWithDepth({ x: cell.position.x, y: cell.position.y, z: -0.48 });
    faces.push({
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

function cellVisibleVoxels(cell) {
  const stacks = new Map();
  for (const [objectIndex, object] of cell.objects.entries()) {
    const sourceKey = `${cellKey(cell.position)}:${objectIndex}`;
    for (const voxel of objectVoxels(cell.position, object, sourceKey)) {
      const key = voxelGeometryKey(voxel);
      const stack = stacks.get(key) || [];
      stack.push(voxel);
      stacks.set(key, stack);
    }
  }
  const voxels = [];
  const occupied = {
    opaque: new Set(),
    bySource: new Set(),
  };
  for (const [key, stack] of stacks) {
    const voxel = compositeVoxelStack(stack);
    if (voxel) {
      voxels.push(voxel);
      for (const sourceKey of voxel.sourceKeys || []) {
        occupied.bySource.add(`${sourceKey}|${key}`);
      }
      if (isOpaqueFill(voxel.fill)) {
        occupied.opaque.add(key);
      }
    } else {
      stacks.delete(key);
    }
  }
  return { voxels, occupied };
}

function compositeVoxelStack(stack) {
  let color = { r: 0, g: 0, b: 0, a: 0 };
  let geometry = null;
  const sourceKeys = new Set();
  for (const voxel of stack) {
    const source = parseColor(voxel.fill);
    if (!source || source.a <= 0) {
      continue;
    }
    color = compositeColor(source, color);
    geometry = voxel;
    if (voxel.sourceKey) {
      sourceKeys.add(voxel.sourceKey);
    }
  }
  if (!geometry || color.a <= 0.001) {
    return null;
  }
  return { ...geometry, fill: formatColor(color), sourceKeys: [...sourceKeys] };
}

function objectVoxels(position, object, sourceKey) {
  const sprite = snapshot.sprites?.[object.sprite];
  if (!sprite) {
    return [{
      fill: cssVar("--top") || "#ffde8a",
      scale: 1,
      grid: { x: 0, y: 0, z: 0 },
      position: { ...position },
      bounds: voxelBounds(position, 1),
      sourceKey,
    }];
  }
  const blocks = bitmapBlocks(sprite.bitmap || []);
  return spriteVoxels(position, blocks, sprite.palette || {}, sourceKey).voxels;
}

function spriteVoxels(position, blocks, palette, sourceKey = null) {
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
          scale,
          grid,
          position: voxelPosition,
          bounds: voxelBounds(voxelPosition, scale),
          sourceKey,
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
          ownerCell,
          side: face.side,
          origin: info.origin,
          scale: voxel.scale,
          planeIndex: info.planeIndex,
          fill,
        },
      };
    },
    face: (group, rect) => projectFace(mergedVoxelFaceCorners(group, rect), group.fill, group.ownerCell),
  });
}

function isVoxelFaceOccluded(voxel, offset, occupied) {
  const adjacentKey = adjacentVoxelGeometryKey(voxel, offset);
  if (occupied.opaque.has(adjacentKey)) {
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

function projectFace(corners, fill, ownerCell = null) {
  const projected = corners.map(projectWithDepth);
  return {
    kind: "face",
    points: projected.map(({ x, y }) => ({ x, y })),
    depth: projected.reduce((total, point) => total + point.depth, 0) / projected.length,
    gridOrder: faceGridOrder(corners),
    ownerCell,
    renderPriority: 0,
    fill,
  };
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

function compositeColor(source, destination) {
  const alpha = source.a + destination.a * (1 - source.a);
  if (alpha <= 0) {
    return { r: 0, g: 0, b: 0, a: 0 };
  }
  return {
    r: (source.r * source.a + destination.r * destination.a * (1 - source.a)) / alpha,
    g: (source.g * source.a + destination.g * destination.a * (1 - source.a)) / alpha,
    b: (source.b * source.a + destination.b * destination.a * (1 - source.a)) / alpha,
    a: alpha,
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

window.addEventListener("resize", schedulePuzzle3Resize);

if (window.ResizeObserver) {
  const resizeObserver = new ResizeObserver(schedulePuzzle3Resize);
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

  const levelMenu = activeLevelMenuComponent();
  if (levelMenu) {
    if (event.key === "ArrowUp" || event.key === "ArrowLeft") {
      event.preventDefault();
      levelMenuCursor = Math.max(0, levelMenuCursor - 1);
      renderScene();
      return;
    }
    if (event.key === "ArrowDown" || event.key === "ArrowRight") {
      event.preventDefault();
      const levelCount = puzzle3Component.levelEntries(levelMenu.levels).length;
      levelMenuCursor = Math.min(
        Math.max(0, levelCount - 1),
        levelMenuCursor + 1,
      );
      renderScene();
      return;
    }
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      applyStartLevels(levelMenu.action, levelMenuCursor);
      return;
    }
  } else if (!puzzle3ComponentFor(currentScene())) {
    const buttons = sceneButtonComponents();
    if (buttons.length > 0 && (event.key === "ArrowUp" || event.key === "ArrowLeft")) {
      event.preventDefault();
      sceneButtonCursor = Math.max(0, sceneButtonCursor - 1);
      renderScene();
      return;
    }
    if (buttons.length > 0 && (event.key === "ArrowDown" || event.key === "ArrowRight")) {
      event.preventDefault();
      sceneButtonCursor = Math.min(buttons.length - 1, sceneButtonCursor + 1);
      renderScene();
      return;
    }
    if (buttons.length > 0 && (event.key === "Enter" || event.key === " ")) {
      event.preventDefault();
      applySelectedSceneButton();
      return;
    }
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

if (!effectiveComponentEmbedMode()) {
  window.addEventListener("keydown", handleStandaloneKeydown);
}

window.addEventListener("message", (event) => {
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
    loadSnapshotData(event.data.snapshot, {
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
    applySceneRawInput({
      key: String(event.data.key || ""),
      code: String(event.data.code || ""),
    });
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
    forward: ["KeyW", "ArrowUp"],
    backward: ["KeyS", "ArrowDown"],
  };
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
    return;
  }
  applySceneAction(control);
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

function applySceneRawInput(raw) {
  const input = inputForRawInput(raw, puzzle3ComponentFor(currentScene()));
  if (!input) {
    return false;
  }
  return applySceneInput(input);
}

resizeCanvas();
loadSnapshot();

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
