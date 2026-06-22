const themeStoreKey = "PuzzleStudioEditorTheme:v1";
const previewDefaultLogicalWidth = 4;
const previewDefaultLogicalHeight = 3;
const previewMinimumHeight = 720;
const previewMinimumLogHeight = 72;
const wasmCompilerAssetVersion = Date.now().toString(36);
const solverFeedbackTickMs = 250;
const solutionPlaybackBaseIntervalMs = 350;
const WASM_SECTION_BLOCK_NAMES = Object.freeze({
  scratch: "scratch",
  group: "group",
  groups: "group",
  layer: "layers",
  layers: "layers",
  legend: "legend",
  legends: "legend",
  win_condition: "win_conditions",
  win_conditions: "win_conditions",
  lose_condition: "lose_conditions",
  lose_conditions: "lose_conditions",
  sprite: "sprites",
  sprites: "sprites",
  asset: "assets",
  assets: "assets",
  screen: "screen",
  view: "layout",
  layout: "layout",
  main: "main",
  rule: "rules",
  rules: "rules",
  transition: "transitions",
  transitions: "transitions",
  level: "levels",
  levels: "levels",
  on_display: "on_display",
  level_start: "on_level_start",
  on_level_start: "on_level_start",
  level_clear: "on_level_clear",
  on_level_clear: "on_level_clear",
  scene_start: "on_scene_start",
  on_scene_start: "on_scene_start",
  state: "state",
  keys: "keys",
  resources: "resources",
  row: "row",
  column: "column",
  box: "box",
  level_menu: "level_menu",
});
const WASM_SECTION_BOUNDARY_BLOCKS = new Set([
  "map",
  "on_level_start",
  "on_level_clear",
  "on_display",
  "scratch",
  "group",
  "layers",
  "collision_layers",
  "legend",
  "sprites",
  "assets",
  "screen",
  "layout",
  "effect",
  "rules",
  "main",
  "transitions",
  "levels",
  "level",
  "resources",
  "win_conditions",
  "lose_conditions",
]);
const WASM_INLINE_BLOCKS = new Set([
  ...WASM_SECTION_BOUNDARY_BLOCKS,
  "state",
  "keys",
  "on_scene_start",
  "transition",
  "input",
  "component_effect",
  "action",
  "if",
  "row",
  "column",
  "box",
  "for",
  "level_menu",
  "fix",
  "repeat",
  "once",
  "once_all",
  "once_per_level",
  "display",
]);
let previewVirtualHeight = previewMinimumHeight;
let previewViewportAspect = previewDefaultLogicalWidth / previewDefaultLogicalHeight;
let previewVirtualWidth = Math.round(previewVirtualHeight * previewViewportAspect);
const boardVirtualCellSize = 56;
const levelEditorEdgeSize = 24;
const levelEditorGap = 6;
const SPRITE_COLOR_PRESETS = [
  "#000000", "#1d2b53", "#7e2553", "#008751",
  "#ab5236", "#5f574f", "#c2c3c7", "#fff1e8",
  "#ff004d", "#ffa300", "#ffec27", "#00e436",
  "#29adff", "#83769c", "#ff77a8", "#ffccaa",
];
const SPRITE_COLOR_TOKENS = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
const PREVIEW_THEME_PRESETS = {
  clean: {
    colorScheme: "light",
    bg: "#f5f3ef",
    ink: "#1f2428",
    muted: "#66727c",
    line: "#d7dde2",
    panelBg: "rgba(255, 255, 255, 0.94)",
    background: "var(--preview-game-bg)",
  },
  terminal: {
    colorScheme: "dark",
    bg: "#000000",
    ink: "#ffffff",
    muted: "#ffffff",
    line: "#ffffff",
    panelBg: "#000000",
    background: "var(--preview-game-bg)",
  },
  paper: {
    colorScheme: "light",
    bg: "#f4ecd9",
    ink: "#2b2419",
    muted: "#756852",
    line: "#cdbd9a",
    panelBg: "rgba(255, 250, 240, 0.96)",
    background: "linear-gradient(rgba(255, 255, 255, 0.26), rgba(255, 255, 255, 0.26)), repeating-linear-gradient(0deg, transparent 0 23px, rgba(141, 93, 42, 0.08) 23px 24px), var(--preview-game-bg)",
  },
  pixel: {
    colorScheme: "dark",
    bg: "#08080c",
    ink: "#f8f8f8",
    muted: "#d8d8d8",
    line: "#f8f8f8",
    panelBg: "#08080c",
    background: "var(--preview-game-bg)",
  },
  puzzlescript: {
    colorScheme: "dark",
    bg: "#000000",
    ink: "#ffffff",
    muted: "#ffffff",
    line: "#ffffff",
    panelBg: "#000000",
    background: "var(--preview-game-bg)",
  },
  candy: {
    colorScheme: "light",
    bg: "#fff7fb",
    ink: "#33404a",
    muted: "#7a8790",
    line: "#efbfd3",
    panelBg: "rgba(255, 255, 255, 0.96)",
    background: "repeating-linear-gradient(135deg, rgba(215, 111, 151, 0.045) 0 14px, transparent 14px 28px), var(--preview-game-bg)",
  },
  blueprint: {
    colorScheme: "dark",
    bg: "#0d334e",
    ink: "#e9f8ff",
    muted: "#aad0e0",
    line: "#78c7e8",
    panelBg: "rgba(11, 42, 64, 0.94)",
    background: "repeating-linear-gradient(0deg, rgba(120, 199, 232, 0.11) 0 1px, transparent 1px 24px), repeating-linear-gradient(90deg, rgba(120, 199, 232, 0.11) 0 1px, transparent 1px 24px), var(--preview-game-bg)",
  },
  noir: {
    colorScheme: "dark",
    bg: "#101010",
    ink: "#f4f1e8",
    muted: "#a9a097",
    line: "#59544e",
    panelBg: "rgba(24, 24, 24, 0.96)",
    background: "linear-gradient(90deg, rgba(242, 193, 78, 0.055), transparent 38%, transparent 62%, rgba(242, 193, 78, 0.035)), var(--preview-game-bg)",
  },
};

function spriteEditorScaleFactor(scaleInput, maxSize) {
  const factor = Math.trunc(Number(scaleInput?.value) || 2);
  return Math.max(2, Math.min(maxSize, factor));
}

function renderSpriteScaleControl({
  size,
  maxSize,
  scaleInput,
  scaleUpButton,
  scaleDownButton,
  canScaleDown,
  noun,
}) {
  if (!scaleInput || !scaleUpButton || !scaleDownButton) {
    return;
  }
  const maxScale = Math.floor(maxSize / size);
  const factor = spriteEditorScaleFactor(scaleInput, maxSize);
  scaleInput.max = String(Math.max(2, size, maxScale));
  scaleInput.disabled = false;
  scaleUpButton.disabled = maxScale < 2 || factor > maxScale;
  scaleUpButton.title = maxScale < 2
    ? "Max size"
    : `Scale up by ${factor}x`;
  scaleDownButton.disabled = !canScaleDown(factor);
  scaleDownButton.title = scaleDownButton.disabled
    ? "Not divisible"
    : `Scale down by ${factor}x`;
}

const editorHoverTooltipSelector = "button";
let editorHoverTooltip = null;
let editorHoverTooltipTarget = null;

function editorTooltipTargetFromEventTarget(target) {
  const element = target instanceof Element ? target.closest(editorHoverTooltipSelector) : null;
  if (!element || element.classList.contains("sprite-cell") || !element.querySelector("svg")) {
    return null;
  }
  return element;
}

function compactEditorTooltipText(text) {
  const cleaned = String(text || "")
    .replace(/\s*\([^)]*\)\s*/g, " ")
    .replace(/\s+/g, " ")
    .trim();
  const exact = new Map([
    ["Add converted .puzzle file", "Add file"],
    ["Bucket active", "Bucket fill"],
    ["Cell and stage frames", "Frames"],
    ["Copy converted .puzzle", "Copy"],
    ["Copy music sound line", "Copy music"],
    ["Copy SFX sound line", "Copy SFX"],
    ["Copy solution", "Copy"],
    ["Discard new color", "Discard"],
    ["Generate .puzzle translation", "Generate"],
    ["Hide current tool pane", "Hide pane"],
    ["Hide file pane", "Hide files"],
    ["Maximize Preview pane", "Maximize"],
    ["Maximize Source pane", "Maximize"],
    ["More file actions", "File actions"],
    ["Paint empty voxel", "Eraser"],
    ["Paint transparent", "Eraser"],
    ["Pick color from screen", "Pick color"],
    ["Pick selected color", "Pick color"],
    ["Remove selected color", "Remove color"],
    ["Reset 3D preview camera", "Reset camera"],
    ["Reset solution preview", "Reset"],
    ["Screen color picker is not available in this browser", "Unavailable"],
    ["Show file pane", "Show files"],
    ["Show game pane", "Show game"],
    ["Tag selected color", "Tag color"],
    ["Tag shape by name", "Tag shape"],
    ["Toggle grid", "Grid"],
    ["Unlink color tag", "Unlink tag"],
    ["Unlink shape tag", "Unlink tag"],
    ["Update matching music in source", "Update music"],
    ["Update matching SFX in source", "Update SFX"],
  ]);
  if (exact.has(cleaned)) {
    return exact.get(cleaned);
  }
  const switchMatch = cleaned.match(/^Switch to (light|dark) mode$/i);
  if (switchMatch) {
    return `${switchMatch[1][0].toUpperCase()}${switchMatch[1].slice(1)} mode`;
  }
  const refreshMatch = cleaned.match(/^Refresh\b/i);
  if (refreshMatch) {
    return "Refresh";
  }
  const hideShowPaneMatch = cleaned.match(/^(Hide|Show) (.+?) pane$/i);
  if (hideShowPaneMatch) {
    return `${hideShowPaneMatch[1]} ${hideShowPaneMatch[2]}`;
  }
  const toggleMatch = cleaned.match(/^Toggle (.+)$/i);
  if (toggleMatch) {
    return toggleMatch[1]
      .replace(/^(level|top-down|occupied cell and stage)\s+/i, "")
      .replace(/\s+in the 3D preview$/i, "")
      .replace(/\bexpansion\b/i, "expand")
      .replace(/\bshrinking\b/i, "shrink")
      .trim();
  }
  const moveSliceMatch = cleaned.match(/^Move to (lower|higher)(?: 3D level)? slice$/i);
  if (moveSliceMatch) {
    return `${moveSliceMatch[1][0].toUpperCase()}${moveSliceMatch[1].slice(1)} slice`;
  }
  const colorTagMatch = cleaned.match(/^(Color|Shape) tag:/i);
  if (colorTagMatch) {
    return `${colorTagMatch[1]} tag`;
  }
  const unlinkTagMatch = cleaned.match(/^Unlink (color|shape) tag\b/i);
  if (unlinkTagMatch) {
    return "Unlink tag";
  }
  return cleaned;
}

function editorTooltipText(element) {
  return compactEditorTooltipText(
    element?.dataset?.tooltip
      || element?.getAttribute("title")
      || element?.dataset?.hoverTitle
      || element?.getAttribute("aria-label")
      || "",
  );
}

function ensureEditorHoverTooltip() {
  if (!editorHoverTooltip) {
    editorHoverTooltip = document.createElement("div");
    editorHoverTooltip.className = "editor-hover-tooltip";
    editorHoverTooltip.hidden = true;
    document.body.append(editorHoverTooltip);
  }
  return editorHoverTooltip;
}

function positionEditorHoverTooltip() {
  if (!editorHoverTooltipTarget || !editorHoverTooltip || editorHoverTooltip.hidden) {
    return;
  }
  const margin = 8;
  const gap = 6;
  const targetRect = editorHoverTooltipTarget.getBoundingClientRect();
  const tooltipRect = editorHoverTooltip.getBoundingClientRect();
  const maxLeft = Math.max(margin, window.innerWidth - tooltipRect.width - margin);
  const left = Math.min(maxLeft, Math.max(margin, targetRect.left + (targetRect.width - tooltipRect.width) / 2));
  const topAbove = targetRect.top - tooltipRect.height - gap;
  const placeBelow = topAbove < margin;
  const top = placeBelow
    ? Math.min(window.innerHeight - tooltipRect.height - margin, targetRect.bottom + gap)
    : topAbove;
  editorHoverTooltip.dataset.placement = placeBelow ? "below" : "above";
  editorHoverTooltip.style.left = `${Math.round(left)}px`;
  editorHoverTooltip.style.top = `${Math.round(Math.max(margin, top))}px`;
}

function showEditorHoverTooltip(element) {
  const text = editorTooltipText(element);
  if (!text) {
    hideEditorHoverTooltip(element);
    return;
  }
  editorHoverTooltipTarget = element;
  if (element.hasAttribute("title")) {
    element.dataset.hoverTitle = element.getAttribute("title") || "";
    element.removeAttribute("title");
  }
  const tooltip = ensureEditorHoverTooltip();
  tooltip.textContent = text;
  tooltip.hidden = false;
  positionEditorHoverTooltip();
}

function hideEditorHoverTooltip(element = editorHoverTooltipTarget) {
  if (element?.dataset?.hoverTitle !== undefined) {
    if (!element.hasAttribute("title")) {
      element.setAttribute("title", element.dataset.hoverTitle);
    }
    delete element.dataset.hoverTitle;
  }
  if (editorHoverTooltip) {
    editorHoverTooltip.hidden = true;
    editorHoverTooltip.textContent = "";
  }
  if (!element || element === editorHoverTooltipTarget) {
    editorHoverTooltipTarget = null;
  }
}

function installEditorHoverTooltips() {
  document.addEventListener("pointerover", (event) => {
    const target = editorTooltipTargetFromEventTarget(event.target);
    if (!target || target === editorHoverTooltipTarget) {
      return;
    }
    showEditorHoverTooltip(target);
  });
  document.addEventListener("pointerout", (event) => {
    const target = editorTooltipTargetFromEventTarget(event.target);
    if (!target || target !== editorHoverTooltipTarget || target.contains(event.relatedTarget)) {
      return;
    }
    hideEditorHoverTooltip(target);
  });
  document.addEventListener("focusin", (event) => {
    const target = editorTooltipTargetFromEventTarget(event.target);
    if (target) {
      showEditorHoverTooltip(target);
    }
  });
  document.addEventListener("focusout", (event) => {
    const target = editorTooltipTargetFromEventTarget(event.target);
    if (target && target === editorHoverTooltipTarget) {
      hideEditorHoverTooltip(target);
    }
  });
  window.addEventListener("scroll", positionEditorHoverTooltip, true);
  window.addEventListener("resize", positionEditorHoverTooltip);
}

let latestHtml = "";
let previewExport = null;
let previewTimer = 0;
let previewFrameObjectUrl = "";
let previewFrameLoadId = 0;
let previewViewportSyncFrame = 0;
let previewViewportSyncPasses = 0;
let currentPreviewTheme = null;
let previewDocumentLoaded = false;
let previewFrameHasEditorLevelState = false;
let boardScaleSyncFrame = 0;
let boardScaleSyncPasses = 0;
const paneStatusClearTimers = new Map();
let editorStatusClearTimer = 0;
let activePreviewRequest = null;
let wasmCompiler = null;
let wasmCompilerPromise = null;
let wasmCoreRuntimePromise = null;
let previewLogEntries = [];
let latestPreviewState = null;
let latestPreviewRuntimeStatus = null;
let pendingPreviewKeyStateSync = 0;
let previewPaneSourceKey = "";
let activeLevelIndex = 0;
let solverLevelIndex = 0;
let activeLevelSolveRequest = null;
let levelSolutionPreview = null;
let solverObservationPreview = null;
let solverStateOverride = null;
let solverSceneOverride = null;
let solverPuzzle3dSnapshotOverride = null;
let stagedSolverCells = null;
let levelSolveSummaryText = "";
let levelSolutionTimer = 0;
let levelSolveFlashTimer = 0;
let levelSolveFlashRestore = null;
let levelSolveFeedbackTimer = 0;
let levelSolveStartedAt = 0;
let currentPreviewMode = "play";
let currentEditorDimension = "2d";
let currentLevelPaneMode = "edit";
let currentSpritePaneMode = "sprite";
let levelPaintDrag = null;
let levelBucketActive = false;
let levelResizeMode = null;
let levelGridVisible = false;
let levelPlaytestActive = false;
let levelPlaytestStateData = null;
let levelPlaytestTransitionBusy = false;
let levelPlaytestRuntime = null;
let levelPlaytestRuntimeSourceKey = "";
let levelPlaytestRuntimeStateData = null;
let spritePaintDrag = null;
let sprite3dPaintDrag = null;
let level = {
  width: 9,
  height: 5,
  selectedObjectId: 0,
  activeLayer: 0,
  hiddenLayers: [],
  paletteCollapsed: false,
  palette: [],
  regions: [],
  cells: [],
};
let levelDisplayCells = null;
let sprite = {
  size: 5,
  editDocumentId: null,
  editSourceStart: null,
  editSourceEnd: null,
  editSourceBodyStart: null,
  editSourceBodyEnd: null,
  editSourceName: "",
  selectedColorIndex: 0,
  addPaletteOpen: false,
  editPaletteOpen: false,
  customColorOpen: false,
  addDraftColorIndex: null,
  paletteBind: null,
  shapeBind: null,
  solidSource: false,
  cells: [],
  palette: [
    { color: "#ff004d" },
  ],
};
let sprite3d = {
  size: 5,
  axis: "z",
  slice: 0,
  editScope: "slice",
  selectedColorIndex: 0,
  addPaletteOpen: false,
  editPaletteOpen: false,
  customColorOpen: false,
  addDraftColorIndex: null,
  palette: [
    { color: "#ff004d" },
  ],
  sliceClipboard: null,
  hoverSlice: null,
  camera: {
    yawDegrees: 340,
    pitchDegrees: 28,
    zoom: 1,
  },
  cells: [],
};
let sounds = {
  mode: "sfx",
  context: null,
  sfxPlayer: null,
  musicPlayer: null,
  musicPlaying: false,
  musicProgress: 0,
  musicRestartTimer: 0,
  progressFrame: 0,
  initialized: false,
};
const visualEditHistoryLimit = 200;
const visualEditHistories = {
  level: { undo: [], redo: [] },
  level3d: { undo: [], redo: [] },
  sprite: { undo: [], redo: [] },
  sprite3d: { undo: [], redo: [] },
};

function cloneVisualEditValue(value) {
  return JSON.parse(JSON.stringify(value));
}

function visualEditDocumentForKind(kind) {
  if (kind === "level3d" && typeof level3dSourceDocument === "function") {
    return level3dSourceDocument();
  }
  if (kind === "sprite" && typeof activeSpriteEditDocument === "function") {
    return activeSpriteEditDocument();
  }
  if (kind === "sprite3d" && typeof activeSprite3dEditDocument === "function") {
    return activeSprite3dEditDocument();
  }
  if (kind === "level" && typeof activePreviewDocument === "function") {
    return activePreviewDocument();
  }
  return null;
}

function visualEditSnapshot(kind) {
  const editDocument = visualEditDocumentForKind(kind);
  const tracksSource = kind === "level3d" || kind === "sprite" || kind === "sprite3d";
  const base = {
    kind,
    documentId: tracksSource ? editDocument?.id || "" : "",
    source: tracksSource && editDocument && isTextDocument(editDocument) ? editDocument.source || "" : "",
  };
  if (kind === "level") {
    return {
      ...base,
      state: {
        width: level.width,
        height: level.height,
        activeLayer: level.activeLayer,
        regions: cloneVisualEditValue(level.regions || []),
        cells: cloneVisualEditValue(level.cells || []),
      },
    };
  }
  if (kind === "level3d") {
    return {
      ...base,
      state: {
        width: level3d.width,
        depth: level3d.depth,
        height: level3d.height,
        slice: level3d.slice,
        slices: cloneVisualEditValue(level3d.slices || []),
        sourceDocumentId: level3d.sourceDocumentId || "",
        sourceKey: level3d.sourceKey || "",
      },
    };
  }
  if (kind === "sprite") {
    return {
      ...base,
      state: {
        size: sprite.size,
        palette: cloneVisualEditValue(sprite.palette || []),
        cells: cloneVisualEditValue(sprite.cells || []),
        paletteBind: cloneVisualEditValue(sprite.paletteBind || null),
        shapeBind: cloneVisualEditValue(sprite.shapeBind || null),
        solidSource: Boolean(sprite.solidSource),
      },
    };
  }
  if (kind === "sprite3d") {
    return {
      ...base,
      state: {
        size: sprite3d.size,
        axis: sprite3d.axis,
        slice: sprite3d.slice,
        editScope: sprite3d.editScope,
        palette: cloneVisualEditValue(sprite3d.palette || []),
        cells: cloneVisualEditValue(sprite3d.cells || []),
        sliceClipboard: cloneVisualEditValue(sprite3d.sliceClipboard || null),
        hoverSlice: sprite3d.hoverSlice,
      },
    };
  }
  return base;
}

function sameVisualEditSnapshot(left, right) {
  return JSON.stringify(left?.state || null) === JSON.stringify(right?.state || null)
    && (left?.source || "") === (right?.source || "");
}

function pushVisualEditUndoSnapshot(kind, beforeSnapshot, afterSnapshot = visualEditSnapshot(kind)) {
  if (!beforeSnapshot || sameVisualEditSnapshot(beforeSnapshot, afterSnapshot)) {
    return false;
  }
  const history = visualEditHistories[kind];
  if (!history) {
    return false;
  }
  history.undo.push(beforeSnapshot);
  if (history.undo.length > visualEditHistoryLimit) {
    history.undo.shift();
  }
  history.redo = [];
  return true;
}

function withVisualEditHistory(kind, mutate) {
  const before = visualEditSnapshot(kind);
  const result = mutate();
  const after = visualEditSnapshot(kind);
  if (result !== false) {
    pushVisualEditUndoSnapshot(kind, before, after);
  }
  return result;
}

function restoreVisualEditDocument(snapshot) {
  if (!snapshot?.documentId) {
    return;
  }
  const editDocument = documents.find((candidate) => candidate.id === snapshot.documentId);
  if (!editDocument || !isTextDocument(editDocument)) {
    return;
  }
  editDocument.source = snapshot.source || "";
  if (editDocument.id === activeDocument()?.id) {
    setSourceEditorValue(editDocument.source, { resetUndo: false });
  }
}

function restoreVisualEditSnapshot(snapshot) {
  if (!snapshot?.kind) {
    return false;
  }
  restoreVisualEditDocument(snapshot);
  const state = snapshot.state || {};
  if (snapshot.kind === "level") {
    level.width = Math.max(1, Math.trunc(Number(state.width) || 1));
    level.height = Math.max(1, Math.trunc(Number(state.height) || 1));
    level.activeLayer = Math.max(0, Math.trunc(Number(state.activeLayer) || 0));
    level.regions = cloneVisualEditValue(state.regions || []);
    level.cells = cloneVisualEditValue(state.cells || []);
    clearSolutionPreview();
    levelDisplayCells = null;
    renderLevelBoard();
    renderLevelSourcePreview();
    syncPreviewStateFromLevel();
  } else if (snapshot.kind === "level3d") {
    level3d.width = Math.max(1, Math.trunc(Number(state.width) || 1));
    level3d.depth = Math.max(1, Math.trunc(Number(state.depth) || 1));
    level3d.height = Math.max(1, Math.trunc(Number(state.height) || 1));
    level3d.slice = Math.max(0, Math.min(level3d.height - 1, Math.trunc(Number(state.slice) || 0)));
    level3d.slices = cloneVisualEditValue(state.slices || []);
    level3d.sourceDocumentId = state.sourceDocumentId || level3d.sourceDocumentId || "";
    level3d.sourceKey = state.sourceKey || "";
    level3dStageHit = null;
    renderLevel3dBuilder();
    sendLevel3dSnapshotToRuntime();
    sendLevel3dLayerSnapshotToRuntime();
  } else if (snapshot.kind === "sprite") {
    sprite.size = clampSpriteSize(state.size);
    sprite.palette = cloneVisualEditValue(state.palette || [{ color: "#ff004d" }]);
    sprite.cells = cloneVisualEditValue(state.cells || []);
    sprite.paletteBind = cloneVisualEditValue(state.paletteBind || null);
    sprite.shapeBind = cloneVisualEditValue(state.shapeBind || null);
    sprite.solidSource = Boolean(state.solidSource);
    sprite.addPaletteOpen = false;
    sprite.editPaletteOpen = false;
    sprite.customColorOpen = false;
    sprite.addDraftColorIndex = null;
    renderSpriteBuilder();
  } else if (snapshot.kind === "sprite3d") {
    sprite3d.size = clampSprite3dSize(state.size);
    sprite3d.axis = ["x", "y", "z"].includes(state.axis) ? state.axis : "z";
    sprite3d.slice = Math.max(0, Math.min(sprite3d.size - 1, Math.trunc(Number(state.slice) || 0)));
    sprite3d.editScope = state.editScope === "all" ? "all" : "slice";
    sprite3d.palette = cloneVisualEditValue(state.palette || [{ color: "#ff004d" }]);
    sprite3d.cells = cloneVisualEditValue(state.cells || []);
    sprite3d.sliceClipboard = cloneVisualEditValue(state.sliceClipboard || null);
    sprite3d.hoverSlice = Number.isInteger(state.hoverSlice) ? state.hoverSlice : null;
    sprite3d.addPaletteOpen = false;
    sprite3d.editPaletteOpen = false;
    sprite3d.customColorOpen = false;
    sprite3d.addDraftColorIndex = null;
    renderSprite3dBuilder();
  } else {
    return false;
  }
  scheduleLocalSave();
  return true;
}

function currentVisualEditKind() {
  if (currentPreviewMode === "edit") {
    return "level";
  }
  if (currentPreviewMode === "level3d") {
    return "level3d";
  }
  if (currentPreviewMode === "sprite") {
    return "sprite";
  }
  if (currentPreviewMode === "sprite3d") {
    return "sprite3d";
  }
  return "";
}

function undoVisualEdit(kind = currentVisualEditKind()) {
  if ((kind === "sprite" || kind === "sprite3d") && typeof commitSpriteColorEditHistory === "function") {
    commitSpriteColorEditHistory(kind);
  }
  const history = visualEditHistories[kind];
  const snapshot = history?.undo.pop();
  if (!snapshot) {
    return false;
  }
  history.redo.push(visualEditSnapshot(kind));
  restoreVisualEditSnapshot(snapshot);
  return true;
}

function redoVisualEdit(kind = currentVisualEditKind()) {
  const history = visualEditHistories[kind];
  const snapshot = history?.redo.pop();
  if (!snapshot) {
    return false;
  }
  history.undo.push(visualEditSnapshot(kind));
  restoreVisualEditSnapshot(snapshot);
  return true;
}

function isTextEntryTarget(target) {
  const tagName = target?.tagName || "";
  if (target?.closest?.(".sprite-code-glyph")) {
    return false;
  }
  return target?.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(tagName);
}

function handleVisualEditUndoShortcut(event) {
  if (event.altKey || isTextEntryTarget(event.target)) {
    return false;
  }
  const key = event.key.length === 1 ? event.key.toLowerCase() : event.key;
  const modifier = (event.metaKey && !event.ctrlKey) || (event.ctrlKey && !event.metaKey);
  if (!modifier) {
    return false;
  }
  const redo = (key === "z" && event.shiftKey) || (!event.metaKey && key === "y");
  if (!redo && (event.shiftKey || key !== "z")) {
    return false;
  }
  const kind = currentVisualEditKind();
  if (!kind) {
    return false;
  }
  const handled = redo ? redoVisualEdit(kind) : undoVisualEdit(kind);
  if (!handled) {
    return false;
  }
  event.preventDefault();
  event.stopPropagation();
  return true;
}

initializeEditorTheme();
configureFolderImport();
configureDesktopHost();

function initializeEditorTheme() {
  const theme = normalizeTheme(document.documentElement.dataset.theme);
  applyEditorTheme(theme);
}

function normalizeTheme(theme) {
  return theme === "light" ? "light" : "dark";
}

function applyEditorTheme(theme) {
  const normalized = normalizeTheme(theme);
  document.documentElement.dataset.theme = normalized;
  if (!themeToggleButton) {
    return;
  }
  const dark = normalized === "dark";
  themeToggleButton.setAttribute("aria-pressed", dark ? "true" : "false");
  themeToggleButton.setAttribute("aria-label", dark ? "Switch to light mode" : "Switch to dark mode");
  themeToggleButton.title = dark ? "Switch to light mode" : "Switch to dark mode";
  if (!previewDocumentLoaded) {
    applyUnloadedPreviewTheme();
  }
  if (typeof renderSprite3dPreview === "function") {
    window.requestAnimationFrame(renderSprite3dPreview);
  }
}

function setEditorTheme(theme) {
  const normalized = normalizeTheme(theme);
  try {
    window.localStorage.setItem(themeStoreKey, normalized);
  } catch {
    // Theme persistence is optional; private browsing can reject localStorage.
  }
  applyEditorTheme(normalized);
}

function toggleEditorTheme() {
  setEditorTheme(normalizeTheme(document.documentElement.dataset.theme) === "dark" ? "light" : "dark");
}

async function requestText(url, options = {}) {
  const response = await fetch(url, options);
  const contentType = response.headers.get("content-type") || "";
  if (!response.ok) {
    let message = response.statusText;
    if (contentType.includes("application/json")) {
      const body = await response.json();
      message = body.error || response.statusText;
    } else {
      message = await response.text();
    }
    const error = new Error(message);
    error.status = response.status;
    throw error;
  }
  return response.text();
}

async function requestJson(url) {
  const response = await fetch(url);
  const body = await response.json();
  if (!response.ok) {
    throw new Error(body.error || response.statusText);
  }
  return body;
}

function applyGameCss(css) {
  let style = document.querySelector("#gameStyle");
  if (!style) {
    style = document.createElement("style");
    style.id = "gameStyle";
    const link = document.querySelector("#gameStyleLink");
    if (link) {
      link.replaceWith(style);
    } else {
      document.head.append(style);
    }
  }
  style.textContent = scopeGameCss(css || "");
}

function scopeGameCss(css, scope = ".game-preview-scope") {
  return scopeCssBlock(String(css || ""), scope);
}

function scopeCssBlock(css, scope) {
  let output = "";
  let index = 0;
  while (index < css.length) {
    const open = css.indexOf("{", index);
    if (open < 0) {
      output += css.slice(index);
      break;
    }
    const selector = css.slice(index, open).trim();
    const close = matchingCssBrace(css, open);
    if (close < 0) {
      output += css.slice(index);
      break;
    }
    const body = css.slice(open + 1, close);
    if (selector.startsWith("@media") || selector.startsWith("@supports") || selector.startsWith("@container")) {
      output += `${selector}{${scopeCssBlock(body, scope)}}`;
    } else if (selector.startsWith("@")) {
      output += `${selector}{${body}}`;
    } else {
      output += `${scopeSelectorList(selector, scope)}{${body}}`;
    }
    index = close + 1;
  }
  return output;
}

function matchingCssBrace(css, openIndex) {
  let depth = 0;
  let quote = "";
  for (let index = openIndex; index < css.length; index += 1) {
    const char = css[index];
    const previous = css[index - 1];
    if (quote) {
      if (char === quote && previous !== "\\") {
        quote = "";
      }
      continue;
    }
    if (char === '"' || char === "'") {
      quote = char;
      continue;
    }
    if (char === "{") {
      depth += 1;
    } else if (char === "}") {
      depth -= 1;
      if (depth === 0) {
        return index;
      }
    }
  }
  return -1;
}

function scopeSelectorList(selector, scope) {
  return splitCssSelectors(selector)
    .map((part) => scopeSelector(part, scope))
    .join(", ");
}

function splitCssSelectors(selector) {
  const parts = [];
  let start = 0;
  let depth = 0;
  let quote = "";
  for (let index = 0; index < selector.length; index += 1) {
    const char = selector[index];
    const previous = selector[index - 1];
    if (quote) {
      if (char === quote && previous !== "\\") {
        quote = "";
      }
      continue;
    }
    if (char === '"' || char === "'") {
      quote = char;
      continue;
    }
    if (char === "(" || char === "[") {
      depth += 1;
    } else if (char === ")" || char === "]") {
      depth = Math.max(0, depth - 1);
    } else if (char === "," && depth === 0) {
      parts.push(selector.slice(start, index).trim());
      start = index + 1;
    }
  }
  parts.push(selector.slice(start).trim());
  return parts.filter(Boolean);
}

function scopeSelector(selector, scope) {
  if (selector === ":root" || selector === "html" || selector === "body") {
    return scope;
  }
  if (selector.startsWith(":root ")) {
    return `${scope}${selector.slice(5)}`;
  }
  if (selector.startsWith("html ") || selector.startsWith("body ")) {
    return `${scope} ${selector.slice(5)}`;
  }
  const descendant = `${scope} ${selector}`;
  if (/^[.#[:]/.test(selector)) {
    return `${scope}${selector}, ${descendant}`;
  }
  return descendant;
}

function applyPreviewTheme(theme) {
  const root = playPreview;
  if (!root) {
    return;
  }
  const resolved = resolvePreviewTheme(theme);
  currentPreviewTheme = resolved;
  for (const target of previewThemeRoots(root)) {
    setPreviewThemeProperties(target, resolved);
  }
}

function setPreviewDocumentLoaded(loaded) {
  previewDocumentLoaded = Boolean(loaded);
  playPreview?.classList.toggle("is-preview-unloaded", !previewDocumentLoaded);
  if (!previewDocumentLoaded) {
    applyUnloadedPreviewTheme();
  }
}

function terminatePreviewGame() {
  if (activePreviewRequest) {
    activePreviewRequest.abort();
    activePreviewRequest = null;
  }
  previewFrameHasEditorLevelState = false;
  latestPreviewState = null;
  latestPreviewRuntimeStatus = null;
  pendingPreviewKeyStateSync = 0;
  setPreviewDocumentLoaded(false);
  setPreviewFrameHtml(emptyPreviewDocument());
}

function applyUnloadedPreviewTheme() {
  const root = playPreview;
  if (!root) {
    return;
  }
  currentPreviewTheme = editorPreviewTheme();
  for (const target of previewThemeRoots(root)) {
    setPreviewThemeProperties(target, currentPreviewTheme);
  }
}

function previewThemeRoots(root) {
  const roots = new Set([root]);
  const pane = root.closest(".preview-pane");
  if (pane) {
    roots.add(pane);
  }
  document.querySelectorAll(".preview-pane").forEach((previewPane) => roots.add(previewPane));
  return [...roots];
}

function setPreviewThemeProperties(root, theme) {
  root.style.setProperty("--preview-game-bg", theme.bg);
  root.style.setProperty("--preview-game-ink", theme.ink);
  root.style.setProperty("--preview-game-muted", theme.muted);
  root.style.setProperty("--preview-game-line", theme.line);
  root.style.setProperty("--preview-game-panel-bg", theme.panelBg);
  root.style.setProperty("--preview-game-background", theme.background);
  root.style.colorScheme = theme.colorScheme;
}

function editorPreviewTheme() {
  const light = normalizeTheme(document.documentElement.dataset.theme) === "light";
  return {
    colorScheme: light ? "light" : "dark",
    bg: editorCssVariable("--workspace-bg", light ? "#edf2f6" : "#1e1e1e"),
    ink: editorCssVariable("--ink", light ? "#20272e" : "#d4d4d4"),
    muted: editorCssVariable("--muted", light ? "#65727d" : "#9da3aa"),
    line: editorCssVariable("--line", light ? "#d6dde3" : "#3c3c3c"),
    danger: editorCssVariable("--danger", light ? "#b32634" : "#b43b43"),
    panelBg: editorCssVariable("--side-bg", light ? "#f8fafc" : "#181818"),
    background: editorCssVariable("--workspace-bg", light ? "#edf2f6" : "#1e1e1e"),
  };
}

function editorCssVariable(name, fallback) {
  return window.getComputedStyle(document.documentElement).getPropertyValue(name).trim() || fallback;
}

function resolvePreviewTheme(theme) {
  const name = previewThemePresetName(theme?.name);
  const preset = PREVIEW_THEME_PRESETS[name] || PREVIEW_THEME_PRESETS.clean;
  const resolved = { ...preset };
  for (const [rawName, rawValue] of Object.entries(theme?.variables || {})) {
    const name = previewThemeVariableName(rawName);
    const value = safePreviewCssValue(rawValue);
    if (!value) {
      continue;
    }
    if (name === "bg") {
      resolved.bg = value;
      resolved.background = "var(--preview-game-bg)";
    } else if (name === "ink") {
      resolved.ink = value;
    } else if (name === "muted") {
      resolved.muted = value;
    } else if (name === "line") {
      resolved.line = value;
    } else if (name === "panel-bg") {
      resolved.panelBg = value;
    }
  }
  return resolved;
}

function previewThemePresetName(name) {
  const normalized = String(name || "clean")
    .replace(/[^a-zA-Z0-9_-]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .toLowerCase();
  return normalized || "clean";
}

function previewThemeVariableName(name) {
  const normalized = String(name || "")
    .replace(/^--/, "")
    .replace(/_/g, "-")
    .toLowerCase();
  return /^[a-z0-9-]*[a-z][a-z0-9-]*$/.test(normalized) ? normalized : "";
}

function safePreviewCssValue(value) {
  const text = String(value || "").trim();
  return /^[a-zA-Z0-9#.,%()+_/: -]+$/.test(text) ? text : "";
}

function ensureGameVisualsRuntime() {
  if (!window.PuzzleSpriteRegistry) {
    window.PuzzleSpriteRegistry = {
      create(config = {}) {
        return {
          aliases: { ...(config.aliases || {}) },
          sprites: { ...(config.sprites || {}) },
          boardClass: config.boardClass || "",
          themeClass: config.themeClass || "",
          editorPuzzle: { ...(config.editorPuzzle || {}) },
          autoAdvanceDelayMs: config.autoAdvanceDelayMs,
        };
      },
    };
  }

  if (window.PuzzleStudio?.registerAssetScript && window.PuzzleStudio?.disposeAssetScripts) {
    return;
  }

  const assetScripts = [];
  const renderCallbacks = [];
  const disposers = [];

  function ensureVisuals() {
    if (!window.GameVisuals) {
      window.GameVisuals = window.PuzzleSpriteRegistry.create();
    }
    return window.GameVisuals;
  }

  function apiFor(definition = {}) {
    return {
      name: definition.name || "",
      onRender(callback) {
        if (typeof callback === "function") {
          renderCallbacks.push(callback);
        }
      },
      setBoardClass(name) {
        ensureVisuals().boardClass = String(name || "");
      },
      setThemeClass(name) {
        ensureVisuals().themeClass = String(name || "");
      },
      addDisposer(callback) {
        if (typeof callback === "function") {
          disposers.push(callback);
        }
      },
      assetUrl(path) {
        return window.PuzzleAssets?.url ? window.PuzzleAssets.url(path) : String(path || "");
      },
    };
  }

  window.PuzzleStudio = {
    registerAssetScript(definition = {}) {
      assetScripts.push(definition);
      if (typeof definition.setup === "function") {
        definition.setup(apiFor(definition));
      }
    },
    dispatchRender(payload = {}) {
      if (!renderCallbacks.length) {
        return;
      }
      window.requestAnimationFrame(() => {
        const event = {
          ...payload,
          board: payload.board || document.querySelector("#board"),
          screenView: payload.screenView || document.querySelector("#screenView"),
          scene: payload.scene || window.__PuzzleCurrentScene,
          state: window.__PuzzleCurrentState,
          assetUrl: (path) => (window.PuzzleAssets?.url ? window.PuzzleAssets.url(path) : String(path || "")),
        };
        for (const callback of renderCallbacks) {
          callback(event);
        }
      });
    },
    disposeAssetScripts() {
      while (disposers.length) {
        const dispose = disposers.pop();
        dispose();
      }
      renderCallbacks.length = 0;
      assetScripts.length = 0;
    },
  };
}

function applyGameVisuals(script) {
  ensureGameVisualsRuntime();
  window.PuzzleStudio.disposeAssetScripts();
  window.GameVisuals = window.PuzzleSpriteRegistry.create();
  if (!script) {
    return;
  }
  try {
    Function(script)();
  } catch (error) {
    window.PuzzleStudio.disposeAssetScripts();
    window.GameVisuals = window.PuzzleSpriteRegistry.create();
    console.error(error);
  }
}

function schedulePreview() {
  window.clearTimeout(previewTimer);
  markPreviewDirty();
}

async function renderPreview() {
  persistCurrentDocument();
  const document = activePreviewDocument();
  if (!isPuzzleDocument(document)) {
    setStatus("No game entry for preview", "is-error");
    runButton.disabled = true;
    return;
  }

  const source = document.source || "";
  updateSourceMeta();
  resetPreviewLog(`Compiling ${document.puzzlePath || "preview"}`);
  setStatus("Compiling", "");
  runButton.disabled = true;

  if (activePreviewRequest) {
    activePreviewRequest.abort();
  }

  const controller = new AbortController();
  activePreviewRequest = controller;

  try {
    const html = await window.PuzzleStudioHost.preview({
      source,
      puzzlePath: document.puzzlePath,
      workspaceRoot: document.workspaceRoot || "",
      gameCss: effectiveGameCss(document),
      gameVisualsJs: effectiveGameVisualsJs(document),
    }, { signal: controller.signal });
    applyCompiledPreviewHtml(html, document, source);
  } catch (error) {
    if (error.name === "AbortError") {
      return;
    }
    if (previewBackendUnavailable(error)) {
      try {
        appendPreviewLog("system", "Compiling in browser", { source: "compiler" });
        const html = await compilePreviewWithWasm(document, source);
        applyCompiledPreviewHtml(html, document, source);
        appendPreviewLog("system", "Preview ready", { source: "compiler" });
        return;
      } catch (wasmError) {
        if (editorSeed) {
          appendPreviewLog("warn", "Run Preview needs the editor server or generated browser assets.", { source: "compiler" });
          appendPreviewLog("error", userFacingRuntimeError(wasmError), { source: "wasm compiler" });
          setStatus("Run Preview unavailable", "is-error");
          downloadButton.disabled = !latestHtml;
          return;
        }
        error = wasmError;
      }
    }
    downloadButton.disabled = true;
    appendCompileDiagnostics(error, { source: "compiler", document, sourceText: source });
    setStatus("Compile error", "is-error");
  } finally {
    if (activePreviewRequest === controller) {
      activePreviewRequest = null;
    }
    runButton.disabled = Boolean(activePreviewRequest) || !isPuzzleDocument(activePreviewDocument());
  }
}

async function ensurePreviewExportForLevelAction(options = {}) {
  ensurePreviewTargetsActiveDocument();
  let exportData = previewExport || extractPreviewExport(latestHtml);
  if (exportData) {
    return exportData;
  }

  const document = activePreviewDocument();
  if (!isPuzzleDocument(document)) {
    const message = options.noDocumentMessage || "No game entry for preview";
    if (typeof options.status === "function") {
      options.status(message, "is-error");
    } else {
      setStatus(message, "is-error");
    }
    return null;
  }

  const compilingMessage = options.compilingMessage || "Compiling preview";
  if (typeof options.status === "function") {
    options.status(compilingMessage, "");
  } else {
    setStatus(compilingMessage, "");
  }
  await renderPreview();
  exportData = previewExport || extractPreviewExport(latestHtml);
  if (!exportData) {
    const message = options.failureMessage || "Preview compile failed";
    if (typeof options.status === "function") {
      options.status(message, "is-error");
    } else {
      setStatus(message, "is-error");
    }
    return null;
  }
  return exportData;
}

function runPreviewFromSourcePane() {
  openPreviewModePane("play", { focus: false });
  renderPreview();
}

function applyCompiledPreviewHtml(html, document, source) {
  latestHtml = html;
  const previousLevelIndex = currentEditableLevelIndex(previewExport);
  const previousSolverLevelIndex = currentSolverLevelIndex(previewExport);
  previewExport = extractPreviewExport(html);
  syncPreviewViewportAspect();
  setPreviewDocumentLoaded(true);
  applyPreviewTheme(previewExport?.theme || null);
  setActiveLevelIndex(previousLevelIndex, previewExport);
  setSolverLevelIndex(previousSolverLevelIndex, previewExport);
  clearSolverTargetOverride();
  latestPreviewState = null;
  latestPreviewRuntimeStatus = null;
  previewFrameHasEditorLevelState = false;
  setPreviewFrameHtml(editorPreviewDocument(html));
  document.source = source;
  document.previewHtml = html;
  applyGameCss(effectiveGameCss(document));
  applyGameVisuals(effectiveGameVisualsJs(document));
  resetLevelBuilderFromPreviewSource();
  syncSolverLevelSelector();
  if (!level3dBuilder.hidden) {
    renderLevel3dBuilder();
  }
  scheduleLocalSave();
  downloadButton.disabled = false;
  appendPreviewLog("system", "Preview ready", { source: "compiler" });
  setStatus("Preview ready", "is-ok");
}

async function compilePreviewWithWasm(document, source) {
  const compiler = await loadWasmCompiler();
  const expandedSource = expandPuzzleImportsForWasm(source, document.puzzlePath || "game.puzzle");
  const html = compiler.compile_preview(
    expandedSource,
    document.puzzlePath || "game.puzzle",
    effectiveGameCss(document),
    effectiveGameVisualsJs(document),
  );
  return await embedStandaloneRuntimeWasm(html);
}

async function embedStandaloneRuntimeWasm(html) {
  const sourceHtml = String(html || "");
  const runtimeAssets = window.PuzzleStudioGameWasmAssets;
  if (!runtimeAssets?.moduleUrl || !runtimeAssets?.wasmUrl) {
    return html;
  }
  const markers = [
    "window.PuzzleExport = JSON.parse(",
    "window.Puzzle3DFixture = JSON.parse(",
  ];
  const markerIndex = markers
    .map((marker) => sourceHtml.indexOf(marker))
    .filter((index) => index >= 0)
    .sort((left, right) => left - right)[0] ?? -1;
  if (markerIndex < 0) {
    return html;
  }
  const scriptEnd = sourceHtml.indexOf("</script>", markerIndex);
  if (scriptEnd < 0) {
    return html;
  }
  const bootstrap = await standaloneRuntimeWasmBootstrapScript(runtimeAssets);
  const frameAssetBootstrap = JSON.stringify(bootstrap);
  const injection = `\n${bootstrap}\nif (window.Puzzle3DFrameAssets && !window.Puzzle3DFrameAssets.embeddedWasmJs) {\n  window.Puzzle3DFrameAssets.embeddedWasmJs = ${frameAssetBootstrap};\n}\n`;
  return `${sourceHtml.slice(0, scriptEnd)}${injection}${sourceHtml.slice(scriptEnd)}`;
}

let standaloneRuntimeWasmAssetPromise = null;

async function standaloneRuntimeWasmBootstrapScript(runtimeAssets) {
  if (!runtimeAssets?.moduleUrl || !runtimeAssets?.wasmUrl) {
    return "";
  }
  const embedded = await loadStandaloneRuntimeWasmAssets(runtimeAssets);
  return `window.PuzzleStandaloneEmbeddedWasm = { moduleSource: ${JSON.stringify(embedded.moduleSource)}, wasmBase64: ${JSON.stringify(embedded.wasmBase64)} };
window.PuzzleRuntimeWasmLoader = window.PuzzleRuntimeWasmLoader || (() => {
  let modulePromise = null;
  function base64ToUint8Array(value) {
    const binary = atob(value);
    const bytes = new Uint8Array(binary.length);
    for (let index = 0; index < binary.length; index += 1) {
      bytes[index] = binary.charCodeAt(index);
    }
    return bytes;
  }
  return {
    async load() {
      if (!modulePromise) {
        const embedded = window.PuzzleStandaloneEmbeddedWasm;
        const moduleUrl = URL.createObjectURL(new Blob([embedded.moduleSource], { type: "text/javascript" }));
        modulePromise = import(\`\${moduleUrl}#runtime\`).then(async (module) => {
          await module.default({ module_or_path: base64ToUint8Array(embedded.wasmBase64) });
          return module;
        }).finally(() => URL.revokeObjectURL(moduleUrl));
      }
      return modulePromise;
    },
  };
})();`;
}

async function loadStandaloneRuntimeWasmAssets(runtimeAssets) {
  if (!standaloneRuntimeWasmAssetPromise) {
    standaloneRuntimeWasmAssetPromise = Promise.all([
      fetchRequiredRuntimeAsset(runtimeAssets.moduleUrl, "game runtime module").then((response) => response.text()),
      fetchRequiredRuntimeAsset(runtimeAssets.wasmUrl, "game runtime wasm").then((response) => response.arrayBuffer()),
    ]).then(([moduleSource, wasmBuffer]) => ({
      moduleSource,
      wasmBase64: arrayBufferToBase64(wasmBuffer),
    }));
  }
  return standaloneRuntimeWasmAssetPromise;
}

async function fetchRequiredRuntimeAsset(url, label) {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`Failed to load ${label}: ${response.status} ${response.statusText}`);
  }
  return response;
}

function arrayBufferToBase64(buffer) {
  const bytes = new Uint8Array(buffer);
  let binary = "";
  const chunkSize = 0x8000;
  for (let offset = 0; offset < bytes.length; offset += chunkSize) {
    binary += String.fromCharCode(...bytes.subarray(offset, offset + chunkSize));
  }
  return btoa(binary);
}

function expandPuzzleImportsForWasm(source, puzzlePath, importStack = []) {
  const normalizedPath = normalizePath(puzzlePath || "game.puzzle");
  if (importStack.includes(normalizedPath)) {
    throw new Error(`cyclic import: ${[...importStack, normalizedPath].join(" -> ")}`);
  }
  const nextStack = [...importStack, normalizedPath];
  const baseDir = directoryName(normalizedPath);
  const out = [];
  for (const line of expandPuzzleSectionHeadersForWasm(source).split("\n")) {
    const trimmed = line.split("//", 1)[0].trim();
    const match = trimmed.match(/^import\s+"([^"]+)"\s*$/);
    if (!match) {
      out.push(line);
      continue;
    }
    const importPath = resolveWasmImportPath(baseDir, match[1]);
    const imported = documentByPath(importPath);
    if (!imported || !isTextDocument(imported)) {
      throw new Error(`import not found: ${match[1]} from ${normalizedPath}`);
    }
    out.push(expandPuzzleImportsForWasm(imported.source || "", importPath, nextStack));
  }
  return out.join("\n");
}

function expandPuzzleSectionHeadersForWasm(source) {
  const lines = String(source || "").split("\n");
  const out = [];
  let openSection = null;
  let i = 0;
  while (i < lines.length) {
    const section = sectionHeaderAtForWasm(lines, i);
    if (section) {
      if (openSection) {
        out.push("end");
      }
      out.push(section.block);
      openSection = section;
      i += 3;
      continue;
    }

    const line = lines[i];
    const trimmed = stripLineCommentForWasm(line).trim();
    if (openSection && trimmed) {
      const normalizedLine = braceNormalizedLineForSectionForWasm(trimmed);
      if (normalizedLine === "end") {
        if (openSection.nestedDepth === 0) {
          out.push("end");
          openSection = null;
        } else {
          openSection.nestedDepth -= 1;
        }
      } else {
        const tokens = normalizedLine.split(/\s+/).filter(Boolean);
        if (openSection.nestedDepth === 0 && sectionBoundaryForWasm(openSection.block, tokens)) {
          out.push("end");
          openSection = null;
          continue;
        }
        if (startsNestedBlockForWasm(openSection.block, tokens, normalizedLine)) {
          openSection.nestedDepth += 1;
        }
      }
    }

    out.push(line);
    i += 1;
  }
  if (openSection) {
    out.push("end");
  }
  return out.join("\n");
}

function sectionHeaderAtForWasm(lines, start) {
  if (start + 2 >= lines.length) {
    return null;
  }
  const first = stripLineCommentForWasm(lines[start]).trim();
  const title = stripLineCommentForWasm(lines[start + 1]).trim();
  const last = stripLineCommentForWasm(lines[start + 2]).trim();
  if (!isSectionSeparatorForWasm(first) || !isSectionSeparatorForWasm(last)) {
    return null;
  }
  const block = sectionBlockNameForWasm(title);
  return block ? { block, nestedDepth: 0 } : null;
}

function isSectionSeparatorForWasm(line) {
  return line.length >= 3 && /^=+$/.test(line);
}

function sectionBlockNameForWasm(title) {
  const normalized = normalizeSectionTitleForWasm(title);
  if (!normalized) {
    return "";
  }
  return WASM_SECTION_BLOCK_NAMES[normalized] || "";
}

function normalizeSectionTitleForWasm(title) {
  let normalized = "";
  let previousSeparator = false;
  for (const ch of String(title || "").trim()) {
    if (/^[A-Za-z0-9]$/.test(ch)) {
      normalized += ch.toLowerCase();
      previousSeparator = false;
    } else if (/^\s$/.test(ch) || ch === "_" || ch === "-") {
      if (normalized && !previousSeparator) {
        normalized += "_";
        previousSeparator = true;
      }
    } else {
      return "";
    }
  }
  return previousSeparator ? normalized.slice(0, -1) : normalized;
}

function sectionBoundaryForWasm(block, tokens) {
  if (!tokens.length) {
    return false;
  }
  if (block === "legend") {
    return !isLegendRowForWasm(tokens);
  }
  if (["scratch", "group", "layers", "collision_layers", "win_conditions", "lose_conditions", "transitions", "levels", "sprites", "assets", "on_display"].includes(block)) {
    return startsPuzzleSectionForWasm(tokens);
  }
  return false;
}

function isLegendRowForWasm(tokens) {
  return tokens.length >= 3 && tokens[1] === "=";
}

function startsPuzzleSectionForWasm(tokens) {
  const first = tokens[0] || "";
  return WASM_SECTION_BOUNDARY_BLOCKS.has(sectionBlockNameForWasm(first) || first);
}

function startsNestedBlockForWasm(block, tokens, line) {
  if (block === "legend") {
    return false;
  }
  if (block === "levels") {
    return tokens[0] === "level" || (tokens.length === 1 && isIdentifierForWasm(tokens[0])) || startsInlineBlockForWasm(tokens, line);
  }
  return startsInlineBlockForWasm(tokens, line);
}

function startsInlineBlockForWasm(tokens, line) {
  const first = tokens[0] || "";
  const block = sectionBlockNameForWasm(first) || first;
  return WASM_INLINE_BLOCKS.has(block)
    || (tokens[0] === "menu" && (tokens.length === 2 || (tokens.length === 5 && tokens[2] === "=" && tokens[4] === "with")))
    || (tokens[0] === "button" && line.trimEnd().endsWith(" with"));
}

function braceNormalizedLineForSectionForWasm(line) {
  if (line === "}") {
    return "end";
  }
  if (line === "else {" || line === "else{") {
    return "else";
  }
  if (line.endsWith("{")) {
    return line.slice(0, -1).trimEnd();
  }
  return line;
}

function stripLineCommentForWasm(line) {
  return String(line || "").split("//", 1)[0];
}

function isIdentifierForWasm(value) {
  return /^[_A-Za-z][_A-Za-z0-9]*$/.test(value || "");
}

function resolveWasmImportPath(baseDir, importPath) {
  const normalized = normalizePath(importPath);
  if (!normalized || normalized.startsWith("/")) {
    return normalizePath(normalized.replace(/^\/+/, ""));
  }
  return normalizePathSegments(baseDir ? `${baseDir}/${normalized}` : normalized);
}

function normalizePathSegments(path) {
  const parts = [];
  for (const part of normalizePath(path).split("/")) {
    if (!part || part === ".") {
      continue;
    }
    if (part === "..") {
      parts.pop();
      continue;
    }
    parts.push(part);
  }
  return parts.join("/");
}

function documentByPath(path) {
  const target = normalizePath(path);
  const preferredRoot = activeDocument()?.workspaceRoot || workspaceRoot || "";
  return documents.find((candidate) =>
    normalizePath(candidate.puzzlePath) === target
    && (!preferredRoot || !candidate.workspaceRoot || normalizePath(candidate.workspaceRoot) === normalizePath(preferredRoot))
  ) || documents.find((candidate) => normalizePath(candidate.puzzlePath) === target) || null;
}

async function loadWasmCompiler() {
  if (!wasmCompilerPromise) {
    const version = encodeURIComponent(wasmCompilerAssetVersion);
    wasmCompilerPromise = loadExternalWasmCompiler(version).catch((error) => {
      wasmCompiler = null;
      wasmCompilerPromise = null;
      throw error;
    });
  }
  return wasmCompilerPromise;
}

async function loadExternalWasmCompiler(version) {
  const module = await import(`./wasm/puzzle_wasm.js?v=${version}`);
  await module.default({ module_or_path: `./wasm/puzzle_wasm_bg.wasm?v=${version}` });
  wasmCompiler = module;
  return module;
}

function wasmSolverWorkerConfig() {
  const version = encodeURIComponent(wasmCompilerAssetVersion);
  return {
    moduleUrl: new URL(`./wasm/puzzle_wasm.js?v=${version}`, document.baseURI).href,
    wasmUrl: new URL(`./wasm/puzzle_wasm_bg.wasm?v=${version}`, document.baseURI).href,
  };
}

async function loadWasmCoreRuntime() {
  if (!wasmCoreRuntimePromise) {
    const version = encodeURIComponent(wasmCompilerAssetVersion);
    wasmCoreRuntimePromise = import(`./wasm_core/puzzle_core_wasm.js?v=${version}`).then(async (module) => {
      await module.default({ module_or_path: `./wasm_core/puzzle_core_wasm_bg.wasm?v=${version}` });
      return module;
    }).catch((error) => {
      wasmCoreRuntimePromise = null;
      throw error;
    });
  }
  return wasmCoreRuntimePromise;
}

function createWasmSolveWorker() {
  const workerSource = `
async function loadSolverModule(wasm) {
  const module = await import(wasm.moduleUrl);
  await module.default({ module_or_path: wasm.wasmUrl });
  return module;
}

self.onmessage = async (event) => {
  const data = event.data || {};
  if (data.type !== "solve") {
    return;
  }
  const requestId = data.requestId;
  try {
    const module = await loadSolverModule(data.wasm || {});
    const solve = module.solve_state;
    if (typeof solve !== "function") {
      throw new Error("WASM solver is not available");
    }
    const args = [
      data.source,
      data.puzzlePath,
      data.stateJson,
      Number(data.maxDepth || 0),
      Number(data.maxNodes || 0),
      Number(data.maxMs || 0),
    ];
    const solutionJson = solve(...args);
    self.postMessage({
      type: "result",
      requestId,
      solution: JSON.parse(solutionJson),
    });
  } catch (error) {
    self.postMessage({
      type: "error",
      requestId,
      error: String(error?.message || error),
    });
  }
};
`;
  const url = URL.createObjectURL(new Blob([workerSource], { type: "text/javascript" }));
  try {
    const worker = new Worker(url, { type: "module" });
    worker.__puzzleStudioObjectUrl = url;
    return worker;
  } catch (error) {
    URL.revokeObjectURL(url);
    throw error;
  }
}

function disposeWasmSolveWorker(worker) {
  if (!worker) {
    return;
  }
  worker.terminate();
  if (worker.__puzzleStudioObjectUrl) {
    URL.revokeObjectURL(worker.__puzzleStudioObjectUrl);
    worker.__puzzleStudioObjectUrl = "";
  }
}

function userFacingWorkerError(error) {
  const message = error?.message || error?.error?.message || "";
  if (message) {
    return userFacingRuntimeError(message);
  }
  if (error instanceof Event) {
    return "solver worker failed to load";
  }
  return userFacingRuntimeError(error);
}

function previewBackendUnavailable(error) {
  if (error instanceof TypeError) {
    return true;
  }
  return [404, 405, 501].includes(Number(error?.status));
}

function appendCompileDiagnostics(error, options = {}) {
  const diagnostics = Array.isArray(error?.diagnostics) ? error.diagnostics : [];
  if (!diagnostics.length) {
    appendPlainCompileError(error, options);
    return;
  }
  let searchStart = 0;
  for (const diagnostic of diagnostics) {
    const location = diagnosticSourceLocation(diagnostic, { ...options, searchStart });
    if (location && Number.isInteger(location.offset)) {
      searchStart = Math.max(searchStart, location.offset + 1);
    }
    appendPreviewLog("error", diagnosticLogMessage(diagnostic), {
      ...options,
      origin: diagnosticOrigin(diagnostic, location),
      location,
    });
  }
}

function appendPlainCompileError(error, options = {}) {
  const messages = plainCompileErrorMessages(error);
  for (const message of messages) {
    appendPreviewLog("error", message, options);
  }
}

function plainCompileErrorMessages(error) {
  const message = String(error?.message || error || "Compile error");
  const lines = message
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  return lines.length ? lines : ["Compile error"];
}

function diagnosticLogMessage(diagnostic) {
  const message = String(diagnostic?.message || "Compile error");
  const sourceLine = String(diagnostic?.sourceLine || "").trim();
  return sourceLine ? `${message}: ${sourceLine}` : message;
}

function diagnosticOrigin(diagnostic, location = null) {
  const file = String(diagnostic?.file || "").trim();
  const line = positiveInteger(diagnostic?.line) || positiveInteger(location?.line);
  const column = positiveInteger(diagnostic?.column) || positiveInteger(location?.column);
  if (Number.isFinite(line) && line > 0 && Number.isFinite(column) && column > 0) {
    return file ? `${file}:${line}:${column}` : `line ${line}:${column}`;
  }
  if (Number.isFinite(line) && line > 0) {
    return file ? `${file}:${line}` : `line ${line}`;
  }
  return file;
}

function diagnosticSourceLocation(diagnostic, options = {}) {
  const document = options.document || activePreviewDocument();
  const sourceText = String(options.sourceText ?? options.source ?? document?.source ?? "");
  const line = positiveInteger(diagnostic?.line);
  const column = positiveInteger(diagnostic?.column) || 1;
  if (line) {
    const offset = sourceOffsetForLineColumn(sourceText, line, column);
    return {
      documentId: document?.id || "",
      line,
      column,
      offset,
      sourceLine: String(diagnostic?.sourceLine || ""),
    };
  }
  const sourceLine = String(diagnostic?.sourceLine || "").trim();
  if (!sourceLine || !sourceText) {
    return null;
  }
  const resolved = sourceLocationForDiagnosticLine(sourceText, sourceLine, options.searchStart || 0);
  if (!resolved) {
    return null;
  }
  return {
    documentId: document?.id || "",
    line: resolved.line,
    column: resolved.column,
    offset: resolved.offset,
    sourceLine,
  };
}

function sourceLocationForDiagnosticLine(source, sourceLine, searchStart = 0) {
  const wanted = String(sourceLine || "").trim();
  if (!wanted) {
    return null;
  }
  const lines = editorSourceLinesWithOffsets(source);
  const startOffset = Math.max(0, Math.trunc(Number(searchStart) || 0));
  const passes = [
    lines.filter((line) => line.absoluteEnd > startOffset),
    lines,
  ];
  for (const pass of passes) {
    for (const line of pass) {
      const trimmed = sourceCodeForDiagnosticMatch(line.raw).trim();
      if (trimmed !== wanted) {
        continue;
      }
      const column = firstEditorSourceCodeIndex(line) - line.start + 1;
      return {
        line: sourceLineIndex(lines, line) + 1,
        column,
        offset: firstEditorSourceCodeIndex(line),
      };
    }
    if (pass.length === lines.length) {
      break;
    }
  }
  return null;
}

function sourceLineIndex(lines, target) {
  const index = lines.findIndex((line) => line.start === target.start && line.absoluteEnd === target.absoluteEnd);
  return index >= 0 ? index : 0;
}

function sourceCodeForDiagnosticMatch(line) {
  const text = String(line || "");
  let inString = false;
  for (let index = 0; index < text.length - 1; index += 1) {
    const ch = text[index];
    if (ch === "\"" && text[index - 1] !== "\\") {
      inString = !inString;
      continue;
    }
    if (!inString && ch === "/" && text[index + 1] === "/" && text[index - 1] !== ":") {
      return text.slice(0, index);
    }
  }
  return text;
}

function sourceOffsetForLineColumn(source, line, column = 1) {
  const lines = editorSourceLinesWithOffsets(source);
  const index = Math.max(0, Math.min(lines.length - 1, line - 1));
  const target = lines[index] || { start: 0, raw: "" };
  const rawLength = String(target.raw || "").length;
  const columnOffset = Math.max(0, Math.min(rawLength, (positiveInteger(column) || 1) - 1));
  return target.start + columnOffset;
}

function positiveInteger(value) {
  const number = Number(value);
  return Number.isInteger(number) && number > 0 ? number : null;
}

function markEmbeddedPreviewDirty() {
  markPreviewDirty();
}

function markPreviewDirty() {
  const current = activeDocument();
  if (current && isTextDocument(current)) {
    current.source = sourceEditor.value;
  }
  // Keep the last compiled export available for play/edit/solver rendering
  // while marking it stale. Run Preview performs the explicit recompile when
  // a server backend is available.
  latestPreviewState = null;
  scheduleLocalSave();
  downloadButton.disabled = true;
  setStatus("Preview is stale", "");
}

function updateSourceMeta() {
  const source = sourceEditor.value;
  const lineCount = source.length ? source.split("\n").length : 0;
  sourceMeta.textContent = `${lineCount} lines`;
  if (typeof renderSourceLineNumbers === "function") {
    renderSourceLineNumbers();
  }
}

function paneStatusClassName(className = "") {
  return `pane-status ${className || ""}`.trim();
}

function activeStatusPaneId() {
  return typeof workPaneIdForPreviewMode === "function"
    ? workPaneIdForPreviewMode(currentPreviewMode || "play")
    : "preview";
}

function statusElementForPane(paneId) {
  const normalized = typeof normalizePaneId === "function"
    ? normalizePaneId(paneId)
    : (paneId || "");
  if (typeof paneStatusElementForPaneId === "function") {
    return paneStatusElementForPaneId(normalized);
  }
  return document.querySelector(`[data-pane-status="${normalized}"]`);
}

function clearPaneStatus(paneId) {
  const normalized = typeof normalizePaneId === "function"
    ? normalizePaneId(paneId)
    : (paneId || "");
  const timer = paneStatusClearTimers.get(normalized);
  if (timer) {
    window.clearTimeout(timer);
    paneStatusClearTimers.delete(normalized);
  }
  const element = statusElementForPane(normalized);
  if (element) {
    element.className = paneStatusClassName();
    element.textContent = "";
  }
  schedulePreviewViewportSync(2);
}

function setPaneStatus(paneId, text, className = "", options = {}) {
  const normalized = typeof normalizePaneId === "function"
    ? normalizePaneId(paneId)
    : (paneId || "");
  const timer = paneStatusClearTimers.get(normalized);
  if (timer) {
    window.clearTimeout(timer);
    paneStatusClearTimers.delete(normalized);
  }
  const element = statusElementForPane(normalized);
  if (!element) {
    return;
  }
  element.className = paneStatusClassName(className);
  element.textContent = text || "";
  schedulePreviewViewportSync(2);
  if (text && className === "is-ok") {
    const clearDelayMs = Number(options.clearDelayMs) || 1800;
    const nextTimer = window.setTimeout(() => {
      if (element.textContent === text && element.classList.contains("is-ok")) {
        element.textContent = "";
        element.className = paneStatusClassName();
        schedulePreviewViewportSync(2);
      }
      paneStatusClearTimers.delete(normalized);
    }, clearDelayMs);
    paneStatusClearTimers.set(normalized, nextTimer);
  }
}

function setStatus(text, className) {
  setPaneStatus(activeStatusPaneId(), text, className);
}

function setEditorStatus(text, className) {
  window.clearTimeout(editorStatusClearTimer);
  editorStatusLabel.className = `document-status ${className || ""}`.trim();
  editorStatusLabel.textContent = text;
  if (text && className === "is-ok") {
    editorStatusClearTimer = window.setTimeout(() => {
      if (editorStatusLabel.textContent === text && editorStatusLabel.classList.contains("is-ok")) {
        editorStatusLabel.textContent = "";
        editorStatusLabel.className = "document-status";
      }
    }, 1800);
  }
}

function resetPreviewLog(message = "waiting for preview output") {
  previewLogEntries = [];
  appendPreviewLog("system", message, { source: "editor" });
}

function appendPreviewLog(level, message, options = {}) {
  const normalizedLevel = ["system", "info", "log", "warn", "error", "debug"].includes(level)
    ? level
    : "log";
  const text = String(message || "").trimEnd();
  const source = previewLogSourceLabel(options.source || "editor");
  const origin = previewLogOriginLabel(options.origin);
  previewLogEntries.push({
    level: normalizedLevel,
    message: text || "(empty)",
    source,
    origin,
    location: previewLogLocation(options.location),
    time: new Date(),
  });
  if (previewLogEntries.length > 200) {
    previewLogEntries = previewLogEntries.slice(-200);
  }
  renderPreviewLog();
}

function previewLogSourceLabel(value) {
  const source = String(value || "").trim().toLowerCase();
  if (!source) {
    return "editor";
  }
  return source.replace(/\s+/g, " ");
}

function previewLogOriginLabel(value) {
  const origin = String(value || "").trim();
  if (!origin) {
    return "";
  }
  return origin.replace(/^.*\/([^/:]+:\d+:\d+)$/, "$1");
}

function previewLogTimeLabel(value) {
  const date = value instanceof Date ? value : new Date(value);
  const hours = String(date.getHours()).padStart(2, "0");
  const minutes = String(date.getMinutes()).padStart(2, "0");
  return `${hours}:${minutes}`;
}

function clearPreviewLog() {
  previewLogEntries = [];
  clearPaneStatus("preview");
  renderPreviewLog();
}

function renderPreviewLog() {
  if (!previewLogOutput) {
    return;
  }
  previewLogOutput.replaceChildren();
  if (!previewLogEntries.length) {
    const empty = document.createElement("div");
    empty.className = "preview-log-line is-muted";
    empty.textContent = "$ waiting for preview output";
    previewLogOutput.append(empty);
    return;
  }
  for (const [index, entry] of previewLogEntries.entries()) {
    const line = document.createElement("div");
    const classLevel = entry.level === "log" || entry.level === "info" || entry.level === "debug"
      ? ""
      : ` is-${entry.level}`;
    line.className = `preview-log-line${classLevel}`;
    if (entry.location) {
      line.classList.add("is-navigable");
      line.tabIndex = 0;
      line.setAttribute("role", "button");
      line.dataset.previewLogIndex = String(index);
      line.title = previewLogLocationTitle(entry.location);
    }
    const source = entry.source || "editor";
    const origin = entry.origin ? ` (${entry.origin})` : "";
    const label = entry.level === "system" ? "system" : entry.level;
    line.textContent = `[${previewLogTimeLabel(entry.time)}] ${source} ${label}${origin}: ${entry.message}`;
    previewLogOutput.append(line);
  }
  previewLogOutput.scrollTop = previewLogOutput.scrollHeight;
}

function previewLogLocation(location) {
  if (!location || typeof location !== "object") {
    return null;
  }
  const line = positiveInteger(location.line);
  const offset = Number.isInteger(location.offset) ? Math.max(0, location.offset) : null;
  if (!line && offset === null) {
    return null;
  }
  return {
    documentId: String(location.documentId || ""),
    line,
    column: positiveInteger(location.column) || 1,
    offset,
    sourceLine: String(location.sourceLine || ""),
  };
}

function previewLogLocationTitle(location) {
  const line = positiveInteger(location?.line);
  const column = positiveInteger(location?.column);
  if (line && column) {
    return `Go to line ${line}:${column}`;
  }
  if (line) {
    return `Go to line ${line}`;
  }
  return "Go to source";
}

function activatePreviewLogLocationFromEvent(event) {
  const target = event.target?.closest?.("[data-preview-log-index]");
  if (!target || !previewLogOutput?.contains(target)) {
    return false;
  }
  const index = Number(target.dataset.previewLogIndex);
  const entry = Number.isInteger(index) ? previewLogEntries[index] : null;
  if (!entry?.location) {
    return false;
  }
  event.preventDefault();
  revealPreviewLogLocation(entry.location);
  return true;
}

function revealPreviewLogLocation(location) {
  const targetDocument = documents.find((document) => document.id === location.documentId)
    || activePreviewDocument()
    || activeDocument();
  if (!targetDocument || !isTextDocument(targetDocument)) {
    setStatus("No source document for preview error", "is-error");
    return false;
  }
  const source = targetDocument.id === activeDocument()?.id
    ? sourceEditor.value || ""
    : targetDocument.source || "";
  const offset = Number.isInteger(location.offset)
    ? Math.max(0, Math.min(source.length, location.offset))
    : sourceOffsetForLineColumn(source, positiveInteger(location.line) || 1, positiveInteger(location.column) || 1);
  if (!revealSourceLocation({ document: targetDocument, start: offset }, { recordHistory: true })) {
    setStatus("Could not reveal preview error source", "is-error");
    return false;
  }
  sourceEditor.focus({ preventScroll: true });
  setStatus(previewLogLocationTitle(location), "");
  return true;
}

function emptyPreviewDocument() {
  return `<!doctype html>
<html>
  <head>
    <meta charset="utf-8">
    <style>
      body {
        margin: 0;
        min-height: 100vh;
        background: transparent;
      }
    </style>
  </head>
  <body></body>
</html>`;
}

function setPreviewFrameHtml(html) {
  if (!previewViewport || !previewFrame) {
    return;
  }

  schedulePreviewViewportSync(6);
  const loadId = previewFrameLoadId + 1;
  previewFrameLoadId = loadId;
  const previousFrame = previewFrame;
  const previousObjectUrl = previewFrameObjectUrl;
  let nextObjectUrl = "";
  const nextFrame = document.createElement("iframe");
  nextFrame.className = "preview-frame";
  nextFrame.title = "Compiled puzzle preview";
  nextFrame.setAttribute("sandbox", "allow-scripts");
  nextFrame.setAttribute("scrolling", "no");
  nextFrame.setAttribute("aria-hidden", "true");
  nextFrame.style.visibility = "hidden";
  previewViewport.append(nextFrame);

  nextFrame.addEventListener("load", () => {
    if (loadId !== previewFrameLoadId) {
      nextFrame.remove();
      if (nextObjectUrl) {
        URL.revokeObjectURL(nextObjectUrl);
      }
      return;
    }
    previousFrame.removeAttribute("id");
    nextFrame.id = "previewFrame";
    nextFrame.removeAttribute("aria-hidden");
    nextFrame.style.visibility = "";
    previousFrame.remove();
    previewFrame = nextFrame;
    previewFrameObjectUrl = nextObjectUrl;
    schedulePreviewViewportSync(6);
    if (currentPreviewMode === "level3d" && typeof sendLevel3dSnapshotToRuntime === "function") {
      sendLevel3dSnapshotToRuntime();
    } else if (activePreviewModeAcceptsLevelState()) {
      sendLevelStateToPreview();
    }
    if (previousObjectUrl) {
      URL.revokeObjectURL(previousObjectUrl);
    }
  }, { once: true });

  nextFrame.srcdoc = html;
}

function editorPreviewDocument(html) {
  const consoleScript = `<script id="puzzle-studio-editor-preview-log-script">
(() => {
  const formatArg = (value, depth = 0) => {
    if (typeof value === "string") {
      return value;
    }
    if (value instanceof Error) {
      return value.stack || value.message || String(value);
    }
    if (value === undefined) {
      return "undefined";
    }
    if (value === null || typeof value === "number" || typeof value === "boolean" || typeof value === "bigint") {
      return String(value);
    }
    if (depth > 1) {
      return Object.prototype.toString.call(value);
    }
    try {
      return JSON.stringify(value, (_key, nested) => {
        if (typeof nested === "function") {
          return "[Function]";
        }
        return nested;
      });
    } catch (_error) {
      return String(value);
    }
  };
  const postLog = (level, args) => {
    try {
      window.parent.postMessage({
        type: "PuzzleStudioPreviewLog",
        level,
        source: "preview console",
        origin: stackOrigin(),
        message: Array.from(args || []).map((arg) => formatArg(arg)).join(" "),
      }, "*");
    } catch (_error) {
      // Logging must not affect the preview runtime.
    }
  };
  const stackOrigin = () => {
    const stack = new Error().stack || "";
    const lines = stack.split("\\n").slice(1);
    for (const line of lines) {
      const text = String(line || "").trim();
      if (!text || text.includes("postLog") || text.includes("stackOrigin") || text.includes("console.")) {
        continue;
      }
      const match = text.match(/(?:at\\s+)?(?:.*?\\()?([^()\\s]+:\\d+:\\d+)\\)?$/);
      if (match) {
        return match[1];
      }
    }
    return "";
  };
  for (const level of ["debug", "log", "info", "warn", "error"]) {
    const original = console[level]?.bind(console);
    console[level] = (...args) => {
      postLog(level, args);
      if (original) {
        original(...args);
      }
    };
  }
  window.addEventListener("error", (event) => {
    try {
      window.parent.postMessage({
        type: "PuzzleStudioPreviewLog",
        level: "error",
        source: "preview runtime",
        origin: event.filename && event.lineno ? String(event.filename) + ":" + event.lineno + ":" + (event.colno || 0) : "",
        message: formatArg(event.error || event.message || "Runtime error"),
      }, "*");
    } catch (_error) {
      // Logging must not affect the preview runtime.
    }
  });
  window.addEventListener("unhandledrejection", (event) => {
    try {
      window.parent.postMessage({
        type: "PuzzleStudioPreviewLog",
        level: "error",
        source: "preview promise",
        origin: "",
        message: formatArg(event.reason || "Unhandled promise rejection"),
      }, "*");
    } catch (_error) {
      // Logging must not affect the preview runtime.
    }
  });
  const postLoaded = () => {
    try {
      window.parent.postMessage({
        type: "PuzzleStudioPreviewLoaded",
        title: document.title || "",
        href: location.href || "",
      }, "*");
    } catch (_error) {
      // Runtime observability must not affect the preview runtime.
    }
  };
  if (document.readyState === "complete") {
    queueMicrotask(postLoaded);
  } else {
    window.addEventListener("load", postLoaded, { once: true });
  }
})();
<\/script>`;
  let next = html;
  if (!next.includes("puzzle-studio-editor-preview-log-script")) {
    if (next.includes("</head>")) {
      next = next.replace("</head>", `${consoleScript}\n  </head>`);
    } else if (next.includes("<body")) {
      next = next.replace("<body", `${consoleScript}\n<body`);
    } else {
      next = `${consoleScript}\n${next}`;
    }
  }
  return next;
}

function updatePreviewFrameLayout(layout) {
  void layout;
  previewVirtualHeight = previewMinimumHeight;
  syncPreviewVirtualSize();
  syncPreviewViewportScale();
}

function syncPreviewViewportAspect(sceneName = latestPreviewState?.screen || "") {
  setPreviewViewportAspect(previewAspectForScene(previewExport, sceneName));
}

function setPreviewViewportAspect(aspect) {
  const next = Number.isFinite(aspect) && aspect > 0
    ? aspect
    : previewDefaultLogicalWidth / previewDefaultLogicalHeight;
  if (Math.abs(next - previewViewportAspect) < 0.0001) {
    return;
  }
  previewViewportAspect = next;
  syncPreviewVirtualSize();
  schedulePreviewViewportSync(2);
}

function syncPreviewVirtualSize() {
  previewVirtualWidth = Math.max(1, Math.round(previewVirtualHeight * previewViewportAspect));
  previewFrameWrap?.style.setProperty("--preview-virtual-width", `${previewVirtualWidth}px`);
  previewFrameWrap?.style.setProperty("--preview-virtual-height", `${previewVirtualHeight}px`);
}

function previewAspectForScene(exportData = previewExport, sceneName = "") {
  const scenes = exportData?.scenes || exportData?.screens || [];
  let scene = sceneName
    ? scenes.find((candidate) => candidate?.name === sceneName)
    : null;
  if (!scene) {
    const initialName = exportData?.currentScene || exportData?.screen || scenes[0]?.name || "";
    scene = initialName
      ? scenes.find((candidate) => candidate?.name === initialName)
      : null;
  }
  const width = Number(scene?.layout?.size?.width);
  const height = Number(scene?.layout?.size?.height);
  if (Number.isFinite(width) && Number.isFinite(height) && width > 0 && height > 0) {
    return width / height;
  }
  return previewDefaultLogicalWidth / previewDefaultLogicalHeight;
}

function syncPreviewViewportScale() {
  if (!previewFrameWrap || !previewViewport) {
    return;
  }
  if (previewFrameWrap.getClientRects().length === 0 || previewViewport.getClientRects().length === 0) {
    return;
  }
  // The editor preview owns only the outer device rectangle; the game owns its aspect.
  // Keep the iframe viewport fixed and fit the rendered frame as a whole.
  const available = editorFrameAvailableSize(previewFrameWrap, {
    container: playPreview,
    reservedBlock: previewLogReservedBlockSize(),
  });
  const viewportSize = fitPreviewViewportSize(available.width, available.height, previewViewportAspect);
  const viewportWidth = viewportSize.width;
  const viewportHeight = viewportSize.height;
  const framePaddingAndBorder = 0;
  previewFrameWrap.style.setProperty("--preview-scale", viewportSize.scale.toFixed(6));
  previewFrameWrap.style.setProperty("--preview-virtual-width", `${viewportSize.virtualWidth}px`);
  previewFrameWrap.style.setProperty("--preview-virtual-height", `${viewportSize.virtualHeight}px`);
  previewFrameWrap.style.setProperty("--preview-viewport-width", `${viewportWidth}px`);
  previewFrameWrap.style.setProperty("--preview-viewport-height", `${viewportHeight}px`);
  previewFrameWrap.style.setProperty("--preview-frame-height", `${viewportHeight + framePaddingAndBorder}px`);
  syncPreviewAutoLogHeight(viewportHeight + framePaddingAndBorder);
}

function fitPreviewViewportSize(availableWidth, availableHeight, aspect) {
  return fitEditorAspectFrame(
    { width: availableWidth, height: availableHeight },
    aspect,
    previewMinimumHeight,
  );
}

function schedulePreviewViewportSync(passes = 2) {
  previewViewportSyncPasses = Math.max(
    previewViewportSyncPasses,
    Math.max(1, Math.trunc(Number(passes) || 1)),
  );
  if (previewViewportSyncFrame) {
    return;
  }
  const tick = () => {
    previewViewportSyncFrame = 0;
    syncPreviewViewportScale();
    previewViewportSyncPasses -= 1;
    if (previewViewportSyncPasses > 0) {
      previewViewportSyncFrame = requestAnimationFrame(tick);
    }
  };
  previewViewportSyncFrame = requestAnimationFrame(tick);
}

function syncPreviewAutoLogHeight(frameHeight) {
  if (!playPreview || !previewFrameWrap || !previewLogPanel || previewLogHeightPinned) {
    return;
  }
  const available = editorFrameAvailableSize(previewFrameWrap, { container: playPreview });
  if (available.height <= 0) {
    return;
  }
  const measuredFrameHeight = previewFrameWrap.getBoundingClientRect().height || frameHeight;
  const logMargins = elementBlockMargins(previewLogPanel);
  const next = Math.max(previewMinimumLogHeight, available.height - logMargins - Math.ceil(measuredFrameHeight));
  playPreview.style.setProperty("--preview-log-height", `${Math.round(next)}px`);
}

function previewLogReservedBlockSize() {
  return previewLogPanel
    ? previewMinimumLogHeight + elementBlockMargins(previewLogPanel)
    : 0;
}


function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

async function downloadHtml() {
  if (!latestHtml) {
    return;
  }
  const filename = htmlDownloadFileName();
  if (window.PuzzleStudioHost?.mode?.() === "tauri" && window.PuzzleStudioHost?.exportHtml) {
    try {
      const result = await window.PuzzleStudioHost.exportHtml({
        html: latestHtml,
        filename,
      });
      if (result?.canceled) {
        setEditorStatus("Export canceled");
        return;
      }
      if (result?.ok) {
        setEditorStatus(`Exported ${fileName(result.path) || filename}`);
        return;
      }
    } catch (error) {
      console.error(error);
      setEditorStatus(`Export failed: ${error.message || error}`, "is-error");
      return;
    }
  }
  const blob = new Blob([latestHtml], { type: "text/html;charset=utf-8" });
  downloadBlob(blob, filename);
}

function htmlDownloadFileName() {
  const previewDocument = activePreviewDocument();
  const path = previewDocument?.puzzlePath || previewDocument?.name || "";
  const sourceName = path ? fileName(path) : "";
  const baseName = sourceName
    .replace(/\.puzzle$/i, "")
    .replace(/\.html?$/i, "") || "game";
  return `${sanitizeFileName(baseName) || "game"}.html`;
}

function downloadPuzzle() {
  persistCurrentDocument();
  const selected = selectedTreeNode();
  if (selected?.kind === "folder") {
    downloadFolder(selected);
    return;
  }
  downloadFile(selected?.kind === "file" ? selected : documents[currentDocumentIndex]);
}

function downloadFile(document) {
  if (!document) {
    return;
  }
  const blob = document.encoding === "data_url"
    ? new Blob([bytesForDocument(document)], { type: document.mimeType || "application/octet-stream" })
    : new Blob([document.source || sourceEditor.value], { type: `${document.mimeType || "text/plain"};charset=utf-8` });
  const name = document.name || fileName(document.puzzlePath);
  downloadBlob(blob, name || "file");
}

function downloadFolder(folder) {
  const entries = folderZipEntries(folder);
  if (!entries.length) {
    setEditorStatus("Folder is empty", "is-error");
    return;
  }
  const zip = zipBlob(entries);
  downloadBlob(zip, `${sanitizeFileName(folder.name || "folder") || "folder"}.zip`);
}

function folderZipEntries(folder) {
  const entries = [];
  const rootName = sanitizeFileName(folder.name || "folder") || "folder";
  collectFolderZipEntries(folder, rootName, entries);
  return entries;
}

function collectFolderZipEntries(node, parentPath, entries) {
  for (const child of node.children || []) {
    const childName = sanitizeZipPathSegment(child.name || fileName(child.puzzlePath));
    const childPath = joinPath(parentPath, childName);
    if (child.kind === "folder") {
      collectFolderZipEntries(child, childPath, entries);
      continue;
    }
    entries.push({
      path: childPath,
      bytes: bytesForDocument(child),
    });
  }
}

function bytesForDocument(document) {
  if (document.encoding === "data_url") {
    return dataUrlBytes(document.dataUrl || "");
  }
  return new TextEncoder().encode(document.source || "");
}

function dataUrlBytes(dataUrl) {
  const match = String(dataUrl).match(/^data:([^,]*),(.*)$/);
  if (!match) {
    return new Uint8Array();
  }
  const meta = match[1] || "";
  const data = match[2] || "";
  if (meta.includes(";base64")) {
    const raw = atob(data);
    const bytes = new Uint8Array(raw.length);
    for (let index = 0; index < raw.length; index += 1) {
      bytes[index] = raw.charCodeAt(index);
    }
    return bytes;
  }
  return new TextEncoder().encode(decodeURIComponent(data));
}

function sanitizeZipPathSegment(name) {
  return sanitizeFileName(name).replace(/^\.|\.$/g, "") || "item";
}

function downloadBlob(blob, filename) {
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  link.style.display = "none";
  document.body.appendChild(link);
  link.click();
  window.setTimeout(() => {
    URL.revokeObjectURL(url);
    link.remove();
  }, 0);
}

function zipBlob(entries) {
  const encoder = new TextEncoder();
  const parts = [];
  const centralParts = [];
  let offset = 0;
  const now = new Date();
  const dosTime = ((now.getHours() & 31) << 11) | ((now.getMinutes() & 63) << 5) | ((Math.floor(now.getSeconds() / 2)) & 31);
  const dosDate = (((now.getFullYear() - 1980) & 127) << 9) | (((now.getMonth() + 1) & 15) << 5) | (now.getDate() & 31);

  for (const entry of entries) {
    const nameBytes = encoder.encode(normalizePath(entry.path));
    const dataBytes = entry.bytes || new Uint8Array();
    const crc = crc32(dataBytes);
    const localHeader = new Uint8Array(30 + nameBytes.length);
    const localView = new DataView(localHeader.buffer);
    localView.setUint32(0, 0x04034b50, true);
    localView.setUint16(4, 20, true);
    localView.setUint16(6, 0x0800, true);
    localView.setUint16(8, 0, true);
    localView.setUint16(10, dosTime, true);
    localView.setUint16(12, dosDate, true);
    localView.setUint32(14, crc, true);
    localView.setUint32(18, dataBytes.length, true);
    localView.setUint32(22, dataBytes.length, true);
    localView.setUint16(26, nameBytes.length, true);
    localHeader.set(nameBytes, 30);
    parts.push(localHeader, dataBytes);

    const centralHeader = new Uint8Array(46 + nameBytes.length);
    const centralView = new DataView(centralHeader.buffer);
    centralView.setUint32(0, 0x02014b50, true);
    centralView.setUint16(4, 20, true);
    centralView.setUint16(6, 20, true);
    centralView.setUint16(8, 0x0800, true);
    centralView.setUint16(10, 0, true);
    centralView.setUint16(12, dosTime, true);
    centralView.setUint16(14, dosDate, true);
    centralView.setUint32(16, crc, true);
    centralView.setUint32(20, dataBytes.length, true);
    centralView.setUint32(24, dataBytes.length, true);
    centralView.setUint16(28, nameBytes.length, true);
    centralView.setUint32(42, offset, true);
    centralHeader.set(nameBytes, 46);
    centralParts.push(centralHeader);
    offset += localHeader.length + dataBytes.length;
  }

  const centralSize = centralParts.reduce((sum, part) => sum + part.length, 0);
  const end = new Uint8Array(22);
  const endView = new DataView(end.buffer);
  endView.setUint32(0, 0x06054b50, true);
  endView.setUint16(8, entries.length, true);
  endView.setUint16(10, entries.length, true);
  endView.setUint32(12, centralSize, true);
  endView.setUint32(16, offset, true);

  return new Blob([...parts, ...centralParts, end], { type: "application/zip" });
}

function crc32(bytes) {
  let crc = 0xffffffff;
  for (const byte of bytes) {
    crc ^= byte;
    for (let bit = 0; bit < 8; bit += 1) {
      crc = (crc >>> 1) ^ (0xedb88320 & -(crc & 1));
    }
  }
  return (crc ^ 0xffffffff) >>> 0;
}

async function importFiles(fileList) {
  return importFilesIntoFolder(fileList, activeFolder());
}

async function importFilesIntoFolder(fileList, targetFolder) {
  const files = Array.from(fileList || []);
  if (!files.length) {
    return;
  }
  persistCurrentDocument();
  selectedFolderId = targetFolder?.kind === "folder" && targetFolder !== fileTree ? targetFolder.id : "";
  selectedTreeId = selectedFolderId || selectedTreeId;
  let firstImportedPuzzleId = "";
  let importedCount = 0;
  for (const file of files) {
    if (isZipFileName(file.name, file.type)) {
      const result = await importZipFile(file, targetFolder);
      importedCount += result.count;
      if (!firstImportedPuzzleId && result.firstImportedPuzzleId) {
        firstImportedPuzzleId = result.firstImportedPuzzleId;
      }
      continue;
    }

    let imported = null;
    if (isTextFileName(file.name, file.type)) {
      imported = importWorkspaceFile(file.webkitRelativePath || file.name, {
        encoding: "text",
        source: await file.text(),
        mimeType: file.type || mimeTypeForPath(file.name),
      }, targetFolder);
    } else {
      imported = importWorkspaceFile(file.webkitRelativePath || file.name, {
        encoding: "data_url",
        dataUrl: await readFileAsDataUrl(file),
        mimeType: file.type || mimeTypeForPath(file.name),
      }, targetFolder);
    }
    if (!firstImportedPuzzleId && isPuzzleDocument(imported)) {
      firstImportedPuzzleId = imported.id;
    }
    if (imported) {
      importedCount += 1;
    }
  }
  if (!importedCount) {
    setEditorStatus("No importable files", "is-error");
    return;
  }
  if (firstImportedPuzzleId) {
    activeFileId = firstImportedPuzzleId;
  }
  syncDocumentsFromTree();
  currentDocumentIndex = activeDocumentIndex();
  renderDocumentSelect();
  loadEmbeddedDocument(currentDocumentIndex);
  if (!editorSeed) {
    await renderPreview();
  }
  saveDocumentStore(false);
  const folderName = targetFolder && targetFolder !== fileTree ? folderPath(targetFolder) || targetFolder.name : "Files";
  setEditorStatus(`Imported to ${folderName}`, "is-ok");
}

async function importZipFile(file, targetFolder) {
  const entries = await unzipFileEntries(file);
  let firstImportedPuzzleId = "";
  let count = 0;
  for (const entry of entries) {
    const entryPath = safeZipEntryPath(entry.path);
    if (!entryPath) {
      continue;
    }

    let imported = null;
    if (isTextFileName(entryPath, entry.mimeType)) {
      imported = importWorkspaceFile(entryPath, {
        encoding: "text",
        source: new TextDecoder().decode(entry.bytes),
        mimeType: entry.mimeType || mimeTypeForPath(entryPath),
      }, targetFolder);
    } else {
      imported = importWorkspaceFile(entryPath, {
        encoding: "data_url",
        dataUrl: bytesToDataUrl(entry.bytes, entry.mimeType || mimeTypeForPath(entryPath)),
        mimeType: entry.mimeType || mimeTypeForPath(entryPath),
      }, targetFolder);
    }

    if (!firstImportedPuzzleId && isPuzzleDocument(imported)) {
      firstImportedPuzzleId = imported.id;
    }
    if (imported) {
      count += 1;
    }
  }
  return { count, firstImportedPuzzleId };
}

async function unzipFileEntries(file) {
  const bytes = new Uint8Array(await file.arrayBuffer());
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const endOffset = findZipEndOffset(view);
  if (endOffset < 0) {
    throw new Error("Invalid zip file");
  }

  const entryCount = view.getUint16(endOffset + 10, true);
  let centralOffset = view.getUint32(endOffset + 16, true);
  const entries = [];

  for (let index = 0; index < entryCount; index += 1) {
    if (centralOffset + 46 > bytes.length || view.getUint32(centralOffset, true) !== 0x02014b50) {
      throw new Error("Invalid zip directory");
    }
    const flags = view.getUint16(centralOffset + 8, true);
    const method = view.getUint16(centralOffset + 10, true);
    const compressedSize = view.getUint32(centralOffset + 20, true);
    const nameLength = view.getUint16(centralOffset + 28, true);
    const extraLength = view.getUint16(centralOffset + 30, true);
    const commentLength = view.getUint16(centralOffset + 32, true);
    const localOffset = view.getUint32(centralOffset + 42, true);
    const nameStart = centralOffset + 46;
    const nameBytes = bytes.slice(nameStart, nameStart + nameLength);
    const path = decodeZipName(nameBytes, flags);
    centralOffset = nameStart + nameLength + extraLength + commentLength;

    if (!path || path.endsWith("/")) {
      continue;
    }
    if (localOffset + 30 > bytes.length || view.getUint32(localOffset, true) !== 0x04034b50) {
      throw new Error("Invalid zip entry");
    }

    const localNameLength = view.getUint16(localOffset + 26, true);
    const localExtraLength = view.getUint16(localOffset + 28, true);
    const dataStart = localOffset + 30 + localNameLength + localExtraLength;
    const compressed = bytes.slice(dataStart, dataStart + compressedSize);
    const entryBytes = method === 0
      ? compressed
      : method === 8
        ? await inflateZipDeflate(compressed)
        : null;
    if (!entryBytes) {
      throw new Error(`Unsupported zip compression for ${path}`);
    }
    entries.push({
      path,
      bytes: entryBytes,
      mimeType: mimeTypeForPath(path),
    });
  }

  return entries;
}

function findZipEndOffset(view) {
  const minOffset = Math.max(0, view.byteLength - 0xffff - 22);
  for (let offset = view.byteLength - 22; offset >= minOffset; offset -= 1) {
    if (view.getUint32(offset, true) === 0x06054b50) {
      return offset;
    }
  }
  return -1;
}

function decodeZipName(bytes, flags) {
  const decoder = flags & 0x0800 ? new TextDecoder("utf-8") : new TextDecoder();
  return decoder.decode(bytes);
}

async function inflateZipDeflate(bytes) {
  if (typeof DecompressionStream !== "function") {
    throw new Error("Zip deflate is not supported in this browser");
  }
  try {
    const stream = new Blob([bytes]).stream().pipeThrough(new DecompressionStream("deflate-raw"));
    return new Uint8Array(await new Response(stream).arrayBuffer());
  } catch (error) {
    const stream = new Blob([bytes]).stream().pipeThrough(new DecompressionStream("deflate"));
    return new Uint8Array(await new Response(stream).arrayBuffer());
  }
}

function safeZipEntryPath(path) {
  const normalized = normalizePath(path);
  if (!normalized || normalized.startsWith("/") || /^[A-Za-z]:\//.test(normalized)) {
    return "";
  }
  const parts = normalized.split("/").filter(Boolean);
  if (!parts.length || parts.includes("..") || parts[0] === "__MACOSX" || parts.at(-1) === ".DS_Store") {
    return "";
  }
  return parts.map(sanitizeZipPathSegment).join("/");
}

function bytesToDataUrl(bytes, mimeType = "application/octet-stream") {
  let binary = "";
  const chunkSize = 0x8000;
  for (let index = 0; index < bytes.length; index += chunkSize) {
    const chunk = bytes.slice(index, index + chunkSize);
    binary += String.fromCharCode(...chunk);
  }
  return `data:${mimeType};base64,${btoa(binary)}`;
}

function readFileAsDataUrl(file) {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.addEventListener("load", () => resolve(String(reader.result || "")));
    reader.addEventListener("error", () => reject(reader.error || new Error("File read failed")));
    reader.readAsDataURL(file);
  });
}

function importWorkspaceFile(fileNameValue, fileData, targetFolder = activeFolder()) {
  const current = documents[currentDocumentIndex] || {};
  const parts = String(fileNameValue || "imported.file").split(/[\\/]/).filter(Boolean);
  const name = sanitizeFileName(parts.pop() || "imported.file");
  let folder = targetFolder || fileTree;
  for (const part of parts) {
    folder = childFolder(folder, part, workspaceRootForFolder(folder));
  }
  const file = makeFile(uniqueChildName(folder, name), fileData.source || "", {
    parentPath: folderPath(folder),
    workspaceRoot: workspaceRootForFolder(folder),
    gameCss: current.gameCss || editorSeed?.gameCss || "",
    gameVisualsJs: current.gameVisualsJs || editorSeed?.gameVisualsJs || "",
  });
  file.encoding = fileData.encoding || "text";
  file.mimeType = fileData.mimeType || mimeTypeForPath(name);
  file.source = fileData.source || "";
  file.dataUrl = fileData.dataUrl || "";
  if (!isPuzzleDocument(file)) {
    file.previewHtml = "";
    file.gameCss = "";
    file.gameVisualsJs = "";
  }
  folder.children.push(file);
  selectedFolderId = folder.id;
  activeFileId = file.id;
  return file;
}

function setPuzzleScriptImportStatus(message, tone = "") {
  if (!psImportStatus) {
    return;
  }
  psImportStatus.textContent = message;
  psImportStatus.className = "ps-import-status tool-feedback-bar";
  psImportStatus.classList.toggle("is-ok", tone === "is-ok");
  psImportStatus.classList.toggle("is-error", tone === "is-error");
  setPaneStatus("psimport", message, tone);
}

function resetPuzzleScriptImportConversion() {
  if (psImportOutput) {
    psImportOutput.value = "";
  }
  if (psImportCopyButton) {
    psImportCopyButton.disabled = true;
  }
  if (psImportAddFileButton) {
    psImportAddFileButton.disabled = true;
  }
  setPuzzleScriptImportStatus("", "");
}

async function convertPuzzleScriptImport() {
  const source = psImportSourceInput?.value || "";
  if (!source.trim()) {
    if (psImportOutput) {
      psImportOutput.value = "";
    }
    if (psImportCopyButton) {
      psImportCopyButton.disabled = true;
    }
    if (psImportAddFileButton) {
      psImportAddFileButton.disabled = true;
    }
    setPuzzleScriptImportStatus("", "");
    return "";
  }
  setPuzzleScriptImportStatus("Converting", "");
  const compiler = await loadWasmCompiler();
  if (typeof compiler.translate_puzzlescript !== "function") {
    throw new Error("PuzzleScript import is unavailable in this editor build.");
  }
  const canonical = compiler.translate_puzzlescript(source);
  if (psImportOutput) {
    psImportOutput.value = canonical;
  }
  if (psImportCopyButton) {
    psImportCopyButton.disabled = false;
  }
  if (psImportAddFileButton) {
    psImportAddFileButton.disabled = false;
  }
  setPuzzleScriptImportStatus("Converted", "is-ok");
  return canonical;
}

function puzzleScriptImportTitle(source, canonical) {
  const explicitTitle = String(source || "")
    .split("\n")
    .map((line) => line.split("//", 1)[0].trim())
    .find((line) => /^title(?:\s+|$)/i.test(line))
    ?.replace(/^title\s*/i, "")
    .trim();
  if (explicitTitle) {
    return explicitTitle;
  }
  const canonicalTitle = String(canonical || "")
    .split("\n")
    .find((line) => /^title(?:\s+|$)/.test(line.trim()))
    ?.trim()
    .replace(/^title\s*/, "")
    .trim();
  if (canonicalTitle) {
    try {
      return JSON.parse(canonicalTitle);
    } catch {
      return canonicalTitle.replace(/^"|"$/g, "");
    }
  }
  return "PuzzleScript import";
}

async function copyPuzzleScriptImportOutput() {
  const output = psImportOutput?.value || "";
  if (!output.trim()) {
    setPuzzleScriptImportStatus("Generate import first", "is-error");
    return;
  }
  try {
    psImportCopyButton?.focus({ preventScroll: true });
    await copyTextToClipboard(output);
    setPuzzleScriptImportStatus("Copied", "is-ok");
  } catch (error) {
    setPuzzleScriptImportStatus("Copy failed", "is-error");
    setStatus(`Could not copy PuzzleScript import: ${error?.message || error}`, "is-error");
  }
}

async function addPuzzleScriptImportFile() {
  const output = psImportOutput?.value || "";
  if (!output.trim()) {
    setPuzzleScriptImportStatus("Generate import first", "is-error");
    return;
  }

  persistCurrentDocument();
  const targetFolder = activeFolder();
  targetFolder.expanded = true;
  const title = puzzleScriptImportTitle(psImportSourceInput?.value || "", output);
  const fileNameValue = uniqueChildName(targetFolder, ensurePuzzleExtension(title || "PuzzleScript import"));
  const parentPath = folderPath(targetFolder);
  const editorPath = joinPath(parentPath, fileNameValue);

  if (!editorSeed && typeof window.PuzzleStudioHost.createSourceFile === "function") {
    await window.PuzzleStudioHost.createSourceFile({
      source: output,
      puzzlePath: hostPathForEditorPath(editorPath, workspaceRootForFolder(targetFolder)),
      workspaceRoot: workspaceRootForFolder(targetFolder),
    });
  }

  const current = documents[currentDocumentIndex] || {};
  const file = makeFile(fileNameValue, output, {
    parentPath,
    workspaceRoot: workspaceRootForFolder(targetFolder),
    gameCss: current.gameCss || editorSeed?.gameCss || "",
    gameVisualsJs: current.gameVisualsJs || editorSeed?.gameVisualsJs || "",
  });
  targetFolder.children.push(file);
  activeFileId = file.id;
  selectedTreeId = file.id;
  selectedFolderId = targetFolder === fileTree ? "" : targetFolder.id;
  syncDocumentsFromTree();
  loadEmbeddedDocument(activeDocumentIndex());
  saveDocumentStore(false);
  setPuzzleScriptImportStatus(`Added ${fileNameValue}`, "is-ok");
}

function normalizeEditorDimension(dimension) {
  return String(dimension || "").toLowerCase() === "3d" ? "3d" : "2d";
}

function editorDimensionLabel(dimension = currentEditorDimension) {
  return normalizeEditorDimension(dimension) === "3d" ? "3D" : "2D";
}

function levelModeForEditorDimension(dimension = currentEditorDimension) {
  return normalizeEditorDimension(dimension) === "3d" ? "level3d" : "edit";
}

function spriteModeForEditorDimension(dimension = currentEditorDimension) {
  return normalizeEditorDimension(dimension) === "3d" ? "sprite3d" : "sprite";
}

function focusedPuzzleSourceContext(document = activeDocument()) {
  if (!isPuzzleDocument(document) || !isTextDocument(document)) {
    return null;
  }
  const source = sourceForDocument(document);
  return { document, source };
}

function firstFocusedPuzzleEntry(kind, context = focusedPuzzleSourceContext()) {
  return focusedPuzzleEntries(kind, context)[0] || null;
}

function focusedPuzzleEntries(kind, context = focusedPuzzleSourceContext()) {
  if (!context?.document) {
    return [];
  }
  return [
    ...focusedPuzzleEntriesForDimension(kind, "2d", context),
    ...focusedPuzzleEntriesForDimension(kind, "3d", context),
  ]
    .filter((item) => Number.isFinite(item?.target?.start))
    .sort((left, right) => left.target.start - right.target.start);
}

function firstFocusedPuzzleEntryForDimension(kind, dimension, context = focusedPuzzleSourceContext()) {
  return focusedPuzzleEntriesForDimension(kind, dimension, context)[0]?.target || null;
}

function focusedPuzzleEntriesForDimension(kind, dimension, context = focusedPuzzleSourceContext()) {
  if (!context?.document) {
    return [];
  }
  const normalized = normalizeEditorDimension(dimension);
  const entries = normalized === "3d"
    ? (kind === "sprite" ? focusedPuzzleSprite3dEntries(context.source) : focusedPuzzleLevel3dEntries(context.source))
    : (kind === "sprite" ? focusedPuzzleSprite2dEntries(context.source) : focusedPuzzleLevel2dEntries(context.source, context.document));
  return uniqueFocusedPuzzleEntries(entries)
    .map((entry) => ({ dimension: normalized, target: { ...entry, document: context.document } }));
}

function uniqueFocusedPuzzleEntries(entries) {
  const seen = new Set();
  const unique = [];
  for (const entry of entries || []) {
    if (!Number.isFinite(entry?.start)) {
      continue;
    }
    const key = `${entry.start}:${entry.end ?? ""}:${entry.bodyStart ?? ""}:${entry.bodyEnd ?? ""}`;
    if (seen.has(key)) {
      continue;
    }
    seen.add(key);
    unique.push(entry);
  }
  return unique.sort((left, right) => left.start - right.start);
}

function firstFocusedPuzzleEntryDimension(kind, context = focusedPuzzleSourceContext()) {
  return firstFocusedPuzzleEntry(kind, context)?.dimension || null;
}

function modeForFocusedPuzzleEntry(kind, context = focusedPuzzleSourceContext()) {
  const dimension = firstFocusedPuzzleEntryDimension(kind, context);
  if (!dimension) {
    return null;
  }
  return kind === "sprite"
    ? spriteModeForEditorDimension(dimension)
    : levelModeForEditorDimension(dimension);
}

function syncPaneModesFromFocusedPuzzleSource(options = {}) {
  const context = focusedPuzzleSourceContext();
  const firstLevel = firstFocusedPuzzleEntry("level", context);
  const firstSprite = firstFocusedPuzzleEntry("sprite", context);
  const levelMode = modeForFocusedPuzzleEntry("level", context);
  const spriteMode = modeForFocusedPuzzleEntry("sprite", context);
  if (levelMode) {
    currentLevelPaneMode = levelMode;
  } else if (currentPreviewMode === "edit" || currentPreviewMode === "level3d") {
    currentLevelPaneMode = "none";
  }
  if (spriteMode) {
    currentSpritePaneMode = spriteMode;
  } else if (currentPreviewMode === "sprite" || currentPreviewMode === "sprite3d") {
    currentSpritePaneMode = "none";
  }

  let nextMode = null;
  if (currentPreviewMode === "edit" || currentPreviewMode === "level3d") {
    nextMode = levelMode;
  } else if (currentPreviewMode === "sprite" || currentPreviewMode === "sprite3d") {
    nextMode = spriteMode;
  }

  if (nextMode && options.switchOpenPane !== false && nextMode !== currentPreviewMode) {
    setPreviewMode(nextMode);
  } else {
    const inferredDimension = editorDimensionForPreviewMode(nextMode || levelMode || spriteMode || currentLevelPaneMode || currentSpritePaneMode);
    currentEditorDimension = normalizeEditorDimension(inferredDimension);
    syncPreviewModeButtonState();
  }
  if (options.loadFirst !== false) {
    if (currentPreviewMode === "edit" || currentPreviewMode === "level3d") {
      loadFocusedPuzzleEntry("level", firstLevel, { silent: true, recordHistory: false });
    } else if (currentPreviewMode === "sprite" || currentPreviewMode === "sprite3d") {
      loadFocusedPuzzleEntry("sprite", firstSprite, { silent: true, recordHistory: false });
    }
  }
  if (
    ((currentPreviewMode === "edit" || currentPreviewMode === "level3d") && !firstLevel)
    || ((currentPreviewMode === "sprite" || currentPreviewMode === "sprite3d") && !firstSprite)
  ) {
    applyPaneVisibility();
  }
  return nextMode || null;
}

function sourcePositionInsideRanges(position, ranges) {
  return ranges.some((range) => (
    Number.isFinite(position)
    && Number.isFinite(range?.start)
    && Number.isFinite(range?.end)
    && position >= range.start
    && position <= range.end
  ));
}

function firstFocusedPuzzleLevel2dEntry(source, document) {
  return focusedPuzzleLevel2dEntries(source, document)[0] || null;
}

function focusedPuzzleLevel2dEntries(source, document) {
  const entries = [];
  const level3dRanges = typeof findLevels3Ranges === "function" ? findLevels3Ranges(source) || [] : [];
  if (typeof findLevelsRanges === "function" && typeof findLevelDefinitions === "function") {
    for (const range of findLevelsRanges(source) || []) {
      if (sourcePositionInsideRanges(range.start, level3dRanges)) {
        continue;
      }
      entries.push(...(findLevelDefinitions(source, range) || []).map((entry) => ({
        ...entry,
        namespace: range?.namespace || "",
      })));
    }
  }
  if (typeof findLevelSourceEntries === "function") {
    entries.push(...findLevelSourceEntries(source, document)
      .filter((entry) => !sourcePositionInsideRanges(entry.start, level3dRanges))
    );
  }
  return uniqueFocusedPuzzleEntries(entries);
}

function firstFocusedPuzzleLevel2dStart(source, document) {
  return firstFocusedPuzzleLevel2dEntry(source, document)?.start ?? null;
}

function firstFocusedPuzzleLevel3dEntry(source) {
  return focusedPuzzleLevel3dEntries(source)[0] || null;
}

function focusedPuzzleLevel3dEntries(source) {
  if (typeof findLevels3Ranges !== "function" || typeof findLevel3dDefinitions !== "function") {
    return [];
  }
  const entries = [];
  for (const range of findLevels3Ranges(source) || []) {
    entries.push(...(findLevel3dDefinitions(source, range) || []).map((entry) => ({
      ...entry,
      bundle: range?.bundle || "",
      model: range?.model || "",
    })));
  }
  return uniqueFocusedPuzzleEntries(entries);
}

function firstFocusedPuzzleLevel3dStart(source) {
  return firstFocusedPuzzleLevel3dEntry(source)?.start ?? null;
}

function firstFocusedPuzzleSprite2dEntry(source) {
  return focusedPuzzleSprite2dEntries(source)[0] || null;
}

function focusedPuzzleSprite2dEntries(source) {
  void source;
  return [];
}

function firstFocusedPuzzleSprite2dStart(source) {
  return firstFocusedPuzzleSprite2dEntry(source)?.start ?? null;
}

function firstFocusedPuzzleSprite3dEntry(source) {
  return focusedPuzzleSprite3dEntries(source)[0] || null;
}

function focusedPuzzleSprite3dEntries(source) {
  if (typeof findSprite3dDefinitions !== "function") {
    return [];
  }
  const blocks = typeof findSprites3dBlocks === "function"
    ? findSprites3dBlocks(source)
    : (typeof findSprites3dBlock === "function" ? [findSprites3dBlock(source)].filter(Boolean) : []);
  const entries = [];
  for (const block of blocks) {
    entries.push(...(findSprite3dDefinitions(source, block) || []));
  }
  return uniqueFocusedPuzzleEntries(entries);
}

function firstFocusedPuzzleSprite3dStart(source) {
  return firstFocusedPuzzleSprite3dEntry(source)?.start ?? null;
}

function loadFirstFocusedPuzzleEntry(kind, mode, context = focusedPuzzleSourceContext()) {
  return loadFocusedPuzzleEntry(kind, firstFocusedPuzzleEntry(kind, context), {
    silent: true,
    recordHistory: false,
  });
}

function currentFocused2dLevelEntry(context = focusedPuzzleSourceContext()) {
  if (!context?.document || context.document.id !== activeDocument()?.id) {
    return null;
  }
  const target = currentLevelSourceLocation();
  if (!target || target.document?.id !== context.document.id) {
    return null;
  }
  const levelIndex = Number.isInteger(target.levelIndex) ? target.levelIndex : currentEditableLevelIndex();
  return {
    dimension: "2d",
    target: {
      ...target,
      document: context.document,
      name: previewExport?.levels?.[levelIndex]?.name || target.name || "",
      levelIndex,
    },
  };
}

function focusedLevelEntryForPaneMode(mode, context = focusedPuzzleSourceContext()) {
  const normalizedMode = normalizePreviewMode(mode);
  const dimension = editorDimensionForPreviewMode(normalizedMode);
  if (normalizedMode === "edit" && dimension === "2d") {
    const current = currentFocused2dLevelEntry(context);
    if (current) {
      return current;
    }
  }
  const target = firstFocusedPuzzleEntryForDimension("level", dimension, context);
  return target ? { dimension, target } : null;
}

function loadLevelPaneEntryForMode(mode, context = focusedPuzzleSourceContext(), options = {}) {
  const entry = focusedLevelEntryForPaneMode(mode, context);
  if (!entry) {
    return false;
  }
  const targetMode = levelModeForEditorDimension(entry.dimension);
  openPreviewModePane(targetMode);
  return loadFocusedPuzzleEntry("level", entry, {
    silent: options.silent !== false,
    recordHistory: Boolean(options.recordHistory),
  });
}

function loadFocusedPuzzleEntry(kind, entry, options = {}) {
  if (!entry?.target || entry.target.document?.id !== activeDocument()?.id) {
    return false;
  }
  const dimension = normalizeEditorDimension(entry.dimension);
  const target = entry.target;
  const mode = kind === "sprite"
    ? spriteModeForEditorDimension(dimension)
    : levelModeForEditorDimension(dimension);
  if ((mode === "edit" || mode === "level3d") && kind !== "level") {
    return false;
  }
  if ((mode === "sprite" || mode === "sprite3d") && kind !== "sprite") {
    return false;
  }
  if (mode === "edit" && dimension === "2d") {
    currentLevelPaneMode = "edit";
    return finishFocusedPuzzleEntryLoad(loadLevelSourceTarget(target, { silent: options.silent !== false, recordHistory: Boolean(options.recordHistory) }));
  }
  if (mode === "level3d" && dimension === "3d" && typeof loadLevel3dSourceTarget === "function") {
    currentLevelPaneMode = "level3d";
    return finishFocusedPuzzleEntryLoad(loadLevel3dSourceTarget(target, { silent: options.silent !== false, recordHistory: Boolean(options.recordHistory), switchMode: true }));
  }
  if (mode === "sprite" && dimension === "2d" && typeof loadSpriteSourceTarget === "function") {
    currentSpritePaneMode = "sprite";
    return finishFocusedPuzzleEntryLoad(loadSpriteSourceTarget(target, { silent: options.silent !== false, recordHistory: Boolean(options.recordHistory), switchMode: true }));
  }
  if (mode === "sprite3d" && dimension === "3d" && typeof loadSprite3dSourceTarget === "function") {
    currentSpritePaneMode = "sprite3d";
    return finishFocusedPuzzleEntryLoad(loadSprite3dSourceTarget(target, { silent: options.silent !== false, recordHistory: Boolean(options.recordHistory), switchMode: true }));
  }
  return false;
}

function finishFocusedPuzzleEntryLoad(result) {
  const loaded = Boolean(result);
  if (loaded) {
    hideEditorHoverTooltip();
  }
  return loaded;
}

function focusedPuzzleTextDocument() {
  const document = activeDocument();
  return document && isPuzzleDocument(document) && isTextDocument(document) ? document : null;
}

function focusedPuzzleTextSource(document = focusedPuzzleTextDocument()) {
  return document?.id === activeDocument()?.id ? sourceEditor.value || "" : document?.source || "";
}

function applyFocusedPuzzleSourceChange(document, source) {
  document.source = source;
  if (document.id === activeDocument()?.id) {
    setSourceEditorValue(source, { resetUndo: false });
  }
  scheduleLocalSave();
  schedulePreview();
}

function defaultEmptyLevel2dSourceData() {
  const rows = Array.from({ length: 5 }, () => ".....");
  return { rows, localLegends: [] };
}

function insertLevelWithDefaultBlock(source, name, levelData, namespace = "") {
  const inserted = insertLevel(source, name, levelData, namespace);
  if (inserted) {
    return inserted;
  }
  const levelSource = levelDefinitionSource(name, levelData, "", { leadingBlank: false, bodyIndent: "" }).trimEnd();
  const prefix = source.trimEnd() ? `${source.trimEnd()}\n\n` : "";
  const namespaceSuffix = sanitizeLevelNamespace(namespace) ? ` ${sanitizeLevelNamespace(namespace)}` : "";
  return `${prefix}levels${namespaceSuffix} {\n${levelSource}\n}\n`;
}

function addEmptyLevel2dToFocusedSource() {
  const document = focusedPuzzleTextDocument();
  if (!document) {
    setStatus("No puzzle source for level", "is-error");
    return false;
  }
  const name = "";
  const sourceData = defaultEmptyLevel2dSourceData();
  const nextSource = insertLevelWithDefaultBlock(focusedPuzzleTextSource(document), name, sourceData, "");
  applyFocusedPuzzleSourceChange(document, nextSource);
  currentLevelPaneMode = "edit";
  openPreviewModePane("edit");
  setLevelNameInputs(name);
  level.width = 5;
  level.height = 5;
  level.regions = defaultLevelRegions(level.width, level.height);
  level.cells = makeEmptyCells(level.width, level.height);
  renderLevelBoard();
  renderLevelSourcePreview();
  applyPaneVisibility();
  setStatus("Added 2D level", "is-ok");
  hideEditorHoverTooltip();
  return true;
}

function defaultEmptyLevel3dSourceData() {
  return { rows: [LEVEL3D_EMPTY_CHAR], unknownCells: 0 };
}

function insertLevel3dWithDefaultBlock(source, name, levelData, bundle = "levels") {
  const inserted = typeof insertLevel3d === "function" ? insertLevel3d(source, name, levelData, bundle) : "";
  if (inserted) {
    return inserted;
  }
  const normalizedBundle = sanitizeLevel3dBundle(bundle || "levels");
  const levelSource = levelDefinition3dSource(name, levelData, "  ", { bodyIndent: "    " }).trimEnd();
  const prefix = source.trimEnd() ? `${source.trimEnd()}\n\n` : "";
  return `${prefix}levels3 ${normalizedBundle} {\n  legend {\n    ${LEVEL3D_EMPTY_CHAR} = empty\n  }\n\n${levelSource}\n}\n`;
}

function addEmptyLevel3dToFocusedSource() {
  const document = focusedPuzzleTextDocument();
  if (!document) {
    setPaneStatus("level", "No puzzle source for 3D level", "is-error");
    return false;
  }
  const name = "level_1";
  const bundle = "levels";
  const sourceData = defaultEmptyLevel3dSourceData();
  const nextSource = insertLevel3dWithDefaultBlock(focusedPuzzleTextSource(document), name, sourceData, bundle);
  applyFocusedPuzzleSourceChange(document, nextSource);
  currentLevelPaneMode = "level3d";
  openPreviewModePane("level3d");
  if (level3dNameInput) {
    level3dNameInput.value = name;
  }
  if (level3dBundleInput) {
    level3dBundleInput.value = bundle;
  }
  if (typeof level3d !== "undefined") {
    level3d.width = 1;
    level3d.depth = 1;
    level3d.height = 1;
    level3d.slice = 0;
    level3d.selectedChar = LEVEL3D_EMPTY_CHAR;
    level3d.palette = [{ char: LEVEL3D_EMPTY_CHAR, objects: [] }];
    level3d.slices = [[LEVEL3D_EMPTY_CHAR]];
    level3d.sourceDocumentId = document.id || "";
    level3d.sourceKey = "";
  }
  if (typeof renderLevel3dBuilder === "function") {
    renderLevel3dBuilder();
  }
  applyPaneVisibility();
  setPaneStatus("level", "Added 3D level", "is-ok");
  hideEditorHoverTooltip();
  return true;
}

function addEmptySprite2dToFocusedSource() {
  currentSpritePaneMode = "sprite";
  openPreviewModePane("sprite");
  if (typeof addEmptySpriteToSource === "function") {
    addEmptySpriteToSource();
  }
  applyPaneVisibility();
  hideEditorHoverTooltip();
}

function addEmptySprite3dToFocusedSource() {
  currentSpritePaneMode = "sprite3d";
  openPreviewModePane("sprite3d");
  if (typeof addEmptySprite3dToSource === "function") {
    addEmptySprite3dToSource();
  }
  applyPaneVisibility();
  hideEditorHoverTooltip();
}

function openLevelPaneForCurrentDimension() {
  const context = focusedPuzzleSourceContext();
  ensurePreviewTargetsActiveDocument();
  const mode = levelModeForEditorDimension(currentEditorDimension);
  let loaded = loadLevelPaneEntryForMode(mode, context, { silent: true, recordHistory: false });
  if (!loaded) {
    const first = firstFocusedPuzzleEntry("level", context);
    if (first) {
      openPreviewModePane(levelModeForEditorDimension(first.dimension));
      loaded = loadFocusedPuzzleEntry("level", first, { silent: true, recordHistory: false });
    }
  }
  if (!loaded) {
    openPreviewModePane(mode);
    currentLevelPaneMode = "none";
    applyPaneVisibility();
    hideEditorHoverTooltip();
  }
}

function openSpritePaneForCurrentDimension() {
  const context = focusedPuzzleSourceContext();
  const first = firstFocusedPuzzleEntry("sprite", context);
  const mode = first
    ? spriteModeForEditorDimension(first.dimension)
    : spriteModeForEditorDimension(currentEditorDimension);
  openPreviewModePane(mode);
  if (first) {
    loadFocusedPuzzleEntry("sprite", first, { silent: true, recordHistory: false });
  }
  return mode;
}

function editorDimensionForPreviewMode(mode) {
  if (mode === "level3d" || mode === "sprite3d") {
    return "3d";
  }
  if (mode === "edit" || mode === "sprite") {
    return "2d";
  }
  return currentEditorDimension;
}

function setEditorDimensionMode(dimension) {
  currentEditorDimension = normalizeEditorDimension(dimension);
  currentLevelPaneMode = levelModeForEditorDimension(currentEditorDimension);
  currentSpritePaneMode = spriteModeForEditorDimension(currentEditorDimension);

  if (currentPreviewMode === "edit" || currentPreviewMode === "level3d") {
    setPreviewMode(currentLevelPaneMode);
    return currentLevelPaneMode;
  }
  if (currentPreviewMode === "sprite" || currentPreviewMode === "sprite3d") {
    setPreviewMode(currentSpritePaneMode);
    return currentSpritePaneMode;
  }
  applyPaneVisibility();
  if (isPaneVisible("level")) {
    if (currentLevelPaneMode === "level3d") {
      renderLevel3dBuilder();
    } else {
      renderLevelBoard();
    }
  }
  if (isPaneVisible("sprite")) {
    if (currentSpritePaneMode === "sprite3d") {
      renderSprite3dBuilder();
    } else {
      renderSpriteBuilder();
    }
  }
  return currentPreviewMode;
}

function paneBindLabel(kind) {
  if (kind === "level") {
    if (currentLevelPaneMode === "edit") {
      return "2D";
    }
    if (currentLevelPaneMode === "level3d") {
      return "3D";
    }
    return "none";
  }
  if (currentSpritePaneMode === "sprite") {
    return "2D";
  }
  if (currentSpritePaneMode === "sprite3d") {
    return "3D";
  }
  return "none";
}

function syncPaneBindLabels() {
  if (levelPaneModeSwitch) {
    levelPaneModeSwitch.textContent = paneBindLabel("level");
  }
  if (spritePaneModeSwitch) {
    spritePaneModeSwitch.textContent = paneBindLabel("sprite");
  }
}

function syncPreviewModeButtonState() {
  const previewMode = normalizePreviewMode(currentPreviewMode);
  const paneVisible = isPaneVisible(workPaneIdForPreviewMode(previewMode));
  const spritePaneVisible = isPaneVisible("sprite");
  const dimensionLabel = editorDimensionLabel();
  playModeButton.classList.toggle("is-active", paneVisible && previewMode === "play");
  editModeButton.classList.toggle("is-active", isPaneVisible("level"));
  solverModeButton.classList.toggle("is-active", paneVisible && previewMode === "solver");
  spriteModeButton.classList.toggle("is-active", spritePaneVisible);
  sprite3dModeButton?.classList.toggle("is-active", spritePaneVisible && currentSpritePaneMode === "sprite3d");
  editModeButton.title = `Open ${dimensionLabel} level editor`;
  editModeButton.setAttribute("aria-label", `Open ${dimensionLabel} level editor`);
  spriteModeButton.title = `Open ${dimensionLabel} sprite editor`;
  spriteModeButton.setAttribute("aria-label", `Open ${dimensionLabel} sprite editor`);
  if (editorDimensionSwitch) {
    editorDimensionSwitch.dataset.mode = currentEditorDimension;
  }
  syncPaneBindLabels();
  for (const button of editorDimensionButtons) {
    const active = normalizeEditorDimension(button.dataset.editorDimension) === currentEditorDimension;
    button.classList.toggle("is-active", active);
    button.setAttribute("aria-pressed", String(active));
  }
  for (const button of levelPaneModeButtons) {
    const active = isPaneVisible("level") && button.dataset.levelPaneMode === currentLevelPaneMode;
    button.classList.toggle("is-active", active);
    button.setAttribute("aria-pressed", String(active));
  }
  for (const button of spritePaneModeButtons) {
    const active = spritePaneVisible && button.dataset.spritePaneMode === currentSpritePaneMode;
    button.classList.toggle("is-active", active);
    button.setAttribute("aria-pressed", String(active));
  }
  soundsTopbarButton.classList.toggle("is-active", paneVisible && previewMode === "sounds");
  psImportTopbarButton?.classList.toggle("is-active", paneVisible && previewMode === "psimport");
  docsTopbarButton?.classList.toggle("is-active", paneVisible && previewMode === "docs");
}

function setPreviewMode(mode, options = {}) {
  const wasLevelMode = isPaneVisible("level") || isPaneVisible("solver");
  const wasSpriteMode = currentPreviewMode === "sprite";
  const previewMode = normalizePreviewMode(mode);
  hideEditorHoverTooltip();
  if (previewMode !== "edit" && levelPlaytestActive) {
    stopLevelPlaytest({ syncPreview: false });
  }
  if (wasSpriteMode && previewMode !== "sprite" && sprite) {
    sprite.shapeTagPickerOpen = false;
  }
  if (!options.skipPaneSync) {
    showPreviewModePane(previewMode);
  }
  currentPreviewMode = previewMode;
  workbench.dataset.activePreviewMode = previewMode;
  workbench.dataset.activePreviewPane = workPaneIdForPreviewMode(previewMode);
  syncWorkbenchGridLayout();
  const editMode = previewMode === "edit";
  const level3dMode = previewMode === "level3d";
  const solverMode = previewMode === "solver";
  const enteringLevelMode = (editMode || solverMode) && !wasLevelMode;
  const spriteMode = previewMode === "sprite";
  const sprite3dMode = previewMode === "sprite3d";
  const soundsMode = previewMode === "sounds";
  const psImportMode = previewMode === "psimport";
  if (editMode || level3dMode) {
    currentEditorDimension = editorDimensionForPreviewMode(previewMode);
    currentLevelPaneMode = levelModeForEditorDimension(currentEditorDimension);
  } else if (spriteMode || sprite3dMode) {
    currentEditorDimension = editorDimensionForPreviewMode(previewMode);
    currentSpritePaneMode = spriteModeForEditorDimension(currentEditorDimension);
  }
  if (levelPaneModeSwitch) {
    levelPaneModeSwitch.hidden = !isPaneVisible("level");
  }
  if (spritePaneModeSwitch) {
    spritePaneModeSwitch.hidden = !isPaneVisible("sprite");
  }
  if (soundsHeaderTools) {
    soundsHeaderTools.hidden = !isPaneVisible("sounds");
  }
  syncPreviewModeButtonState();
  if (gamePaneTitle) {
    gamePaneTitle.textContent = "Preview";
  }
  if (runButton) {
    runButton.hidden = false;
  }
  applyPaneVisibility();
  ensureLevel3dPaneFrameWidth();
  syncPreviewViewportScale();
  scheduleBoardScaleSync(3);
  if (!isPaneVisible("sounds")) {
    stopSoundPlayback();
  }
  if (editMode) {
    resetLevelBuilderFromSource(false);
    if (enteringLevelMode || !level.cells.length) {
      loadLevelFromPreviewState();
    } else if (levelSolutionPreview) {
      clearSolutionPreview();
      renderLevelBoard();
    }
  }
  if (solverMode) {
    syncSolverLevelSelector();
    renderSolverBoard();
    updateSolutionControls();
  }
  if (spriteMode) {
    renderSpriteBuilder();
  }
  if (sprite3dMode) {
    renderSprite3dBuilder();
  }
  if (level3dMode) {
    renderLevel3dBuilder();
  }
  if (soundsMode) {
    renderSoundsBuilder();
  }
  if (previewMode === "play") {
    restoreCompiledGamePreview();
  }
}

function requestFocusedPreviewState() {
  if (!previewFrame?.contentWindow) {
    return false;
  }
  previewFrame.contentWindow.postMessage({ type: "PuzzleStudioRequestPreviewState" }, "*");
  return true;
}

function restoreCompiledGamePreview() {
  if (!previewFrameHasEditorLevelState || !latestHtml || !previewFrame) {
    return;
  }
  previewFrameHasEditorLevelState = false;
  latestPreviewState = null;
  setPreviewFrameHtml(editorPreviewDocument(latestHtml));
}

function activePreviewModeAcceptsLevelState() {
  return currentPreviewMode === "edit";
}

function resetLevelBuilderFromSource(resetCells = true) {
  levelDisplayCells = null;
  level.palette = levelPaletteFromExport(levelReferenceSource());
  level.activeLayer = normalizedLevelActiveLayer(level.activeLayer);
  const size = initialLevelSize();
  if (resetCells) {
    level.width = size.width || level.width;
    level.height = size.height || level.height;
    level.regions = defaultLevelRegions(level.width, level.height);
    level.cells = makeEmptyCells(level.width, level.height);
  }
  if (!level.palette.some((entry) => entry.id === level.selectedObjectId)) {
    level.selectedObjectId = level.palette[0]?.id ?? 0;
  }
  updateLevelSizeLabel();
  renderLevelPalette();
  renderLevelBoard();
}

function resetLevelBuilderFromPreviewSource() {
  resetLevelBuilderFromSource(false);
  if (!loadLevelFromPreviewState()) {
    resetLevelBuilderFromSource(true);
  }
}

function blockLines(source, name) {
  const block = findNamedBlock(source, name);
  if (!block) {
    return [];
  }
  return source.slice(block.bodyStart, block.bodyEnd).split("\n");
}

function titleLabel(value) {
  return String(value || "Tile")
    .replace(/[:_-]+/g, " ")
    .replace(/\s+/g, " ")
    .trim()
    .replace(/\b\w/g, (letter) => letter.toUpperCase());
}

function levelPaletteFromExport(source) {
  const placeableObjects = sourcePlaceableObjectNames(source, previewExport);
  const objects = engineObjects().filter((object) => placeableObjects.has(object.name));
  return [
    { id: 0, name: "Eraser", layer: null, sprite: "eraser" },
    ...objects,
  ];
}

function levelReferenceSource(exportData = previewExport) {
  return exportData?.source || activePreviewSource();
}

function sourcePlaceableObjectNames(source, exportData = previewExport) {
  return new Set(sourceCharEntries(source, exportData)
    .filter((entry) => entry.objects.length === 1)
    .map((entry) => entry.objects[0]));
}

function engineObjects(exportData = previewExport) {
  return [...(exportData?.engine?.objects || [])]
    .sort((left, right) => left.layer - right.layer || left.name.localeCompare(right.name));
}

function engineObjectById(objectId, exportData = previewExport) {
  return (exportData?.engine?.objects || []).find((object) => object.id === objectId) || null;
}

function isVisualObject(object, exportData = previewExport) {
  return (exportData?.engine?.visualObjects || []).includes(object.id);
}

function visualObjectNameSet(exportData = previewExport) {
  const visualIds = new Set(exportData?.engine?.visualObjects || []);
  return new Set((exportData?.engine?.objects || [])
    .filter((object) => visualIds.has(object.id))
    .map((object) => object.name));
}

function layerCount(exportData = previewExport) {
  return exportData?.engine?.layerCount
    || exportData?.levels?.[0]?.initialState?.layerCount
    || 1;
}

function sourceLayerNameEntries(source = levelReferenceSource(), exportData = previewExport) {
  const objectsByName = new Map(engineObjects(exportData).map((object) => [object.name, object]));
  const namesByLayer = new Map();
  for (const row of sourceLayerRows(source)) {
    const match = String(row || "").match(/^\s*(@?[_A-Za-z][_A-Za-z0-9]*)\s*=\s*(.+?)\s*$/);
    if (!match) {
      continue;
    }
    const [, name, expression] = match;
    for (const objectName of expression.trim().split(/\s+/)) {
      const object = objectsByName.get(objectName);
      if (object && Number.isInteger(object.layer) && !namesByLayer.has(object.layer)) {
        namesByLayer.set(object.layer, name);
      }
    }
  }
  return namesByLayer;
}

function sourceLayerRows(source) {
  const lines = sourceLinesWithOffsets(source);
  const rawLines = lines.map((line) => line.raw);
  const rows = [];

  for (let index = 0; index < lines.length; index += 1) {
    const section = sectionHeaderAtForWasm(rawLines, index);
    if (section?.block === "layers") {
      const result = collectSectionLayerRows(lines, rawLines, index + 3);
      rows.push(...result.rows);
      index = result.endIndex;
      continue;
    }

    const code = levelScannerCode(lines[index].raw);
    const tokens = splitLevelTokens(code);
    if (tokens[0] === "layers" && tokens.at(-1) === "{") {
      const result = collectLayerBlockRows(lines, index + 1);
      rows.push(...result.rows);
      index = result.endIndex;
    }
  }

  return rows;
}

function collectSectionLayerRows(lines, rawLines, startIndex) {
  const rows = [];
  let endIndex = startIndex - 1;
  for (let index = startIndex; index < lines.length; index += 1) {
    if (sectionHeaderAtForWasm(rawLines, index)) {
      break;
    }
    const code = levelScannerCode(lines[index].raw);
    const tokens = splitLevelTokens(code);
    if (code && sectionBoundaryForWasm("layers", tokens)) {
      break;
    }
    if (code) {
      rows.push(code);
    }
    endIndex = index;
  }
  return { rows, endIndex };
}

function collectLayerBlockRows(lines, startIndex) {
  const rows = [];
  let endIndex = startIndex - 1;
  for (let index = startIndex; index < lines.length; index += 1) {
    const code = levelScannerCode(lines[index].raw);
    if (code === "}" || code === "end") {
      endIndex = index;
      break;
    }
    if (code) {
      rows.push(code);
    }
    endIndex = index;
  }
  return { rows, endIndex };
}

function initialLevelSize() {
  const state = previewExport?.levels?.[currentEditableLevelIndex()]?.initialState;
  if (state?.width && state?.height) {
    return { width: state.width, height: state.height };
  }
  return { width: 9, height: 5 };
}

function currentEditableLevelIndex(exportData = previewExport) {
  return setActiveLevelIndex(activeLevelIndex, exportData);
}

function setActiveLevelIndex(index, exportData = previewExport) {
  const levels = exportData?.levels || [];
  if (!levels.length) {
    activeLevelIndex = 0;
    return 0;
  }
  const fallback = exportData.initialLevelIndex ?? 0;
  const rawIndex = index ?? fallback;
  activeLevelIndex = Math.max(0, Math.min(levels.length - 1, Math.trunc(Number(rawIndex) || 0)));
  return activeLevelIndex;
}

function currentSolverLevelIndex(exportData = previewExport) {
  return setSolverLevelIndex(solverLevelIndex, exportData);
}

function setSolverLevelIndex(index, exportData = previewExport) {
  const levels = exportData?.levels || [];
  if (!levels.length) {
    solverLevelIndex = 0;
    return 0;
  }
  const fallback = exportData.initialLevelIndex ?? 0;
  const rawIndex = index ?? fallback;
  solverLevelIndex = Math.max(0, Math.min(levels.length - 1, Math.trunc(Number(rawIndex) || 0)));
  return solverLevelIndex;
}

function clearSolverTargetOverride() {
  solverStateOverride = null;
  solverSceneOverride = null;
  solverPuzzle3dSnapshotOverride = null;
  stagedSolverCells = null;
}

function setSolverTargetFromState({ exportData = previewExport, levelIndex = 0, stateData = null, scene = null, puzzle3dSnapshot = null } = {}) {
  const targetIndex = setSolverLevelIndex(levelIndex, exportData);
  solverStateOverride = stateData ? JSON.parse(JSON.stringify(stateData)) : null;
  solverSceneOverride = scene ? JSON.parse(JSON.stringify(scene)) : null;
  solverPuzzle3dSnapshotOverride = puzzle3dSnapshot ? JSON.parse(JSON.stringify(puzzle3dSnapshot)) : null;
  stagedSolverCells = solverSceneOverride ? sceneCellsToSlots(solverSceneOverride, []) : null;
  syncSolverLevelSelector(exportData);
  return targetIndex;
}

function solverLevelEntries(exportData = previewExport || extractPreviewExport(latestHtml)) {
  return Array.isArray(exportData?.levels) ? exportData.levels : [];
}

function solverLevelDisplayName(levelEntry, index) {
  return levelEntry?.label || levelEntry?.name || `Level ${index + 1}`;
}

function syncSolverLevelSelector(exportData = previewExport || extractPreviewExport(latestHtml)) {
  if (!solverLevelSelect) {
    return;
  }
  const levels = solverLevelEntries(exportData);
  const selectedIndex = levels.length ? currentSolverLevelIndex(exportData) : 0;
  solverLevelSelect.replaceChildren();
  if (!levels.length) {
    const option = document.createElement("option");
    option.value = "0";
    option.textContent = "No levels";
    solverLevelSelect.append(option);
    solverLevelSelect.disabled = true;
    return;
  }
  levels.forEach((levelEntry, index) => {
    const option = document.createElement("option");
    option.value = String(index);
    option.textContent = `${index + 1}. ${solverLevelDisplayName(levelEntry, index)}`;
    solverLevelSelect.append(option);
  });
  solverLevelSelect.value = String(selectedIndex);
  solverLevelSelect.disabled = Boolean(activeLevelSolveRequest);
}

function selectSolverLevel(index, options = {}) {
  const exportData = previewExport || extractPreviewExport(latestHtml);
  const levels = solverLevelEntries(exportData);
  if (!levels.length) {
    syncSolverLevelSelector(exportData);
    return false;
  }
  const levelIndex = setSolverLevelIndex(index, exportData);
  clearSolverTargetOverride();
  clearSolutionPreview();
  renderSolverBoard();
  syncSolverLevelSelector(exportData);
  if (!options.silent) {
    setLevelSolveStatus(`Selected ${solverLevelDisplayName(levels[levelIndex], levelIndex)}`, "");
  }
  return true;
}

function levelRows(source) {
  const block = findNamedBlock(source, "levels");
  if (!block) {
    return [];
  }
  return source
    .slice(block.bodyStart, block.bodyEnd)
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line && !line.includes("{") && !line.includes("}") && !line.includes("="));
}

function loadLevelFromSourceClick(event = null) {
  if (event?.defaultPrevented) {
    return;
  }
  if (!isPuzzleDocument(activeDocument()) || !isTextDocument(activeDocument())) {
    return;
  }
  const source = sourceEditor.value || "";
  const clickOffset = sourceOffsetFromEditorClick(event, source);
  const position = clickOffset ?? sourceEditor.selectionStart;
  if (typeof loadSourceEditableTargetFromPosition === "function") {
    const key = loadSourceEditableTargetFromPosition(position);
    if (key) {
      event?.preventDefault();
      event?.stopImmediatePropagation?.();
    }
  }
}

function loadLevelFromSourcePosition(position, options = {}) {
  if (!isPuzzleDocument(activeDocument()) || !isTextDocument(activeDocument())) {
    return null;
  }
  const source = sourceEditor.value || "";
  const entry = findLevelDefinitionAtPosition(source, position);
  if (!entry) {
    return null;
  }
  ensurePreviewTargetsActiveDocument();
  if (!previewExport?.levels?.length) {
    if (!options.silent) {
      setStatus("No preview level to edit", "is-error");
    }
    return null;
  }
  if (options.recordHistory) {
    pushSourceNavigationHistory();
  }
  openPreviewModePane("edit");
  const levelIndex = setActiveLevelIndex(previewLevelIndexForSourceEntry(entry));
  const levelName = previewExport?.levels?.[levelIndex]?.name || entry.name || `level_${levelIndex + 1}`;
  if (!loadLevelFromSourceEntry(source, entry, { levelIndex, levelName })) {
    loadLevelFromPreviewState();
  }
  setLevelNameInputs(editableLevelNameForSourceEntry(entry, levelName));
  if (!options.silent) {
    setStatus(`Loaded level ${levelName}`, "is-ok");
  }
  return `level:${levelIndex}:${levelName}`;
}

async function resolveSourceTargetFromWasm(source, position) {
  const compiler = await loadWasmCompiler();
  if (typeof compiler?.resolve_source_target !== "function") {
    return null;
  }
  const cursorByteOffset = sourceByteOffset(source, position);
  const raw = compiler.resolve_source_target(source, cursorByteOffset);
  const payload = JSON.parse(raw || "{}");
  return normalizeResolvedSourceTarget(source, payload?.target || null, position);
}

function normalizeResolvedSourceTarget(source, target, position = null) {
  if (!target || typeof target !== "object") {
    return null;
  }
  const normalized = { ...target };
  for (const key of ["start", "end", "bodyStart", "bodyEnd"]) {
    if (Number.isInteger(normalized[key])) {
      normalized[key] = sourceUtf16OffsetFromByteOffset(source, normalized[key]);
    }
  }
  if (normalized.kind === "sprite") {
    const sprite3dTarget = sourceSprite3dTargetAtPosition(
      source,
      Number.isInteger(position) ? position : normalized.start,
    );
    if (sprite3dTarget) {
      return sprite3dTarget;
    }
  }
  return normalized;
}

function sourceSprite3dTargetAtPosition(source, position) {
  if (typeof findSprite3dDefinitionAtPosition !== "function") {
    return null;
  }
  const entry = findSprite3dDefinitionAtPosition(source, position);
  if (!entry) {
    return null;
  }
  return {
    kind: "sprite3d",
    name: entry.name,
    start: entry.start,
    end: entry.end,
    bodyStart: entry.bodyStart,
    bodyEnd: entry.bodyEnd,
  };
}

function loadLevelSourceTarget(target, options = {}) {
  if (!isPuzzleDocument(activeDocument()) || !isTextDocument(activeDocument())) {
    return null;
  }
  ensurePreviewTargetsActiveDocument();
  if (!previewExport?.levels?.length) {
    if (!options.silent) {
      setStatus("No preview level to edit", "is-error");
    }
    return null;
  }
  const levels = previewExport.levels || [];
  let levelIndex = previewLevelIndexForSourceEntry(target);
  if (!levels[levelIndex]) {
    return null;
  }
  if (options.recordHistory) {
    pushSourceNavigationHistory();
  }
  openPreviewModePane("edit");
  levelIndex = setActiveLevelIndex(levelIndex);
  const levelName = levels[levelIndex]?.name || target.name || `level_${levelIndex + 1}`;
  const source = sourceEditor.value || "";
  const sourceEntry = sourceEditableEntryFromTarget(source, target, {
    find: findLevelDefinitionAtPosition,
    defaultName: "level_1",
  });
  if (!loadLevelFromSourceEntry(source, sourceEntry, { levelIndex, levelName })) {
    loadLevelFromPreviewState();
  }
  setLevelNameInputs(editableLevelNameForSourceEntry(sourceEntry, levelName));
  if (!options.silent) {
    setStatus(`Loaded level ${levelName}`, "is-ok");
  }
  return `level:${levelIndex}:${levelName}`;
}

function loadLevelFromSourceEntry(source, entry, options = {}) {
  const exportData = previewExport || extractPreviewExport(latestHtml);
  const state = sourceLevelStateFromEntry(source, entry, exportData, options);
  if (!state) {
    return false;
  }
  clearSolutionPreview();
  stopLevelPlaytest({ syncPreview: false });
  levelDisplayCells = null;
  level.width = state.width;
  level.height = state.height;
  level.regions = state.regions;
  level.cells = state.cells;
  renderLevelBoard();
  sendLevelStateToPreview(options.levelIndex ?? currentEditableLevelIndex(exportData), levelStateData(exportData), {
    materializeLevelStart: false,
    materializeDisplay: false,
    silent: true,
  });
  return true;
}

function sourceLevelStateFromEntry(source, entry, exportData = previewExport, options = {}) {
  if (!entry || !exportData?.engine?.objects?.length) {
    return null;
  }
  const parsed = sourceLevelRowsAndLocalLegends(source, entry);
  if (!parsed.rows.length) {
    return null;
  }
  const charEntries = [
    ...sourceCharEntries(source, exportData),
    ...parsed.localLegends.map((row) => legendEntryFromRow(row, new Set(engineObjects(exportData).map((object) => object.name)))).filter(Boolean),
  ];
  const charMap = new Map(charEntries.map((charEntry) => [charEntry.char, charEntry.objects]));
  const groups = sourceLevelRowGroups(parsed.rows);
  if (!groups.length) {
    return null;
  }
  const regions = [];
  let width = 0;
  let height = 0;
  for (const group of groups) {
    const regionWidth = Math.max(1, ...group.map((row) => [...row].length));
    const regionHeight = Math.max(1, group.length);
    regions.push({ index: regions.length, x: width, y: 0, width: regionWidth, height: regionHeight });
    width += regionWidth;
    height = Math.max(height, regionHeight);
  }
  const cells = makeEmptyCells(width, height, exportData);
  const objectIdsByName = new Map(engineObjects(exportData).map((object) => [object.name, object.id]));
  for (const [regionIndex, group] of groups.entries()) {
    const region = regions[regionIndex];
    for (let y = 0; y < group.length; y += 1) {
      const chars = [...group[y]];
      for (let x = 0; x < region.width; x += 1) {
        const char = chars[x] ?? ".";
        const objects = /\s/.test(char) ? [] : charMap.get(char);
        if (!objects) {
          return null;
        }
        const slots = makeEmptyCell(exportData);
        for (const objectName of objects) {
          const objectId = objectIdsByName.get(objectName) || 0;
          const object = engineObjectById(objectId, exportData);
          if (!object) {
            return null;
          }
          slots[object.layer] = object.id;
        }
        cells[((region.y + y) * width) + region.x + x] = slots;
      }
    }
  }
  return {
    width,
    height,
    regions: normalizedLevelRegions(regions, width, height),
    cells,
  };
}

function sourceLevelRowsAndLocalLegends(source, entry) {
  const lines = sourceLinesWithOffsets(String(source || "").slice(entry.start, entry.end));
  const rows = [];
  const localLegends = [];
  let sawMapRow = false;
  const firstTokens = splitLevelTokens(levelScannerCode(lines[0]?.raw || ""));
  let index = sourceLevelEntryHasHeader(firstTokens) ? 1 : 0;
  while (index < lines.length) {
    const code = levelScannerCode(lines[index].raw);
    if (!code) {
      if (sawMapRow && rows.at(-1) !== "") {
        rows.push("");
      }
      index += 1;
      continue;
    }
    const normalized = braceNormalizedLineForSectionForWasm(code);
    const tokens = splitLevelTokens(normalized);
    if (tokens[0] === "legend") {
      if (tokens.length > 1) {
        localLegends.push(code.slice("legend".length).trim());
        index += 1;
      } else {
        const result = collectLegendBlockRows(lines, index + 1, []);
        localLegends.push(...result.rows);
        index = Math.max(index + 1, result.endIndex + 1);
      }
      continue;
    }
    if (isLevelLifecycleHeader(tokens) || startsLevelBodyBlock(tokens, normalized)) {
      index = skipLevelBodySourceBlock(lines, index);
      continue;
    }
    if (isLevelEventSugarCode(code) || normalized === "}" || normalized === "end") {
      index += 1;
      continue;
    }
    rows.push(code);
    sawMapRow = true;
    index += 1;
  }
  while (rows.at(-1) === "") {
    rows.pop();
  }
  return { rows, localLegends };
}

function sourceLevelEntryHasHeader(tokens) {
  return tokens[0] === "level"
    || (tokens.length === 1 && tokens[0] === "{")
    || (tokens.at(-1) === "{" && tokens[0] !== "legend");
}

function sourceLevelRowGroups(rows) {
  const groups = [];
  let current = [];
  for (const row of rows) {
    if (String(row || "").trim() === "") {
      if (current.length) {
        groups.push(current);
        current = [];
      }
      continue;
    }
    current.push(String(row || ""));
  }
  if (current.length) {
    groups.push(current);
  }
  return groups;
}

function previewLevelIndexForSourceEntry(entry, exportData = previewExport) {
  const levels = exportData?.levels || [];
  const requestedName = String(entry?.name || "").trim();
  const rawIndex = Number.isInteger(entry?.levelIndex) ? entry.levelIndex : -1;
  const indexed = levels[rawIndex] || null;
  if (indexed && (!requestedName || sourceTitleMatches(requestedName, indexed.name))) {
    return rawIndex;
  }
  if (requestedName) {
    const byName = levels.findIndex((level) => sourceTitleMatches(requestedName, level.name));
    if (byName >= 0) {
      return byName;
    }
  }
  return Math.max(0, Math.min(levels.length - 1, rawIndex));
}

function loadResolvedSourceTarget(target, options = {}) {
  if (!target?.kind) {
    return null;
  }
  if (target.kind === "level3d" && typeof loadLevel3dSourceTarget === "function") {
    return loadLevel3dSourceTarget(target, options);
  }
  if (target.kind === "level") {
    return loadLevelSourceTarget(target, options);
  }
  if (target.kind === "sprite" && typeof loadSpriteSourceTarget === "function") {
    return loadSpriteSourceTarget(target, options);
  }
  if (target.kind === "sprite3d" && typeof loadSprite3dSourceTarget === "function") {
    return loadSprite3dSourceTarget(target, options);
  }
  if (target.kind === "sounds" && typeof loadSoundSourceTarget === "function") {
    return loadSoundSourceTarget(target, options);
  }
  return null;
}

function finishSourceTargetSync(key, options = {}) {
  if (!key) {
    sourceCursorPreviewKey = "";
    return false;
  }
  if (!options.force && key === sourceCursorPreviewKey) {
    return true;
  }
  sourceCursorPreviewKey = key;
  return true;
}

function syncPreviewModeFromSourceCursor(options = {}) {
  const document = activeDocument();
  sourceTargetRequestId += 1;
  const requestId = sourceTargetRequestId;
  if (!isPuzzleDocument(document) || !isTextDocument(document)) {
    sourceCursorPreviewKey = "";
    return false;
  }
  if (!options.allowInactiveMode && !["edit", "level3d", "sprite", "sprite3d", "sounds"].includes(currentPreviewMode)) {
    sourceCursorPreviewKey = "";
    return false;
  }
  const source = sourceEditor.value || "";
  const documentId = document.id || "";
  const position = Math.max(
    0,
    Math.min(source.length, Math.trunc(Number(options.position ?? sourceEditor.selectionStart) || 0)),
  );
  const loadOptions = {
    silent: true,
    switchMode: true,
    recordHistory: options.recordHistory === true,
  };
  resolveSourceTargetFromWasm(source, position)
    .then((target) => {
      if (
        requestId !== sourceTargetRequestId
        || documentId !== (activeDocument()?.id || "")
        || source !== (sourceEditor.value || "")
      ) {
        return false;
      }
      const key = target ? loadResolvedSourceTarget(target, loadOptions) || "" : "";
      return finishSourceTargetSync(key, options);
    })
    .catch((error) => {
      if (
        requestId !== sourceTargetRequestId
        || documentId !== (activeDocument()?.id || "")
        || source !== (sourceEditor.value || "")
      ) {
        return false;
      }
      sourceCursorPreviewKey = "";
      setStatus(`Source target sync failed: ${userFacingRuntimeError(error)}`, "is-error");
      return false;
    });
  return false;
}

function syncPreviewModeFromSourcePointer(event) {
  const source = sourceEditor.value || "";
  const clickOffset = sourceOffsetFromEditorClick(event, source);
  return syncPreviewModeFromSourceCursor(Number.isInteger(clickOffset)
    ? { position: clickOffset }
    : {});
}

function syncSourceFromPreviewPane(mode = currentPreviewMode, options = {}) {
  if (!isTextDocument(activePreviewDocument())) {
    return false;
  }
  const target = sourceLocationForPreviewPane(mode);
  if (!target) {
    return false;
  }
  const key = `${mode}:${target.key}`;
  if (!options.force && key === previewPaneSourceKey) {
    return true;
  }
  if (!revealSourceLocation(target, { revealPane: options.revealPane === true })) {
    return false;
  }
  previewPaneSourceKey = key;
  return true;
}

function sourceLocationForPreviewPane(mode) {
  if (mode === "edit" || mode === "solver") {
    return currentLevelSourceLocation();
  }
  if (mode === "level3d") {
    return currentLevel3dSourceLocation();
  }
  if (mode === "sprite") {
    return currentSpriteSourceLocation();
  }
  if (mode === "sprite3d") {
    return currentSprite3dSourceLocation();
  }
  if (mode === "sounds") {
    return currentSoundSourceLocation();
  }
  return null;
}

function revealSourceLocation(target, options = {}) {
  if (!target?.document) {
    return false;
  }
  if (options.revealPane === false && !isPaneVisible(SOURCE_WORK_PANE_ID)) {
    return false;
  }
  if (options.recordHistory !== false) {
    pushSourceNavigationHistory();
  }
  if (options.revealPane !== false) {
    revealCodePane();
  }
  const preservedMode = currentPreviewMode;
  const preservedLevelIndex = activeLevelIndex;
  const index = documents.findIndex((document) => document.id === target.document.id);
  if (index >= 0 && index !== currentDocumentIndex) {
    persistCurrentDocument();
    loadEmbeddedDocument(index);
    if (preservedMode === "edit" || preservedMode === "solver") {
      setActiveLevelIndex(Number.isInteger(target.levelIndex) ? target.levelIndex : preservedLevelIndex);
      loadLevelFromPreviewState({ requestRender: false });
    }
  }
  const start = Math.max(0, Math.min(sourceEditor.value.length, target.start || 0));
  sourceEditor.setSelectionRange(start, start);
  scrollSourceEditorToPosition(start);
  if (typeof updateSourceMeta === "function") {
    updateSourceMeta();
  }
  return true;
}

function scrollSourceEditorToPosition(position) {
  const source = sourceEditor.value || "";
  const lines = editorSourceLinesWithOffsets(source);
  const lineIndex = Math.max(0, lines.findIndex((line) => position >= line.start && position <= line.absoluteEnd));
  const style = window.getComputedStyle(sourceEditor);
  const lineHeight = Number.parseFloat(style.lineHeight) || 20;
  const paddingTop = Number.parseFloat(style.paddingTop) || 0;
  const targetTop = paddingTop + lineIndex * lineHeight;
  sourceEditor.scrollTop = Math.max(0, targetTop - sourceEditor.clientHeight * 0.28);
  sourceEditor.scrollLeft = 0;
  if (typeof syncSourceHighlightScroll === "function") {
    syncSourceHighlightScroll();
  }
}

function currentLevelSourceLocation() {
  const levelIndex = currentEditableLevelIndex();
  const levelName = previewExport?.levels?.[levelIndex]?.name || "";
  const allEntries = [];
  for (const document of puzzleTextDocuments()) {
    const source = sourceForDocument(document);
    const entries = findLevelSourceEntries(source, document);
    allEntries.push(...entries);
    const entry = levelName
      ? entries.find((candidate) => sourceTitleMatches(candidate.name, levelName))
      : null;
    if (entry) {
      return {
        document: entry.document,
        start: entry.start,
        end: entry.end,
        levelIndex,
        key: `${entry.document.id}:level:${levelIndex}:${levelName}:${entry.start}`,
      };
    }
  }
  const fallback = allEntries[levelIndex] || null;
  if (fallback) {
    return {
      document: fallback.document,
      start: fallback.start,
      end: fallback.end,
      levelIndex,
      key: `${fallback.document.id}:level:${levelIndex}:${levelName}:${fallback.start}`,
    };
  }
  return null;
}

function findLevelSourceEntries(source, document) {
  const entries = [];
  const seen = new Set();
  if (typeof findLevelsRanges === "function" && typeof findLevelDefinitions === "function") {
    for (const range of findLevelsRanges(source) || []) {
      for (const entry of findLevelDefinitions(source, range) || []) {
        const key = `${entry.start}:${entry.end}`;
        if (seen.has(key)) {
          continue;
        }
        seen.add(key);
        entries.push({
          document,
          name: entry.name || "",
          sourceName: Object.prototype.hasOwnProperty.call(entry, "sourceName") ? entry.sourceName : entry.name || "",
          namespace: entry.namespace || "",
          start: entry.start,
          end: entry.end,
          levelIndex: entry.levelIndex,
        });
      }
    }
  }

  const lines = editorSourceLinesWithOffsets(source);
  for (const line of lines) {
    const code = stripLineCommentForWasm(line.raw).trim();
    const match = code.match(/^level(?:\s+(.+?))?\s*(?:\{|$)/);
    if (!match) {
      continue;
    }
    const start = firstEditorSourceCodeIndex(line);
    const key = `${start}:${line.absoluteEnd}`;
    if (seen.has(key)) {
      continue;
    }
    seen.add(key);
    const rawName = String(match[1] || "").trim();
    entries.push({
      document,
      name: rawName.replace(/\s*\{\s*$/, ""),
      start,
      end: line.absoluteEnd,
    });
  }
  return entries;
}

function currentSpriteSourceLocation() {
  if (!Number.isInteger(sprite.editSourceStart) || !Number.isInteger(sprite.editSourceEnd)) {
    return null;
  }
  for (const document of puzzleTextDocuments()) {
    if (document.id === sprite.editDocumentId) {
      return {
        document,
        start: sprite.editSourceStart,
        end: sprite.editSourceEnd,
        key: `${document.id}:sprite:${sprite.editSourceName || ""}:${sprite.editSourceStart}`,
      };
    }
  }
  return null;
}

function currentSprite3dSourceLocation() {
  if (typeof findSprites3dBlock !== "function" || typeof findSprite3dDefinitionBlock !== "function") {
    return null;
  }
  const name = typeof sprite3dObjectName === "function" ? sprite3dObjectName() : "";
  if (!name) {
    return null;
  }
  for (const document of puzzleTextDocuments()) {
    const source = sourceForDocument(document);
    const entry = typeof findSprite3dDefinitionByName === "function"
      ? findSprite3dDefinitionByName(source, name)
      : (() => {
        const block = findSprites3dBlock(source);
        return block ? findSprite3dDefinitionBlock(source, block, name) : null;
      })();
    if (entry) {
      return {
        document,
        start: entry.start,
        end: entry.end,
        key: `${document.id}:sprite3d:${name}:${entry.start}`,
      };
    }
  }
  return null;
}

function currentSoundSourceLocation() {
  const kind = sounds?.mode === "music" ? "music" : "sfx";
  const titleInput = kind === "music" ? soundsMusicTitleInput : soundsSfxTitleInput;
  const fallback = kind === "music" ? "music" : "sfx";
  const name = typeof soundIdentifierAtom === "function"
    ? soundIdentifierAtom(titleInput?.value, fallback)
    : String(titleInput?.value || fallback).trim();
  if (!kind || !name) {
    return null;
  }
  for (const document of puzzleTextDocuments()) {
    const source = sourceForDocument(document);
    const entry = findSoundsDefinitionByName(source, kind, name);
    if (entry) {
      return {
        document,
        start: entry.start,
        end: entry.end,
        key: `${document.id}:sounds:${kind}:${name}:${entry.start}`,
      };
    }
  }
  return null;
}

function findSoundsDefinitionByName(source, kind, name) {
  const lines = editorSourceLinesWithOffsets(source);
  for (const line of lines) {
    const parsed = typeof parseSoundsDefinitionLine === "function"
      ? parseSoundsDefinitionLine(line.raw)
      : null;
    if (parsed?.kind === kind && parsed?.name === name) {
      return { start: firstEditorSourceCodeIndex(line), end: line.absoluteEnd };
    }
  }
  return null;
}

function puzzleTextDocuments() {
  return documents.filter((document) => isPuzzleDocument(document) && isTextDocument(document));
}

function sourceForDocument(document) {
  return document?.id === activeDocument()?.id && isTextDocument(document)
    ? sourceEditor.value || ""
    : document?.source || "";
}

function replaceEditorSourceRangePreservingLineBoundary(source, start, end, replacement) {
  const text = String(source || "");
  const safeStart = Math.max(0, Math.min(text.length, start || 0));
  const safeEnd = Math.max(safeStart, Math.min(text.length, end || safeStart));
  const removed = text.slice(safeStart, safeEnd);
  const trailingBoundary = removed.match(/((?:\r?\n[\t ]*)+)$/)?.[1] || "";
  const replacementText = String(replacement || "");
  let suffix = text.slice(safeEnd);
  let boundary = "";
  if (trailingBoundary && (suffix || !replacementText.endsWith(trailingBoundary))) {
    boundary = trailingBoundary;
  } else if (suffix && !suffix.startsWith("\n") && !suffix.startsWith("\r") && !/[\n\r]$/.test(replacementText)) {
    boundary = "\n";
  }
  return `${text.slice(0, safeStart)}${replacementText}${boundary}${suffix}`;
}

function editorSourceLinesWithOffsets(source) {
  const lines = [];
  let start = 0;
  const text = String(source || "");
  for (const raw of text.split("\n")) {
    const end = start + raw.length;
    const hasNewline = end < text.length;
    lines.push({
      raw,
      text: hasNewline ? `${raw}\n` : raw,
      start,
      end,
      absoluteEnd: end + (hasNewline ? 1 : 0),
      hasNewline,
    });
    start = end + 1;
  }
  return lines;
}

function firstEditorSourceCodeIndex(line) {
  const offset = String(line?.raw || "").search(/\S/);
  return (line?.start || 0) + Math.max(0, offset);
}

function sourceOffsetFromEditorClick(event, source) {
  if (!event || !sourceEditorWrap?.contains(event.target)) {
    return null;
  }
  if (typeof sourceOffsetFromVisualPoint !== "function") {
    throw new Error("Source visual offset mapper is unavailable.");
  }
  return sourceOffsetFromVisualPoint(event.clientX, event.clientY, source);
}

function findLevelDefinitionAtPosition(source, position) {
  const levelsRange = findLevelsRangeAtPosition(source, position);
  if (levelsRange) {
    const entry = findLevelDefinitions(source, levelsRange)
      .find((entry) => position >= entry.start && position <= entry.end) || null;
    return entry || findLevelHeaderAtPosition(source, position);
  }
  return findStandaloneLevelDefinitionAtPosition(source, position)
    || findLevelHeaderAtPosition(source, position);
}

function findLevelHeaderAtPosition(source, position) {
  const lines = sourceLinesWithOffsets(source);
  const lineIndex = sourceLineIndexAtOffset(lines, position);
  const line = lines[lineIndex];
  if (!line || position < line.start || position > line.end) {
    return null;
  }
  const code = levelScannerCode(line.raw);
  const tokens = splitLevelTokens(code);
  if (tokens[0] !== "level") {
    return null;
  }
  const nameTokens = tokens.at(-1) === "{" ? tokens.slice(1, -1) : tokens.slice(1);
  let levelIndex = 0;
  for (const previous of lines.slice(0, lineIndex)) {
    const previousTokens = splitLevelTokens(levelScannerCode(previous.raw));
    if (previousTokens[0] === "level") {
      levelIndex += 1;
    }
  }
  return {
    name: levelNameFromTokens(nameTokens),
    start: firstCodeIndex(line),
    end: line.absoluteEnd,
    nextIndex: lineIndex + 1,
    levelIndex,
  };
}

function findLevelsRangeAtPosition(source, position) {
  const ranges = findLevelsRanges(source);
  return ranges.find((range) => position >= range.bodyStart && position <= range.bodyEnd) || null;
}

function findLevelsRanges(source) {
  const lines = sourceLinesWithOffsets(source);
  const rawLines = lines.map((line) => line.raw);
  const ranges = [];

  for (let index = 0; index < lines.length; index += 1) {
    const section = sectionHeaderAtForWasm(rawLines, index);
    if (section?.block === "levels") {
      ranges.push({
        headerStart: lines[index].start,
        bodyStart: lines[index + 2].end + (lines[index + 2].hasNewline ? 1 : 0),
        bodyEnd: findSectionLevelsEnd(lines, rawLines, index + 3),
        indent: "",
        namespace: "",
      });
      index += 2;
      continue;
    }

    const code = levelScannerCode(lines[index].raw);
    const tokens = splitLevelTokens(code);
    if (tokens[0] === "levels" && tokens.at(-1) === "{") {
      const openIndex = source.indexOf("{", lines[index].start);
      const closeIndex = findMatchingBrace(source, openIndex);
      if (openIndex >= 0 && closeIndex >= 0) {
        ranges.push({
          headerStart: lines[index].start,
          bodyStart: openIndex + 1,
          bodyEnd: closeIndex,
          indent: lineIndent(lines[index].raw),
          namespace: levelsNamespaceFromTokens(tokens),
        });
      }
      continue;
    }

    if (
      tokens.length >= 1
      && tokens.length <= 2
      && tokens[0] === "levels"
      && !isSectionTitleLine(rawLines, index)
    ) {
      ranges.push({
        headerStart: lines[index].start,
        bodyStart: lines[index].end + (lines[index].hasNewline ? 1 : 0),
        bodyEnd: findEndDelimitedLevelsEnd(lines, index + 1),
        indent: lineIndent(lines[index].raw),
        namespace: levelsNamespaceFromTokens(tokens),
      });
      continue;
    }
  }
  return ranges;
}

function findSectionLevelsEnd(lines, rawLines, startIndex) {
  let nestedDepth = 0;
  for (let index = startIndex; index < lines.length; index += 1) {
    if (nestedDepth === 0 && sectionHeaderAtForWasm(rawLines, index)) {
      return lines[index].start;
    }
    const code = levelScannerCode(lines[index].raw);
    if (!code) {
      continue;
    }
    const normalized = braceNormalizedLineForSectionForWasm(code);
    const tokens = splitLevelTokens(normalized);
    if (nestedDepth === 0) {
      if (normalized === "}") {
        return lines[index].start;
      }
    }
    if (normalized === "end" || normalized === "}") {
      nestedDepth = Math.max(0, nestedDepth - 1);
    } else if (startsLevelNestedBlock(tokens, normalized)) {
      nestedDepth += 1;
    }
  }
  return lines.at(-1)?.absoluteEnd ?? 0;
}

function findEndDelimitedLevelsEnd(lines, startIndex) {
  let nestedDepth = 0;
  for (let index = startIndex; index < lines.length; index += 1) {
    const code = levelScannerCode(lines[index].raw);
    if (!code) {
      continue;
    }
    const normalized = braceNormalizedLineForSectionForWasm(code);
    const tokens = splitLevelTokens(normalized);
    if (normalized === "end") {
      if (nestedDepth === 0) {
        return lines[index].start;
      }
      nestedDepth -= 1;
    } else if (startsLevelNestedBlock(tokens, normalized)) {
      nestedDepth += 1;
    }
  }
  return lines.at(-1)?.absoluteEnd ?? 0;
}

function findLevelDefinitions(source, levelsRange) {
  const lines = sourceLinesWithOffsets(source);
  const entries = [];
  let index = lines.findIndex((line) => line.absoluteEnd >= levelsRange.bodyStart);
  if (index < 0) {
    return entries;
  }

  while (index < lines.length && lines[index].start <= levelsRange.bodyEnd) {
    const line = lines[index];
    if (line.start < levelsRange.bodyStart) {
      index += 1;
      continue;
    }
    const code = levelScannerCode(line.raw);
    if (!code) {
      index += 1;
      continue;
    }
    const tokens = splitLevelTokens(code);
    if (tokens[0] === "legend") {
      const result = collectLegendBlockRows(lines, index + 1, []);
      index = Math.max(index + 1, result.endIndex + 1);
      continue;
    }
    if (isLevelsSectionBoundary(tokens) || code === "}" || code === "end") {
      break;
    }

    let entry = null;
    let sourceName = "";
    const ordinal = entries.length + 1;
    if (tokens[0] === "level") {
      const nameTokens = tokens.at(-1) === "{" ? tokens.slice(1, -1) : tokens.slice(1);
      sourceName = levelNameFromTokens(nameTokens);
      const name = levelDefinitionName(levelsRange, sourceName, ordinal);
      entry = tokens.at(-1) === "{"
        ? bracedLevelEntry(source, lines, index, name, levelsRange.bodyEnd)
        : unbracedLevelEntry(lines, index, index + 1, name, levelsRange.bodyEnd);
    } else if (tokens.length === 1 && tokens[0] === "{") {
      entry = bracedLevelEntry(source, lines, index, levelDefinitionName(levelsRange, "", ordinal), levelsRange.bodyEnd);
    } else if (tokens.at(-1) === "{") {
      sourceName = levelNameFromTokens(tokens.slice(0, -1));
      entry = bracedLevelEntry(
        source,
        lines,
        index,
        levelDefinitionName(levelsRange, sourceName, ordinal),
        levelsRange.bodyEnd,
      );
    } else {
      entry = unbracedLevelEntry(lines, index, index, levelDefinitionName(levelsRange, "", ordinal), levelsRange.bodyEnd);
    }

    if (!entry) {
      index += 1;
      continue;
    }
    entry.sourceName = sourceName;
    entry.namespace = levelsRange.namespace || "";
    entries.push(entry);
    index = Math.max(index + 1, entry.nextIndex);
  }
  return assignLevelLevelIndexes(entries);
}

function findStandaloneLevelDefinitionAtPosition(source, position) {
  const lines = sourceLinesWithOffsets(source);
  for (let index = 0; index < lines.length; index += 1) {
    const code = levelScannerCode(lines[index].raw);
    const tokens = splitLevelTokens(code);
    if (tokens[0] !== "level") {
      continue;
    }
    const entry = tokens.at(-1) === "{"
      ? bracedLevelEntry(source, lines, index, levelNameFromTokens(tokens.slice(1, -1)), source.length)
      : endDelimitedStandaloneLevelEntry(lines, index, levelNameFromTokens(tokens.slice(1)));
    if (entry && position >= entry.start && position <= entry.end) {
      return assignLevelLevelIndexes([entry])[0] || null;
    }
  }
  return null;
}

function endDelimitedStandaloneLevelEntry(lines, headerIndex, name) {
  let index = headerIndex + 1;
  let nestedDepth = 0;
  let lastContentEnd = lines[headerIndex].end;
  while (index < lines.length) {
    const line = lines[index];
    const code = levelScannerCode(line.raw);
    if (code) {
      const normalized = braceNormalizedLineForSectionForWasm(code);
      const tokens = splitLevelTokens(normalized);
      if (normalized === "end") {
        if (nestedDepth === 0) {
          return {
            name,
            start: firstCodeIndex(lines[headerIndex]),
            end: line.start,
            nextIndex: index + 1,
          };
        }
        nestedDepth -= 1;
      } else if (startsLevelBodyBlock(tokens, normalized)) {
        nestedDepth += 1;
      }
    }
    lastContentEnd = line.end;
    index += 1;
  }
  return {
    name,
    start: firstCodeIndex(lines[headerIndex]),
    end: lastContentEnd,
    nextIndex: lines.length,
  };
}

function bracedLevelEntry(source, lines, lineIndex, name, rangeEnd) {
  const line = lines[lineIndex];
  const openIndex = source.indexOf("{", line.start);
  const closeIndex = findMatchingBrace(source, openIndex);
  if (openIndex < 0 || closeIndex < 0 || closeIndex > rangeEnd) {
    return null;
  }
  return {
    name,
    start: firstCodeIndex(line),
    end: closeIndex,
    nextIndex: nextLineIndexAfterPosition(lines, closeIndex),
  };
}

function unbracedLevelEntry(lines, headerIndex, contentIndex, name, rangeEnd) {
  let index = contentIndex;
  let nestedDepth = 0;
  let lastContentEnd = lines[headerIndex].end;
  while (index < lines.length && lines[index].start <= rangeEnd) {
    const line = lines[index];
    const code = levelScannerCode(line.raw);
    if (nestedDepth === 0 && (!code || code === "end" || code === "}" || isLevelHeaderCode(code) || isLevelsSectionBoundary(splitLevelTokens(code)))) {
      break;
    }
    if (code) {
      const normalized = braceNormalizedLineForSectionForWasm(code);
      const tokens = splitLevelTokens(normalized);
      if (normalized === "end" || normalized === "}") {
        nestedDepth = Math.max(0, nestedDepth - 1);
      } else if (startsLevelBodyBlock(tokens, normalized)) {
        nestedDepth += 1;
      }
    }
    lastContentEnd = Math.min(line.end, rangeEnd);
    index += 1;
  }
  return {
    name,
    start: firstCodeIndex(lines[headerIndex]),
    end: lastContentEnd,
    nextIndex: index,
  };
}

function isLevelHeaderCode(code) {
  const tokens = splitLevelTokens(code);
  return tokens[0] === "level"
    || (tokens.length === 1 && tokens[0] === "{")
    || (tokens.at(-1) === "{" && tokens[0] !== "legend");
}

function startsLevelBodyBlock(tokens, line) {
  return (tokens.length === 1 && tokens[0] === "legend") || isLevelLifecycleHeader(tokens);
}

function startsLevelNestedBlock(tokens, line) {
  return (tokens[0] === "level" && tokens.at(-1) === "{")
    || (tokens.length === 1 && tokens[0] === "{")
    || (tokens.at(-1) === "{" && tokens[0] !== "level")
    || (tokens[0] !== "level" && startsInlineBlockForWasm(tokens, line));
}

function isLevelsSectionBoundary(tokens) {
  return startsPuzzleSectionForWasm(tokens) && !["level"].includes(tokens[0] || "");
}

function levelNameFromTokens(tokens) {
  return tokens.filter(Boolean).join(" ");
}

function levelsNamespaceFromTokens(tokens) {
  const parts = tokens.at(-1) === "{" ? tokens.slice(1, -1) : tokens.slice(1);
  if (!parts.length) {
    return "";
  }
  const ofIndex = parts.indexOf("of");
  const namespaceParts = ofIndex >= 0 ? parts.slice(0, ofIndex) : parts;
  return namespaceParts.length === 1 ? namespaceParts[0] : "";
}

function levelDefinitionName(levelsRange, name, ordinal) {
  const namespace = String(levelsRange?.namespace || "").trim();
  const rawName = String(name || "").trim();
  if (!rawName) {
    return namespace ? `${namespace}.${ordinal}` : "";
  }
  if (namespace && !rawName.startsWith(`${namespace}.`)) {
    return `${namespace}.${rawName}`;
  }
  return rawName;
}

function levelScannerCode(line) {
  return stripLineCommentForWasm(line).trim();
}

function splitLevelTokens(line) {
  return String(line || "").split(/\s+/).filter(Boolean);
}

function sourceLinesWithOffsets(source) {
  const lines = [];
  let start = 0;
  const text = String(source || "");
  for (const raw of text.split("\n")) {
    const end = start + raw.length;
    const hasNewline = end < text.length;
    lines.push({
      raw,
      start,
      end,
      absoluteEnd: end + (hasNewline ? 1 : 0),
      hasNewline,
    });
    start = end + 1;
  }
  return lines;
}

function firstCodeIndex(line) {
  const offset = line.raw.search(/\S/);
  return line.start + Math.max(0, offset);
}

function nextLineIndexAfterPosition(lines, position) {
  const index = lines.findIndex((line) => line.start > position);
  return index < 0 ? lines.length : index;
}

function isSectionTitleLine(rawLines, index) {
  return index > 0
    && index + 1 < rawLines.length
    && isSectionSeparatorForWasm(stripLineCommentForWasm(rawLines[index - 1]).trim())
    && isSectionSeparatorForWasm(stripLineCommentForWasm(rawLines[index + 1]).trim());
}

function assignLevelLevelIndexes(entries) {
  const levels = previewExport?.levels || [];
  const usedIndexes = new Set();
  return entries.map((entry, ordinal) => {
    const sourceName = Object.prototype.hasOwnProperty.call(entry, "sourceName")
      ? entry.sourceName
      : entry.name || "";
    let levelIndex = -1;
    if (entry.name) {
      levelIndex = levels.findIndex((levelData, index) => (
        !usedIndexes.has(index) && levelData?.name === entry.name
      ));
    }
    if (levelIndex < 0 && ordinal < levels.length && !usedIndexes.has(ordinal)) {
      levelIndex = ordinal;
    }
    if (levelIndex < 0) {
      levelIndex = Math.max(0, Math.min(levels.length - 1, ordinal));
    }
    usedIndexes.add(levelIndex);
    const levelName = levels[levelIndex]?.name || unnamedLevelEntryName(entry, ordinal);
    return {
      ...entry,
      sourceName,
      name: entry.name || levelName,
      levelIndex,
    };
  });
}

function unnamedLevelEntryName(entry, ordinal) {
  if (String(entry?.name || "").trim()) {
    return "";
  }
  const index = Number.isInteger(entry?.levelIndex) ? entry.levelIndex : ordinal;
  return `unnamed level ${Math.max(0, index) + 1}`;
}

function makeEmptyCells(width, height) {
  return Array.from({ length: width * height }, () => makeEmptyCell());
}

function makeEmptyCell(exportData = previewExport) {
  return Array.from({ length: layerCount(exportData) }, () => 0);
}

function cloneCellSlots(slots, exportData = previewExport) {
  const next = makeEmptyCell(exportData);
  if (Array.isArray(slots)) {
    for (let index = 0; index < Math.min(slots.length, next.length); index += 1) {
      next[index] = Number(slots[index]) || 0;
    }
  }
  return next;
}

function renderLevelPalette() {
  const toggleButton = levelPaletteCollapseButton;
  const eraserButton = renderLevelEraserButton();
  levelPalette.replaceChildren(...[toggleButton, levelFillButton, eraserButton].filter(Boolean));
  levelPalette.classList.add("is-sprite-only");
  levelPalette.classList.toggle("is-collapsed", level.paletteCollapsed);
  levelPaletteCollapseButton.classList.toggle("is-active", level.paletteCollapsed);
  levelPaletteCollapseButton.classList.toggle("is-collapsed", level.paletteCollapsed);
  levelPaletteCollapseButton.setAttribute("aria-expanded", String(!level.paletteCollapsed));
  levelPaletteCollapseButton.setAttribute("aria-label", level.paletteCollapsed ? "Show palette" : "Hide palette");
  levelPaletteCollapseButton.title = level.paletteCollapsed ? "Show palette" : "Hide palette";
  const mainObjects = level.palette.filter((object) => object.id !== 0 && !isVisualObject(object));
  const visualObjects = level.palette.filter((object) => object.id !== 0 && isVisualObject(object));
  renderLevelPaletteGroup("", mainObjects);
  renderLevelPaletteGroup("Visual", visualObjects);
  syncLevelLayerVisibilityControl();
  updateLevelPlaytestControls();
}

function renderLevelEraserButton() {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "level-palette-tool-button sprite-icon-button level-eraser-button";
  button.classList.toggle("is-active", level.selectedObjectId === 0);
  button.setAttribute("aria-label", "Paint Eraser");
  button.setAttribute("aria-pressed", String(level.selectedObjectId === 0));
  button.title = "Eraser";
  button.dataset.tooltip = "Eraser";
  button.append(renderLevelEraserIcon());
  button.addEventListener("click", () => {
    level.selectedObjectId = 0;
    setLevelActiveLayerForObject(0);
    renderLevelPalette();
  });
  return button;
}

function renderLevelPaletteGroup(label, objects) {
  if (!objects.length) {
    return;
  }
  const group = document.createElement("div");
  group.className = "level-palette-group";
  if (label) {
    const heading = document.createElement("div");
    heading.className = "level-palette-heading";
    heading.textContent = label;
    group.append(heading);
  }
  for (const object of objects) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "level-token";
    button.classList.toggle("is-selected", object.id === level.selectedObjectId);
    button.title = object.name;
    button.dataset.label = object.name;
    button.setAttribute("aria-label", `Paint ${object.name}`);
    button.append(renderObjectPreview(object));

    const label = document.createElement("span");
    label.className = "tile-label";
    label.textContent = object.name;
    button.append(label);

    button.addEventListener("click", () => {
      level.selectedObjectId = object.id;
      setLevelActiveLayerForObject(object.id);
      renderLevelPalette();
    });
    group.append(button);
  }
  levelPalette.append(group);
}

function renderLevelBoard() {
  updateLevelSizeLabel();
  syncLevelResizeControls();
  syncLevelLayerVisibilityControl();
  renderLevelSourcePreview();
  const cells = displayedLevelCells();
  const visibleCells = levelVisibleCells(cells);
  if (levelRenderer) {
    levelRenderer.render(levelScene(visibleCells));
    syncLevelGridVisibility();
    levelBoard.querySelectorAll(".cell").forEach((cell, index) => {
      cell.dataset.index = String(index);
      cell.setAttribute("aria-label", cellLabel(visibleCells[index]));
      cell.setAttribute("role", "button");
      cell.tabIndex = 0;
    });
    syncLevelBoardScale();
    scheduleBoardScaleSync();
    renderSolverBoard();
    return;
  }
  levelBoard.replaceChildren();
  syncLevelBoardScale();
  scheduleBoardScaleSync();
  renderSolverBoard();
}

function syncLevelGridVisibility() {
  levelBoard?.classList.remove("has-occupied-cell-grid", "has-all-cell-grid");
  levelBoard?.classList.toggle("has-all-cell-grid", levelGridVisible);
  syncLevelGridButton();
}

function toggleLevelGrid() {
  levelGridVisible = !levelGridVisible;
  syncLevelGridVisibility();
  setStatus(levelGridVisible ? "Level grid visible" : "Level grid hidden", "is-ok");
}

function syncLevelGridButton() {
  if (!levelGridButton) {
    return;
  }
  levelGridButton.classList.toggle("is-selected", levelGridVisible);
  levelGridButton.setAttribute("aria-pressed", levelGridVisible ? "true" : "false");
  levelGridButton.title = "Toggle grid";
  levelGridButton.dataset.tooltip = "Toggle grid";
  levelGridButton.setAttribute("aria-label", "Toggle level grid");
}

function renderSolverBoard() {
  if (!solverBoard) {
    return;
  }
  const exportData = previewExport || extractPreviewExport(latestHtml);
  if (isPuzzle3dExport(exportData) && typeof renderPuzzle3dSolverPreview === "function") {
    renderPuzzle3dSolverPreview();
    return;
  }
  if (typeof clearPuzzle3dSolverPreview === "function") {
    clearPuzzle3dSolverPreview();
  }
  const scene = displayedSolverScene(exportData);
  if (solverRenderer) {
    solverRenderer.render(scene);
    syncSolverBoardScale();
    scheduleBoardScaleSync();
    return;
  }
  solverBoard.replaceChildren();
  syncSolverBoardScale();
  scheduleBoardScaleSync();
}

function scheduleBoardScaleSync(passes = 2) {
  boardScaleSyncPasses = Math.max(boardScaleSyncPasses, Math.max(1, Math.trunc(Number(passes) || 1)));
  if (boardScaleSyncFrame) {
    return;
  }
  const tick = () => {
    boardScaleSyncFrame = 0;
    if (!levelBuilder.hidden) {
      syncLevelBoardScale();
    }
    if (!solverPanel.hidden) {
      syncSolverBoardScale();
    }
    boardScaleSyncPasses -= 1;
    if (boardScaleSyncPasses > 0) {
      boardScaleSyncFrame = requestAnimationFrame(tick);
    }
  };
  boardScaleSyncFrame = requestAnimationFrame(tick);
}

function syncLevelBoardScale() {
  const wrap = levelBoardViewport?.closest(".level-board-wrap");
  syncBoardViewportScale(wrap, levelBoardViewport, levelBoard, boardFrameSize(levelBoard, level.width, level.height), {
    width: levelEditorEdgeSize * 2 + levelEditorGap * 2,
    height: levelEditorEdgeSize * 2 + levelEditorGap * 2,
    availableHeight: editorRemainingContentBlockSize(levelBuilder, wrap),
  });
}

function syncSolverBoardScale() {
  const wrap = solverBoardViewport?.closest(".solver-board-wrap");
  const scene = displayedSolverScene();
  syncBoardViewportScale(wrap, solverBoardViewport, solverBoard, boardFrameSize(solverBoard, scene?.width || level.width, scene?.height || level.height));
}

function syncBoardViewportScale(wrap, viewport, board, frame, chrome = {}) {
  if (!wrap || !viewport || !board || !frame) {
    return;
  }
  if (wrap.getClientRects().length === 0 || viewport.getClientRects().length === 0) {
    return;
  }
  const frameWidth = Math.max(1, Number(frame.width || 1));
  const frameHeight = Math.max(1, Number(frame.height || 1));
  const chromeWidth = Math.max(0, Number(chrome.width || 0));
  const chromeHeight = Math.max(0, Number(chrome.height || 0));
  const availableWidth = editorFrameContentInlineSize(wrap) - chromeWidth;
  if (availableWidth <= 0) {
    return;
  }
  const availableFrameHeight = Number.isFinite(chrome.availableHeight)
    ? Math.max(0, Number(chrome.availableHeight) - elementBlockOuterSpacing(wrap) - chromeHeight)
    : Number.POSITIVE_INFINITY;
  const maxCellSize = Math.max(1, Math.floor(editorPuzzleCellSize()));
  const fitCellSizeByWidth = Math.floor(availableWidth / frameWidth);
  const fitCellSizeByHeight = Number.isFinite(availableFrameHeight) && availableFrameHeight > 0
    ? Math.floor(availableFrameHeight / frameHeight)
    : maxCellSize;
  const fitCellSize = Math.max(1, Math.min(maxCellSize, fitCellSizeByWidth, fitCellSizeByHeight));
  const cellSize = quantizedEditorCellSize(fitCellSize, editorPuzzleQuantum(board));
  const boardWidth = frameWidth * cellSize;
  const boardHeight = frameHeight * cellSize;
  const naturalWidth = boardWidth + chromeWidth;
  const naturalHeight = boardHeight + chromeHeight;
  wrap.style.setProperty("--editor-board-cell-size", `${cellSize}px`);
  wrap.style.setProperty("--board-natural-width", `${Math.ceil(naturalWidth)}px`);
  wrap.style.setProperty("--board-natural-height", `${Math.ceil(naturalHeight)}px`);
  wrap.style.setProperty("--board-scale", "1");
  wrap.style.setProperty("--board-viewport-width", `${Math.ceil(naturalWidth)}px`);
  wrap.style.setProperty("--board-viewport-height", `${Math.ceil(naturalHeight)}px`);
}

function editorRemainingContentBlockSize(container, target) {
  if (!container || !target || !container.contains(target)) {
    return Number.POSITIVE_INFINITY;
  }
  const containerHeight = elementContentHeight(container);
  if (containerHeight <= 0) {
    return Number.POSITIVE_INFINITY;
  }
  const style = window.getComputedStyle(container);
  const gap = Math.max(0, Number.parseFloat(style.rowGap || style.gap || "0") || 0);
  const visibleChildren = [...container.children].filter((child) => {
    if (child === target) {
      return true;
    }
    return window.getComputedStyle(child).display !== "none";
  });
  const siblingHeight = visibleChildren
    .filter((child) => child !== target)
    .reduce((sum, child) => sum + elementOuterBlockSize(child), 0);
  const gapHeight = Math.max(0, visibleChildren.length - 1) * gap;
  return Math.max(1, containerHeight - siblingHeight - gapHeight);
}

function editorPuzzleCellSize() {
  const configured = Number(window.GameVisuals?.editorPuzzle?.cellSize);
  return Number.isFinite(configured) && configured > 0 ? configured : boardVirtualCellSize;
}

function editorPuzzleQuantum(board) {
  let quantum = 1;
  for (const sprite of board.querySelectorAll(".visual-sprite")) {
    const style = window.getComputedStyle(sprite);
    const cols = Math.max(1, Math.trunc(Number(style.getPropertyValue("--sprite-cols")) || 1));
    const rows = Math.max(1, Math.trunc(Number(style.getPropertyValue("--sprite-rows")) || 1));
    quantum = boundedLeastCommonMultiple(quantum, cols, 512);
    quantum = boundedLeastCommonMultiple(quantum, rows, 512);
  }
  return quantum > 1 && quantum <= 128 ? quantum : 1;
}

function quantizedEditorCellSize(size, quantum) {
  const cellSize = Math.max(1, Math.floor(size));
  const step = Math.max(1, Math.floor(quantum || 1));
  if (step <= 1 || cellSize < step) {
    return cellSize;
  }
  return Math.max(step, Math.floor(cellSize / step) * step);
}

function boundedLeastCommonMultiple(a, b, limit) {
  const left = Math.max(1, Math.trunc(Number(a) || 1));
  const right = Math.max(1, Math.trunc(Number(b) || 1));
  const value = (left / greatestCommonDivisor(left, right)) * right;
  return value > limit ? limit + 1 : value;
}

function greatestCommonDivisor(a, b) {
  let left = Math.abs(Math.trunc(a));
  let right = Math.abs(Math.trunc(b));
  while (right) {
    const next = left % right;
    left = right;
    right = next;
  }
  return left || 1;
}

function boardFrameSize(board, fallbackWidth, fallbackHeight) {
  const width = Math.max(1, Number(board?.dataset.frameWidth || fallbackWidth || 1));
  const height = Math.max(1, Number(board?.dataset.frameHeight || fallbackHeight || 1));
  return { width, height };
}

function loadLevelFromPreviewState(options = {}) {
  const requestRender = options.requestRender !== false;
  const levelIndex = currentEditableLevelIndex();
  const scene = previewSceneForLevel(levelIndex);
  if (!scene?.width || !scene?.height || !Array.isArray(scene.cells)) {
    return false;
  }
  clearSolutionPreview();
  stopLevelPlaytest({ syncPreview: false });
  levelDisplayCells = null;
  level.width = scene.width;
  level.height = scene.height;
  level.regions = normalizedLevelRegions(scene.regions, level.width, level.height);
  level.cells = scene.cells.map((cell) => cellSlotsFromLayers(cell.layers || []));
  const levelName = previewExport?.levels?.[levelIndex]?.name;
  if (levelName) {
    setLevelNameInputs(levelName);
  }
  renderLevelBoard();
  if (requestRender) {
    sendLevelStateToPreview(levelIndex, levelStateData(previewExport));
  }
  return true;
}

function applyPreviewSceneToLevel(scene) {
  if (!scene?.width || !scene?.height || !Array.isArray(scene.cells)) {
    return false;
  }
  clearSolutionPreview();
  stopLevelPlaytest({ syncPreview: false });
  levelDisplayCells = null;
  level.width = scene.width;
  level.height = scene.height;
  level.regions = normalizedLevelRegions(scene.regions, level.width, level.height);
  level.cells = scene.cells.map((cell) => cellSlotsFromLayers(cell.layers || []));
  renderLevelBoard();
  scheduleBoardScaleSync(2);
  return true;
}

function initialPreviewScene() {
  return previewSceneForLevel(previewExport?.initialLevelIndex || 0);
}

function previewSceneForLevel(levelIndex, exportData = previewExport) {
  const index = Math.max(0, Math.trunc(Number(levelIndex) || 0));
  const state = exportData?.levels?.[index]?.initialState;
  if (!state) {
    return null;
  }
  const regions = exportData.levels?.[index]?.regions || [];
  return sceneFromStateData(state, { regions, exportData });
}

function sceneFromStateData(state, options = {}) {
  if (!state?.width || !state?.height || !state?.layerCount || !Array.isArray(state.slots)) {
    return null;
  }
  const exportData = options.exportData || previewExport;
  const objectsById = new Map((exportData.engine?.objects || []).map((object) => [object.id, object]));
  const cells = [];
  for (let y = 0; y < state.height; y += 1) {
    for (let x = 0; x < state.width; x += 1) {
      const layers = [];
      for (let layer = 0; layer < state.layerCount; layer += 1) {
        const objectId = state.slots[((y * state.width + x) * state.layerCount) + layer];
        const object = objectsById.get(objectId);
        if (object) {
          layers.push({
            layer,
            objectId,
            object: object.name,
            sprite: object.sprite,
          });
        }
      }
      cells.push({ x, y, layers });
    }
  }
  return {
    width: state.width,
    height: state.height,
    layerCount: state.layerCount,
    regions: options.regions || [],
    cells,
  };
}

function cellSlotsFromLayers(layers, exportData = previewExport) {
  const slots = makeEmptyCell(exportData);
  for (const layer of layers) {
    if (Number.isInteger(layer.layer) && layer.layer >= 0 && layer.layer < slots.length) {
      slots[layer.layer] = objectIdForLayer(layer, exportData);
    }
  }
  return slots;
}

function objectIdForLayer(layer, exportData = previewExport) {
  const explicit = Number(layer?.objectId) || 0;
  if (explicit) {
    return explicit;
  }
  const name = layer?.object || "";
  const sprite = layer?.sprite || "";
  const object = (exportData?.engine?.objects || []).find((entry) =>
    (name && entry.name === name) || (sprite && entry.sprite === sprite)
  );
  return object?.id || 0;
}

function renderObjectPreview(object) {
  if (!object?.id) {
    return renderLevelEraserPreview();
  }
  const root = document.createElement("span");
  root.className = "game-preview-scope level-token-visual board";
  root.setAttribute("aria-hidden", "true");
  if (window.PuzzleRenderer) {
    new window.PuzzleRenderer(root, { renderMode: "dom", themeRoot: root }).render(objectScene(object));
  }
  return root;
}

function renderLevelEraserPreview() {
  const root = document.createElement("span");
  root.className = "level-token-visual";
  root.setAttribute("aria-hidden", "true");
  root.append(renderLevelEraserIcon());
  return root;
}

function renderLevelEraserIcon() {
  const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  svg.setAttribute("class", "level-token-eraser");
  svg.setAttribute("viewBox", "0 0 24 24");
  svg.setAttribute("aria-hidden", "true");
  for (const d of [
    "m7 21-4.3-4.3c-1-1-1-2.5 0-3.4l9.6-9.6c1-1 2.5-1 3.4 0l5.6 5.6c1 1 1 2.5 0 3.4L13 21",
    "M22 21H7",
    "m5 11 9 9",
  ]) {
    const path = document.createElementNS("http://www.w3.org/2000/svg", "path");
    path.setAttribute("d", d);
    svg.append(path);
  }
  return svg;
}

function levelListFilterIconSvg() {
  return `
    <svg class="lucide lucide-list-filter level-layer-visibility-icon" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" aria-hidden="true">
      <path d="M3 6h18"></path>
      <path d="M7 12h10"></path>
      <path d="M10 18h4"></path>
    </svg>
  `;
}

function levelLayerVisibilityEntries(exportData = previewExport) {
  const count = Math.max(1, layerCount(exportData));
  const layerNames = sourceLayerNameEntries(levelReferenceSource(exportData), exportData);
  return Array.from({ length: count }, (_, layerIndex) => ({
    layer: layerIndex,
    label: layerNames.get(layerIndex) || "",
  }));
}

function normalizedLevelHiddenLayers(exportData = previewExport) {
  const count = Math.max(1, layerCount(exportData));
  const hidden = new Set();
  for (const layerIndex of level.hiddenLayers || []) {
    const normalized = Math.trunc(Number(layerIndex));
    if (Number.isInteger(normalized) && normalized >= 0 && normalized < count) {
      hidden.add(normalized);
    }
  }
  level.hiddenLayers = [...hidden].sort((left, right) => left - right);
  return hidden;
}

function levelLayerIsVisible(layerIndex, exportData = previewExport) {
  return !normalizedLevelHiddenLayers(exportData).has(Math.trunc(Number(layerIndex) || 0));
}

function levelVisibleLayerIndexes(exportData = previewExport) {
  const count = Math.max(1, layerCount(exportData));
  const hidden = normalizedLevelHiddenLayers(exportData);
  return Array.from({ length: count }, (_unused, layerIndex) => layerIndex)
    .filter((layerIndex) => !hidden.has(layerIndex));
}

function levelFilteredCellSlots(slots, exportData = previewExport) {
  const hidden = normalizedLevelHiddenLayers(exportData);
  if (!hidden.size) {
    return slots;
  }
  const next = cloneCellSlots(slots, exportData);
  for (const layerIndex of hidden) {
    if (layerIndex >= 0 && layerIndex < next.length) {
      next[layerIndex] = 0;
    }
  }
  return next;
}

function levelVisibleCells(sourceCells = displayedLevelCells(), exportData = previewExport) {
  const hidden = normalizedLevelHiddenLayers(exportData);
  if (!hidden.size) {
    return sourceCells;
  }
  return (sourceCells || []).map((slots) => levelFilteredCellSlots(slots, exportData));
}

function syncLevelLayerVisibilityControl() {
  if (!levelLayerVisibilityButton || !levelLayerVisibilityMenu) {
    return;
  }
  const hasHiddenLayers = normalizedLevelHiddenLayers().size > 0;
  const isOpen = !levelLayerVisibilityMenu.hidden;
  levelLayerVisibilityButton.classList.toggle("has-hidden-layers", hasHiddenLayers);
  levelLayerVisibilityButton.classList.toggle("is-open", isOpen);
  levelLayerVisibilityButton.setAttribute("aria-expanded", String(isOpen));
  levelLayerVisibilityButton.disabled = levelPlaytestActive;
  renderLevelLayerVisibilityMenu();
}

function renderLevelLayerVisibilityMenu() {
  if (!levelLayerVisibilityMenu) {
    return;
  }
  const entries = levelLayerVisibilityEntries();
  levelLayerVisibilityMenu.replaceChildren(...entries.map((entry) => {
    const label = document.createElement("label");
    label.className = "level-layer-visibility-option";
    label.setAttribute("role", "menuitemcheckbox");
    label.setAttribute("aria-checked", String(levelLayerIsVisible(entry.layer)));
    const checkbox = document.createElement("input");
    checkbox.type = "checkbox";
    checkbox.checked = levelLayerIsVisible(entry.layer);
    checkbox.dataset.levelVisibilityLayer = String(entry.layer);
    checkbox.disabled = levelPlaytestActive;
    const text = document.createElement("span");
    text.className = "level-layer-visibility-label";
    text.textContent = entry.label;
    label.append(checkbox, text);
    return label;
  }));
}

function toggleLevelLayerVisibilityMenu() {
  if (!levelLayerVisibilityMenu || !levelLayerVisibilityButton || levelPlaytestActive) {
    return;
  }
  levelLayerVisibilityMenu.hidden = !levelLayerVisibilityMenu.hidden;
  syncLevelLayerVisibilityControl();
}

function setLevelLayerVisible(layerIndex, visible) {
  const normalized = normalizedLevelActiveLayer(layerIndex);
  const hidden = normalizedLevelHiddenLayers();
  if (visible) {
    hidden.delete(normalized);
  } else {
    hidden.add(normalized);
  }
  level.hiddenLayers = [...hidden].sort((left, right) => left - right);
  renderLevelBoard();
  syncLevelLayerVisibilityControl();
  setStatus(level.hiddenLayers.length ? "Layer visibility filtered" : "All layers visible", "is-ok");
}

function objectScene(object) {
  const slots = makeEmptyCell();
  if (object?.id && Number.isInteger(object.layer) && object.layer >= 0 && object.layer < slots.length) {
    slots[object.layer] = object.id;
  }
  return sceneFromCellSlots([slots], {
    width: 1,
    height: 1,
    regions: [],
  });
}

function levelScene(sourceCells = level.cells) {
  return sceneFromCellSlots(sourceCells, {
    width: level.width,
    height: level.height,
    regions: levelRegions(),
  });
}

function sceneFromCellSlots(sourceCells, options = {}) {
  const width = Math.max(1, Number(options.width || level.width || 1));
  const height = Math.max(1, Number(options.height || level.height || 1));
  const cells = sourceCells.map((slots, index) => ({
    x: index % width,
    y: Math.floor(index / width),
    layers: layersForSlots(normalizedCellSlots(slots)),
  }));
  return {
    width,
    height,
    layerCount: layerCount(),
    regions: options.regions || [],
    cells,
  };
}

function normalizedCellSlots(slots, exportData = previewExport) {
  if (Array.isArray(slots) && slots.length === layerCount(exportData)) {
    return slots;
  }
  const next = makeEmptyCell(exportData);
  for (const objectId of slots || []) {
    const object = engineObjectById(objectId, exportData);
    if (object) {
      next[object.layer] = object.id;
    }
  }
  return next;
}

function displayedLevelCells() {
  return levelPlaytestActive && levelDisplayCells?.length === level.cells.length ? levelDisplayCells : level.cells;
}

function displayedSolverScene(exportData = previewExport || extractPreviewExport(latestHtml)) {
  if (levelSolutionPreview?.kind !== "puzzle3d" && Array.isArray(levelSolutionPreview?.cells)) {
    const fallback = solverSceneOverride || previewSceneForLevel(currentSolverLevelIndex(exportData), exportData);
    return sceneFromCellSlots(levelSolutionPreview.cells, {
      width: fallback?.width || level.width,
      height: fallback?.height || level.height,
      regions: fallback?.regions || [],
    });
  }
  if (solverObservationPreview?.cells) {
    const fallback = solverSceneOverride || previewSceneForLevel(currentSolverLevelIndex(exportData), exportData);
    return sceneFromCellSlots(solverObservationPreview.cells, {
      width: fallback?.width || level.width,
      height: fallback?.height || level.height,
      regions: fallback?.regions || [],
    });
  }
  if (solverSceneOverride) {
    return solverSceneOverride;
  }
  return previewSceneForLevel(currentSolverLevelIndex(exportData), exportData) || levelScene(displayedLevelCells());
}

function displayedSolverCells() {
  if (levelSolutionPreview?.kind !== "puzzle3d" && Array.isArray(levelSolutionPreview?.cells)) {
    return levelSolutionPreview.cells;
  }
  if (solverObservationPreview) {
    return solverObservationPreview.cells;
  }
  if (stagedSolverCells?.length) {
    return stagedSolverCells;
  }
  const scene = displayedSolverScene();
  return sceneCellsToSlots(scene, displayedLevelCells());
}

function layersForSlots(slots, exportData = previewExport) {
  return cloneCellSlots(slots, exportData)
    .map((objectId) => engineObjectById(objectId, exportData))
    .filter(Boolean)
    .map(layerForObject)
    .sort((left, right) => left.layer - right.layer);
}

function layerForObject(object) {
  return {
    layer: object.layer,
    objectId: object.id,
    object: object.name,
    sprite: object.sprite,
  };
}

function cellLabel(slots) {
  const names = layersForSlots(slots).map((layer) => layer.object);
  return names.length ? names.join(", ") : "Empty";
}

function addLevelEdge(edge) {
  resizeLevelEdge(edge, "expand");
}

function shrinkLevelEdge(edge) {
  resizeLevelEdge(edge, "shrink");
}

function resizeLevelEdge(edge, mode = levelResizeMode || "expand") {
  const normalizedMode = mode === "shrink" ? "shrink" : "expand";
  if (levelPlaytestActive) {
    return;
  }
  const before = visualEditSnapshot("level");
  clearSolutionPreview();
  stopLevelPlaytest({ syncPreview: false });
  levelDisplayCells = null;
  const delta = normalizedMode === "shrink" ? -1 : 1;
  const nextWidth = level.width + ((edge === "left" || edge === "right") ? delta : 0);
  const nextHeight = level.height + ((edge === "top" || edge === "bottom") ? delta : 0);
  if (nextWidth < 1 || nextHeight < 1) {
    setStatus("Level cannot shrink further", "is-error");
    return;
  }
  if (nextWidth > 40 || nextHeight > 30) {
    setStatus("Level size limit", "is-error");
    return;
  }

  const nextCells = makeEmptyCells(nextWidth, nextHeight);
  const targetOffsetX = normalizedMode === "expand" && edge === "left" ? 1 : 0;
  const targetOffsetY = normalizedMode === "expand" && edge === "top" ? 1 : 0;
  const sourceOffsetX = normalizedMode === "shrink" && edge === "left" ? 1 : 0;
  const sourceOffsetY = normalizedMode === "shrink" && edge === "top" ? 1 : 0;
  for (let y = 0; y < nextHeight; y += 1) {
    for (let x = 0; x < nextWidth; x += 1) {
      const sourceX = normalizedMode === "expand" ? x - targetOffsetX : x + sourceOffsetX;
      const sourceY = normalizedMode === "expand" ? y - targetOffsetY : y + sourceOffsetY;
      if (sourceX >= 0 && sourceX < level.width && sourceY >= 0 && sourceY < level.height) {
        nextCells[y * nextWidth + x] = cloneCellSlots(level.cells[sourceY * level.width + sourceX]);
      }
    }
  }

  level.width = nextWidth;
  level.height = nextHeight;
  level.regions = resizeLevelRegions(levelRegions(), edge, nextWidth, nextHeight, delta);
  level.cells = nextCells;
  setLevelSolveStatus("");
  renderLevelBoard();
  pushVisualEditUndoSnapshot("level", before);
  setStatus(normalizedMode === "shrink" ? "Level shrunk" : "Level expanded", "is-ok");
}

function levelStageResizeMode() {
  return levelResizeMode === "expand" || levelResizeMode === "shrink" ? levelResizeMode : null;
}

function setLevelResizeMode(mode) {
  levelResizeMode = mode === "expand" || mode === "shrink" ? mode : null;
  if (levelResizeMode) {
    levelBucketActive = false;
  }
  syncLevelBucketButton();
  syncLevelResizeControls();
}

function toggleLevelResizeMode(mode) {
  if (levelPlaytestActive) {
    return;
  }
  setLevelResizeMode(levelStageResizeMode() === mode ? null : mode);
  setStatus(levelStageResizeMode() === "expand"
    ? "Expand: click an edge to add space"
    : levelStageResizeMode() === "shrink"
      ? "Shrink: click an edge to remove space"
      : "Brush: paint individual cells", "is-ok");
}

function syncLevelResizeControls() {
  const mode = levelStageResizeMode();
  levelBoardEditor?.classList.toggle("is-resize-mode", Boolean(mode));
  levelBoardEditor?.classList.toggle("is-resize-expand", mode === "expand");
  levelBoardEditor?.classList.toggle("is-resize-shrink", mode === "shrink");
  if (levelExpandButton) {
    const active = mode === "expand";
    levelExpandButton.classList.toggle("is-selected", active);
    levelExpandButton.setAttribute("aria-pressed", active ? "true" : "false");
    levelExpandButton.disabled = levelPlaytestActive;
  }
  if (levelShrinkButton) {
    const active = mode === "shrink";
    levelShrinkButton.classList.toggle("is-selected", active);
    levelShrinkButton.setAttribute("aria-pressed", active ? "true" : "false");
    levelShrinkButton.disabled = levelPlaytestActive;
  }
  levelEdgeButtons.forEach((button) => {
    const edge = button.dataset.levelEdge || "";
    const action = mode === "shrink" ? "Remove" : "Add";
    const axis = edge === "left" || edge === "right" ? "column" : "row";
    const side = {
      top: "above",
      bottom: "below",
      left: "left",
      right: "right",
    }[edge] || edge;
    button.textContent = mode === "shrink" ? "−" : "+";
    button.classList.toggle("is-shrink", mode === "shrink");
    button.setAttribute("aria-label", `${action} ${axis} ${side}`.trim());
    button.title = `${action} ${axis}`.trim();
    button.disabled = levelPlaytestActive || !mode;
  });
}

function syncLevelBucketButton() {
  if (!levelFillButton) {
    return;
  }
  levelFillButton.classList.toggle("is-active", levelBucketActive);
  levelFillButton.setAttribute("aria-pressed", String(levelBucketActive));
  levelFillButton.setAttribute("aria-label", "Fill");
  levelFillButton.title = "Fill";
  levelFillButton.dataset.tooltip = "Fill";
}

function toggleLevelBucketMode() {
  if (levelPlaytestActive) {
    return;
  }
  levelBucketActive = !levelBucketActive;
  if (levelBucketActive) {
    setLevelResizeMode(null);
  }
  syncLevelBucketButton();
  setStatus(levelBucketActive ? "Bucket: click a connected area" : "Brush: paint individual cells", "is-ok");
}

function transformLevelCells({ nextWidth, nextHeight, mapCell, mapRegion, message }) {
  if (levelPlaytestActive) {
    return false;
  }
  const before = visualEditSnapshot("level");
  clearSolutionPreview();
  stopLevelPlaytest({ syncPreview: false });
  levelDisplayCells = null;
  const previousWidth = level.width;
  const previousHeight = level.height;
  const previousCells = level.cells;
  const previousRegions = levelRegions();
  const nextCells = makeEmptyCells(nextWidth, nextHeight);
  for (let y = 0; y < nextHeight; y += 1) {
    for (let x = 0; x < nextWidth; x += 1) {
      const source = mapCell(x, y, previousWidth, previousHeight);
      if (
        source
        && source.x >= 0
        && source.x < previousWidth
        && source.y >= 0
        && source.y < previousHeight
      ) {
        nextCells[y * nextWidth + x] = cloneCellSlots(previousCells[source.y * previousWidth + source.x]);
      }
    }
  }
  level.width = nextWidth;
  level.height = nextHeight;
  level.regions = normalizedLevelRegions(previousRegions.map((region) => mapRegion(region, previousWidth, previousHeight)), nextWidth, nextHeight);
  level.cells = nextCells;
  setLevelSolveStatus("");
  renderLevelBoard();
  pushVisualEditUndoSnapshot("level", before);
  setStatus(message, "is-ok");
  return true;
}

function rotateLevelLeft() {
  return transformLevelCells({
    nextWidth: level.height,
    nextHeight: level.width,
    mapCell: (x, y, width) => ({ x: width - 1 - y, y: x }),
    mapRegion: (region, width) => ({
      ...region,
      x: region.y,
      y: width - region.x - region.width,
      width: region.height,
      height: region.width,
    }),
    message: "Rotated level left",
  });
}

function rotateLevelRight() {
  return transformLevelCells({
    nextWidth: level.height,
    nextHeight: level.width,
    mapCell: (x, y, _width, height) => ({ x: y, y: height - 1 - x }),
    mapRegion: (region, _width, height) => ({
      ...region,
      x: height - region.y - region.height,
      y: region.x,
      width: region.height,
      height: region.width,
    }),
    message: "Rotated level right",
  });
}

function flipLevelHorizontal() {
  return transformLevelCells({
    nextWidth: level.width,
    nextHeight: level.height,
    mapCell: (x, y, width) => ({ x: width - 1 - x, y }),
    mapRegion: (region, width) => ({
      ...region,
      x: width - region.x - region.width,
    }),
    message: "Flipped level horizontal",
  });
}

function flipLevelVertical() {
  return transformLevelCells({
    nextWidth: level.width,
    nextHeight: level.height,
    mapCell: (x, y, _width, height) => ({ x, y: height - 1 - y }),
    mapRegion: (region, _width, height) => ({
      ...region,
      y: height - region.y - region.height,
    }),
    message: "Flipped level vertical",
  });
}

function updateLevelSizeLabel() {
  levelSizeLabel.textContent = `${level.width} × ${level.height}`;
}

function normalizedLevelActiveLayer(layer = level.activeLayer, exportData = previewExport) {
  const count = Math.max(1, layerCount(exportData));
  return Math.max(0, Math.min(count - 1, Math.trunc(Number(layer) || 0)));
}

function setLevelActiveLayerForObject(objectId) {
  const object = engineObjectById(objectId, exportData);
  if (object && Number.isInteger(object.layer)) {
    level.activeLayer = normalizedLevelActiveLayer(object.layer);
  } else {
    level.activeLayer = normalizedLevelActiveLayer(level.activeLayer);
  }
}

function paintLevelCellFromElement(element) {
  const index = levelCellIndexFromElement(element);
  return paintLevelCellAtIndex(index, level.selectedObjectId);
}

function levelCellIndexFromElement(element) {
  const cell = element?.closest?.(".cell");
  if (!cell || !levelBoard.contains(cell)) {
    return -1;
  }
  const index = Number(cell.dataset.index);
  if (!Number.isInteger(index) || index < 0 || index >= level.cells.length) {
    return -1;
  }
  return index;
}

function bucketFillLevelFromElement(element) {
  return bucketFillLevelFromIndex(levelCellIndexFromElement(element));
}

function bucketFillLevelFromIndex(index) {
  if (levelPlaytestActive || !Number.isInteger(index) || index < 0 || index >= level.cells.length) {
    return false;
  }
  clearSolutionPreview();
  levelDisplayCells = null;
  const replacement = paintCellSlots(level.cells[index], level.selectedObjectId);
  const target = cloneCellSlots(level.cells[index]);
  if (sameCellSlotsForVisibleLayers(target, replacement)) {
    setStatus("Connected area already has that tile", "is-ok");
    return false;
  }

  const visited = new Uint8Array(level.cells.length);
  const stack = [index];
  let changed = 0;
  while (stack.length) {
    const current = stack.pop();
    if (visited[current] || !sameCellSlotsForVisibleLayers(level.cells[current], target)) {
      continue;
    }
    visited[current] = 1;
    level.cells[current] = cloneCellSlots(replacement);
    changed += 1;
    const x = current % level.width;
    const y = Math.floor(current / level.width);
    if (x > 0) {
      stack.push(current - 1);
    }
    if (x < level.width - 1) {
      stack.push(current + 1);
    }
    if (y > 0) {
      stack.push(current - level.width);
    }
    if (y < level.height - 1) {
      stack.push(current + level.width);
    }
  }
  if (!changed) {
    return false;
  }
  setLevelSolveStatus("");
  renderLevelBoard();
  setStatus(level.selectedObjectId ? "Filled connected area" : "Erased connected area", "is-ok");
  return true;
}

function paintLevelCellAtIndex(index, objectId, options = {}) {
  if (levelPlaytestActive) {
    return false;
  }
  clearSolutionPreview();
  levelDisplayCells = null;
  if (!Number.isInteger(index) || index < 0 || index >= level.cells.length) {
    return false;
  }
  const next = paintCellSlots(level.cells[index], objectId);
  if (sameCellSlots(level.cells[index], next)) {
    return false;
  }
  level.cells[index] = next;
  setLevelSolveStatus("");
  renderLevelBoard();
  return true;
}

function paintLevelCellFromPoint(clientX, clientY, objectId) {
  return paintLevelCellAtIndex(
    levelCellIndexFromElement(document.elementFromPoint(clientX, clientY)),
    objectId,
  );
}

function startLevelPaint(event) {
  if (levelPlaytestActive) {
    focusLevelInputTarget();
    event.preventDefault();
    return;
  }
  if (event.button !== 0) {
    return;
  }
  const objectId = level.selectedObjectId;
  const index = levelCellIndexFromElement(document.elementFromPoint(event.clientX, event.clientY));
  if (!Number.isInteger(index) || index < 0) {
    return;
  }
  event.preventDefault();
  if (levelBucketActive) {
    withVisualEditHistory("level", () => bucketFillLevelFromIndex(index));
    return;
  }
  levelPaintDrag = {
    pointerId: event.pointerId,
    objectId,
    lastIndex: -1,
    beforeSnapshot: visualEditSnapshot("level"),
    changed: false,
  };
  if (levelBoard.setPointerCapture) {
    levelBoard.setPointerCapture(event.pointerId);
  }
  paintLevelDragIndex(index);
}

function continueLevelPaint(event) {
  if (!levelPaintDrag || levelPaintDrag.pointerId !== event.pointerId) {
    return;
  }
  event.preventDefault();
  const element = document.elementFromPoint(event.clientX, event.clientY);
  paintLevelDragIndex(levelCellIndexFromElement(element));
}

function stopLevelPaint(event) {
  if (!levelPaintDrag || levelPaintDrag.pointerId !== event.pointerId) {
    return;
  }
  if (levelBoard.hasPointerCapture?.(event.pointerId)) {
    levelBoard.releasePointerCapture(event.pointerId);
  }
  if (levelPaintDrag.changed) {
    pushVisualEditUndoSnapshot("level", levelPaintDrag.beforeSnapshot);
  }
  levelPaintDrag = null;
}

function paintLevelDragIndex(index) {
  if (!levelPaintDrag || !Number.isInteger(index) || index < 0) {
    return;
  }
  if (index === levelPaintDrag.lastIndex) {
    return;
  }
  levelPaintDrag.lastIndex = index;
  if (paintLevelCellAtIndex(index, levelPaintDrag.objectId)) {
    levelPaintDrag.changed = true;
  }
}

async function startLevelPlaytest() {
  if (levelPlaytestActive) {
    return;
  }
  const exportData = await ensurePreviewExportForLevelAction({
    noDocumentMessage: "No level to play",
    compilingMessage: "Compiling preview for play",
    failureMessage: "Preview compile failed",
  });
  if (!exportData) {
    return;
  }
  const stateData = levelStateData(exportData);
  if (!stateData) {
    setStatus("No level to play", "is-error");
    return;
  }
  clearSolutionPreview();
  const levelIndex = currentEditableLevelIndex(exportData);
  let playableState = stateData;
  try {
    playableState = await materializeLevelStartForPlaytest(stateData, exportData, levelIndex);
  } catch (error) {
    setStatus(`Play failed: ${userFacingRuntimeError(error)}`, "is-error");
    return;
  }
  levelPlaytestActive = true;
  levelPlaytestStateData = playableState;
  levelPlaytestRuntimeStateData = playableState;
  levelDisplayCells = stateDataToLevelCells(playableState, exportData);
  pendingPreviewKeyStateSync = 0;
  updateLevelPlaytestControls();
  renderLevelBoard();
  sendLevelStateToPreview(levelIndex, playableState, {
    acceptModelInput: true,
    materializeLevelStart: false,
    materializeDisplay: true,
    silent: false,
  });
  focusLevelInputTarget();
  requestAnimationFrame(focusLevelInputTarget);
}

function stopLevelPlaytest(options = {}) {
  if (!levelPlaytestActive && !levelDisplayCells) {
    updateLevelPlaytestControls();
    return;
  }
  levelPlaytestActive = false;
  levelPlaytestStateData = null;
  levelPlaytestTransitionBusy = false;
  resetLevelPlaytestRuntime();
  levelDisplayCells = null;
  pendingPreviewKeyStateSync = 0;
  if (levelPaintDrag && levelBoard.hasPointerCapture?.(levelPaintDrag.pointerId)) {
    levelBoard.releasePointerCapture(levelPaintDrag.pointerId);
  }
  levelPaintDrag = null;
  updateLevelPlaytestControls();
  renderLevelBoard();
  if (options.syncPreview !== false) {
    const exportData = previewExport || extractPreviewExport(latestHtml);
    const stateData = exportData ? levelStateData(exportData) : null;
    if (stateData) {
      sendLevelStateToPreview(currentEditableLevelIndex(exportData), stateData, {
        materializeLevelStart: false,
        materializeDisplay: false,
        silent: true,
      });
    }
  }
}

function focusLevelInputTarget() {
  if (!levelBoard) {
    return;
  }
  levelBoard.tabIndex = 0;
  levelBoard.focus({ preventScroll: true });
  if (document.activeElement !== levelBoard) {
    levelBoard.querySelector(".cell")?.focus?.({ preventScroll: true });
  }
}

async function materializeLevelStartForPlaytest(stateData, exportData, levelIndex) {
  let current = JSON.parse(JSON.stringify(stateData));
  resetLevelPlaytestRuntime();
  let cancelled = false;
  if (exportData.engine?.levelStartProgram?.length) {
    const outcome = await transitionPlaytestProgram(exportData, "level_start", levelIndex, current, 0);
    current = outcome.state || current;
    cancelled = cancelled || outcome.cancelled === true;
  } else if (exportData.engine?.runRulesOnLevelStart) {
    const outcome = await transitionPlaytestProgram(exportData, "run_rules_on_level_start", levelIndex, current, 0);
    current = outcome.state || current;
    cancelled = cancelled || outcome.cancelled === true;
  }
  if (!cancelled && exportData.levels?.[levelIndex]?.levelStartProgram?.length) {
    const outcome = await transitionPlaytestProgram(exportData, "level_start_local", levelIndex, current, 0);
    current = outcome.state || current;
  }
  return current;
}

async function transitionPlaytestProgram(exportData, programKey, levelIndex, stateData, inputId) {
  const runtime = await levelPlaytestCoreRuntime(exportData);
  if (levelPlaytestRuntimeStateData !== stateData) {
    runtime.set_state(JSON.stringify(stateData));
    levelPlaytestRuntimeStateData = stateData;
  }
  const outcome = JSON.parse(runtime.transition_current_outcome(
    programKey,
    Number.isFinite(levelIndex) ? Math.trunc(levelIndex) : -1,
    Number(inputId || 0),
  ));
  if (outcome.state) {
    levelPlaytestRuntimeStateData = outcome.state;
  }
  return outcome;
}

async function levelPlaytestCoreRuntime(exportData) {
  if (!exportData?.compiledPlay) {
    throw new Error("Compiled play data is not available");
  }
  const module = await loadWasmCoreRuntime();
  if (typeof module.WasmCompiledCoreRuntime !== "function") {
    throw new Error("Compiled core runtime is not available");
  }
  const sourceKey = JSON.stringify(exportData.compiledPlay);
  if (!levelPlaytestRuntime || levelPlaytestRuntimeSourceKey !== sourceKey) {
    resetLevelPlaytestRuntime();
    levelPlaytestRuntime = new module.WasmCompiledCoreRuntime(JSON.stringify({
      compiledPlay: exportData.compiledPlay,
    }));
    levelPlaytestRuntimeSourceKey = sourceKey;
  }
  return levelPlaytestRuntime;
}

function resetLevelPlaytestRuntime() {
  if (levelPlaytestRuntime && typeof levelPlaytestRuntime.free === "function") {
    levelPlaytestRuntime.free();
  }
  levelPlaytestRuntime = null;
  levelPlaytestRuntimeSourceKey = "";
  levelPlaytestRuntimeStateData = null;
}

function inputIdForPreviewKey(event, exportData = previewExport) {
  const rawKey = String(event.key || "");
  const rawCode = String(event.code || "");
  const tokens = new Set([rawKey, rawCode]);
  if (rawKey.length === 1) {
    tokens.add(rawKey.toLowerCase());
  }
  return (exportData?.inputs || []).find((input) =>
    tokens.has(input.key)
    || tokens.has(input.arrow)
    || (input.keys || []).some((candidate) => tokens.has(candidate))
  )?.id;
}

async function applyLevelPlaytestKey(event) {
  if (!levelPlaytestActive || levelPlaytestTransitionBusy) {
    return;
  }
  const exportData = previewExport || extractPreviewExport(latestHtml);
  const inputId = inputIdForPreviewKey(event, exportData);
  if (!exportData || !Number.isInteger(inputId) || !levelPlaytestStateData) {
    return;
  }
  levelPlaytestTransitionBusy = true;
  try {
    const levelIndex = currentEditableLevelIndex(exportData);
    const outcome = await transitionPlaytestProgram(exportData, "main", levelIndex, levelPlaytestStateData, inputId);
    levelPlaytestStateData = outcome.state || levelPlaytestStateData;
    levelDisplayCells = stateDataToLevelCells(levelPlaytestStateData, exportData);
    renderLevelBoard();
    sendLevelStateToPreview(levelIndex, levelPlaytestStateData, {
      acceptModelInput: true,
      animationEvents: outcome.animationEvents,
      materializeLevelStart: false,
      materializeDisplay: true,
      silent: false,
    });
  } catch (error) {
    setStatus(`Play input failed: ${userFacingRuntimeError(error)}`, "is-error");
  } finally {
    levelPlaytestTransitionBusy = false;
  }
}

function toggleLevelPlaytest() {
  if (levelPlaytestActive) {
    stopLevelPlaytest();
  } else {
    startLevelPlaytest().catch((error) => {
      setStatus(`Play failed: ${userFacingRuntimeError(error)}`, "is-error");
    });
  }
}

function updateLevelPlaytestControls() {
  if (!levelBuilder) {
    return;
  }
  levelBuilder.classList.toggle("is-playtesting", levelPlaytestActive);
  if (levelPlaytestActive) {
    levelBucketActive = false;
    levelResizeMode = null;
  }
  syncLevelBucketButton();
  syncLevelResizeControls();
  if (levelPlaytestButton) {
    const label = levelPlaytestActive ? "Stop level playtest" : "Play level";
    const tooltip = levelPlaytestActive ? "Stop" : "Play";
    levelPlaytestButton.classList.toggle("is-playing", levelPlaytestActive);
    levelPlaytestButton.setAttribute("aria-label", label);
    levelPlaytestButton.title = tooltip;
    levelPlaytestButton.dataset.tooltip = tooltip;
  }
  for (const element of [
    levelNamespaceInput,
    levelNameInput,
    copyLevelButton,
    addLevelButton,
    updateLevelButton,
    levelPaletteCollapseButton,
    levelExpandButton,
    levelShrinkButton,
    levelLayerVisibilityButton,
    levelRotateLeftButton,
    levelRotateRightButton,
    levelFlipHorizontalButton,
    levelFlipVerticalButton,
    levelFillButton,
  ]) {
    if (element) {
      element.disabled = levelPlaytestActive;
    }
  }
  levelPalette?.querySelectorAll("button").forEach((button) => {
    button.disabled = levelPlaytestActive;
  });
  levelEdgeButtons.forEach((button) => {
    button.disabled = levelPlaytestActive;
  });
}

function sameCellSlots(left, right) {
  if (!Array.isArray(left) || !Array.isArray(right) || left.length !== right.length) {
    return false;
  }
  return left.every((value, index) => value === right[index]);
}

function sameCellSlotsForVisibleLayers(left, right, exportData = previewExport) {
  const leftSlots = cloneCellSlots(left);
  const rightSlots = cloneCellSlots(right);
  return levelVisibleLayerIndexes(exportData).every((layerIndex) => leftSlots[layerIndex] === rightSlots[layerIndex]);
}

function paintCellSlots(slots, objectId, exportData = previewExport) {
  const visibleLayers = new Set(levelVisibleLayerIndexes(exportData));
  const next = cloneCellSlots(slots, exportData);
  for (const layerIndex of visibleLayers) {
    next[layerIndex] = 0;
  }
  if (!objectId) {
    return next;
  }
  const object = engineObjectById(objectId, exportData);
  if (!object) {
    return cloneCellSlots(slots);
  }
  if (visibleLayers.has(object.layer)) {
    next[object.layer] = object.id;
  }
  return next;
}

function syncPreviewStateFromLevel() {
  const exportData = previewExport || extractPreviewExport(latestHtml);
  if (!exportData) {
    return;
  }
  if (isPuzzle3dExport(exportData) && typeof sendLevel3dSnapshotToRuntime === "function") {
    if (currentPreviewMode !== "level3d") {
      return;
    }
    latestPreviewState = {
      ...(latestPreviewState || {}),
      levelIndex: currentEditableLevelIndex(exportData),
      scene: null,
    };
    sendLevel3dSnapshotToRuntime();
    return;
  }
  if (!activePreviewModeAcceptsLevelState()) {
    return;
  }
  const stateData = levelStateData(exportData);
  if (!stateData) {
    return;
  }

  const levelIndex = currentEditableLevelIndex(exportData);
  latestPreviewState = {
    ...(latestPreviewState || {}),
    levelIndex,
    scene: null,
  };

  sendLevelStateToPreview(levelIndex, stateData);
}

function sendLevelStateToPreview(levelIndex = currentEditableLevelIndex(), stateData = null, options = {}) {
  if (!activePreviewModeAcceptsLevelState()) {
    return;
  }
  const exportData = previewExport || extractPreviewExport(latestHtml);
  const state = stateData || levelStateData(exportData);
  if (!state) {
    return;
  }
  previewFrameHasEditorLevelState = true;
  const materializeLevelStart = options.materializeLevelStart ?? (currentPreviewMode === "play" || levelPlaytestActive);
  const materializeDisplay = options.materializeDisplay ?? (currentPreviewMode === "play" || levelPlaytestActive);
  previewFrame.contentWindow?.postMessage({
    type: "PuzzleStudioSetState",
    levelIndex,
    state,
    animationEvents: Array.isArray(options.animationEvents) ? options.animationEvents : [],
    regions: levelRegions(),
    acceptModelInput: options.acceptModelInput === true,
    materializeLevelStart,
    materializeDisplay,
    silent: options.silent ?? (currentPreviewMode !== "play" && !levelPlaytestActive),
  }, "*");
}

async function solveLevel(options = {}) {
  if (activeLevelSolveRequest) {
    cancelLevelSolve();
    return;
  }
  const exportData = await ensurePreviewExportForLevelAction({
    status: setLevelSolveStatus,
    noDocumentMessage: "No preview to solve",
    compilingMessage: "Compiling preview for solve",
    failureMessage: "Preview compile failed",
  });
  if (!exportData) {
    return;
  }
  const stateData = options.stateData || solverStateData(exportData);
  if (!stateData) {
    setLevelSolveStatus("No level state", "is-error");
    return;
  }

  clearSolutionPreview({ clearStagedSolver: false });
  renderLevelBoard();
  const requestId = createDocumentId();
  const solveRequest = {
    source: exportData.source || activePreviewSource(),
    puzzlePath: exportData.puzzlePath || activePreviewDocument()?.puzzlePath || "game.puzzle",
    stateJson: JSON.stringify(stateData),
    maxDepth: 512,
    maxNodes: 5_000_000,
    maxMs: 0,
  };
  let worker = null;
  try {
    worker = createWasmSolveWorker();
  } catch (error) {
    setSolveLevelButtonState(false);
    stopLevelSolveFeedback();
    setLevelSolveStatus(`Solver worker failed: ${userFacingWorkerError(error)}`, "is-error");
    return;
  }
  activeLevelSolveRequest = { id: requestId, backend: "wasm-worker", worker };
  worker.onmessage = (event) => {
    const message = event.data || {};
    if (message.type === "result") {
      handleLevelSolveResult({ requestId, solution: message.solution });
      return;
    }
    if (message.type === "error") {
      handleLevelSolveResult({
        requestId,
        error: `Solver worker failed: ${userFacingWorkerError(message.error)}`,
      });
    }
  };
  worker.onerror = (error) => {
    error?.preventDefault?.();
    handleLevelSolveResult({
      requestId,
      error: `Solver worker failed: ${userFacingWorkerError(error)}`,
    });
  };
  setSolveLevelButtonState(true);
  startLevelSolveFeedback("Solving");
  try {
    worker.postMessage({
      type: "solve",
      requestId,
      wasm: wasmSolverWorkerConfig(),
      ...solveRequest,
    });
  } catch (error) {
    handleLevelSolveResult({
      requestId,
      error: `Solver worker failed: ${userFacingWorkerError(error)}`,
    });
  }
}

async function solveEditedLevelFromEditor() {
  ensurePreviewTargetsActiveDocument();
  const exportData = await ensurePreviewExportForLevelAction({
    status: setLevelSolveStatus,
    noDocumentMessage: "No level state",
    compilingMessage: "Compiling preview for solve",
    failureMessage: "Preview compile failed",
  });
  if (!exportData) {
    return;
  }
  const levelIndex = currentEditableLevelIndex(exportData);
  if (isPuzzle3dExport(exportData)) {
    const snapshot = typeof level3dRuntimeSnapshot === "function" ? level3dRuntimeSnapshot() : null;
    const stateData = puzzle3dStateDataFromSnapshot(snapshot, exportData, { levelIndex });
    if (!stateData || !snapshot) {
      setLevelSolveStatus("No 3D level state", "is-error");
      return;
    }
    setSolverTargetFromState({
      exportData,
      levelIndex,
      stateData,
      puzzle3dSnapshot: snapshot,
    });
  } else {
    const stateData = levelStateData(exportData);
    const scene = stateData ? sceneFromStateData(stateData, {
      regions: levelRegions(),
      exportData,
    }) : null;
    if (!stateData || !scene) {
      setLevelSolveStatus("No level state", "is-error");
      return;
    }
    setSolverTargetFromState({
      exportData,
      levelIndex,
      stateData,
      scene,
    });
  }
  openPreviewModePane("solver");
  syncSourceFromPreviewPane(isPuzzle3dExport(exportData) ? "level3d" : "solver");
  renderSolverBoard();
  solveLevel({ preserveStagedSolver: true });
}

function cancelLevelSolve() {
  if (!activeLevelSolveRequest) {
    return;
  }
  if (activeLevelSolveRequest.backend === "wasm-worker") {
    disposeWasmSolveWorker(activeLevelSolveRequest.worker);
    activeLevelSolveRequest = null;
    setSolveLevelButtonState(false);
    stopLevelSolveFeedback();
    setLevelSolveStatus("Cancelled", "");
    return;
  }
  if (!previewFrame.contentWindow) {
    return;
  }
  previewFrame.contentWindow.postMessage({
    type: "PuzzleStudioCancelSolve",
    requestId: activeLevelSolveRequest.id,
  }, "*");
  setLevelSolveStatus("Cancelling", "");
}

function setSolveLevelButtonState(isSolving) {
  const label = isSolving ? "Cancel" : "Solve";
  const visibleLabel = isSolving ? "Cancel" : "Solve";
  for (const button of [solveLevelButton, levelSolveShortcutButton, level3dSolveShortcutButton]) {
    if (!button) {
      continue;
    }
    button.classList.toggle("is-solving", Boolean(isSolving));
    button.setAttribute("aria-label", label);
    button.title = visibleLabel;
    button.dataset.tooltip = visibleLabel;
    const labelElement = button.querySelector(".solve-button-label");
    if (labelElement) {
      labelElement.textContent = visibleLabel;
    }
  }
  solverBoardViewport?.classList.toggle("is-solving", Boolean(isSolving));
  solverBoardViewport?.closest(".solver-board-wrap")?.classList.toggle("is-solving", Boolean(isSolving));
  syncSolverLevelSelector();
}

function startLevelSolveFeedback(initialText = "Solving") {
  levelSolveStartedAt = Date.now();
  if (levelSolveFeedbackTimer) {
    window.clearInterval(levelSolveFeedbackTimer);
  }
  setLevelSolveStatus(`${initialText}: starting search, ${formatSeconds(0)}`, "");
  levelSolveFeedbackTimer = window.setInterval(tickLevelSolveFeedback, solverFeedbackTickMs);
}

function stopLevelSolveFeedback() {
  if (levelSolveFeedbackTimer) {
    window.clearInterval(levelSolveFeedbackTimer);
    levelSolveFeedbackTimer = 0;
  }
  levelSolveStartedAt = 0;
  solverBoardViewport?.classList.remove("is-solving");
  solverBoardViewport?.closest(".solver-board-wrap")?.classList.remove("is-solving");
}

function tickLevelSolveFeedback() {
  if (!activeLevelSolveRequest || !levelSolveStartedAt) {
    stopLevelSolveFeedback();
    return;
  }
  const elapsedMs = Date.now() - levelSolveStartedAt;
  setLevelSolveStatus(`Solving: starting search, ${formatSeconds(elapsedMs)}`, "");
}

function handleLevelSolveResult(message) {
  if (!activeLevelSolveRequest || message.requestId !== activeLevelSolveRequest.id) {
    return;
  }
  disposeWasmSolveWorker(activeLevelSolveRequest.worker);
  activeLevelSolveRequest = null;
  setSolveLevelButtonState(false);
  stopLevelSolveFeedback();

  if (message.error) {
    setLevelSolveStatus(message.error, "is-error");
    return;
  }

  const solution = message.solution;
  if (!solution) {
    setLevelSolveStatus("No solver result", "is-error");
    return;
  }

  if (solution.result === "solved") {
    showSolutionPreview(solution);
    return;
  }

  if (solution.result === "cancelled") {
    setLevelSolveStatus("Cancelled", "");
    return;
  }

  const stats = solution.stats;
  const reason = solution.reason ? `: ${solution.reason}` : "";
  const suffix = stats
    ? ` (${stats.visited} states, depth ${stats.maxDepthReached}, ${stats.elapsedMs}ms)`
    : "";
  setLevelSolveStatus(`${titleLabel(solution.result)}${reason}${suffix}`, "is-error");
}

function setLevelSolveStatus(text, className = "") {
  if (levelSolveFlashTimer) {
    window.clearTimeout(levelSolveFlashTimer);
    levelSolveFlashTimer = 0;
    levelSolveFlashRestore = null;
  }
  levelSolveStatus.className = `level-solve-status tool-feedback-bar ${className}`.trim();
  levelSolveStatus.textContent = text;
  if (!levelSolutionPreview) {
    setLevelSolveSummary(text, className);
  }
  setPaneStatus("solver", text, className);
}

function clearSharedPaneStatus() {
  clearPaneStatus(activeStatusPaneId());
}

function setLevelSolveSummary(text, className = "") {
  levelSolveSummaryText = text || "";
  if (!levelSolutionPreview) {
    updateSolutionControls();
  }
}

function userFacingRuntimeError(error) {
  const message = String(error?.message || error || "unknown error");
  return /\b(wasm|webassembly|rust)\b/i.test(message)
    ? "browser runtime could not start"
    : message;
}

function flashLevelSolveStatus(text, className = "", duration = 900) {
  const restore = levelSolveFlashRestore || {
    text: levelSolveStatus.textContent,
    className: [...levelSolveStatus.classList]
      .filter((name) => name !== "level-solve-status")
      .filter((name) => name !== "tool-feedback-bar")
      .join(" "),
  };
  setLevelSolveStatus(text, className);
  levelSolveFlashRestore = restore;
  levelSolveFlashTimer = window.setTimeout(() => {
    const next = levelSolveFlashRestore;
    levelSolveFlashTimer = 0;
    levelSolveFlashRestore = null;
    setLevelSolveStatus(next?.text || "", next?.className || "");
  }, duration);
}

function showSolutionPreview(solution) {
  const steps = Array.isArray(solution.steps) ? solution.steps : [];
  if (!steps.length) {
    setLevelSolveStatus("Solved, but no steps were returned", "is-error");
    return;
  }
  if (solution.model === "puzzle3d" || steps[0]?.scene?.kind === "puzzle3d") {
    solverObservationPreview = null;
    if (typeof showPuzzle3dSolutionPreview === "function") {
      showPuzzle3dSolutionPreview(solution);
      return;
    }
    setLevelSolveStatus("3D solution replay is not available", "is-error");
    return;
  }
  levelSolutionPreview = {
    steps,
    moves: solutionMoves(solution),
    index: 0,
    cells: sceneCellsToSlots(steps[0].scene, displayedSolverCells()),
  };
  solverObservationPreview = null;
  updateSolutionControls();
  renderSolverBoard();
  setLevelSolveStatus(solution.depth ? `Solved in ${solution.depth} moves` : "Already solved", "is-ok");
}

function solutionMoves(solution) {
  if (Array.isArray(solution.moves) && solution.moves.length) {
    return solution.moves;
  }
  return (solution.steps || [])
    .map((step) => step.move)
    .filter(Boolean);
}

function sceneCellsToSlots(scene, fallback = []) {
  const cells = (scene?.cells || []).map((cell) => cellSlotsFromLayers(cell.layers || []));
  return cells.length ? cells : fallback.map(cloneCellSlots);
}

function stateDataToLevelCells(stateData, exportData = previewExport) {
  const scene = sceneFromStateData(stateData, {
    regions: levelRegions(),
    exportData,
  });
  const cells = sceneCellsToSlots(scene, []);
  return cells.length === level.cells.length ? cells : null;
}

function setSolutionStep(index) {
  if (!levelSolutionPreview) {
    return;
  }
  if (levelSolutionPreview.kind === "puzzle3d" && typeof setPuzzle3dSolutionStep === "function") {
    setPuzzle3dSolutionStep(index);
    return;
  }
  const nextIndex = Math.max(0, Math.min(levelSolutionPreview.steps.length - 1, index));
  levelSolutionPreview.index = nextIndex;
  levelSolutionPreview.cells = sceneCellsToSlots(
    levelSolutionPreview.steps[nextIndex].scene,
    levelSolutionPreview.cells.length ? levelSolutionPreview.cells : displayedSolverCells(),
  );
  updateSolutionControls();
  renderSolverBoard();
}

function updateSolutionControls() {
  const active = Boolean(levelSolutionPreview);
  levelSolutionControls.hidden = false;
  levelSolutionControls.classList.toggle("is-empty", !active && !levelSolveSummaryText);
  if (!active) {
    solutionPrevButton.disabled = true;
    solutionNextButton.disabled = true;
    solutionPlayButton.disabled = true;
    solutionSpeedSelect.disabled = true;
    solutionResetButton.disabled = true;
    solutionExportButton.disabled = true;
    solutionSeekInput.disabled = true;
    solutionSeekInput.max = "0";
    solutionSeekInput.value = "0";
    solutionStepText.textContent = "0/0";
    solutionPlayButton.classList.remove("is-playing");
    solutionPlayButton.setAttribute("aria-label", "Play solution");
    solutionPlayButton.title = "Play solution";
    solutionText.textContent = levelSolveSummaryText || "No solution yet";
    solutionText.title = levelSolveSummaryText;
    return;
  }
  const index = levelSolutionPreview.index;
  const maxIndex = levelSolutionPreview.steps.length - 1;
  solutionPrevButton.disabled = index <= 0;
  solutionNextButton.disabled = index >= maxIndex;
  solutionPlayButton.disabled = maxIndex <= 0;
  solutionSpeedSelect.disabled = maxIndex <= 0;
  solutionResetButton.disabled = index <= 0;
  solutionExportButton.disabled = maxIndex <= 0;
  solutionSeekInput.disabled = maxIndex <= 0;
  solutionSeekInput.max = String(maxIndex);
  solutionSeekInput.value = String(index);
  solutionStepText.textContent = `${index}/${maxIndex}`;
  solutionStepText.title = `Step ${index} of ${maxIndex}`;
  const playLabel = levelSolutionTimer ? "Pause solution" : "Play solution";
  solutionPlayButton.classList.toggle("is-playing", Boolean(levelSolutionTimer));
  solutionPlayButton.setAttribute("aria-label", playLabel);
  solutionPlayButton.title = playLabel;
  const move = levelSolutionPreview.steps[index]?.move?.name;
  const label = move ? `Step ${index}/${maxIndex}: ${move}` : `Step ${index}/${maxIndex}`;
  levelSolveStatus.title = label;
  updateSolutionText();
}

function seekSolutionStep(event) {
  if (!levelSolutionPreview) {
    return;
  }
  const nextIndex = Math.trunc(Number(event.currentTarget.value) || 0);
  stopSolutionPlayback();
  setSolutionStep(nextIndex);
}

function toggleSolutionPlayback() {
  if (!levelSolutionPreview) {
    return;
  }
  if (levelSolutionTimer) {
    stopSolutionPlayback();
    return;
  }
  startSolutionPlayback();
}

function startSolutionPlayback() {
  if (!levelSolutionPreview) {
    return;
  }
  levelSolutionTimer = window.setInterval(() => {
    if (!levelSolutionPreview) {
      stopSolutionPlayback();
      return;
    }
    if (levelSolutionPreview.index >= levelSolutionPreview.steps.length - 1) {
      stopSolutionPlayback();
      return;
    }
    setSolutionStep(levelSolutionPreview.index + 1);
  }, solutionPlaybackIntervalMs());
  updateSolutionControls();
}

function solutionPlaybackIntervalMs() {
  const speed = Math.max(0.25, Number(solutionSpeedSelect.value) || 1);
  return Math.max(40, Math.round(solutionPlaybackBaseIntervalMs / speed));
}

function changeSolutionPlaybackSpeed() {
  if (!levelSolutionTimer) {
    return;
  }
  stopSolutionPlayback();
  startSolutionPlayback();
}

function stopSolutionPlayback() {
  if (levelSolutionTimer) {
    window.clearInterval(levelSolutionTimer);
    levelSolutionTimer = 0;
  }
  updateSolutionControls();
}

function clearSolutionPreview(options = {}) {
  if (levelSolutionTimer) {
    window.clearInterval(levelSolutionTimer);
    levelSolutionTimer = 0;
  }
  levelSolutionPreview = null;
  solverObservationPreview = null;
  if (options.clearStagedSolver !== false) {
    stagedSolverCells = null;
  }
  levelSolveSummaryText = "";
  levelSolveStatus.title = "";
  updateSolutionControls();
  if (currentPreviewMode === "level3d" && typeof renderLevel3dBuilder === "function") {
    renderLevel3dBuilder();
  }
  if (typeof clearPuzzle3dSolverPreview === "function") {
    clearPuzzle3dSolverPreview();
  }
}

function resetSolutionPreview() {
  if (!levelSolutionPreview) {
    return;
  }
  stopSolutionPlayback();
  setSolutionStep(0);
}

function updateSolutionText() {
  const text = solutionTextForUdlr();
  const displayText = text ? abbreviatedSolutionText(text) : solutionSummaryText();
  solutionText.textContent = displayText;
  solutionText.title = text ? `Solution: ${text}` : displayText;
  const label = "Copy solution as UDLR";
  solutionExportButton.setAttribute("aria-label", label);
  solutionExportButton.title = label;
}

function abbreviatedSolutionText(text) {
  const maxLength = 36;
  return text.length <= maxLength ? text : `${text.slice(0, maxLength)}...`;
}

function solutionSummaryText() {
  if (!levelSolutionPreview) {
    return "";
  }
  const moveCount = Math.max(0, (levelSolutionPreview.steps || []).length - 1);
  return moveCount === 1 ? "1 move" : `${moveCount} moves`;
}

function solutionTextForUdlr() {
  if (!levelSolutionPreview) {
    return "";
  }
  const tokens = (levelSolutionPreview.moves || [])
    .map(solutionMoveToken)
    .filter(Boolean);
  return tokens.every((token) => token.length === 1)
    ? tokens.join("")
    : tokens.join(" ");
}

function solutionMoveToken(move) {
  const direction = solutionMoveDirection(move);
  if (direction) {
    return {
      up: "u",
      down: "d",
      left: "l",
      right: "r",
      front: "f",
      back: "b",
    }[direction] || `[${direction}]`;
  }
  if (/^[udlr]$/i.test(move?.key || "")) {
    return move.key.toLowerCase();
  }
  return move?.name ? `[${move.name}]` : "?";
}

function solutionMoveDirection(move) {
  const explicit = String(move?.direction || "").toLowerCase();
  const canonicalExplicit = canonicalPuzzle3DirectionName(explicit);
  if (["up", "down", "left", "right", "front", "back"].includes(canonicalExplicit)) {
    return canonicalExplicit;
  }
  const name = String(move?.name || "").toLowerCase();
  const canonicalName = canonicalPuzzle3DirectionName(name);
  if (["up", "down", "left", "right", "front", "back"].includes(canonicalName)) {
    return canonicalName;
  }
  const arrow = String(move?.arrow || "");
  if (arrow === "ArrowUp") {
    return "up";
  }
  if (arrow === "ArrowDown") {
    return "down";
  }
  if (arrow === "ArrowLeft") {
    return "left";
  }
  if (arrow === "ArrowRight") {
    return "right";
  }
  const key = String(move?.key || "").toLowerCase();
  return { w: "up", s: "down", a: "left", d: "right" }[key] || "";
}

function canonicalPuzzle3DirectionName(name) {
  if (name === "forward") {
    return "front";
  }
  if (name === "backward") {
    return "back";
  }
  return name;
}

async function exportSolution() {
  const text = solutionTextForUdlr();
  if (!text) {
    setLevelSolveStatus("No solution to copy", "is-error");
    return;
  }
  try {
    window.focus();
    solutionExportButton.focus({ preventScroll: true });
    await copyTextToClipboard(text);
    flashLevelSolveStatus("Copied solution", "is-ok");
  } catch (error) {
    setLevelSolveStatus(`Could not copy solution: ${error?.message || error}`, "is-error");
  }
}

async function copyTextToClipboard(text) {
  if (copyTextWithCopyEvent(text)) {
    return;
  }

  if (navigator.clipboard?.writeText) {
    try {
      await navigator.clipboard.writeText(text);
      return;
    } catch (_error) {
      // Fall through for embedded or unfocused contexts.
    }
  }

  if (copyTextWithSelection(text)) {
    return;
  }

  throw new Error("clipboard copy was rejected");
}

function copyTextWithCopyEvent(text) {
  let handled = false;
  const onCopy = (event) => {
    event.clipboardData?.setData("text/plain", text);
    event.preventDefault();
    handled = true;
  };
  document.addEventListener("copy", onCopy);
  try {
    return document.execCommand("copy") && handled;
  } finally {
    document.removeEventListener("copy", onCopy);
  }
}

function copyTextWithSelection(text) {
  const textarea = document.createElement("textarea");
  textarea.value = text;
  textarea.setAttribute("readonly", "");
  textarea.style.position = "fixed";
  textarea.style.left = "-9999px";
  textarea.style.top = "0";
  document.body.append(textarea);
  textarea.select();
  try {
    return document.execCommand("copy");
  } finally {
    textarea.remove();
  }
}

function handleSolutionKey(event) {
  if (!levelSolutionPreview) {
    return false;
  }
  const key = event.key.length === 1 ? event.key.toLowerCase() : event.key;
  if (key !== "r") {
    return false;
  }
  resetSolutionPreview();
  event.preventDefault();
  event.stopPropagation();
  return true;
}

function formatNumber(value) {
  return new Intl.NumberFormat("en-US").format(Number(value) || 0);
}

function formatSeconds(milliseconds) {
  return `${((Number(milliseconds) || 0) / 1000).toFixed(1)}s`;
}

function sendPreviewKey(event) {
  if (!levelBuilder.hidden && levelPlaytestActive) {
    pendingPreviewKeyStateSync += 1;
    applyLevelPlaytestKey(event).catch((error) => {
      setStatus(`Play input failed: ${userFacingRuntimeError(error)}`, "is-error");
    });
    return;
  }
  previewFrame.contentWindow?.postMessage({
    type: "PuzzleStudioKey",
    key: event.key,
  }, "*");
}

async function copyLevelToClipboard() {
  const levelName = sanitizeLevelName(levelNameInput.value);
  const source = levelSourceText();
  try {
    await copyTextToClipboard(source);
    setStatus(levelName ? `Copied level ${levelName}` : "Copied unnamed level", "is-ok");
  } catch (error) {
    setStatus(`Could not copy level: ${error?.message || error}`, "is-error");
  }
}

function renderLevelSourcePreview() {
  if (!levelSourcePreview) {
    return;
  }
  syncLevelNameOptions();
  try {
    levelSourcePreview.textContent = levelSourceText();
  } catch (error) {
    levelSourcePreview.textContent = `Could not render level source: ${error?.message || error}`;
  }
}

function levelSourceText() {
  const levelName = sanitizeLevelName(levelNameInput.value);
  return levelDefinitionSource(levelName, levelSourceData(), "", { leadingBlank: false, bodyIndent: "" });
}

function addLevelToSource() {
  ensurePreviewTargetsActiveDocument();
  const previewDocument = activePreviewDocument();
  if (!previewDocument) {
    setStatus("No game entry for level", "is-error");
    return;
  }
  const levelName = sanitizeLevelName(levelNameInput.value);
  const levelNamespace = sanitizeLevelNamespace(levelNamespaceInput.value);
  let sourceData = null;
  try {
    sourceData = levelSourceData();
  } catch (error) {
    setStatus(`Could not create level source: ${error?.message || error}`, "is-error");
    return;
  }
  const nextSource = insertLevel(activePreviewSource(), levelName, sourceData, levelNamespace);
  if (!nextSource) {
    setStatus(levelNamespace ? `No levels named ${levelNamespace}` : "No levels block", "is-error");
    return;
  }
  previewDocument.source = nextSource;
  if (previewDocument.id === activeDocument()?.id) {
    setSourceEditorValue(nextSource, { resetUndo: false });
  }
  levelNameInput.value = nextLevelName(levelName);
  syncLevelNameOptions();
  scheduleLocalSave();
  if (editorSeed && levelName && appendLevelToPreview(levelName, sourceData.rows)) {
    return;
  }
  schedulePreview();
}

function replaceLevelSourceEntry(source, entry, name, levelData) {
  if (!entry) {
    return null;
  }
  const indent = levelDefinitionIndent(source, entry);
  const bodyIndent = levelDefinitionBodyIndent(source, entry, indent);
  const lifecycle = levelLifecycleSourceData(source, entry);
  const replacement = levelDefinitionSource(name, levelData, indent, { leadingBlank: false, lifecycle, bodyIndent });
  const replacementEnd = source[entry.end] === "}" ? entry.end + 1 : entry.end;
  return {
    source: replaceEditorSourceRangePreservingLineBoundary(source, entry.start, replacementEnd, replacement),
  };
}

function updateLevelInSource() {
  ensurePreviewTargetsActiveDocument();
  const previewDocument = activePreviewDocument();
  if (!previewDocument) {
    setStatus("No game entry for level", "is-error");
    return;
  }
  const levelName = sanitizeLevelName(levelNameInput.value);
  const levelNamespace = sanitizeLevelNamespace(levelNamespaceInput.value);
  let sourceData = null;
  try {
    sourceData = levelSourceData();
  } catch (error) {
    setStatus(`Could not create level source: ${error?.message || error}`, "is-error");
    return;
  }
  const source = activePreviewSource();
  const result = levelName
    ? replaceLevelByName(source, levelName, sourceData, levelNamespace)
    : (() => {
      const target = currentLevelSourceLocation();
      if (!target || target.document?.id !== previewDocument.id) {
        return null;
      }
      const entry = sourceEditableEntryFromTarget(source, target, {
        find: findLevelDefinitionAtPosition,
        defaultName: "",
      });
      return replaceLevelSourceEntry(source, entry, "", sourceData);
    })();
  if (!result) {
    setStatus(levelName ? `No level named ${qualifiedLevelName(levelNamespace, levelName)}` : "No current unnamed level", "is-error");
    return;
  }
  previewDocument.source = result.source;
  if (previewDocument.id === activeDocument()?.id) {
    setSourceEditorValue(result.source, { resetUndo: false });
  }
  scheduleLocalSave();
  schedulePreview();
  setStatus(levelName ? `Updated level ${levelName}` : "Updated unnamed level", "is-ok");
}

function appendLevelToPreview(levelName, rows) {
  const exportData = previewExport || extractPreviewExport(latestHtml);
  if (!exportData) {
    markEmbeddedPreviewDirty();
    return false;
  }

  const levelData = exportLevelData(exportData, levelName);
  if (!levelData) {
    markEmbeddedPreviewDirty();
    return false;
  }

  const nextExport = JSON.parse(JSON.stringify(exportData));
  levelData.index = nextExport.levels.length;
  nextExport.levels.push(levelData);
  nextExport.initialLevelIndex = levelData.index;

  const nextHtml = replacePreviewExport(latestHtml, nextExport);
  if (!nextHtml) {
    markEmbeddedPreviewDirty();
    return false;
  }

  previewExport = nextExport;
  latestHtml = nextHtml;
  setActiveLevelIndex(levelData.index, nextExport);
  latestPreviewState = {
    ...(latestPreviewState || {}),
    levelIndex: levelData.index,
    scene: previewSceneForLevel(levelData.index),
  };
  const previewDocument = activePreviewDocument();
  if (previewDocument) {
    previewDocument.previewHtml = nextHtml;
  }
  scheduleLocalSave();
  setPreviewFrameHtml(editorPreviewDocument(nextHtml));
  downloadButton.disabled = false;
  setPreviewMode("play");
  setStatus("Preview updated", "is-ok");
  return true;
}

function exportLevelData(exportData, levelName) {
  const initialState = levelStateData(exportData);
  if (!initialState) {
    return null;
  }

  return {
    index: exportData.levels.length,
    name: levelName,
    regions: levelRegions(),
    initialState,
  };
}

function levelStateData(exportData) {
  const width = level.width;
  const height = level.height;
  const layerCount = exportData?.engine?.layerCount;
  if (!width || !height || !layerCount) {
    return null;
  }

  const slots = Array.from({ length: width * height * layerCount }, () => 0);
  level.cells.forEach((cellSlots, cellIndex) => {
    const sourceSlots = cloneCellSlots(cellSlots, exportData);
    for (let layer = 0; layer < layerCount; layer += 1) {
      slots[(cellIndex * layerCount) + layer] = sourceSlots[layer] || 0;
    }
  });

  const levelIndex = currentEditableLevelIndex(exportData);
  const globalsLength = exportData.levels?.[levelIndex]?.initialState?.globals?.length
    || exportData.levels?.[0]?.initialState?.globals?.length
    || 0;

  return {
    width,
    height,
    layerCount,
    levelIndex,
    slots,
    globals: Array.from({ length: globalsLength }, () => 0),
  };
}

function stateDataFromScene(scene, exportData, levelIndex = currentEditableLevelIndex(exportData)) {
  const width = Math.max(1, Math.trunc(Number(scene?.width) || 0));
  const height = Math.max(1, Math.trunc(Number(scene?.height) || 0));
  const layerCount = exportData?.engine?.layerCount;
  if (!width || !height || !layerCount || !Array.isArray(scene?.cells)) {
    return null;
  }

  const slots = Array.from({ length: width * height * layerCount }, () => 0);
  scene.cells.forEach((cell, cellIndex) => {
    const x = Math.trunc(Number(cell?.x));
    const y = Math.trunc(Number(cell?.y));
    const targetIndex = Number.isInteger(x) && Number.isInteger(y) && x >= 0 && x < width && y >= 0 && y < height
      ? (y * width) + x
      : cellIndex;
    if (targetIndex >= width * height) {
      return;
    }
    const sourceSlots = cellSlotsFromLayers(cell.layers || [], exportData);
    for (let layer = 0; layer < layerCount; layer += 1) {
      slots[(targetIndex * layerCount) + layer] = sourceSlots[layer] || 0;
    }
  });

  const globalsLength = exportData.levels?.[levelIndex]?.initialState?.globals?.length
    || exportData.levels?.[0]?.initialState?.globals?.length
    || 0;

  return {
    width,
    height,
    layerCount,
    levelIndex,
    slots,
    globals: Array.from({ length: globalsLength }, () => 0),
  };
}

function compiledLevelStateData(exportData, levelIndex = currentSolverLevelIndex(exportData)) {
  const state = exportData?.levels?.[levelIndex]?.initialState;
  if (!state) {
    return null;
  }
  return {
    ...JSON.parse(JSON.stringify(state)),
    levelIndex,
  };
}

function solverStateData(exportData) {
  if (solverStateOverride) {
    return JSON.parse(JSON.stringify(solverStateOverride));
  }
  if (isPuzzle3dExport(exportData)) {
    return level3dStateData(exportData);
  }
  return compiledLevelStateData(exportData);
}

function level3dStateData(exportData) {
  return puzzle3dStateDataFromSnapshot(solverPuzzle3dPreviewSnapshot(exportData), exportData);
}

function solverPuzzle3dPreviewSnapshot(exportData = previewExport || extractPreviewExport(latestHtml)) {
  if (solverPuzzle3dSnapshotOverride) {
    return JSON.parse(JSON.stringify(solverPuzzle3dSnapshotOverride));
  }
  return puzzle3dSnapshotForLevel(exportData, currentSolverLevelIndex(exportData));
}

function puzzle3dSnapshotForLevel(exportData, levelIndex = currentSolverLevelIndex(exportData)) {
  if (!isPuzzle3dExport(exportData)) {
    return null;
  }
  const levelEntry = exportData?.levels?.[levelIndex] || {};
  const snapshot = JSON.parse(JSON.stringify(exportData));
  snapshot.levelIndex = levelIndex;
  snapshot.size = { ...(levelEntry.size || exportData?.size || {}) };
  snapshot.cells = Array.isArray(levelEntry.cells) ? JSON.parse(JSON.stringify(levelEntry.cells)) : [];
  return snapshot;
}

function puzzle3dStateDataFromSnapshot(snapshot, exportData = previewExport || extractPreviewExport(latestHtml), options = {}) {
  if (!snapshot) {
    return null;
  }
  const levelIndex = options.levelIndex ?? (Number.isInteger(Number(snapshot.levelIndex))
    ? Math.trunc(Number(snapshot.levelIndex))
    : currentSolverLevelIndex(exportData));
  const levelEntry = exportData?.levels?.[levelIndex] || {};
  const size = snapshot.size || levelEntry.size || exportData?.size || {};
  const width = Math.max(1, Math.trunc(Number(size.width) || 1));
  const depth = Math.max(1, Math.trunc(Number(size.depth) || 1));
  const height = Math.max(1, Math.trunc(Number(size.height) || 1));
  const layerCount = Math.max(1, Math.trunc(Number(exportData?.layerCount) || inferredPuzzle3dLayerCount(exportData)));
  const slots = Array.from({ length: width * depth * height * layerCount }, () => 0);
  const cells = Array.isArray(snapshot.cells)
    ? snapshot.cells
    : Array.isArray(levelEntry.cells)
      ? levelEntry.cells
      : [];
  for (const cell of cells) {
    const position = cell?.position || {};
    const x = Math.trunc(Number(position.x));
    const y = Math.trunc(Number(position.y));
    const z = Math.trunc(Number(position.z));
    if (!Number.isInteger(x) || !Number.isInteger(y) || !Number.isInteger(z)) {
      continue;
    }
    if (x < 0 || x >= width || y < 0 || y >= depth || z < 0 || z >= height) {
      continue;
    }
    for (const object of cell.objects || []) {
      const id = Number(object.id) || 0;
      const layer = Number.isInteger(Number(object.layer))
        ? Math.trunc(Number(object.layer))
        : puzzle3dLayerForObjectId(exportData, id);
      if (!id || layer < 0 || layer >= layerCount) {
        continue;
      }
      const cellIndex = ((z * depth) + y) * width + x;
      slots[(cellIndex * layerCount) + layer] = id;
    }
  }
  return {
    kind: "puzzle3d",
    width,
    depth,
    height,
    layerCount,
    levelIndex,
    slots,
    levelFiredRules: Array.isArray(snapshot.levelFiredRules)
      ? snapshot.levelFiredRules
      : [],
    materializedLevelStart: options.materializedLevelStart === true,
  };
}

function inferredPuzzle3dLayerCount(exportData) {
  const layers = Object.values(exportData?.objects || {})
    .map((object) => Number(object?.layer))
    .filter((layer) => Number.isInteger(layer) && layer >= 0);
  return layers.length ? Math.max(...layers) + 1 : 1;
}

function puzzle3dLayerForObjectId(exportData, objectId) {
  const object = Object.values(exportData?.objects || {})
    .find((candidate) => Number(candidate?.id) === Number(objectId));
  return Number.isInteger(Number(object?.layer)) ? Math.trunc(Number(object.layer)) : -1;
}

function extractPreviewExport(html) {
  if (!html) {
    return null;
  }
  for (const candidate of [
    { kind: "puzzle3d", pattern: /window\.Puzzle3DFrameFixture\s*=\s*JSON\.parse\(("(?:(?:\\.)|[^"\\])*")\);/ },
    { kind: "puzzle3d", pattern: /window\.Puzzle3DFixture\s*=\s*JSON\.parse\(("(?:(?:\\.)|[^"\\])*")\);/ },
    { kind: "puzzle2d", pattern: /window\.PuzzleExport\s*=\s*JSON\.parse\(("(?:(?:\\.)|[^"\\])*")\);/ },
  ]) {
    const match = html.match(candidate.pattern);
    if (!match) {
      continue;
    }
    try {
      const parsed = JSON.parse(JSON.parse(match[1]));
      if (parsed && typeof parsed === "object" && !parsed.__kind) {
        parsed.__kind = candidate.kind;
      }
      return parsed;
    } catch (error) {
      console.error(error);
      return null;
    }
  }
  return null;
}

function replacePreviewExport(html, exportData) {
  const encoded = JSON.stringify(JSON.stringify(exportData));
  const pattern = exportData?.__kind === "puzzle3d"
    ? /window\.Puzzle3DFrameFixture\s*=\s*JSON\.parse\("(?:(?:\\.)|[^"\\])*"\);|window\.Puzzle3DFixture\s*=\s*JSON\.parse\("(?:(?:\\.)|[^"\\])*"\);/
    : /window\.PuzzleExport\s*=\s*JSON\.parse\("(?:(?:\\.)|[^"\\])*"\);/;
  const globalName = exportData?.__kind === "puzzle3d" ? "Puzzle3DFrameFixture" : "PuzzleExport";
  const nextHtml = html.replace(pattern, `window.${globalName} = JSON.parse(${encoded});`);
  return nextHtml === html ? "" : nextHtml;
}

function sanitizeLevelName(value) {
  const cleaned = editableLevelName(value)
    .trim()
    .replace(/[^\w]+/g, "_")
    .replace(/^_+|_+$/g, "");
  return cleaned;
}

function sanitizeLevelNamespace(value) {
  return String(value || "")
    .trim()
    .replace(/[^\w.]+/g, "_")
    .replace(/^_+|_+$/g, "");
}

function setLevelNameInputs(qualifiedName) {
  levelNamespaceInput.value = editableLevelNamespace(qualifiedName);
  levelNameInput.value = editableLevelName(qualifiedName);
  syncLevelNameOptions();
}

function focusedLevelNameControlConfig(source = focusedPuzzleSourceContext()?.source || activePreviewSource(), options = {}) {
  return {
    source,
    nameInput: options.nameInput || levelNameInput,
    datalist: options.datalist || levelNameOptions,
    collectEntries: () => focusedLevelNameControlEntries(source),
  };
}

function focusedLevelNameControlEntries(source = focusedPuzzleSourceContext()?.source || activePreviewSource()) {
  const document = focusedPuzzleSourceContext()?.document || activeDocument();
  const context = document ? { document, source } : focusedPuzzleSourceContext();
  return focusedPuzzleEntries("level", context).map((item) => {
    const target = item.target || {};
    const name = String(target.name || "").trim();
    const sourceName = Object.prototype.hasOwnProperty.call(target, "sourceName")
      ? String(target.sourceName || "").trim()
      : editableLevelName(name);
    const scope = item.dimension === "3d"
      ? String(target.bundle || "levels").trim()
      : sanitizeLevelNamespace(target.namespace || editableLevelNamespace(name));
    const scopedName = item.dimension === "2d" && scope && sourceName && !sourceName.includes(".")
      ? `${scope}.${sourceName}`
      : sourceName || name;
    const displayName = item.dimension === "3d" && scope
      ? `${scope}.${name}`
      : scopedName;
    return {
      range: { namespace: target.namespace || "", bundle: target.bundle || "" },
      entry: target,
      dimension: item.dimension,
      name,
      value: displayName || name,
      label: displayName || name,
    };
  });
}

function levelNameControlConfig(source = focusedPuzzleSourceContext()?.source || activePreviewSource()) {
  return {
    ...focusedLevelNameControlConfig(source, {
      nameInput: levelNameInput,
      datalist: levelNameOptions,
    }),
  };
}

function syncLevelNameOptions() {
  if (typeof syncSourceLevelNameDatalist !== "function") {
    return [];
  }
  return syncSourceLevelNameDatalist(levelNameControlConfig());
}

function levelNamePickerConfig(source = focusedPuzzleSourceContext()?.source || activePreviewSource()) {
  return {
    ...levelNameControlConfig(source),
    load: loadLevelNameEntry,
  };
}

function loadLevelNameEntry(match) {
  const dimension = normalizeEditorDimension(match?.dimension);
  return loadFocusedPuzzleEntry("level", {
    dimension,
    target: {
      ...(match?.entry || {}),
      document: match?.entry?.document || activeDocument(),
    },
  }, { recordHistory: true, silent: false });
}

function showLevelNameOptions() {
  if (typeof showSourceLevelNameMenu !== "function") {
    return syncLevelNameOptions();
  }
  syncLevelNameOptions();
  return showSourceLevelNameMenu(levelNamePickerConfig());
}

function hideLevelNameOptions() {
  if (typeof hideSourceLevelNameMenu === "function") {
    hideSourceLevelNameMenu(levelNameInput);
  }
}

function loadSelectedLevelNameFromInput() {
  if (typeof loadSourceLevelNameSelection !== "function") {
    return false;
  }
  return loadSourceLevelNameSelection(levelNamePickerConfig());
}

function editableLevelNamespace(value) {
  const raw = String(value || "").trim();
  const parts = raw.split(".").filter(Boolean);
  return parts.length > 1 ? parts.slice(0, -1).join(".") : "";
}

function editableLevelName(value) {
  const raw = String(value || "").trim();
  const parts = raw.split(".").filter(Boolean);
  return parts.length ? parts[parts.length - 1] : raw;
}

function editableLevelNameForSourceEntry(entry, fallbackName = "") {
  if (entry && Object.prototype.hasOwnProperty.call(entry, "sourceName")) {
    const sourceName = String(entry.sourceName || "").trim();
    const namespace = sanitizeLevelNamespace(entry.namespace || "");
    return namespace && sourceName && !sourceName.includes(".") ? `${namespace}.${sourceName}` : sourceName;
  }
  return fallbackName;
}

function qualifiedLevelName(namespace, name) {
  const levelName = editableLevelName(name);
  const levelsName = sanitizeLevelNamespace(namespace);
  return levelsName ? `${levelsName}.${levelName}` : levelName;
}

function nextLevelName(name) {
  if (!String(name || "").trim()) {
    return "";
  }
  const match = name.match(/^(.*?)(\d+)$/);
  if (!match) {
    return `${name}_2`;
  }
  return `${match[1]}${Number(match[2]) + 1}`;
}

function levelRows() {
  return levelSourceData().rows;
}

function levelSourceData(source = levelReferenceSource(previewExport || extractPreviewExport(latestHtml)), exportData = previewExport || extractPreviewExport(latestHtml)) {
  const charEntries = sourceCharEntries(source, exportData);
  const allocator = createLevelLegendAllocator(charEntries, sourceReservedLegendChars(source));
  const visualObjects = visualObjectNameSet(exportData);
  const rows = [];
  const regions = levelRegions();
  for (const [regionIndex, region] of regions.entries()) {
    if (regionIndex > 0) {
      rows.push("");
    }
    for (let y = region.y; y < region.y + region.height; y += 1) {
      const row = [];
      for (let x = region.x; x < region.x + region.width; x += 1) {
        row.push(charForSourceCell(level.cells[y * level.width + x], charEntries, allocator, exportData, visualObjects));
      }
      rows.push(row.join(""));
    }
  }
  return { rows, localLegends: allocator.localLegends };
}

function createLevelLegendAllocator(entries, reservedChars = []) {
  const usedChars = new Set([
    ...entries.map((entry) => entry.char),
    ...reservedChars,
  ]);
  const byObjects = new Map();
  const localLegends = [];
  const candidates = levelLegendCandidateChars();

  return {
    localLegends,
    charForObjects(objects) {
      const key = objectSetKey(objects);
      if (byObjects.has(key)) {
        return byObjects.get(key);
      }
      const ch = [...candidates].find((candidate) => !usedChars.has(candidate));
      if (!ch) {
        throw new Error("No unused single-character legend symbol is available for generated level source");
      }
      usedChars.add(ch);
      byObjects.set(key, ch);
      localLegends.push({ char: ch, objects: [...objects] });
      return ch;
    },
  };
}

function levelLegendCandidateChars() {
  const ascii = [..."xyzabcdefghijklmnopqrstuvwABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789@$%&?!~^:;,_+-*/<>|()[]"];
  const ranges = [
    [0x0391, 0x03A1],
    [0x03A3, 0x03FF],
    [0x0400, 0x04FF],
    [0x2190, 0x21FF],
    [0x2200, 0x22FF],
    [0x2460, 0x24FF],
    [0x2500, 0x257F],
    [0x25A0, 0x25FF],
    [0x2600, 0x26FF],
    [0x2700, 0x27BF],
  ];
  const generated = ranges.flatMap(([start, end]) => {
    const chars = [];
    for (let codePoint = start; codePoint <= end; codePoint += 1) {
      chars.push(String.fromCodePoint(codePoint));
    }
    return chars;
  });
  return [...new Set([...ascii, ...generated])].filter((char) => /\S/u.test(char));
}

function charForSourceCell(slots, entries, allocator, exportData = previewExport, _visualObjects = visualObjectNameSet(exportData)) {
  const objects = objectNamesForSlots(slots, exportData);
  const exact = exactCharForObjects(objects, entries);
  if (exact) {
    return exact;
  }
  const commonObjects = objects.filter((object) => commonLegendObjectNames(entries).has(object));
  if (commonObjects.length && commonObjects.length < objects.length) {
    const commonExact = exactCharForObjects(commonObjects, entries);
    if (commonExact) {
      return commonExact;
    }
    return allocator.charForObjects(commonObjects);
  }
  return objects.length ? allocator.charForObjects(objects) : ".";
}

function objectNamesForSlots(slots, exportData = previewExport) {
  return layersForSlots(slots, exportData).map((layer) => layer.object);
}

function commonLegendObjectNames(entries) {
  return new Set(entries
    .filter((entry) => entry.objects.length === 1)
    .map((entry) => entry.objects[0]));
}

function exactCharForObjects(objects, entries) {
  const key = objectSetKey(objects);
  return entries.find((entry) => objectSetKey(entry.objects) === key)?.char || "";
}

function objectSetKey(objects) {
  return [...objects].sort().join("\u0000");
}

function levelRegions() {
  return normalizedLevelRegions(level.regions, level.width, level.height);
}

function defaultLevelRegions(width, height) {
  return [{
    index: 0,
    x: 0,
    y: 0,
    width: Math.max(0, Number(width) || 0),
    height: Math.max(0, Number(height) || 0),
  }];
}

function normalizedLevelRegions(regions, width, height) {
  const boardWidth = Math.max(0, Number(width) || 0);
  const boardHeight = Math.max(0, Number(height) || 0);
  const normalized = (Array.isArray(regions) ? regions : [])
    .map((region, index) => ({
      index: Number.isInteger(region?.index) ? region.index : index,
      x: Math.max(0, Math.trunc(Number(region?.x) || 0)),
      y: Math.max(0, Math.trunc(Number(region?.y) || 0)),
      width: Math.max(0, Math.trunc(Number(region?.width) || 0)),
      height: Math.max(0, Math.trunc(Number(region?.height) || 0)),
    }))
    .map((region) => ({
      ...region,
      width: Math.min(region.width, Math.max(0, boardWidth - region.x)),
      height: Math.min(region.height, Math.max(0, boardHeight - region.y)),
    }))
    .filter((region) => region.width > 0 && region.height > 0)
    .sort((left, right) => left.index - right.index);
  return normalized.length ? normalized : defaultLevelRegions(boardWidth, boardHeight);
}

function resizeLevelRegions(regions, edge, width, height, delta = 1) {
  const normalized = normalizedLevelRegions(regions, level.width, level.height).map((region) => ({ ...region }));
  if (!normalized.length) {
    return defaultLevelRegions(width, height);
  }
  if (delta < 0) {
    if (edge === "top" || edge === "bottom") {
      for (const region of normalized) {
        region.height -= 1;
      }
    } else if (edge === "left") {
      normalized[0].width -= 1;
      for (let index = 1; index < normalized.length; index += 1) {
        normalized[index].x -= 1;
      }
    } else if (edge === "right") {
      normalized[normalized.length - 1].width -= 1;
    }
    return normalizedLevelRegions(normalized, width, height);
  }
  if (edge === "top" || edge === "bottom") {
    for (const region of normalized) {
      region.height += 1;
    }
  } else if (edge === "left") {
    normalized[0].width += 1;
    for (let index = 1; index < normalized.length; index += 1) {
      normalized[index].x += 1;
    }
  } else if (edge === "right") {
    normalized[normalized.length - 1].width += 1;
  }
  return normalizedLevelRegions(normalized, width, height);
}

function sourceCharEntries(source, exportData = previewExport) {
  const entries = [];
  const knownObjects = new Set(engineObjects(exportData).map((object) => object.name));

  for (const row of sourceCommonLegendRows(source)) {
    const entry = legendEntryFromRow(row, knownObjects);
    if (entry) {
      entries.push(entry);
    }
  }

  if (!entries.some((entry) => entry.objects.length === 0)) {
    entries.unshift({ char: ".", objects: [] });
  }

  return entries
    .filter((entry) => entry.char.length === 1)
    .sort((left, right) => right.objects.length - left.objects.length);
}

function sourceCommonLegendRows(source) {
  const lines = sourceLinesWithOffsets(source);
  const rawLines = lines.map((line) => line.raw);
  const levelRanges = sourceLevelLocalRanges(source);
  const rows = [];

  for (let index = 0; index < lines.length; index += 1) {
    if (isOffsetInRanges(lines[index].start, levelRanges)) {
      continue;
    }

    const section = sectionHeaderAtForWasm(rawLines, index);
    if (section?.block === "legend") {
      const result = collectSectionLegendRows(lines, rawLines, index + 3, levelRanges);
      rows.push(...result.rows);
      index = result.endIndex;
      continue;
    }

    const code = levelScannerCode(lines[index].raw);
    if (!code) {
      continue;
    }
    if (/^legend(?:\s*\{)?\s*$/.test(code)) {
      const result = collectLegendBlockRows(lines, index + 1, levelRanges);
      rows.push(...result.rows);
      index = result.endIndex;
      continue;
    }

    const directive = code.match(/^legend\s+(.+)$/);
    if (directive) {
      rows.push(directive[1]);
    }
  }

  return rows;
}

function sourceReservedLegendChars(source) {
  const chars = new Set();
  for (const row of sourceAllLegendRows(source)) {
    const match = String(row || "").match(/^\s*(\S)\s*=/);
    if (match) {
      chars.add(match[1]);
    }
  }
  return chars;
}

function sourceAllLegendRows(source) {
  const lines = sourceLinesWithOffsets(source);
  const rawLines = lines.map((line) => line.raw);
  const rows = [];

  for (let index = 0; index < lines.length; index += 1) {
    const section = sectionHeaderAtForWasm(rawLines, index);
    if (section?.block === "legend") {
      const result = collectSectionLegendRows(lines, rawLines, index + 3, []);
      rows.push(...result.rows);
      index = result.endIndex;
      continue;
    }

    const code = levelScannerCode(lines[index].raw);
    if (!code) {
      continue;
    }
    if (/^legend(?:\s*\{)?\s*$/.test(code)) {
      const result = collectLegendBlockRows(lines, index + 1, []);
      rows.push(...result.rows);
      index = result.endIndex;
      continue;
    }

    const directive = code.match(/^legend\s+(.+)$/);
    if (directive) {
      rows.push(directive[1]);
    }
  }

  return rows;
}

function collectSectionLegendRows(lines, rawLines, startIndex, levelRanges) {
  const rows = [];
  let endIndex = startIndex - 1;
  for (let index = startIndex; index < lines.length; index += 1) {
    if (sectionHeaderAtForWasm(rawLines, index)) {
      break;
    }
    if (isOffsetInRanges(lines[index].start, levelRanges)) {
      continue;
    }
    const code = levelScannerCode(lines[index].raw);
    const tokens = splitLevelTokens(code);
    if (code && sectionBoundaryForWasm("legend", tokens)) {
      break;
    }
    if (isLegendRowForWasm(tokens)) {
      rows.push(code);
    }
    endIndex = index;
  }
  return { rows, endIndex };
}

function collectLegendBlockRows(lines, startIndex, levelRanges) {
  const rows = [];
  let endIndex = startIndex - 1;
  for (let index = startIndex; index < lines.length; index += 1) {
    const code = levelScannerCode(lines[index].raw);
    if (code === "}" || code === "end") {
      endIndex = index;
      break;
    }
    if (!isOffsetInRanges(lines[index].start, levelRanges)) {
      rows.push(code);
    }
    endIndex = index;
  }
  return { rows, endIndex };
}

function sourceLevelLocalRanges(source) {
  const lines = sourceLinesWithOffsets(source);
  const ranges = [];
  for (const levelsRange of findLevelsRanges(source)) {
    let index = lines.findIndex((line) => line.absoluteEnd >= levelsRange.bodyStart);
    if (index < 0) {
      continue;
    }
    while (index < lines.length && lines[index].start <= levelsRange.bodyEnd) {
      if (lines[index].start < levelsRange.bodyStart) {
        index += 1;
        continue;
      }
      const code = levelScannerCode(lines[index].raw);
      const tokens = splitLevelTokens(code);
      if (!code) {
        index += 1;
        continue;
      }
      if (tokens[0] === "legend") {
        const result = collectLegendBlockRows(lines, index + 1, []);
        index = Math.max(index + 1, result.endIndex + 1);
        continue;
      }
      if (isLevelsSectionBoundary(tokens) || code === "}" || code === "end") {
        break;
      }

      let entry = null;
      if (tokens[0] === "level") {
        const nameTokens = tokens.at(-1) === "{" ? tokens.slice(1, -1) : tokens.slice(1);
        entry = tokens.at(-1) === "{"
          ? bracedLevelEntry(source, lines, index, levelNameFromTokens(nameTokens), levelsRange.bodyEnd)
          : unbracedLevelEntry(lines, index, index + 1, levelNameFromTokens(nameTokens), levelsRange.bodyEnd);
      } else if (tokens.length === 1 && tokens[0] === "{") {
        entry = bracedLevelEntry(source, lines, index, "", levelsRange.bodyEnd);
      } else if (tokens.at(-1) === "{" && tokens[0] !== "legend") {
        entry = bracedLevelEntry(source, lines, index, levelNameFromTokens(tokens.slice(0, -1)), levelsRange.bodyEnd);
      }

      if (!entry) {
        index += 1;
        continue;
      }
      ranges.push({ start: entry.start, end: entry.end });
      index = Math.max(index + 1, entry.nextIndex);
    }
  }
  return ranges;
}

function isOffsetInRanges(offset, ranges) {
  return ranges.some((range) => offset >= range.start && offset <= range.end);
}

function legendEntryFromRow(row, knownObjects) {
  const legendMatch = String(row || "").match(/^\s*(\S)\s*=\s*(.+?)\s*$/);
  if (!legendMatch) {
    return null;
  }
  const [, char, expression] = legendMatch;
  const trimmed = expression.trim();
  const parts = trimmed.split(/\s+/);
  const objects = trimmed === "empty"
    ? []
    : parts.filter((part) => knownObjects.has(part));
  return { char, objects };
}

function schemaDomains(source) {
  const domains = new Map();
  for (const line of source.split("\n")) {
    const match = line.match(/^\s*([A-Za-z][\w]*)\s*=\s+([A-Za-z][\w]*(?:\s+[A-Za-z][\w]*)*)\s*$/);
    if (match) {
      domains.set(match[1], match[2].trim().split(/\s+/));
    }
  }
  return domains;
}

function insertLevel(source, name, levelData, namespace = "") {
  const range = findLevelsInsertionRange(source, namespace);
  if (!range) {
    return "";
  }
  const levelIndent = levelInsertionIndent(source, range);
  const bodyIndent = levelInsertionBodyIndent(source, range, levelIndent);
  const levelSource = levelDefinitionSource(name, levelData, levelIndent, { leadingBlank: true, bodyIndent });
  return `${source.slice(0, range.bodyEnd).trimEnd()}\n${levelSource}\n${source.slice(range.bodyEnd)}`;
}

function replaceLevelByName(source, name, levelData, namespace = "") {
  const ranges = findLevelsRanges(source);
  const requestedName = qualifiedLevelName(namespace, name);
  const requestedNamespace = sanitizeLevelNamespace(namespace);
  for (const range of ranges) {
    if (requestedNamespace && sanitizeLevelNamespace(range.namespace) !== requestedNamespace) {
      continue;
    }
    const entry = findLevelDefinitions(source, range)
      .find((candidate) => sourceTitleMatches(candidate.name, requestedName, range.namespace));
    if (!entry) {
      continue;
    }
    const indent = levelDefinitionIndent(source, entry);
    const bodyIndent = levelDefinitionBodyIndent(source, entry, indent);
    const lifecycle = levelLifecycleSourceData(source, entry);
    const replacement = levelDefinitionSource(name, levelData, indent, { leadingBlank: false, lifecycle, bodyIndent });
    const replacementEnd = source[entry.end] === "}" ? entry.end + 1 : entry.end;
    return {
      source: replaceEditorSourceRangePreservingLineBoundary(source, entry.start, replacementEnd, replacement),
    };
  }
  return null;
}

function levelDefinitionSource(name, levelData, levelIndent, options = {}) {
  const { rows, localLegends } = normalizeLevelSourceData(levelData);
  const levelName = sanitizeLevelName(name);
  const lifecycle = options.lifecycle || {};
  const startLifecycleLines = Array.isArray(lifecycle.start) ? lifecycle.start : [];
  const clearLifecycleLines = Array.isArray(lifecycle.clear) ? lifecycle.clear : [];
  const rowIndent = Object.prototype.hasOwnProperty.call(options, "bodyIndent") ? options.bodyIndent : levelIndent;
  const hasRegionBreak = rows.some((row) => row.trim() === "");
  const hasLocalLegends = localLegends.length > 0;
  const hasLifecycle = startLifecycleLines.length > 0 || clearLifecycleLines.length > 0;
  const lines = hasRegionBreak || hasLocalLegends || hasLifecycle
    ? [
      levelName ? `${levelIndent}level ${levelName} {` : `${levelIndent}{`,
      ...levelBodyBlockSourceLines(startLifecycleLines, rowIndent),
      ...levelLegendSourceLines(localLegends, rowIndent),
      ...rows.map((row) => levelMapRowSourceLine(row, rowIndent)),
      ...levelBodyBlockSourceLines(clearLifecycleLines, rowIndent),
      `${levelIndent}}`,
    ]
    : levelName
      ? [
        `${levelIndent}level ${levelName}`,
        ...rows.map((row) => levelMapRowSourceLine(row, rowIndent)),
      ]
      : rows.map((row) => levelMapRowSourceLine(row, rowIndent));
  return `${options.leadingBlank ? "\n" : ""}${lines.join("\n")}`;
}

function levelMapRowSourceLine(row, indent) {
  return String(row || "").length ? `${indent}${row}` : "";
}

function levelBodyBlockSourceLines(lines, indent) {
  const out = [];
  let depth = 0;
  for (const rawLine of lines || []) {
    const line = String(rawLine || "").trim();
    if (!line) {
      out.push("");
      continue;
    }
    const normalized = braceNormalizedLineForSectionForWasm(line);
    const isClose = normalized === "}" || normalized === "end";
    const lineDepth = Math.max(0, depth - (isClose ? 1 : 0));
    out.push(`${indent}${line}`);
    depth = lineDepth + (startsInlineBlockForWasm(splitLevelTokens(normalized), normalized) ? 1 : 0);
  }
  return out;
}

function levelLifecycleSourceData(source, entry) {
  const lines = sourceLinesWithOffsets(source.slice(entry.start, entry.end)).map((line) => line.raw);
  if (lines.length <= 1) {
    return { start: [], clear: [] };
  }
  const start = [];
  const clear = [];
  let sawMapRow = false;
  let index = 1;
  while (index < lines.length) {
    const code = levelScannerCode(lines[index]);
    if (!code) {
      index += 1;
      continue;
    }
    const normalized = braceNormalizedLineForSectionForWasm(code);
    const tokens = splitLevelTokens(normalized);
    if (isLevelLifecycleHeader(tokens)) {
      const block = collectLevelBodySourceBlock(lines, index);
      (tokens[0] === "on_level_start" ? start : clear).push(...block.lines);
      index = block.nextIndex;
      continue;
    }
    if (isLevelEventSugarCode(code)) {
      (sawMapRow ? clear : start).push(code);
      index += 1;
      continue;
    }
    if (startsLevelBodyBlock(tokens, normalized)) {
      index = skipLevelBodySourceBlock(lines, index);
      continue;
    }
    sawMapRow = true;
    index += 1;
  }
  return { start, clear };
}

function isLevelLifecycleHeader(tokens) {
  return tokens.length === 1 && (tokens[0] === "on_level_start" || tokens[0] === "on_level_clear");
}

function isLevelEventSugarCode(code) {
  const tokens = splitLevelTokens(code);
  return code.startsWith("message ")
    || tokens[0] === "wait"
    || (tokens[0] === "sfx" && tokens.length === 2);
}

function collectLevelBodySourceBlock(lines, startIndex) {
  const blockLines = [levelScannerCode(lines[startIndex])];
  let nestedDepth = 0;
  let index = startIndex + 1;
  while (index < lines.length) {
    const code = levelScannerCode(lines[index]);
    if (code) {
      const normalized = braceNormalizedLineForSectionForWasm(code);
      const tokens = splitLevelTokens(normalized);
      blockLines.push(code);
      if (normalized === "end" || normalized === "}") {
        if (nestedDepth === 0) {
          return { lines: blockLines, nextIndex: index + 1 };
        }
        nestedDepth -= 1;
      } else if (startsInlineBlockForWasm(tokens, normalized)) {
        nestedDepth += 1;
      }
    }
    index += 1;
  }
  return { lines: blockLines, nextIndex: index };
}

function skipLevelBodySourceBlock(lines, startIndex) {
  let nestedDepth = 0;
  let index = startIndex + 1;
  while (index < lines.length) {
    const code = levelScannerCode(lines[index]);
    if (code) {
      const normalized = braceNormalizedLineForSectionForWasm(code);
      const tokens = splitLevelTokens(normalized);
      if (normalized === "end" || normalized === "}") {
        if (nestedDepth === 0) {
          return index + 1;
        }
        nestedDepth -= 1;
      } else if (startsInlineBlockForWasm(tokens, normalized)) {
        nestedDepth += 1;
      }
    }
    index += 1;
  }
  return index;
}

function normalizeLevelSourceData(levelData) {
  if (Array.isArray(levelData)) {
    return { rows: levelData, localLegends: [] };
  }
  return {
    rows: Array.isArray(levelData?.rows) ? levelData.rows : [],
    localLegends: Array.isArray(levelData?.localLegends) ? levelData.localLegends : [],
  };
}

function levelLegendSourceLines(localLegends, indent) {
  if (!localLegends.length) {
    return [];
  }
  const bodyIndent = indent;
  return [
    `${indent}legend {`,
    ...localLegends.map((entry) => `${bodyIndent}${entry.char} = ${entry.objects.join(" ")}`),
    `${indent}}`,
  ];
}

function levelDefinitionIndent(source, entry) {
  const lines = sourceLinesWithOffsets(source);
  const line = lines.find((candidate) => entry.start >= candidate.start && entry.start <= candidate.end);
  return line ? lineIndent(line.raw) : "";
}

function sourceTitleMatches(existing, title, namespace = "") {
  const existingTitle = String(existing || "").trim();
  const requested = editableLevelName(title);
  const requestedNamespace = sanitizeLevelNamespace(editableLevelNamespace(title) || namespace);
  const existingNamespace = sanitizeLevelNamespace(editableLevelNamespace(existingTitle) || namespace);
  const editableExisting = editableLevelName(existingTitle);
  const normalizedExisting = sanitizeLevelName(editableExisting);
  const normalizedRequested = sanitizeLevelName(requested);
  return existingTitle === requested
    || existingTitle.endsWith(`.${requested}`)
    || (normalizedExisting && normalizedRequested && normalizedExisting === normalizedRequested)
    || (
      editableExisting === requested
      && (!requestedNamespace || !existingNamespace || requestedNamespace === existingNamespace)
    );
}

function findLevelsInsertionRange(source, namespace = "") {
  const ranges = findLevelsRanges(source);
  if (!ranges.length) {
    return null;
  }
  const requestedNamespace = sanitizeLevelNamespace(namespace);
  const matchingRanges = requestedNamespace
    ? ranges.filter((range) => sanitizeLevelNamespace(range.namespace) === requestedNamespace)
    : ranges;
  if (requestedNamespace && !matchingRanges.length) {
    return null;
  }
  const activePosition = activeDocument()?.id === activePreviewDocument()?.id
    ? sourceEditor.selectionStart
    : -1;
  return matchingRanges.find((range) => activePosition >= range.bodyStart && activePosition <= range.bodyEnd)
    || matchingRanges.at(-1)
    || ranges.at(-1);
}

function levelInsertionIndent(source, range) {
  const existing = findLevelDefinitions(source, range)[0];
  if (existing) {
    const lines = sourceLinesWithOffsets(source);
    const line = lines.find((candidate) => existing.start >= candidate.start && existing.start <= candidate.end);
    if (line) {
      return lineIndent(line.raw);
    }
  }
  return range.indent || "";
}

function levelInsertionBodyIndent(source, range, levelIndent) {
  const existing = findLevelDefinitions(source, range)[0];
  return existing ? levelDefinitionBodyIndent(source, existing, levelIndent) : levelIndent;
}

function levelDefinitionBodyIndent(source, entry, levelIndent) {
  const lines = sourceLinesWithOffsets(source);
  const headerIndex = lines.findIndex((line) => entry.start >= line.start && entry.start <= line.end);
  if (headerIndex >= 0) {
    for (let index = headerIndex + 1; index < lines.length; index += 1) {
      const line = lines[index];
      if (line.start > entry.end) {
        break;
      }
      const code = levelScannerCode(line.raw);
      if (!code || code === "}") {
        continue;
      }
      const indent = lineIndent(line.raw);
      if (indent.startsWith(levelIndent) && indent.length > levelIndent.length) {
        return indent;
      }
      break;
    }
  }
  return levelIndent;
}

function lineIndent(line) {
  return String(line || "").match(/^[\t ]*/)?.[0] || "";
}

function findNamedBlock(source, name) {
  const pattern = new RegExp(`(^|\\n)([\\t ]*)${name}\\s*\\{`, "m");
  const match = pattern.exec(source);
  if (!match) {
    return null;
  }
  const openIndex = source.indexOf("{", match.index + match[0].lastIndexOf(name));
  const closeIndex = findMatchingBrace(source, openIndex);
  if (closeIndex < 0) {
    return null;
  }
  return {
    indent: match[2] || "",
    bodyStart: openIndex + 1,
    bodyEnd: closeIndex,
  };
}

function findMatchingBrace(source, openIndex) {
  let depth = 0;
  for (let index = openIndex; index < source.length; index += 1) {
    if (source[index] === "{") {
      depth += 1;
    } else if (source[index] === "}") {
      depth -= 1;
      if (depth === 0) {
        return index;
      }
    }
  }
  return -1;
}

runButton.addEventListener("click", runPreviewFromSourcePane);
previewRefreshButton?.addEventListener("click", renderPreview);
clearPreviewLogButton?.addEventListener("click", clearPreviewLog);
previewLogOutput?.addEventListener("click", activatePreviewLogLocationFromEvent);
previewLogOutput?.addEventListener("keydown", (event) => {
  if (event.key !== "Enter" && event.key !== " ") {
    return;
  }
  activatePreviewLogLocationFromEvent(event);
});
saveButton.addEventListener("click", () => {
  saveCurrentDocument(true).catch((error) => {
    console.error(error);
    setEditorStatus("Save failed", "is-error");
    saveButton.disabled = false;
  });
});
document.addEventListener("keydown", handleSaveShortcut);
document.addEventListener("keydown", handleExplorerToggleShortcut);
document.addEventListener("keydown", handleVisualEditUndoShortcut);
document.addEventListener("click", (event) => {
  if (fileActionsMenu?.hidden) {
    return;
  }
  if (event.target.closest("#fileActionsMenu, #fileActionsButton")) {
    return;
  }
  setFileActionsMenuOpen(false);
});
document.addEventListener("keydown", (event) => {
  if (event.key === "Escape") {
    setFileActionsMenuOpen(false);
  }
});
newDocumentButton.addEventListener("click", createNewFile);
fileActionsButton?.addEventListener("click", (event) => {
  event.stopPropagation();
  setFileActionsMenuOpen(fileActionsMenu.hidden);
});
newFolderButton.addEventListener("click", () => {
  setFileActionsMenuOpen(false);
  createNewFolder();
});
importButton.addEventListener("click", () => {
  setFileActionsMenuOpen(false);
  importFileInput.click();
});
importFolderButton.addEventListener("click", () => {
  setFileActionsMenuOpen(false);
  importFolderInput.click();
});
document.addEventListener("click", (event) => {
  const button = event.target.closest("[data-open-project], [data-open-workspace]");
  if (!button) {
    return;
  }
  event.preventDefault();
  setFileActionsMenuOpen(false);
  openProjectFromDesktop(button.dataset.openWorkspace || "folder").catch((error) => {
    console.error(error);
    setEditorStatus("Open failed", "is-error");
    setOpenProjectButtonsDisabled(false);
  });
});
downloadButton.addEventListener("click", downloadHtml);
themeToggleButton?.addEventListener("click", toggleEditorTheme);
importFileInput.addEventListener("change", () => {
  importFiles(importFileInput.files).catch((error) => {
    console.error(error);
    setEditorStatus("Import failed", "is-error");
  });
  importFileInput.value = "";
});
importFolderInput.addEventListener("change", () => {
  importFiles(importFolderInput.files).catch((error) => {
    console.error(error);
    setEditorStatus("Import failed", "is-error");
  });
  importFolderInput.value = "";
});
documentTabs?.addEventListener("click", (event) => {
  const closeButton = event.target.closest("[data-close-tab]");
  if (closeButton) {
    event.stopPropagation();
    closeDocumentTab(closeButton.dataset.closeTab);
    return;
  }
  const tab = event.target.closest("[data-document-tab]");
  if (!tab || tab.dataset.documentTab === activeFileId) {
    return;
  }
  activateDocumentTab(tab.dataset.documentTab);
});
documentTabs?.addEventListener("scroll", updateDocumentTabScrollState, { passive: true });
documentTabs?.addEventListener("wheel", (event) => {
  const maxScroll = Math.max(0, documentTabs.scrollWidth - documentTabs.clientWidth);
  if (maxScroll <= 1) {
    return;
  }
  const delta = normalizedDocumentTabWheelDelta(event);
  if (!delta) {
    return;
  }
  event.preventDefault();
  documentTabs.scrollLeft = Math.max(0, Math.min(maxScroll, documentTabs.scrollLeft + delta));
  updateDocumentTabScrollState();
}, { passive: false });
documentTabs?.addEventListener("keydown", (event) => {
  if (event.altKey || event.ctrlKey || event.metaKey) {
    return;
  }
  if (event.key === "ArrowLeft") {
    event.preventDefault();
    moveDocumentTabFocus(-1);
  } else if (event.key === "ArrowRight") {
    event.preventDefault();
    moveDocumentTabFocus(1);
  } else if (event.key === "Home") {
    event.preventDefault();
    activateDocumentTab(openTabIds[0]);
  } else if (event.key === "End") {
    event.preventDefault();
    activateDocumentTab(openTabIds[openTabIds.length - 1]);
  }
});
window.addEventListener("resize", updateDocumentTabScrollState);
if (window.ResizeObserver && documentTabs) {
  new ResizeObserver(updateDocumentTabScrollState).observe(documentTabs);
}
documentList.addEventListener("click", (event) => {
  const actionButton = event.target.closest("[data-tree-action]");
  if (actionButton && documentList.contains(actionButton)) {
    event.preventDefault();
    event.stopPropagation();
    const row = actionButton.closest(".tree-row");
    const node = treeNodeFromRow(row);
    if (!node) {
      return;
    }
    selectedTreeId = node.id;
    if (actionButton.dataset.treeAction === "rename") {
      startRenameEntry(node.id);
    } else if (actionButton.dataset.treeAction === "delete") {
      deleteTreeNode(node.id).catch((error) => {
        console.error(error);
        setEditorStatus(workspaceMutationErrorMessage("Delete failed", error), "is-error");
      });
    } else if (actionButton.dataset.treeAction === "remove-workspace") {
      removeWorkspaceNode(node.id).catch((error) => {
        console.error(error);
        setEditorStatus("Remove failed", "is-error");
      });
    }
    return;
  }

  const row = event.target.closest(".tree-row");
  if (!row) {
    return;
  }
  if (row.dataset.nodeId) {
    const folder = findNode(fileTree, row.dataset.nodeId);
    if (folder?.kind === "folder") {
      if (event.target.closest(".tree-chevron, .tree-icon")) {
        folder.expanded = folder.expanded === false;
      }
      loadFolderPreview(folder);
    }
    return;
  }
  if (row.dataset.fileId) {
    persistCurrentDocument();
    saveDocumentStore(false);
    activeFileId = row.dataset.fileId;
    selectedTreeId = activeFileId;
    selectedFolderId = findParentFolder(fileTree, activeFileId)?.id || "";
    syncDocumentsFromTree();
    loadEmbeddedDocument(activeDocumentIndex());
  }
});
documentList.addEventListener("keydown", (event) => {
  if (!["Enter", " ", "ArrowRight", "ArrowLeft"].includes(event.key)) {
    return;
  }
  const row = event.target.closest(".tree-row");
  if (!row || event.target.closest("input, button")) {
    return;
  }
  event.preventDefault();
  if (row.dataset.nodeId && ["ArrowRight", "ArrowLeft"].includes(event.key)) {
    const folder = findNode(fileTree, row.dataset.nodeId);
    if (folder?.kind === "folder") {
      folder.expanded = event.key === "ArrowRight";
      loadFolderPreview(folder);
    }
    return;
  }
  row.click();
});
documentList.addEventListener("dragstart", (event) => {
  const row = event.target.closest(".tree-row");
  if (!row?.dataset.dragId || row.classList.contains("draft-row")) {
    event.preventDefault();
    return;
  }
  draggedNodeId = row.dataset.dragId;
  row.classList.add("is-dragging");
  event.dataTransfer.effectAllowed = "move";
  event.dataTransfer.setData("text/plain", draggedNodeId);
});
documentList.addEventListener("dragover", (event) => {
  const fileCount = event.dataTransfer?.files?.length || 0;
  const targetFolderId = dropFolderIdForEvent(event);
  if (fileCount && isDesktopHost()) {
    return;
  }
  if (!fileCount && !canDropNodeOnFolder(draggedNodeId, targetFolderId)) {
    return;
  }
  event.preventDefault();
  event.dataTransfer.dropEffect = fileCount ? "copy" : "move";
  markDropTarget(targetFolderId);
});
documentList.addEventListener("dragleave", (event) => {
  if (!documentList.contains(event.relatedTarget)) {
    clearDropTargets();
  }
});
documentList.addEventListener("drop", (event) => {
  event.preventDefault();
  const files = event.dataTransfer?.files;
  const targetFolderId = dropFolderIdForEvent(event);
  clearDropTargets();
  if (files?.length) {
    if (isDesktopHost()) {
      setEditorStatus("Use Open file or Open folder in the desktop app", "is-error");
      return;
    }
    const targetFolder = targetFolderId ? findNode(fileTree, targetFolderId) : fileTree;
    if (targetFolder?.kind === "folder") {
      importFilesIntoFolder(files, targetFolder).catch((error) => {
        console.error(error);
        setEditorStatus("Import failed", "is-error");
      });
    }
    return;
  }
  if (moveNodeToFolder(draggedNodeId, targetFolderId)) {
    setEditorStatus("Moved", "is-ok");
  }
});
documentList.addEventListener("dragend", () => {
  draggedNodeId = "";
  clearDropTargets();
  documentList.querySelectorAll(".is-dragging").forEach((row) => row.classList.remove("is-dragging"));
});
initializePhysicalWorkPanes();
paneToggleButtons.forEach((button) => {
  button.addEventListener("click", () => togglePaneVisibility(button.dataset.paneToggle));
});
workbench.addEventListener("click", (event) => {
  const maximizeButton = event.target.closest("[data-pane-maximize]");
  if (maximizeButton && workbench.contains(maximizeButton)) {
    toggleWorkPaneMaximized(maximizeButton.dataset.paneMaximize);
    return;
  }
  const button = event.target.closest("[data-pane-close]");
  if (!button || !workbench.contains(button)) {
    return;
  }
  closeWorkPane(button.dataset.paneClose);
});
workbench.addEventListener("pointerdown", handleWorkPaneFocus);
workbench.addEventListener("focusin", handleWorkPaneFocus);
workbench.addEventListener("dragstart", (event) => {
  const handle = event.target.closest("[data-pane-drag-handle]");
  if (!handle || !workbench.contains(handle)) {
    return;
  }
  startWorkPaneDrag(event);
});
workbench.addEventListener("dragend", (event) => {
  if (event.target.closest("[data-pane-drag-handle]")) {
    stopWorkPaneDrag();
  }
});
workbench.addEventListener("dragover", handleWorkPaneDragOver);
workbench.addEventListener("drop", handleWorkPaneDrop);
workbench.addEventListener("dragleave", (event) => {
  if (draggingWorkPaneId && !workbench.contains(event.relatedTarget)) {
    clearWorkPaneDropState({ keepDragSource: true });
  }
});
window.addEventListener("message", (event) => {
  if (event.data?.type === "PuzzleStudioPreviewLayout") {
    return;
  }
  if (event.data?.type === "PuzzleStudioPreviewLoaded") {
    latestPreviewRuntimeStatus = {
      title: event.data.title || "",
      href: event.data.href || "",
    };
    return;
  }
  if (event.data?.type === "PuzzleStudioPreviewState") {
    applyPreviewTheme(event.data.theme || previewExport?.theme || null);
    syncPreviewViewportAspect(event.data.screen || "");
    const inLevelMode = !levelBuilder.hidden || !solverPanel.hidden;
    const screenHasPuzzle = event.data.screenHasPuzzle !== false;
    const levelIndex = Number.isInteger(Number(event.data.levelIndex))
      ? Math.trunc(Number(event.data.levelIndex))
      : latestPreviewState?.levelIndex ?? 0;
    latestPreviewState = {
      levelIndex,
      rawScene: event.data.rawScene,
      scene: event.data.scene,
      puzzle3Snapshot: event.data.puzzle3Snapshot || null,
      inputs: event.data.inputs || [],
      screen: event.data.screen || "",
      screenHasPuzzle,
    };
    if (inLevelMode) {
      if (!levelBuilder.hidden && levelPlaytestActive && pendingPreviewKeyStateSync > 0) {
        pendingPreviewKeyStateSync = Math.max(0, pendingPreviewKeyStateSync - 1);
      }
      if (screenHasPuzzle && event.data.scene && (levelPlaytestActive || !solverPanel.hidden)) {
        const displayCells = sceneCellsToSlots(event.data.scene, []);
        levelDisplayCells = displayCells.length === level.cells.length ? displayCells : null;
        renderLevelBoard();
      }
      if (levelSolutionPreview) {
        updateSolutionControls();
      }
    }
    return;
  }
  if (event.data?.type === "PuzzleStudioPreviewLog") {
    appendPreviewLog(event.data.level, event.data.message, {
      source: event.data.source || "preview",
      origin: event.data.origin || "",
    });
    return;
  }
  if (event.data?.type === "PuzzleStudioSolveResult") {
    handleLevelSolveResult(event.data);
  }
});
window.addEventListener("resize", syncPreviewViewportScale);
window.addEventListener("resize", syncLevelBoardScale);
window.addEventListener("resize", syncSolverBoardScale);
if (window.ResizeObserver && previewFrameWrap) {
  const previewWrapObserver = new ResizeObserver(() => schedulePreviewViewportSync(2));
  previewWrapObserver.observe(previewFrameWrap);
}
if (window.ResizeObserver && levelBoardViewport) {
  const levelWrapObserver = new ResizeObserver(syncLevelBoardScale);
  const levelWrap = levelBoardViewport.closest(".level-board-wrap");
  if (levelWrap) {
    levelWrapObserver.observe(levelWrap);
  }
  if (levelBuilder) {
    levelWrapObserver.observe(levelBuilder);
  }
}
if (window.ResizeObserver && solverBoardViewport) {
  const solverWrapObserver = new ResizeObserver(syncSolverBoardScale);
  const solverWrap = solverBoardViewport.closest(".solver-board-wrap");
  if (solverWrap) {
    solverWrapObserver.observe(solverWrap);
  }
  if (solverPanel) {
    solverWrapObserver.observe(solverPanel);
  }
}
paneSplitter.addEventListener("pointerdown", startPaneResize);
previewLogSplitter?.addEventListener("pointerdown", startPreviewLogResize);
explorerSplitter.addEventListener("pointerdown", startExplorerResize);
document.addEventListener("pointermove", resizePanes);
document.addEventListener("pointermove", resizeExplorer);
document.addEventListener("pointermove", resizePreviewLog);
document.addEventListener("pointerup", stopActiveResize);
document.addEventListener("pointercancel", stopActiveResize);
paneSplitter.addEventListener("lostpointercapture", stopPaneResize);
previewLogSplitter?.addEventListener("lostpointercapture", stopPreviewLogResize);
explorerSplitter.addEventListener("lostpointercapture", stopExplorerResize);
window.addEventListener("blur", () => stopActiveResize());
sourceEditor.addEventListener("click", loadLevelFromSourceClick);
playModeButton.addEventListener("click", () => {
  openPreviewModePane("play");
});
editModeButton.addEventListener("click", () => {
  openLevelPaneForCurrentDimension();
});
solverModeButton.addEventListener("click", () => {
  ensurePreviewTargetsActiveDocument();
  openPreviewModePane("solver");
  syncSourceFromPreviewPane("solver");
});
for (const button of editorDimensionButtons) {
  button.addEventListener("click", () => {
    const context = focusedPuzzleSourceContext();
    const previousMode = currentPreviewMode;
    const activeKind = previousMode === "sprite" || previousMode === "sprite3d" ? "sprite" : "level";
    const first = ["edit", "level3d", "sprite", "sprite3d"].includes(previousMode)
      ? firstFocusedPuzzleEntry(activeKind, context)
      : firstFocusedPuzzleEntry("level", context);
    if (first) {
      ensurePreviewTargetsActiveDocument();
      openPreviewModePane(activeKind === "sprite"
        ? spriteModeForEditorDimension(first.dimension)
        : levelModeForEditorDimension(first.dimension));
      loadFocusedPuzzleEntry(activeKind, first, { silent: true, recordHistory: false });
      return;
    }
    setEditorDimensionMode(button.dataset.editorDimension);
  });
}
for (const button of levelPaneModeButtons) {
  button.addEventListener("click", () => {
    const context = focusedPuzzleSourceContext();
    if (!["edit", "level3d"].includes(button.dataset.levelPaneMode)) {
      return;
    }
    ensurePreviewTargetsActiveDocument();
    if (!loadLevelPaneEntryForMode(button.dataset.levelPaneMode, context, { silent: true, recordHistory: false })) {
      return;
    }
  });
}
spriteModeButton.addEventListener("click", () => {
  openSpritePaneForCurrentDimension();
});
newSpriteButton?.addEventListener("click", addEmptySprite2dToFocusedSource);
newSprite3dButton?.addEventListener("click", addEmptySprite3dToFocusedSource);
sprite3dModeButton?.addEventListener("click", () => {
  openSpritePaneForCurrentDimension();
});
for (const button of spritePaneModeButtons) {
  button.addEventListener("click", () => {
    const context = focusedPuzzleSourceContext();
    if (!["sprite", "sprite3d"].includes(button.dataset.spritePaneMode)) {
      return;
    }
    const first = firstFocusedPuzzleEntry("sprite", context);
    if (!first) {
      return;
    }
    openPreviewModePane(spriteModeForEditorDimension(first.dimension));
    loadFocusedPuzzleEntry("sprite", first, { silent: true, recordHistory: false });
  });
}
addEmptyLevel2dButton?.addEventListener("click", addEmptyLevel2dToFocusedSource);
addEmptyLevel3dButton?.addEventListener("click", addEmptyLevel3dToFocusedSource);
soundsTopbarButton.addEventListener("click", () => {
  openPreviewModePane("sounds");
  syncSourceFromPreviewPane("sounds");
});
psImportTopbarButton?.addEventListener("click", () => {
  openPreviewModePane("psimport");
});
docsTopbarButton?.addEventListener("click", () => {
  openPreviewModePane("docs");
  docsSearchInput?.focus();
});
for (const button of docsPageButtons) {
  button.addEventListener("click", () => {
    const pageId = button.dataset.docsPage || "";
    for (const item of docsPageButtons) {
      const active = item.dataset.docsPage === pageId;
      item.classList.toggle("is-active", active);
      item.setAttribute("aria-selected", String(active));
    }
    for (const article of docsArticles) {
      article.hidden = article.dataset.docsArticle !== pageId;
    }
    if (docsPanel) {
      docsPanel.scrollTop = 0;
    }
  });
}
psImportSourceInput?.addEventListener("input", resetPuzzleScriptImportConversion);
psImportConvertButton?.addEventListener("click", () => {
  convertPuzzleScriptImport().catch((error) => {
    console.error(error);
    setPuzzleScriptImportStatus(error.message || String(error), "is-error");
  });
});
psImportCopyButton?.addEventListener("click", () => {
  copyPuzzleScriptImportOutput().catch((error) => {
    console.error(error);
    setPuzzleScriptImportStatus("Copy failed", "is-error");
  });
});
psImportAddFileButton?.addEventListener("click", () => {
  addPuzzleScriptImportFile().catch((error) => {
    console.error(error);
    setPuzzleScriptImportStatus(error.message || String(error), "is-error");
  });
});
levelPaletteCollapseButton.addEventListener("click", () => {
  level.paletteCollapsed = !level.paletteCollapsed;
  renderLevelPalette();
});
levelPlaytestButton?.addEventListener("click", toggleLevelPlaytest);
levelBoard.addEventListener("pointerdown", startLevelPaint);
levelBoard.addEventListener("pointermove", continueLevelPaint);
levelBoard.addEventListener("pointerup", stopLevelPaint);
levelBoard.addEventListener("pointercancel", stopLevelPaint);
levelBoard.addEventListener("keydown", (event) => {
  if (handleSolutionKey(event)) {
    return;
  }
  if (!levelPlaytestActive && (event.key === "Enter" || event.key === " ") && latestPreviewState?.screenHasPuzzle !== false) {
    const mutate = levelBucketActive ? bucketFillLevelFromElement : paintLevelCellFromElement;
    if (withVisualEditHistory("level", () => mutate(event.target))) {
      event.preventDefault();
      event.stopPropagation();
      return;
    }
  }
  if (!levelPlaytestActive) {
    return;
  }
  sendPreviewKey(event);
  event.preventDefault();
  event.stopPropagation();
});
solverBoard.addEventListener("keydown", (event) => {
  handleSolutionKey(event);
});
document.addEventListener("keydown", (event) => {
  if ((levelBuilder.hidden && solverPanel.hidden) || ["INPUT", "TEXTAREA", "SELECT"].includes(event.target.tagName)) {
    return;
  }
  if (handleSolutionKey(event)) {
    return;
  }
  if (levelBuilder.hidden) {
    return;
  }
  if (!levelPlaytestActive) {
    return;
  }
  sendPreviewKey(event);
  event.preventDefault();
});
levelEdgeButtons.forEach((button) => {
  button.addEventListener("click", () => {
    const mode = levelStageResizeMode();
    if (!mode) {
      return;
    }
    resizeLevelEdge(button.dataset.levelEdge, mode);
  });
});
levelExpandButton?.addEventListener("click", () => toggleLevelResizeMode("expand"));
levelShrinkButton?.addEventListener("click", () => toggleLevelResizeMode("shrink"));
levelGridButton?.addEventListener("click", toggleLevelGrid);
levelLayerVisibilityButton?.addEventListener("click", (event) => {
  event.stopPropagation();
  toggleLevelLayerVisibilityMenu();
});
levelLayerVisibilityMenu?.addEventListener("click", (event) => {
  event.stopPropagation();
});
levelLayerVisibilityMenu?.addEventListener("change", (event) => {
  const checkbox = event.target?.closest?.("[data-level-visibility-layer]");
  if (!checkbox) {
    return;
  }
  setLevelLayerVisible(Number(checkbox.dataset.levelVisibilityLayer), checkbox.checked);
});
document.addEventListener("click", (event) => {
  if (!levelLayerVisibilityMenu || levelLayerVisibilityMenu.hidden) {
    return;
  }
  if (levelLayerVisibilityMenu.contains(event.target) || levelLayerVisibilityButton?.contains(event.target)) {
    return;
  }
  levelLayerVisibilityMenu.hidden = true;
  syncLevelLayerVisibilityControl();
});
levelRotateLeftButton?.addEventListener("click", rotateLevelLeft);
levelRotateRightButton?.addEventListener("click", rotateLevelRight);
levelFlipHorizontalButton?.addEventListener("click", flipLevelHorizontal);
levelFlipVerticalButton?.addEventListener("click", flipLevelVertical);
levelFillButton?.addEventListener("click", toggleLevelBucketMode);
syncLevelResizeControls();
levelNamespaceInput.addEventListener("input", () => {
  renderLevelSourcePreview();
  if (document.activeElement === levelNameInput) {
    showLevelNameOptions();
  }
});
levelNamespaceInput.addEventListener("focus", syncLevelNameOptions);
levelNameInput.addEventListener("input", () => {
  renderLevelSourcePreview();
  showLevelNameOptions();
});
levelNameInput.addEventListener("focus", showLevelNameOptions);
levelNameInput.addEventListener("blur", () => window.setTimeout(hideLevelNameOptions, 120));
levelNameInput.addEventListener("keydown", (event) => {
  if (event.key === "Escape") {
    hideLevelNameOptions();
  }
});
levelNameInput.addEventListener("change", () => {
  if (!loadSelectedLevelNameFromInput()) {
    renderLevelSourcePreview();
  }
});
copyLevelButton.addEventListener("click", copyLevelToClipboard);
addLevelButton.addEventListener("click", addLevelToSource);
updateLevelButton.addEventListener("click", updateLevelInSource);
solveLevelButton.addEventListener("click", () => {
  solveLevel().catch((error) => {
    setLevelSolveStatus(`Solve failed: ${userFacingRuntimeError(error)}`, "is-error");
  });
});
solverLevelSelect?.addEventListener("change", () => selectSolverLevel(Number(solverLevelSelect.value)));
levelSolveShortcutButton?.addEventListener("click", () => {
  solveEditedLevelFromEditor().catch((error) => {
    setLevelSolveStatus(`Solve failed: ${userFacingRuntimeError(error)}`, "is-error");
  });
});
level3dSolveShortcutButton?.addEventListener("click", () => {
  solveEditedLevelFromEditor().catch((error) => {
    setLevelSolveStatus(`Solve failed: ${userFacingRuntimeError(error)}`, "is-error");
  });
});
solutionPrevButton.addEventListener("click", () => setSolutionStep((levelSolutionPreview?.index || 0) - 1));
solutionNextButton.addEventListener("click", () => setSolutionStep((levelSolutionPreview?.index || 0) + 1));
solutionPlayButton.addEventListener("click", toggleSolutionPlayback);
solutionSpeedSelect.addEventListener("change", changeSolutionPlaybackSpeed);
solutionResetButton.addEventListener("click", resetSolutionPreview);
solutionExportButton.addEventListener("click", exportSolution);
solutionSeekInput.addEventListener("input", seekSolutionStep);
solutionSeekInput.addEventListener("change", seekSolutionStep);

installEditorHoverTooltips();
bindSourceEditorEvents();
bindSourceEditorPopoverEvents();
registerSourceEditableTarget?.("level", {
  find: findLevelDefinitionAtPosition,
  load: loadLevelFromSourcePosition,
});

applyPaneVisibility();

loadSource().catch((error) => {
  setPreviewDocumentLoaded(false);
  setPreviewFrameHtml(emptyPreviewDocument());
  resetPreviewLog("Load failed");
  appendPreviewLog("error", error?.message || String(error), { source: "workspace" });
  setEditorStatus("Load error", "is-error");
});
