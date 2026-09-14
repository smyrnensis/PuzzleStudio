const colorSchemeStoreKey = "PuzzleStudioEditorColorScheme";
const levelShared = globalThis.PuzzleEditorLevelShared;
if (!levelShared) {
  throw new Error("PuzzleEditorLevelShared is required before editor.js.");
}
const previewDefaultLogicalWidth = 4;
const previewDefaultLogicalHeight = 3;
const previewMinimumLogHeight = 72;
const solverFeedbackTickMs = 250;
const solverObservationLiveIntervalMs = 500;
const solverObservationPlaybackMaxMs = 1600;
const solverObservationPlaybackMinStepMs = 20;
const solverObservationPlaybackMaxStepMs = 80;
const solutionPlaybackBaseIntervalMs = 350;
let previewViewportAspect = previewDefaultLogicalWidth / previewDefaultLogicalHeight;
const boardVirtualCellSize = 56;
const levelEditorEdgeSize = 24;
const levelEditorGap = 6;
const VISUAL_COLOR_PRESETS = [
  "#000000", "#1d2b53", "#7e2553", "#008751",
  "#ab5236", "#5f574f", "#c2c3c7", "#fff1e8",
  "#ff004d", "#ffa300", "#ffec27", "#00e436",
  "#29adff", "#83769c", "#ff77a8", "#ffccaa",
];
const VISUAL_COLOR_TOKENS = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

function visualEditorScaleFactor(scaleInput, maxSize) {
  const factor = Math.trunc(Number(scaleInput?.value) || 2);
  return Math.max(2, Math.min(maxSize, factor));
}

function renderVisualScaleControl({
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
  const factor = visualEditorScaleFactor(scaleInput, maxSize);
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

const editorHoverTooltipSelector = "button, [data-tooltip]";
let editorHoverTooltip = null;
let editorHoverTooltipTarget = null;

function normalizeEditorShortcut(shortcut) {
  if (!shortcut || typeof shortcut !== "object") {
    throw new Error("Editor shortcut must be a structured definition.");
  }
  const keys = Array.isArray(shortcut.keys) ? shortcut.keys : [shortcut.key];
  if (!keys.length || keys.some((key) => (
    typeof key !== "string" || (key !== " " && !key.trim())
  ))) {
    throw new Error("Editor shortcut requires at least one key.");
  }
  const modifiers = Array.isArray(shortcut.modifiers) ? shortcut.modifiers : [];
  if (modifiers.some((modifier) => !["primary", "shift"].includes(modifier))) {
    throw new Error(`Unsupported editor shortcut modifier: ${modifiers.join(", ")}`);
  }
  return {
    keys: keys.map((key) => key === " " ? key : key.trim()),
    modifiers: [...new Set(modifiers)],
  };
}

function setEditorShortcutHint(element, shortcut) {
  if (!element) {
    throw new Error("Editor shortcut hint requires an element.");
  }
  setEditorShortcutHints(element, [shortcut]);
}

function setEditorShortcutHints(element, shortcuts) {
  if (!element) {
    throw new Error("Editor shortcut hint requires an element.");
  }
  if (!Array.isArray(shortcuts) || !shortcuts.length) {
    throw new Error("Editor shortcut hints require at least one shortcut.");
  }
  element.dataset.shortcuts = JSON.stringify(shortcuts.map(normalizeEditorShortcut));
}

function editorShortcutMatches(event, shortcut) {
  const normalized = normalizeEditorShortcut(shortcut);
  const expectsPrimary = normalized.modifiers.includes("primary");
  const expectsShift = normalized.modifiers.includes("shift");
  const hasPrimary = (event.metaKey && !event.ctrlKey) || (event.ctrlKey && !event.metaKey);
  if (hasPrimary !== expectsPrimary || event.altKey || event.shiftKey !== expectsShift) {
    return false;
  }
  const eventKey = event.key.length === 1 ? event.key.toLowerCase() : event.key;
  return normalized.keys.some((key) => (
    (key.length === 1 ? key.toLowerCase() : key) === eventKey
  ));
}

function editorShortcutKeyLabel(key) {
  const labels = {
    " ": "Space",
    ArrowLeft: "←",
    ArrowRight: "→",
    ArrowUp: "↑",
    ArrowDown: "↓",
    Escape: "Esc",
  };
  if (labels[key]) {
    return labels[key];
  }
  return key.length === 1 ? key.toUpperCase() : key;
}

function createEditorShortcutHint(shortcut) {
  const normalized = normalizeEditorShortcut(shortcut);
  const keycap = document.createElement("kbd");
  keycap.className = "editor-hover-shortcut";
  if (normalized.modifiers.includes("primary")) {
    keycap.append(editorIconElement("command"));
  }
  if (normalized.modifiers.includes("shift")) {
    const shift = document.createElement("span");
    shift.textContent = "⇧";
    keycap.append(shift);
  }
  const key = document.createElement("span");
  key.textContent = editorShortcutKeyLabel(normalized.keys[0]);
  keycap.append(key);
  return keycap;
}

function editorTooltipTargetFromEventTarget(target) {
  const element = target instanceof Element ? target.closest(editorHoverTooltipSelector) : null;
  if (!element) {
    return null;
  }
  const hasIconGlyph = element.querySelector("svg")
    || element.classList.contains("visual-brush-size-input");
  const hasHoverContent = element.dataset.tooltip || hasIconGlyph || element.dataset.shortcuts;
  if (
    element.classList.contains("visual-cell")
    || element.classList.contains("source-outline-row")
    || !hasHoverContent
  ) {
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
    ["Hide current tool pane", "Hide pane"],
    ["Hide explorer pane", "Hide explorer"],
    ["Maximize Preview pane", "Maximize"],
    ["Maximize Source pane", "Maximize"],
    ["More file actions", "File actions"],
    ["Paint empty voxel", "Eraser"],
    ["Paint transparent", "Eraser"],
    ["Pick color from screen", "Pick color"],
    ["Edit selected color", "Edit color"],
    ["Remove selected color", "Remove color"],
    ["Reset 3D preview camera", "Reset camera"],
    ["Reset solution preview", "Reset"],
    ["Screen color picker is not available in this browser", "Unavailable"],
    ["Show explorer pane", "Show explorer"],
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
  const scopedEditMatch = cleaned.match(/^(Copy|Cut|Delete) (?:whole (?:3D )?visual|current slice|selected (?:3D )?area)$/i);
  if (scopedEditMatch) {
    return `${scopedEditMatch[1][0].toUpperCase()}${scopedEditMatch[1].slice(1).toLowerCase()}`;
  }
  if (/^Paste into (?:whole (?:3D )?visual|current slice|selected (?:3D )?area)$/i.test(cleaned)) {
    return "Paste";
  }
  if (/^(?:Select edit region in|Clear selected (?:3D )?edit region)/i.test(cleaned)) {
    return cleaned.startsWith("Clear") ? "Clear region" : "Select region";
  }
  if (/^Stop translating (?:3D )?visual$/i.test(cleaned)) {
    return "Stop translate";
  }
  const scopedTransformMatch = cleaned.match(/^(Translate|Fill connected (?:3D component|area)|Rotate|Flip)\b/i);
  if (scopedTransformMatch) {
    const action = scopedTransformMatch[1].toLowerCase();
    if (action === "rotate") {
      return /\bCCW$/i.test(cleaned) ? "Rotate CCW" : /\bCW$/i.test(cleaned) ? "Rotate CW" : "Rotate";
    }
    if (action === "flip") {
      return /horizontally$/i.test(cleaned) ? "Flip horizontal" : /vertically$/i.test(cleaned) ? "Flip vertical" : "Flip";
    }
    return action.startsWith("fill") ? "Fill" : "Translate";
  }
  return cleaned;
}

function editorTooltipText(element) {
  if (element?.dataset?.shortcutOnly === "true") {
    return "";
  }
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

function renderEditorHoverTooltip(text, shortcuts) {
  const tooltip = ensureEditorHoverTooltip();
  tooltip.replaceChildren();
  if (text) {
    const label = document.createElement("span");
    label.className = "editor-hover-label";
    label.textContent = text;
    tooltip.append(label);
  }
  for (const shortcut of shortcuts || []) {
    tooltip.append(createEditorShortcutHint(shortcut));
  }
  return tooltip;
}

function positionEditorHoverTooltip() {
  if (!editorHoverTooltipTarget || !editorHoverTooltip || editorHoverTooltip.hidden) {
    return;
  }
  const margin = 8;
  const gap = 6;
  const targetRect = editorHoverTooltipTarget.getBoundingClientRect();
  const pane = editorHoverTooltipTarget.closest(".explorer-pane, .code-pane, .preview-pane");
  const paneRect = pane?.getBoundingClientRect();
  const bounds = paneRect
    ? {
      left: Math.max(margin, paneRect.left + margin),
      right: Math.min(window.innerWidth - margin, paneRect.right - margin),
      top: Math.max(margin, paneRect.top + margin),
      bottom: Math.min(window.innerHeight - margin, paneRect.bottom - margin),
    }
    : {
      left: margin,
      right: window.innerWidth - margin,
      top: margin,
      bottom: window.innerHeight - margin,
    };
  editorHoverTooltip.style.maxWidth = `${Math.max(0, bounds.right - bounds.left)}px`;
  const tooltipRect = editorHoverTooltip.getBoundingClientRect();
  const maxLeft = Math.max(bounds.left, bounds.right - tooltipRect.width);
  const left = Math.min(maxLeft, Math.max(bounds.left, targetRect.left + (targetRect.width - tooltipRect.width) / 2));
  const topAbove = targetRect.top - tooltipRect.height - gap;
  const placeBelow = topAbove < bounds.top;
  const top = placeBelow
    ? Math.min(bounds.bottom - tooltipRect.height, targetRect.bottom + gap)
    : topAbove;
  editorHoverTooltip.dataset.placement = placeBelow ? "below" : "above";
  editorHoverTooltip.style.left = `${Math.round(left)}px`;
  editorHoverTooltip.style.top = `${Math.round(Math.max(bounds.top, top))}px`;
}

function showEditorHoverTooltip(element) {
  const text = editorTooltipText(element);
  const shortcutsJson = String(element.dataset.shortcuts || "").trim();
  const shortcuts = shortcutsJson ? JSON.parse(shortcutsJson) : [];
  if (!text && !shortcuts.length) {
    hideEditorHoverTooltip(element);
    return;
  }
  editorHoverTooltipTarget = element;
  if (element.hasAttribute("title")) {
    element.dataset.hoverTitle = element.getAttribute("title") || "";
    element.removeAttribute("title");
  }
  const tooltip = renderEditorHoverTooltip(text, shortcuts);
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
    editorHoverTooltip.replaceChildren();
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

let previewBuild = null;
let previewBuildIsStale = false;
let previewSession = null;
let previewSourceProjection = null;
const solverPreparedByBuildId = new Map();
let previewFrameHasCurrentCompiledPreview = false;
let previewFrameReadyPromise = Promise.resolve(false);
let previewRuntimeReady = false;
let previewRuntimeReadyPromise = Promise.resolve(false);
let previewTimer = 0;
const previewRuntimeAssetWindows = new WeakSet();
let previewRuntimeAssetBridgeStatus = [];
const editorRuntimeCommands = new Map();
const editorRuntimeCommittedFrames = new Map();
const editorRuntimeControllers = new Map();
const editorRuntimeControllerByWindow = new WeakMap();
const editorRuntimeConsumerHandlers = new Map();
let previewViewportGeometrySyncFrame = 0;
let previewViewportGeometrySyncPasses = 0;
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
let surfaceEntriesCache = null;
let surfaceEntriesRequest = null;
let previewLogEntries = [];
let previewDebugEnabled = false;
let previewDebugTrace = null;
let previewDebugCursor = -1;
let previewDebugSnapshot = null;
let previewPaneSourceKey = "";
let previewKeyboardFocusOwned = false;
let activeLevelIndex = 0;
let activeSolverTask = null;
let levelGoalSolverTask = null;
let customGoalSolverTask = null;
let solverSelectedPuzzleName = "";
let solverSelectedLevelIndex = null;
let activeLevelSolveRequest = null;
let editorSolverWorker = null;
const editorSolverWorkerRequests = new Map();
let completedSolverTaskKey = "";
let levelSolutionPreview = null;
let levelGoalSolutionPreview = null;
let customGoalSolutionPreview = null;
let levelGoalSolveSummaryText = "";
let customGoalSolveSummaryText = "";
let solverObservationPreview = null;
let levelGoalSolverObservationPreview = null;
let customGoalSolverObservationPreview = null;
const customGoalConstraintsByTask = new Map();
let solverPaneMode = "level-goal";
const solverPaneStatuses = {
  "level-goal": { text: "Ready to solve", className: "" },
  "custom-goal": { text: "Add a goal condition", className: "" },
};
let levelSolveSummaryText = "";
let agentObservationCursor = 0;
let agentObservationPolling = false;
const agentInvestigation = {
  events: [],
  search: null,
};

let levelSolutionTimer = 0;
let solverObservationTimer = 0;
let levelSolveFlashTimer = 0;
let levelSolveFlashRestore = null;
let levelSolveFeedbackTimer = 0;
let levelSolveStartedAt = 0;
let currentPreviewMode = "play";
let currentEditorDimension = "2d";
let currentLevelPaneMode = "edit";
let currentVisualPaneMode = "visual";
let levelPaintDrag = null;
let levelBucketActive = false;
let levelResizeMode = null;
let levelGridVisible = false;
let levelPlaytestActive = false;
let visualPaintDrag = null;
let visual3dPaintDrag = null;
let level = {
  width: 9,
  height: 5,
  editDocumentId: null,
  editSourceStart: null,
  editSourceEnd: null,
  editSourceBodyStart: null,
  editSourceBodyEnd: null,
  editSourceName: "",
  sourceVisualContract: null,
  selectedObjectId: 0,
  addPaletteOpen: false,
  activeLayer: 0,
  layerMode: false,
  showCompositeLayers: false,
  layers: [],
  palette: [],
  regions: [],
  cells: [],
  exportData: null,
  hasLocalDraft: false,
};
let levelDisplayCells = null;
let levelLayerInsertMode = false;
let levelLayerRemoveMode = false;
let visual = {
  width: 5,
  height: 5,
  sizeBound: true,
  editDocumentId: null,
  editSourceStart: null,
  editSourceEnd: null,
  editSourceBodyStart: null,
  editSourceBodyEnd: null,
  editSourceName: "",
  sourceVisualContract: null,
  selectedColorIndex: 0,
  addPaletteOpen: false,
  editPaletteOpen: false,
  customColorOpen: false,
  addDraftColorIndex: null,
  colorTagPickerOpen: false,
  shapeTagPickerOpen: false,
  paletteBind: null,
  shapeBind: null,
  solidSource: false,
  sourcePreludeRows: [],
  sourceSpatialOps: [],
  animationMode: false,
  animationFrameIndex: 0,
  animationDurationMs: 120,
  animationFrameCount: 1,
  animationFrames: [],
  animationPlaybackIndex: 0,
  animationPlaying: false,
  cells: [],
  palette: [
    { color: "#ff004d" },
  ],
};
let visual3d = {
  width: 5,
  height: 5,
  depth: 5,
  sizeBound: true,
  editDocumentId: null,
  editSourceStart: null,
  editSourceEnd: null,
  editSourceBodyStart: null,
  editSourceBodyEnd: null,
  editSourceName: "",
  axis: "z",
  slice: 0,
  editScope: "slice",
  selectedColorIndex: 0,
  addPaletteOpen: false,
  editPaletteOpen: false,
  customColorOpen: false,
  addDraftColorIndex: null,
  colorTagPickerOpen: false,
  shapeTagPickerOpen: false,
  palette: [
    { color: "#ff004d" },
  ],
  hoverSlice: null,
  camera: {
    yawDegrees: 340,
    pitchDegrees: 28,
    zoom: 1,
  },
  cells: [],
  frames: [],
  animationMode: false,
  animationFrameIndex: 0,
  animationFrameCount: 1,
  animationPlaybackIndex: 0,
  animationPlaying: false,
  animationDurationMs: null,
  frameDurationMs: null,
  shapeBind: null,
  sourcePreludeRows: [],
  sourceSpatialOps: [],
};
let sounds = {
  mode: "sfx",
  audio: null,
  audioPromise: null,
  musicBackends: [],
  musicPlaying: false,
  musicProgress: 0,
  musicRestartTimer: 0,
  progressFrame: 0,
  initialized: false,
};
const visualEditHistoryLimit = 200;
const visualEditHistories = {
  visual: { undo: [], redo: [] },
  visual3d: { undo: [], redo: [] },
};

function cloneVisualEditValue(value) {
  return JSON.parse(JSON.stringify(value));
}

function visualEditDocumentForKind(kind) {
  if (kind === "visual" && typeof activeVisualEditDocument === "function") {
    return activeVisualEditDocument();
  }
  if (kind === "visual3d" && typeof activeVisual3dEditDocument === "function") {
    return activeVisual3dEditDocument();
  }
  return null;
}

function visualEditorSourceHistorySnapshot(state) {
  return {
    editDocumentId: state.editDocumentId,
    editSourceStart: state.editSourceStart,
    editSourceEnd: state.editSourceEnd,
    editSourceBodyStart: state.editSourceBodyStart,
    editSourceBodyEnd: state.editSourceBodyEnd,
    editSourceName: state.editSourceName,
    sourceVisualContract: cloneVisualEditValue(state.sourceVisualContract || null),
    sourcePreludeRows: cloneVisualEditValue(state.sourcePreludeRows || []),
    sourceSpatialOps: cloneVisualEditValue(state.sourceSpatialOps || []),
  };
}

function restoreVisualEditorSourceHistory(state, snapshot) {
  state.editDocumentId = snapshot.editDocumentId || null;
  state.editSourceStart = Number.isInteger(snapshot.editSourceStart) ? snapshot.editSourceStart : null;
  state.editSourceEnd = Number.isInteger(snapshot.editSourceEnd) ? snapshot.editSourceEnd : null;
  state.editSourceBodyStart = Number.isInteger(snapshot.editSourceBodyStart) ? snapshot.editSourceBodyStart : null;
  state.editSourceBodyEnd = Number.isInteger(snapshot.editSourceBodyEnd) ? snapshot.editSourceBodyEnd : null;
  state.editSourceName = snapshot.editSourceName || "";
  state.sourceVisualContract = cloneVisualEditValue(snapshot.sourceVisualContract || null);
  state.sourcePreludeRows = cloneVisualEditValue(snapshot.sourcePreludeRows || []);
  state.sourceSpatialOps = cloneVisualEditValue(snapshot.sourceSpatialOps || []);
}

function visualEditSnapshot(kind) {
  const editDocument = visualEditDocumentForKind(kind);
  const tracksSource = kind === "visual" || kind === "visual3d";
  const base = {
    kind,
    documentId: tracksSource ? editDocument?.id || "" : "",
    source: tracksSource && editDocument && isTextDocument(editDocument) ? editDocument.source || "" : "",
  };
  if (kind === "visual") {
    return {
      ...base,
      state: {
        ...visualEditorSourceHistorySnapshot(visual),
        width: visual.width,
        height: visual.height,
        palette: cloneVisualEditValue(visual.palette || []),
        cells: cloneVisualEditValue(visual.cells || []),
        paletteBind: cloneVisualEditValue(visual.paletteBind || null),
        shapeBind: cloneVisualEditValue(visual.shapeBind || null),
        solidSource: Boolean(visual.solidSource),
        animationMode: Boolean(visual.animationMode),
        animationFrameIndex: visual.animationFrameIndex,
        animationDurationMs: visual.animationDurationMs,
        animationFrameCount: visual.animationFrameCount,
        animationFrames: cloneVisualEditValue(visual.animationFrames || []),
        animationPlaybackIndex: visual.animationPlaybackIndex,
      },
    };
  }
  if (kind === "visual3d") {
    return {
      ...base,
      state: {
        ...visualEditorSourceHistorySnapshot(visual3d),
        width: visual3d.width,
        height: visual3d.height,
        depth: visual3d.depth,
        axis: visual3d.axis,
        slice: visual3d.slice,
        editScope: visual3d.editScope,
        palette: cloneVisualEditValue(visual3d.palette || []),
        cells: cloneVisualEditValue(visual3d.cells || []),
        frames: cloneVisualEditValue(visual3d.frames || []),
        animationMode: Boolean(visual3d.animationMode),
        animationFrameIndex: visual3d.animationFrameIndex,
        animationFrameCount: visual3d.animationFrameCount,
        animationPlaybackIndex: visual3d.animationPlaybackIndex,
        animationDurationMs: visual3d.animationDurationMs,
        frameDurationMs: visual3d.frameDurationMs,
        shapeBind: cloneVisualEditValue(visual3d.shapeBind || null),
        hoverSlice: visual3d.hoverSlice,
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
  if (snapshot.kind === "visual") {
    restoreVisualEditorSourceHistory(visual, state);
    visual.width = clampVisualSize(state.width);
    visual.height = clampVisualSize(state.height);
    visual.palette = cloneVisualEditValue(state.palette || [{ color: "#ff004d" }]);
    visual.cells = cloneVisualEditValue(state.cells || []);
    visual.paletteBind = cloneVisualEditValue(state.paletteBind || null);
    visual.shapeBind = cloneVisualEditValue(state.shapeBind || null);
    visual.solidSource = Boolean(state.solidSource);
    visual.animationMode = Boolean(state.animationMode);
    visual.animationFrameIndex = Math.max(0, Math.trunc(Number(state.animationFrameIndex) || 0));
    visual.animationDurationMs = Number.isFinite(Number(state.animationDurationMs))
      ? normalizedVisualAnimationDuration(state.animationDurationMs)
      : normalizedVisualAnimationDuration();
    visual.animationFrameCount = normalizedVisualAnimationFrameCount(state.animationFrameCount);
    visual.animationFrames = cloneVisualEditValue(state.animationFrames || []);
    visual.animationPlaybackIndex = Math.max(0, Math.trunc(Number(state.animationPlaybackIndex) || 0));
    if (visual.animationMode) {
      if (typeof ensureVisualAnimationFrames === "function") {
        ensureVisualAnimationFrames();
      }
      if (visual.animationFrames[visual.animationFrameIndex]) {
        visual.cells = visual.animationFrames[visual.animationFrameIndex];
      }
    } else if (typeof resetVisualAnimationFramesFromCurrentCells === "function") {
      resetVisualAnimationFramesFromCurrentCells();
    }
    visual.addPaletteOpen = false;
    visual.editPaletteOpen = false;
    visual.customColorOpen = false;
    visual.addDraftColorIndex = null;
    renderVisualBuilder();
    if (typeof syncVisualAnimationInputValues === "function") {
      syncVisualAnimationInputValues();
    }
  } else if (snapshot.kind === "visual3d") {
    restoreVisualEditorSourceHistory(visual3d, state);
    visual3d.width = clampVisual3dSize(state.width);
    visual3d.height = clampVisual3dSize(state.height);
    visual3d.depth = clampVisual3dSize(state.depth);
    visual3d.axis = ["x", "y", "z"].includes(state.axis) ? state.axis : "z";
    visual3d.slice = Math.max(0, Math.min(visual3dAxisSize() - 1, Math.trunc(Number(state.slice) || 0)));
    visual3d.editScope = state.editScope === "all" ? "all" : "slice";
    visual3d.palette = cloneVisualEditValue(state.palette || [{ color: "#ff004d" }]);
    visual3d.cells = cloneVisualEditValue(state.cells || []);
    visual3d.frames = cloneVisualEditValue(state.frames || []);
    visual3d.animationMode = Boolean(state.animationMode);
    visual3d.animationFrameCount = Math.max(1, Math.trunc(Number(state.animationFrameCount) || visual3d.frames.length || 1));
    visual3d.animationFrameIndex = Math.max(0, Math.min(visual3d.animationFrameCount - 1, Math.trunc(Number(state.animationFrameIndex) || 0)));
    visual3d.animationPlaybackIndex = Math.max(0, Math.min(visual3d.animationFrameCount - 1, Math.trunc(Number(state.animationPlaybackIndex) || 0)));
    visual3d.animationDurationMs = Number.isFinite(state.animationDurationMs) ? state.animationDurationMs : null;
    visual3d.frameDurationMs = Number.isFinite(state.frameDurationMs) ? state.frameDurationMs : null;
    visual3d.shapeBind = cloneVisualEditValue(state.shapeBind || null);
    visual3d.hoverSlice = Number.isInteger(state.hoverSlice) ? state.hoverSlice : null;
    visual3d.addPaletteOpen = false;
    visual3d.editPaletteOpen = false;
    visual3d.customColorOpen = false;
    visual3d.addDraftColorIndex = null;
    renderVisual3dBuilder();
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
  if (currentPreviewMode === "visual") {
    return "visual";
  }
  if (currentPreviewMode === "visual3d") {
    return "visual3d";
  }
  return "";
}

async function undoVisualEdit(kind = currentVisualEditKind()) {
  if (kind === "level") {
    return Boolean((await dispatchLevelSessionCommand({ type: "undo" }))?.changed);
  }
  if (kind === "level3d" && typeof dispatchLevel3dSessionCommand === "function") {
    return Boolean((await dispatchLevel3dSessionCommand({ type: "undo" }))?.changed);
  }
  if ((kind === "visual" || kind === "visual3d") && typeof commitVisualColorEditHistory === "function") {
    commitVisualColorEditHistory(kind);
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

async function redoVisualEdit(kind = currentVisualEditKind()) {
  if (kind === "level") {
    return Boolean((await dispatchLevelSessionCommand({ type: "redo" }))?.changed);
  }
  if (kind === "level3d" && typeof dispatchLevel3dSessionCommand === "function") {
    return Boolean((await dispatchLevel3dSessionCommand({ type: "redo" }))?.changed);
  }
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
  if (target?.closest?.(".visual-code-glyph")) {
    return false;
  }
  if (typeof isVisualEditUndoTarget === "function" && isVisualEditUndoTarget(target)) {
    return false;
  }
  return target?.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(tagName);
}

initializeEditorColorScheme();
configureFolderImport();
configureDesktopHost();

function initializeEditorColorScheme() {
  const colorScheme = normalizeColorScheme(document.documentElement.dataset.colorScheme);
  applyEditorColorScheme(colorScheme);
}

function normalizeColorScheme(colorScheme) {
  return colorScheme === "light" ? "light" : "dark";
}

function applyEditorColorScheme(colorScheme) {
  const normalized = normalizeColorScheme(colorScheme);
  document.documentElement.dataset.colorScheme = normalized;
  if (!colorSchemeToggleButton) {
    return;
  }
  const dark = normalized === "dark";
  colorSchemeToggleButton.setAttribute("aria-pressed", dark ? "true" : "false");
  colorSchemeToggleButton.setAttribute("aria-label", dark ? "Switch to light mode" : "Switch to dark mode");
  colorSchemeToggleButton.title = dark ? "Switch to light mode" : "Switch to dark mode";
  if (!previewDocumentLoaded) {
    applyUnloadedPreviewColorScheme();
  }
  if (typeof renderVisual3dPreview === "function") {
    window.requestAnimationFrame(renderVisual3dPreview);
  }
}

function setEditorColorScheme(colorScheme) {
  const normalized = normalizeColorScheme(colorScheme);
  try {
    window.localStorage.setItem(colorSchemeStoreKey, normalized);
  } catch {
    // Color scheme persistence is optional; private browsing can reject localStorage.
  }
  applyEditorColorScheme(normalized);
}

function toggleEditorColorScheme() {
  setEditorColorScheme(
    normalizeColorScheme(document.documentElement.dataset.colorScheme) === "dark" ? "light" : "dark",
  );
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

function applyPreviewTheme(theme) {
  const root = playPreview;
  if (!root) {
    return;
  }
  const normalized = normalizeRuntimePreviewTheme(theme);
  currentPreviewTheme = normalized;
  for (const target of previewThemeRoots(root)) {
    setRuntimePreviewThemeProperties(target, normalized);
  }
}

function setPreviewDocumentLoaded(loaded) {
  previewDocumentLoaded = Boolean(loaded);
  playPreview?.classList.toggle("is-preview-unloaded", !previewDocumentLoaded);
  if (!previewDocumentLoaded) {
    applyUnloadedPreviewColorScheme();
  }
}

function syncPreviewActionButtons(options = {}) {
  const busy = options.busy === true;
  const sourceDocument = activeDocument();
  const previewDocument = activePreviewDocument();
  if (runButton) {
    runButton.disabled = busy
      || !isPuzzleDocument(sourceDocument)
      || documentNeedsContentLoad(sourceDocument);
  }
  if (previewRefreshButton) {
    previewRefreshButton.disabled = busy || !isPuzzleDocument(previewDocument);
  }
}

function stopPreviewRuntime() {
  if (activePreviewRequest) {
    activePreviewRequest.abort();
    activePreviewRequest = null;
  }
  previewFrameHasCurrentCompiledPreview = false;
  previewFrameHasEditorLevelState = false;
  previewKeyboardFocusOwned = false;
  previewSession = null;
  setPreviewDocumentLoaded(false);
  stopEditorRuntimeController(previewEditorRuntimeController());
  syncPreviewLevelActionButtons();
}

function applyUnloadedPreviewColorScheme() {
  const root = playPreview;
  if (!root) {
    return;
  }
  currentPreviewTheme = editorPreviewColorScheme();
  for (const target of previewThemeRoots(root)) {
    setPreviewThemeProperties(target, currentPreviewTheme);
    target.style.colorScheme = currentPreviewTheme.colorScheme;
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
  root.style.setProperty("--preview-game-accent", theme.accent);
  root.style.setProperty("--preview-game-panel-bg", theme.panelBg);
  root.style.setProperty("--preview-game-background", theme.background);
}

function setRuntimePreviewThemeProperties(root, theme) {
  setPreviewThemeProperties(root, {
    bg: runtimeLinearRgbaCss(theme.uiSkin.canvas),
    ink: runtimeLinearRgbaCss(theme.uiSkin.text),
    muted: runtimeLinearRgbaCss(theme.uiSkin.mutedText),
    line: runtimeLinearRgbaCss(theme.interactionInk.selected.border),
    accent: runtimeLinearRgbaCss(theme.uiSkin.accent),
    panelBg: runtimeLinearRgbaCss(theme.uiSkin.panel.fill),
    background: "var(--preview-game-bg)",
  });
  root.style.setProperty("--preview-game-control", runtimeLinearRgbaCss(theme.uiSkin.control.fill));
  root.style.setProperty(
    "--preview-game-control-focused",
    runtimeLinearRgbaCss(theme.interactionInk.focus.fill),
  );
  root.style.setProperty(
    "--preview-game-control-selected",
    runtimeLinearRgbaCss(theme.interactionInk.selected.fill),
  );
  root.style.setProperty(
    "--preview-game-control-selected-border",
    runtimeLinearRgbaCss(theme.interactionInk.selected.border),
  );
  root.style.colorScheme = "";
  for (const name of ["heading", "subheading", "body", "caption"]) {
    const style = theme.typography[name];
    root.style.setProperty(`--preview-game-text-${name}-size`, `${style.fontSizePx}px`);
    root.style.setProperty(`--preview-game-text-${name}-line-height`, String(style.lineHeight));
  }
  const layout = theme.uiSkin.control.layout;
  root.style.setProperty("--preview-game-control-padding-horizontal", `${layout.paddingHorizontalPx}px`);
  root.style.setProperty("--preview-game-control-padding-vertical", `${layout.paddingVerticalPx}px`);
  root.style.setProperty("--preview-game-control-margin", `${layout.marginPx}px`);
  root.style.setProperty("--preview-game-control-border-width", `${layout.borderWidthPx}px`);
  root.style.setProperty("--preview-game-control-corner-radius", `${layout.cornerRadiusPx}px`);
}

function editorPreviewColorScheme() {
  const light = normalizeColorScheme(document.documentElement.dataset.colorScheme) === "light";
  return {
    colorScheme: light ? "light" : "dark",
    bg: editorCssVariable("--workspace-bg"),
    ink: editorCssVariable("--ink"),
    muted: editorCssVariable("--muted"),
    line: editorCssVariable("--line"),
    accent: editorCssVariable("--accent"),
    danger: editorCssVariable("--danger"),
    panelBg: editorCssVariable("--side-bg"),
    background: editorCssVariable("--workspace-bg"),
  };
}

function editorCssVariable(name) {
  const value = window.getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  if (!value) {
    throw new Error(`Required editor theme variable ${name} is missing.`);
  }
  return value;
}

function normalizeRuntimePreviewTheme(theme) {
  if (!theme || typeof theme !== "object" || Array.isArray(theme)) {
    throw new Error("Editor preview is missing the required typed runtime theme contract.");
  }
  const normalized = structuredClone(theme);
  for (const name of ["canvas", "text", "mutedText", "accent"]) {
    normalized.uiSkin[name] = normalizeRuntimePreviewLinearRgba(
      theme.uiSkin?.[name],
      `theme.uiSkin.${name}`,
    );
  }
  normalized.uiSkin.panel.fill = normalizeRuntimePreviewLinearRgba(
    theme.uiSkin?.panel?.fill,
    "theme.uiSkin.panel.fill",
  );
  normalized.uiSkin.control.fill = normalizeRuntimePreviewLinearRgba(
    theme.uiSkin?.control?.fill,
    "theme.uiSkin.control.fill",
  );
  for (const cue of ["focus", "selected"]) {
    normalized.interactionInk[cue].fill = normalizeRuntimePreviewLinearRgba(
      theme.interactionInk?.[cue]?.fill,
      `theme.interactionInk.${cue}.fill`,
    );
    normalized.interactionInk[cue].border = normalizeRuntimePreviewLinearRgba(
      theme.interactionInk?.[cue]?.border,
      `theme.interactionInk.${cue}.border`,
    );
  }
  normalized.typography = {};
  for (const name of ["heading", "subheading", "body", "caption"]) {
    const style = theme.typography?.[name];
    const fontSizePx = style?.fontSizePx;
    const lineHeight = style?.lineHeight;
    if (typeof fontSizePx !== "number" || !Number.isFinite(fontSizePx) || fontSizePx <= 0
      || typeof lineHeight !== "number" || !Number.isFinite(lineHeight) || lineHeight <= 0) {
      throw new Error(`Editor preview theme has an invalid typography.${name} contract.`);
    }
    normalized.typography[name] = { fontSizePx, lineHeight };
  }
  normalized.uiSkin.control.layout = {};
  for (const name of [
    "paddingHorizontalPx",
    "paddingVerticalPx",
    "marginPx",
    "borderWidthPx",
    "cornerRadiusPx",
  ]) {
    const value = theme.uiSkin?.control?.layout?.[name];
    if (typeof value !== "number" || !Number.isFinite(value) || value < 0) {
      throw new Error(`Editor preview theme has an invalid uiSkin.control.layout.${name} contract.`);
    }
    normalized.uiSkin.control.layout[name] = value;
  }
  return normalized;
}

function normalizeRuntimePreviewLinearRgba(color, label) {
  const normalized = {};
  for (const name of ["red", "green", "blue", "alpha"]) {
    const value = color?.[name];
    if (typeof value !== "number" || !Number.isFinite(value) || value < 0 || value > 1) {
      throw new Error(`Editor preview theme has an invalid ${label}.${name} channel.`);
    }
    normalized[name] = value;
  }
  return normalized;
}

function runtimeLinearRgbaCss(color) {
  return `color(srgb-linear ${color.red} ${color.green} ${color.blue} / ${color.alpha})`;
}

function schedulePreview() {
  window.clearTimeout(previewTimer);
  markPreviewDirty();
}

function workspaceSourceSnapshot(entryDocument) {
  const root = normalizePath(entryDocument?.workspaceRoot || workspaceRoot || "");
  return documents.filter((document) => {
    const documentRoot = normalizePath(document.workspaceRoot || workspaceRoot || "");
    return isPuzzleDocument(document) && isTextDocument(document) && (!root || documentRoot === root);
  }).map((document) => ({
    documentId: document.id || "",
    path: workspaceCompilerPath(document),
    source: currentSourceForDocument(document),
  }));
}

function workspaceCompilerPath(document) {
  const path = normalizePath(document?.puzzlePath || document?.name || "");
  const root = normalizePath(document?.workspaceRoot || workspaceRoot || "").replace(/\/+$/, "");
  if (!root) {
    return path;
  }
  if (path.startsWith(`${root}/`)) {
    return path.slice(root.length + 1);
  }
  if (path.startsWith("/") || /^[A-Za-z]:\//.test(path)) {
    throw new Error(`Workspace document is outside its root: ${path}`);
  }
  return path;
}

function compilerDocumentsForSnapshot(snapshot) {
  return (snapshot || []).map((document) => ({
    path: document.path,
    source: document.source,
  }));
}

function capturePreviewBuildInput(document, presentationManifest) {
  const documentsSnapshot = workspaceSourceSnapshot(document);
  const entryPath = document.puzzlePath || document.name;
  const entry = documentsSnapshot.find((candidate) => candidate.documentId === document.id)
    || documentsSnapshot.find((candidate) => normalizePath(candidate.path) === normalizePath(entryPath));
  if (!entry) {
    throw new Error(`Preview source snapshot is missing its entry document: ${entryPath || "game"}`);
  }
  return {
    id: createDocumentId(),
    documentId: document.id || "",
    puzzlePath: entry.path,
    workspaceRoot: document.workspaceRoot || workspaceRoot || "",
    source: entry.source,
    documents: documentsSnapshot,
    presentationManifest,
    audioFileDocuments: workspaceAudioFileDocuments(document, presentationManifest),
  };
}

function previewSessionState() {
  return previewSession && previewBuild && previewSession.buildId === previewBuild.id
    ? previewSession.state
    : null;
}

function ensurePreviewSession() {
  if (!previewBuild) {
    previewSession = null;
    return null;
  }
  if (previewSession?.buildId !== previewBuild.id) {
    previewSession = {
      buildId: previewBuild.id,
      state: null,
      runtimeStatus: null,
    };
  }
  return previewSession;
}

function setPreviewSessionState(state) {
  const session = ensurePreviewSession();
  if (!session) {
    return null;
  }
  session.state = state;
  syncPreviewLevelActionButtons();
  return session.state;
}

async function loadPreviewSourceProjection(modelName) {
  const build = previewBuild;
  const selectedModel = String(modelName || "").trim();
  if (!build || !selectedModel) {
    return null;
  }
  if (
    previewSourceProjection?.buildId === build.id
    && previewSourceProjection.modelName === selectedModel
    && previewSourceProjection.exportData
  ) {
    return previewSourceProjection.exportData;
  }
  if (
    previewSourceProjection?.buildId === build.id
    && previewSourceProjection.modelName === selectedModel
    && previewSourceProjection.promise
  ) {
    return previewSourceProjection.promise;
  }
  const request = levelEditorSourceExportData(build.source, selectedModel);
  previewSourceProjection = {
    buildId: build.id,
    modelName: selectedModel,
    exportData: null,
    promise: request,
  };
  try {
    const exportData = await request;
    if (
      previewBuild?.id !== build.id
      || previewSourceProjection?.buildId !== build.id
      || previewSourceProjection.modelName !== selectedModel
    ) {
      return null;
    }
    previewSourceProjection.exportData = exportData;
    previewSourceProjection.promise = null;
    syncPreviewLevelActionButtons();
    syncSolverLevelSelector(exportData);
    return exportData;
  } catch (error) {
    if (previewSourceProjection?.buildId === build.id) {
      previewSourceProjection = null;
    }
    throw error;
  }
}

function previewHasCurrentLevel() {
  const state = previewSessionState();
  return Boolean(
    state
    && state.screenHasPuzzle !== false
    && Number.isInteger(Number(state.levelIndex)),
  );
}

function syncPreviewLevelActionButtons() {
  const hasLevel = previewHasCurrentLevel();
  if (previewEditButton) {
    previewEditButton.disabled = !hasLevel;
  }
  if (previewSolveButton && !activeLevelSolveRequest) {
    previewSolveButton.disabled = !hasLevel || isSolverTaskComplete();
  }
}

async function renderPreview() {
  persistCurrentDocument();
  const document = activePreviewDocument();
  if (!isPuzzleDocument(document)) {
    setStatus("No game entry for preview", "is-error");
    syncPreviewActionButtons();
    return;
  }
  let buildInput = null;
  let requestSource = "";
  updateSourceMeta();
  resetPreviewLog(`Compiling ${document.puzzlePath || "preview"}`);
  setStatus("Compiling", "");
  syncPreviewActionButtons({ busy: true });

  if (activePreviewRequest) {
    activePreviewRequest.abort();
  }

  const controller = new AbortController();
  activePreviewRequest = controller;

  try {
    const presentationManifest = await ensurePreviewDocumentsLoaded(document);
    buildInput = capturePreviewBuildInput(document, presentationManifest);
    requestSource = buildInput.source;
    const compiledPreview = await window.PuzzleStudioHost.preview({
      source: requestSource,
      workspaceDocuments: compilerDocumentsForSnapshot(buildInput.documents),
      puzzlePath: buildInput.puzzlePath,
      workspaceRoot: buildInput.workspaceRoot,
      audioFileDocuments: buildInput.audioFileDocuments,
    }, { signal: controller.signal });
    applyCompiledPreviewBuild(compiledPreview, document, buildInput);
  } catch (error) {
    if (error.name === "AbortError") {
      return;
    }
    const reported = appendCompileDiagnostics(error, {
      source: "compiler",
      document,
      sourceText: requestSource,
    });
    if (!reported) {
      setStatus(userFacingRuntimeError(error), "is-error");
    }
  } finally {
    if (activePreviewRequest === controller) {
      activePreviewRequest = null;
    }
    syncPreviewActionButtons();
  }
}

async function ensureCompiledPreviewForLevelPlaytest(options = {}) {
  const exportData = currentLevelExportData();
  if (!levelEditorAssistanceReady(exportData)) {
    const message = options.noDocumentMessage || "No level to play";
    if (typeof options.status === "function") {
      options.status(message, "is-error");
    } else {
      setStatus(message, "is-error");
    }
    return null;
  }
  if (
    previewBuild
    && !previewBuildIsStale
    && previewFrameHasCurrentCompiledPreview
    && previewRuntimeReady
  ) {
    return exportData;
  }
  if (previewBuild && !previewBuildIsStale) {
    const buildId = previewBuild?.id;
    await previewFrameReadyPromise;
    await previewRuntimeReadyPromise;
    if (
      previewBuild?.id === buildId
      && previewFrameHasCurrentCompiledPreview
      && previewRuntimeReady
    ) {
      return exportData;
    }
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
  if (!previewBuild) {
    const message = options.failureMessage || "Preview compile failed";
    if (typeof options.status === "function") {
      options.status(message, "is-error");
    } else {
      setStatus(message, "is-error");
    }
    return null;
  }
  const buildId = previewBuild?.id;
  await previewFrameReadyPromise;
  await previewRuntimeReadyPromise;
  if (
    previewBuild?.id !== buildId
    || !previewFrameHasCurrentCompiledPreview
    || !previewRuntimeReady
  ) {
    const message = "Compiled preview runtime did not become ready";
    if (typeof options.status === "function") {
      options.status(message, "is-error");
    } else {
      setStatus(message, "is-error");
    }
    return null;
  }
  return exportData;
}

async function saveAndCompilePreview() {
  syncPreviewActionButtons({ busy: true });
  setStatus("Saving before preview", "");
  let saved = false;
  try {
    saved = await saveCurrentDocument(true);
  } catch (error) {
    console.error(error);
    setStatus("Save failed", "is-error");
    saveButton.disabled = false;
    return;
  } finally {
    if (!saved && !activePreviewRequest) {
      syncPreviewActionButtons();
    }
  }
  if (!saved) {
    setStatus("Save failed", "is-error");
    return;
  }
  await renderPreview();
}

async function runPreviewFromSourcePane() {
  const document = activeDocument();
  syncPreviewActionButtons({ busy: true });
  selectPreviewEntryDocument(document);
  try {
    if (isPuzzleDocument(document)) {
      await window.PuzzleStudioHost.selectWorkspaceEntry({
        workspaceRoot: document.workspaceRoot || workspaceRoot || "",
        puzzlePath: document.puzzlePath,
      });
    }
    openPreviewModePane("play", { focus: false });
    await saveAndCompilePreview();
  } catch (error) {
    syncPreviewActionButtons();
    const message = userFacingRuntimeError(error);
    setStatus(`Preview start failed: ${message}`, "is-error");
  }
}

async function refreshPreviewFromPreviewPane() {
  await saveAndCompilePreview();
}

function applyCompiledPreviewBuild(compiledPreview, document, buildInput) {
  const previousLevelIndex = previewBuild?.documentId === buildInput.documentId
    ? currentPreviewRuntimeLevelIndex()
    : null;
  if (
    !compiledPreview
    || typeof compiledPreview !== "object"
    || typeof compiledPreview.runtime?.runtimeExportJson !== "string"
    || !compiledPreview.runtime.runtimeExportJson
    || typeof compiledPreview.runtime?.progressIdentityKey !== "string"
    || !compiledPreview.runtime.progressIdentityKey
  ) {
    throw new Error("Preview compiler returned an invalid typed build.");
  }
  for (const diagnostic of Array.isArray(compiledPreview.diagnostics)
    ? compiledPreview.diagnostics
    : []) {
    appendPreviewLog(
      diagnostic?.severity === "warning" ? "warn" : "info",
      diagnosticLogMessage(diagnostic),
      { source: "compiler" },
    );
  }
  previewBuild = {
    ...buildInput,
    runtime: compiledPreview.runtime,
  };
  previewSourceProjection = null;
  solverPreparedByBuildId.clear();
  previewBuildIsStale = false;
  previewSession = {
    buildId: previewBuild.id,
    state: null,
    runtimeStatus: null,
  };
  setActiveLevelIndex(previousLevelIndex ?? 0, null);
  clearSolverTask();
  previewFrameHasEditorLevelState = false;
  setPreviewRuntime(compiledPreview.runtime, { markDocumentLoaded: true });
  if (isPaneVisible("level")) {
    if (!loadAvailableLevelPaneEntry(focusedPuzzleSourceContext(document), {
      mode: currentLevelPaneMode,
      silent: true,
      recordHistory: false,
      openPane: false,
    })) {
      resetLevelBuilderFromPreviewSource();
    }
  } else {
    resetLevelBuilderFromPreviewSource();
  }
  refreshVisiblePreviewSolverTask();
  syncSolverLevelSelector(currentPreviewExportData());
  syncSolverTaskReadout();
  if (!level3dBuilder.hidden) {
    renderLevel3dBuilder();
  }
  scheduleLocalSave();
  downloadButton.disabled = false;
  appendPreviewLog("system", "Preview compiled", { source: "compiler" });
  setStatus("Starting preview", "");
  syncPreviewLevelActionButtons();
}

function invalidateCompiledPreview(document = activePreviewDocument()) {
  previewBuild = null;
  previewSourceProjection = null;
  previewBuildIsStale = false;
  previewSession = null;
  solverPreparedByBuildId.clear();
  previewFrameHasCurrentCompiledPreview = false;
  previewFrameHasEditorLevelState = false;
  if (document) {
    document.previewHtml = "";
    document.previewError = "";
  }
  setPreviewDocumentLoaded(false);
  stopEditorRuntimeController(previewEditorRuntimeController());
  downloadButton.disabled = true;
  syncPreviewLevelActionButtons();
}

function workspaceCompilerDocuments(entryDocument) {
  return compilerDocumentsForSnapshot(workspaceSourceSnapshot(entryDocument));
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
  if (!window.PuzzleStudioRuntime?.loadWasmCompiler) {
    throw new Error("PuzzleStudio browser runtime is unavailable.");
  }
  const module = await window.PuzzleStudioRuntime.loadWasmCompiler();
  wasmCompiler = module;
  return module;
}

function wasmSolverWorkerConfig() {
  if (!window.PuzzleStudioRuntime?.wasmCompilerConfig) {
    throw new Error("PuzzleStudio browser runtime is unavailable.");
  }
  return window.PuzzleStudioRuntime.wasmCompilerConfig();
}

function createWasmSolveWorker() {
  if (editorSolverWorker) {
    return editorSolverWorker;
  }
  const worker = new Worker(new URL("editor_solver_worker.js", document.baseURI), { type: "module" });
  worker.onmessage = (event) => {
    const message = event.data || {};
    const handler = editorSolverWorkerRequests.get(String(message.requestId || ""));
    if (!handler) return;
    if (handler.onMessage(message) === true) {
      editorSolverWorkerRequests.delete(String(message.requestId || ""));
    }
  };
  worker.onerror = (error) => {
    error?.preventDefault?.();
    for (const handler of editorSolverWorkerRequests.values()) {
      handler.onError(error);
    }
    editorSolverWorkerRequests.clear();
  };
  editorSolverWorker = worker;
  return worker;
}

function disposeWasmSolveWorker(worker) {
  if (!worker) {
    return;
  }
  worker.terminate();
  if (editorSolverWorker === worker) {
    editorSolverWorker = null;
  }
  editorSolverWorkerRequests.clear();
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

function registerEditorSolverWorkerRequest(requestId, onMessage, onError) {
  const key = String(requestId || "");
  if (!key || editorSolverWorkerRequests.has(key)) {
    throw new Error(`Solver worker request is invalid or duplicated: ${key}`);
  }
  editorSolverWorkerRequests.set(key, { onMessage, onError });
}

function prepareEditorSolverArtifact({ documents, puzzlePath, modelName, documentId }) {
  const worker = createWasmSolveWorker();
  const requestId = createDocumentId();
  return new Promise((resolve, reject) => {
    registerEditorSolverWorkerRequest(requestId, (message) => {
      if (message.type === "prepared") {
        const preparedModelName = String(message.modelName || "").trim();
        if (preparedModelName !== String(modelName || "").trim()) {
          reject(new Error(`Solver prepared unexpected model: ${preparedModelName || "missing"}`));
          return true;
        }
        resolve({
          artifactId: message.artifactId,
          modelName: preparedModelName,
          modelKind: message.modelKind,
          objects: Array.isArray(message.objects) ? message.objects : [],
          levels: Array.isArray(message.levels) ? message.levels : [],
          documentId,
        });
        return true;
      }
      if (message.type === "error") {
        reject(new Error(message.error || "Solver prepare failed"));
        return true;
      }
      return false;
    }, (error) => {
      reject(new Error(userFacingWorkerError(error)));
    });
    try {
      worker.postMessage({
        type: "prepare",
        requestId,
        wasm: wasmSolverWorkerConfig(),
        documents,
        puzzlePath,
        modelName,
        displayed: true,
      });
    } catch (error) {
      editorSolverWorkerRequests.delete(requestId);
      reject(error);
    }
  });
}

function setEditorSolverDisplayedArtifact(artifactId = "") {
  if (!editorSolverWorker) return;
  editorSolverWorker.postMessage({ type: "display", artifactId });
}

function appendCompileDiagnostics(error, options = {}) {
  const diagnostics = Array.isArray(error?.diagnostics) ? error.diagnostics : [];
  if (!diagnostics.length) {
    return false;
  }
  const entries = diagnostics.map((diagnostic) => ({
    diagnostic,
    location: diagnosticSourceLocation(diagnostic, options),
  }));
  if (entries.some((entry) => !entry.location)) {
    return false;
  }
  for (const { diagnostic, location } of entries) {
    appendPreviewLog("error", diagnosticLogMessage(diagnostic), {
      ...options,
      origin: diagnosticOrigin(diagnostic, location),
      location,
    });
  }
  return true;
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
  const diagnosticFile = String(diagnostic?.file || "").trim();
  const document = diagnosticFile
    ? documentByPath(diagnosticFile)
    : options.document || activePreviewDocument();
  if (diagnosticFile && !document) {
    return null;
  }
  const sourceText = document
    ? currentSourceForDocument(document)
    : String(options.sourceText ?? options.source ?? "");
  const line = positiveInteger(diagnostic?.line);
  const column = positiveInteger(diagnostic?.column) || 1;
  if (!line) {
    return null;
  }
  const offset = sourceOffsetForLineColumn(sourceText, line, column);
  return {
    documentId: document?.id || "",
    line,
    column,
    offset,
    sourceLine: String(diagnostic?.sourceLine || ""),
  };
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
    current.source = sourceEditorDocumentValue();
  }
  if (previewBuild) {
    previewBuildIsStale = true;
  }
  scheduleLocalSave();
  downloadButton.disabled = true;
  setPaneStatus("preview", previewBuild ? "Preview is out of date" : "Preview requires compile", "");
}

function updateSourceMeta() {
  if (typeof updateSourceAssetPreviewMeta === "function" && updateSourceAssetPreviewMeta()) {
    return;
  }
  const source = sourceEditorDocumentValue();
  const lineCount = source.length ? source.split("\n").length : 0;
  sourceMeta.textContent = `${lineCount} lines`;
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
  schedulePreviewViewportGeometrySync(2);
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
  if (normalized === "preview" && className === "is-error") {
    element.className = paneStatusClassName();
    element.textContent = "";
    appendPreviewLog("error", text, { source: "editor" });
    schedulePreviewViewportGeometrySync(2);
    return;
  }
  element.className = paneStatusClassName(className);
  element.textContent = text || "";
  schedulePreviewViewportGeometrySync(2);
  if (text && className === "is-ok") {
    const clearDelayMs = Number(options.clearDelayMs) || 1800;
    const nextTimer = window.setTimeout(() => {
      if (element.textContent === text && element.classList.contains("is-ok")) {
        element.textContent = "";
        element.className = paneStatusClassName();
        schedulePreviewViewportGeometrySync(2);
      }
      paneStatusClearTimers.delete(normalized);
    }, clearDelayMs);
    paneStatusClearTimers.set(normalized, nextTimer);
  }
}

function setStatus(text, className) {
  setPaneStatus(activeStatusPaneId(), text, className);
}

function setPaneStatusLink(paneId, prefixText, linkText, options = {}) {
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
    return null;
  }
  element.className = paneStatusClassName(options.className || "");
  renderStatusLink(element, prefixText, linkText, options);
  schedulePreviewViewportGeometrySync(2);
  return element;
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

function setEditorStatusLink(prefixText, linkText, options = {}) {
  window.clearTimeout(editorStatusClearTimer);
  editorStatusLabel.className = `document-status ${options.className || ""}`.trim();
  renderStatusLink(editorStatusLabel, prefixText, linkText, options);
}

function renderStatusLink(element, prefixText, linkText, options = {}) {
  element.textContent = "";
  element.append(document.createTextNode(prefixText || ""));
  const link = document.createElement("a");
  link.href = options.href || "#";
  link.textContent = linkText || "";
  if (options.title) {
    link.title = options.title;
  }
  if (options.download) {
    link.download = options.download;
  }
  if (typeof options.onClick === "function") {
    link.addEventListener("click", options.onClick);
  }
  element.append(link);
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
    plain: options.plain === true,
    debugExecutionIndex: Number.isInteger(options.debugExecutionIndex) ? options.debugExecutionIndex : null,
    debugExecutionEnd: Number.isInteger(options.debugExecutionEnd) ? options.debugExecutionEnd : null,
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
  const cargoSource = origin.match(
    /\/([^/]+-\d+\.\d+\.\d+\/src\/[^/:]+:\d+(?::\d+)?)$/,
  );
  if (cargoSource) {
    return cargoSource[1];
  }
  return origin.replace(/^.*\/([^/:]+:\d+(?::\d+)?)$/, "$1");
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
    if (entry.location || Number.isInteger(entry.debugExecutionIndex)) {
      line.classList.add("is-navigable");
      line.tabIndex = 0;
      line.setAttribute("role", "button");
      line.dataset.previewLogIndex = String(index);
      line.title = Number.isInteger(entry.debugExecutionIndex)
        ? entry.location
          ? "Show this rule diff and source"
          : "Show this rule diff"
        : previewLogLocationTitle(entry.location);
    }
    const debugEnd = Number.isInteger(entry.debugExecutionEnd)
      ? entry.debugExecutionEnd
      : entry.debugExecutionIndex;
    if (
      previewDebugEnabled
      && Number.isInteger(entry.debugExecutionIndex)
      && previewDebugCursor >= entry.debugExecutionIndex
      && previewDebugCursor <= debugEnd
    ) {
      line.classList.add("is-selected-debug-rule");
    }
    const source = entry.source || "editor";
    const origin = entry.origin ? ` (${entry.origin})` : "";
    const label = entry.level === "system" ? "system" : entry.level;
    line.textContent = entry.plain
      ? entry.message
      : `${source} ${label}${origin}: ${entry.message}`;
    previewLogOutput.append(line);
  }
  previewLogOutput.scrollTop = previewLogOutput.scrollHeight;
}

function setPreviewDebugEnabled(enabled) {
  const nextEnabled = enabled === true;
  if (nextEnabled === previewDebugEnabled) return;
  const previousController = previewEditorRuntimeController();
  previewDebugEnabled = nextEnabled;
  if (!previewDebugEnabled) {
    previewDebugTrace = null;
    previewDebugCursor = -1;
    previewDebugSnapshot = null;
  }
  syncPreviewDebugControls();
  appendPreviewLog("system", previewDebugEnabled ? "debug mode enabled" : "debug mode disabled", {
    source: "editor",
  });
  if (previewBuild?.runtime && currentPreviewMode === "play") {
    stopEditorRuntimeController(previousController);
    setPreviewRuntime(previewBuild.runtime, { markDocumentLoaded: true });
  } else if (previewKeyboardFocusOwned) {
    requestAnimationFrame(() => focusPreviewInputTarget(previewEditorRuntimeController()));
  }
}

function syncPreviewDebugControls() {
  previewDebugToggleButton?.setAttribute("aria-pressed", previewDebugEnabled ? "true" : "false");
  if (previewDebugControls) {
    previewDebugControls.hidden = !previewDebugEnabled;
  }
  previewLogPanel?.classList.toggle("is-debug-mode", previewDebugEnabled);
  if (previewLogTitle) {
    previewLogTitle.textContent = previewDebugEnabled ? "Debug log" : "Log";
  }
  const executions = previewDebugTrace?.executions || [];
  const hasTrace = previewDebugEnabled && executions.length > 0;
  if (previewDebugPrevButton) {
    previewDebugPrevButton.disabled = !hasTrace || previewDebugCursor <= 0;
  }
  if (previewDebugNextButton) {
    previewDebugNextButton.disabled = !hasTrace || previewDebugCursor >= executions.length - 1;
  }
  if (previewDebugLatestButton) {
    previewDebugLatestButton.disabled = !hasTrace || previewDebugCursor >= executions.length - 1;
  }
  if (!previewDebugStatus) {
    return;
  }
  if (!previewDebugEnabled) {
    previewDebugStatus.textContent = "No rule execution yet";
    renderPreviewLog();
    return;
  }
  if (!executions.length) {
    previewDebugStatus.textContent = "No rule execution yet";
    renderPreviewLog();
    return;
  }
  previewDebugStatus.textContent = `Rule ${previewDebugCursor + 1}/${executions.length}`;
}

function previewDebugRuleSummary(execution) {
  const rule = execution?.rule || {};
  const line = String(rule.sourceLine || "").trim();
  const lineNumber = Number(rule.sourceLineNumber);
  const location = Number.isInteger(lineNumber) && lineNumber > 0 ? `line ${lineNumber}` : `#${execution?.ruleId ?? rule.id ?? "?"}`;
  const stack = Array.isArray(rule.routineStack) && rule.routineStack.length
    ? `${rule.routineStack.join(" > ")}: `
    : "";
  return line ? `${location} ${stack}${line}` : location;
}

function previewDebugRuleLocation(execution) {
  const rule = execution?.rule || {};
  const line = positiveInteger(rule.sourceLineNumber);
  if (!line) {
    return null;
  }
  const document = activePreviewDocument() || activeDocument();
  return {
    documentId: document?.id || "",
    line,
    column: 1,
    sourceLine: String(rule.sourceLine || ""),
  };
}

function previewDebugPositionLabel(position) {
  const x = Number(position?.x);
  const y = Number(position?.y);
  if (!Number.isFinite(x) || !Number.isFinite(y)) {
    return "(?,?)";
  }
  return `(${Math.trunc(x)},${Math.trunc(y)})`;
}

function previewDebugObjectLabel(op, key = "object") {
  const label = String(op?.[key] || "").trim();
  if (label) {
    return label;
  }
  const id = key === "object"
    ? op?.objectId
    : key === "removeObject"
      ? op?.remove
      : key === "addObject"
        ? op?.add
        : op?.[key];
  return id === undefined || id === null ? "object" : `object#${id}`;
}

function previewDebugMarkLabel(op) {
  const label = String(op?.markName || "").trim();
  if (label) {
    return label;
  }
  return op?.mark === undefined || op?.mark === null ? "mark" : `mark#${op.mark}`;
}

function previewDebugVariableLabel(op) {
  const label = String(op?.variable || "").trim();
  if (label) {
    return label;
  }
  return op?.variableId === undefined || op?.variableId === null ? "variable" : `var#${op.variableId}`;
}

function previewDebugPatchDetail(op) {
  const kind = String(op?.kind || "op");
  if (kind === "add") {
    return `add ${previewDebugObjectLabel(op)} at ${previewDebugPositionLabel(op.position)}`;
  }
  if (kind === "remove") {
    return `remove ${previewDebugObjectLabel(op)} at ${previewDebugPositionLabel(op.position)}`;
  }
  if (kind === "move") {
    return `move ${previewDebugObjectLabel(op)} ${previewDebugPositionLabel(op.from)} -> ${previewDebugPositionLabel(op.to)}`;
  }
  if (kind === "replace") {
    const remove = previewDebugObjectLabel(op, "removeObject");
    const add = previewDebugObjectLabel(op, "addObject");
    return `replace ${remove} with ${add} at ${previewDebugPositionLabel(op.position)}`;
  }
  if (kind === "set_mark") {
    return `mark ${previewDebugObjectLabel(op)} with ${previewDebugMarkLabel(op)} at ${previewDebugPositionLabel(op.position)}`;
  }
  if (kind === "remove_mark") {
    return `unmark ${previewDebugObjectLabel(op)} ${previewDebugMarkLabel(op)} at ${previewDebugPositionLabel(op.position)}`;
  }
  if (kind === "update_variable") {
    return `${op?.op || "update"} ${previewDebugVariableLabel(op)} by ${op?.value ?? "?"}`;
  }
  return kind;
}

function previewDebugVisiblePatchOps(patch) {
  const ops = Array.isArray(patch) ? patch : [];
  if (ops.some((op) => typeof op?.visible !== "boolean")) {
    throw new Error("Rust debug patch operations must declare visibility.");
  }
  return ops.filter((op) => op.visible);
}

function previewDebugPatchSummary(patch) {
  const rawOps = Array.isArray(patch) ? patch : [];
  const ops = previewDebugVisiblePatchOps(rawOps);
  if (ops.length) {
    return ops.map(previewDebugPatchDetail).join("; ");
  }
  if (rawOps.length) {
    return "internal markers only";
  }
  return "no visible change";
}

function previewDebugTraceGroups(executions) {
  const groups = [];
  for (const [index, execution] of executions.entries()) {
    const rule = previewDebugRuleSummary(execution);
    const patch = previewDebugPatchSummary(execution.patch);
    const previous = groups[groups.length - 1];
    if (previous && previous.rule === rule && previous.patch === patch) {
      previous.end = index;
      previous.count += 1;
      continue;
    }
    groups.push({
      start: index,
      end: index,
      count: 1,
      rule,
      patch,
    });
  }
  return groups;
}

function previewDebugExecutionRangeLabel(group, total) {
  if (group.start === group.end) {
    return `rule ${group.start + 1}/${total}`;
  }
  return `rules ${group.start + 1}-${group.end + 1}/${total}`;
}

function clearPreviewTraceLogEntries() {
  previewLogEntries = previewLogEntries.filter((entry) => entry.source !== "trace");
}

function previewDebugTurnSummary(debug, executions) {
  const input = debug.input || `input#${debug.inputId ?? "?"}`;
  const visibleChanges = executions.reduce(
    (count, execution) => count + previewDebugVisiblePatchOps(execution.patch).length,
    0,
  );
  const cancelled = debug.cancelled ? " cancelled" : "";
  return `input ${input}: ${executions.length} rule hits, ${visibleChanges} visible changes${cancelled}`;
}

function previewDebugCommandSummary(commands) {
  if (!commands.length) {
    return "no patch";
  }
  return `commands: ${commands.join(", ")}`;
}

function handlePreviewDebugTrace(debug, snapshot = null) {
  if (!previewDebugEnabled || !debug) {
    return;
  }
  previewDebugTrace = debug;
  previewDebugSnapshot = snapshot || null;
  const executions = Array.isArray(debug.executions) ? debug.executions : [];
  previewDebugCursor = executions.length ? executions.length - 1 : -1;
  clearPreviewTraceLogEntries();
  appendPreviewLog("info", previewDebugTurnSummary(debug, executions), {
    source: "trace",
    plain: true,
  });
  for (const group of previewDebugTraceGroups(executions)) {
    const repeat = group.count > 1 ? ` x${group.count}` : "";
    const firstExecution = executions[group.start];
    appendPreviewLog("info", `${previewDebugExecutionRangeLabel(group, executions.length)}${repeat}: ${group.rule}\n  changes: ${group.patch}`, {
      source: "trace",
      plain: true,
      location: previewDebugRuleLocation(firstExecution),
      debugExecutionIndex: group.start,
      debugExecutionEnd: group.end,
    });
  }
  const commands = Array.isArray(debug.commands)
    ? debug.commands.map((command) => command?.kind).filter(Boolean)
    : [];
  if (commands.length) {
    appendPreviewLog("info", previewDebugCommandSummary(commands), { source: "trace", plain: true });
  }
  syncPreviewDebugControls();
  syncPreviewDebugState();
  renderPreviewLog();
}

function setPreviewDebugCursor(index) {
  const executions = previewDebugTrace?.executions || [];
  if (!executions.length) {
    previewDebugCursor = -1;
    syncPreviewDebugControls();
    return;
  }
  previewDebugCursor = Math.max(0, Math.min(executions.length - 1, index));
  syncPreviewDebugControls();
  syncPreviewDebugState();
}

function syncPreviewDebugState() {
  if (!previewDebugEnabled || !previewFrame?.contentWindow) {
    return;
  }
  const state = previewDebugStateDataForCursor();
  if (!state) {
    setStatus("Preview debug trace is missing its Rust-projected state.", "is-error");
    return;
  }
  const exportData = currentPreviewExportData();
  postEditorModelState({
    model: editorModelName(exportData),
    state,
    levelIndex: Number.isInteger(Number(previewDebugSnapshot?.levelIndex))
      ? Math.trunc(Number(previewDebugSnapshot.levelIndex))
      : previewSession?.state?.levelIndex ?? currentEditableLevelIndex(currentPreviewExportData()),
    materializeLevelStart: false,
  });
}

function previewDebugStateDataForCursor() {
  const executions = previewDebugTrace?.executions || [];
  if (previewDebugCursor < 0) {
    return previewDebugTrace?.initialSnapshot || null;
  }
  return executions[previewDebugCursor]?.snapshotAfter || null;
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
  if (Number.isInteger(entry?.debugExecutionIndex)) {
    event.preventDefault();
    setPreviewDebugCursor(entry.debugExecutionIndex);
    if (entry.location) {
      revealPreviewLogLocation(entry.location);
    }
    return true;
  }
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
    ? sourceEditorDocumentValue()
    : targetDocument.source || "";
  const offset = Number.isInteger(location.offset)
    ? Math.max(0, Math.min(source.length, location.offset))
    : sourceOffsetForLineColumn(source, positiveInteger(location.line) || 1, positiveInteger(location.column) || 1);
  if (!revealSourceLocation(
    { document: targetDocument, start: offset },
    { recordHistory: true, scrollAlignment: "center" },
  )) {
    setStatus("Could not reveal preview error source", "is-error");
    return false;
  }
  sourceEditor.focus({ preventScroll: true });
  setStatus(previewLogLocationTitle(location), "");
  return true;
}

async function previewRuntimeAssetValue(kind, controller, playerArtifact) {
  if (kind === "puzzle_wasm_player.runtime-urls") {
    if (controller?.launchProfile === STANDALONE_PLAYER_LAUNCH_PROFILE) {
      if (!window.PuzzleStudioRuntime?.playerRuntimeAssetUrls) {
        throw new Error("PuzzleStudio standalone player runtime assets are unavailable.");
      }
      return window.PuzzleStudioRuntime.playerRuntimeAssetUrls(playerArtifact);
    }
    if (controller?.launchProfile !== EDITOR_PLAYER_LAUNCH_PROFILE
      || !window.PuzzleStudioRuntime?.gameRuntimeAssets) {
      throw new Error("PuzzleStudio game runtime assets are unavailable.");
    }
    return window.PuzzleStudioRuntime.gameRuntimeAssets();
  }
  if (kind === "puzzle_wasm_visual_authoring.runtime-urls") {
    if (!window.PuzzleStudioRuntime?.visualAuthoringRuntimeAssets) {
      throw new Error("PuzzleStudio visual authoring runtime assets are unavailable.");
    }
    return window.PuzzleStudioRuntime.visualAuthoringRuntimeAssets();
  }
  throw new Error(`Unknown editor preview runtime asset: ${kind}`);
}

function previewRuntimeAssetSourceAllowed(source) {
  return source && previewRuntimeAssetWindows.has(source);
}

const U32_MAX_IDENTITY = 4294967295n;
const U64_MAX_IDENTITY = 18446744073709551615n;
let nextEditorPreviewCommandId = 0n;

function checkedIdentityIncrement(value, maximum, label) {
  if (typeof value !== "bigint" || value < 0n || value >= maximum) {
    throw new Error(`${label} identity is exhausted.`);
  }
  return value + 1n;
}

function allocateEditorPreviewCommandId() {
  const identity = checkedIdentityIncrement(
    nextEditorPreviewCommandId,
    U32_MAX_IDENTITY,
    "Editor preview command",
  );
  nextEditorPreviewCommandId = identity;
  return Number(identity);
}

function u32CommandIdentity(value) {
  return typeof value === "number"
    && Number.isInteger(value)
    && value > 0
    && value <= Number(U32_MAX_IDENTITY)
    ? value
    : null;
}

function postEditorPreviewEnvelope(type, payload = {}, targetFrame = editorRuntimeFrame) {
  const target = targetFrame?.contentWindow;
  if (!target) {
    return null;
  }
  const commandId = allocateEditorPreviewCommandId();
  target.postMessage({ ...payload, type, commandId }, window.location.origin);
  return commandId;
}

const postEditorModelState = (payload, targetFrame = editorRuntimeFrame) => (
  postEditorPreviewEnvelope("PuzzleStudioEditorModelState", payload, targetFrame)
);
const postEditorAuthoringProjection = (payload, targetFrame = editorRuntimeFrame) => (
  postEditorPreviewEnvelope("PuzzleStudioEditorAuthoringProjection", payload, targetFrame)
);
const postEditorPointerInput = (payload, targetFrame = editorRuntimeFrame) => (
  postEditorPreviewEnvelope("PuzzleStudioEditorPointerCommand", payload, targetFrame)
);
const postEditorKeyInput = (payload, targetFrame = editorRuntimeFrame) => (
  postEditorPreviewEnvelope("PuzzleStudioEditorKey", payload, targetFrame)
);
const postEditorSnapshotRequest = (targetFrame = editorRuntimeFrame) => (
  postEditorPreviewEnvelope("PuzzleStudioEditorSnapshotRequest", {}, targetFrame)
);

function respondToPreviewRuntimeAssetRequest(event, payload) {
  const target = event.source;
  const kind = String(payload.kind || "");
  const controller = target ? editorRuntimeControllerByWindow.get(target) : null;
  const allowed = Boolean(
    target
    && controller
    && event.origin === window.location.origin
    && previewRuntimeAssetSourceAllowed(target)
  );
  previewRuntimeAssetBridgeStatus.push({ kind, phase: allowed ? "requested" : "rejected" });
  previewRuntimeAssetBridgeStatus = previewRuntimeAssetBridgeStatus.slice(-20);
  if (!allowed) {
    return;
  }
  const requestId = canonicalU64Identity(payload.requestId);
  if (requestId === null || requestId === "0") {
    return;
  }
  previewRuntimeAssetValue(kind, controller, String(payload.playerArtifact || ""))
    .then((value) => {
      previewRuntimeAssetBridgeStatus.push({
        kind,
        phase: "resolved",
        length: JSON.stringify(value).length,
      });
      previewRuntimeAssetBridgeStatus = previewRuntimeAssetBridgeStatus.slice(-20);
      target.postMessage({
        type: "PuzzleStudioRuntimeAssetResponse",
        requestId,
        ok: true,
        value,
      }, window.location.origin);
    })
    .catch((error) => {
      previewRuntimeAssetBridgeStatus.push({
        kind,
        phase: "error",
        message: String(error?.message || error),
      });
      previewRuntimeAssetBridgeStatus = previewRuntimeAssetBridgeStatus.slice(-20);
      target.postMessage({
        type: "PuzzleStudioRuntimeAssetResponse",
        requestId,
        ok: false,
        error: String(error?.message || error),
      }, window.location.origin);
    });
}

window.addEventListener("message", (event) => {
  const payload = event.data || {};
  if (payload.type === "PuzzleStudioRuntimeAssetRequest") {
    respondToPreviewRuntimeAssetRequest(event, payload);
  }
});

const STANDALONE_PLAYER_LAUNCH_PROFILE = "standalonePlayer";
const EDITOR_PLAYER_LAUNCH_PROFILE = "editorPlayer";
const EDITOR_AUTHORING_LAUNCH_PROFILE = "editorAuthoring";
const VISUAL_AUTHORING_LAUNCH_PROFILE = "visualAuthoring";

function editorRuntimeFrameBootstrap(surfaceId, launchProfile) {
  if (typeof surfaceId !== "string" || !surfaceId || typeof launchProfile !== "string" || !launchProfile) {
    throw new Error("Editor runtime frame requires an explicit surface identity and launch profile.");
  }
  return JSON.stringify({ surfaceId, launchProfile });
}

function editorRuntimeLaunchPayload(value, launchProfile) {
  const runtimeExportJson = value?.runtimeExportJson;
  const progressIdentityKey = value?.progressIdentityKey;
  const playerArtifact = value?.playerArtifact;
  const traceProvenance = value?.traceProvenance ?? null;
  const playerRuntime = launchProfile === STANDALONE_PLAYER_LAUNCH_PROFILE
    || launchProfile === EDITOR_PLAYER_LAUNCH_PROFILE;
  if (
    (playerRuntime && (
      typeof runtimeExportJson !== "string"
      || !runtimeExportJson
      || typeof progressIdentityKey !== "string"
      || !progressIdentityKey
      || typeof playerArtifact !== "string"
      || !playerArtifact
    ))
    || (!playerRuntime && (
      runtimeExportJson !== null
      || progressIdentityKey !== null
      || playerArtifact !== null
    ))
  ) {
    throw new Error("Editor runtime launch payload is invalid.");
  }
  return Object.freeze({
    runtimeExportJson,
    progressIdentityKey,
    playerArtifact,
    traceProvenance,
  });
}

function setPreviewRuntime(runtime, options = {}) {
  if (!previewViewport) {
    previewFrameReadyPromise = Promise.resolve(false);
    previewRuntimeReadyPromise = Promise.resolve(false);
    return previewFrameReadyPromise;
  }

  previewFrameHasCurrentCompiledPreview = false;
  previewRuntimeReady = false;
  const controller = previewEditorRuntimeController();
  for (const candidate of editorRuntimeControllers.values()) {
    if (candidate.consumer === "preview") candidate.surface.hidden = candidate !== controller;
  }
  editorRuntimeSurface = controller.surface;
  editorRuntimeFrame = controller.frame;
  previewFrame = controller.frame;
  if (controller.surface.parentElement !== previewViewport) {
    previewViewport.append(controller.surface);
  }
  controller.surface.hidden = false;
  schedulePreviewViewportGeometrySync(6);
  loadEditorRuntimeController(controller, runtime, String(previewBuild?.id || ""));
  previewFrameReadyPromise = controller.loadedPromise;
  previewRuntimeReadyPromise = controller.readyPromise;
  previewFrameReadyPromise.then((loaded) => {
    if (!loaded) {
      return;
    }
    if (options.markDocumentLoaded) {
      previewFrameHasCurrentCompiledPreview = true;
      setPreviewDocumentLoaded(true);
    }
    schedulePreviewViewportGeometrySync(6);
    if (currentPreviewMode === "level3d" && typeof sendLevel3dSnapshotToRuntime === "function") {
      sendLevel3dSnapshotToRuntime();
    } else if (activePreviewModeAcceptsLevelState()) {
      sendLevelDraftToPreview();
    }
  });
  return previewFrameReadyPromise;
}

function initializeEditorRuntimeController({ surfaceId, consumer, launchProfile, surface, frame }) {
  surface.dataset.surfaceId = surfaceId;
  surface.dataset.consumer = consumer;
  frame.setAttribute("name", editorRuntimeFrameBootstrap(surfaceId, launchProfile));
  if (!frame.getAttribute("src")) {
    frame.src = launchProfile === EDITOR_AUTHORING_LAUNCH_PROFILE
      || launchProfile === VISUAL_AUTHORING_LAUNCH_PROFILE
      ? "./editor_visual_runtime_host.html"
      : "./editor_runtime_host.html";
  }
  let resolveHostReady = null;
  const controller = {
    surfaceId,
    launchProfile,
    consumer,
    surface,
    frame,
    presentedFrame: frame,
    hostReady: false,
    hostReadyPromise: new Promise((resolve) => { resolveHostReady = resolve; }),
    resolveHostReady,
    generationId: 0n,
    activeGenerationId: "",
    buildId: "",
    presentedBuildId: "",
    loadingBuildId: "",
    ready: false,
    readyPromise: Promise.resolve(false),
    resolveReady: null,
    loadedPromise: Promise.resolve(false),
    resolveLoaded: null,
    requestId: 0n,
    pendingGeneration: null,
    lastFrameRect: null,
    displayKey: "",
    visualDraftKey: "",
    pending: null,
  };
  const registerFrameWindow = () => {
    if (!frame.contentWindow) return;
    previewRuntimeAssetWindows.add(frame.contentWindow);
    editorRuntimeControllerByWindow.set(frame.contentWindow, controller);
    frame.contentWindow.postMessage(
      { type: "PuzzleStudioRuntimeHostProbe" },
      window.location.origin,
    );
  };
  frame.addEventListener("load", registerFrameWindow);
  registerFrameWindow();
  editorRuntimeControllers.set(surfaceId, controller);
  installEditorRuntimeResizeObserver(controller);
  return controller;
}

function registerEditorRuntimeConsumer(consumer, handler) {
  if (typeof consumer !== "string" || !consumer || !handler || typeof handler !== "object") {
    throw new Error("Editor runtime consumer registration requires a name and handler.");
  }
  if (editorRuntimeConsumerHandlers.has(consumer)) {
    throw new Error(`Editor runtime consumer ${consumer} is already registered.`);
  }
  editorRuntimeConsumerHandlers.set(consumer, Object.freeze({ ...handler }));
}

function editorRuntimeConsumerHandler(controller) {
  return editorRuntimeConsumerHandlers.get(controller?.consumer) || null;
}

function previewEditorRuntimeController() {
  const surfaceId = previewDebugEnabled ? "preview-debug" : "preview";
  const launchProfile = previewDebugEnabled
    ? EDITOR_PLAYER_LAUNCH_PROFILE
    : STANDALONE_PLAYER_LAUNCH_PROFILE;
  const existing = editorRuntimeControllers.get(surfaceId);
  if (existing) return existing;
  if (previewDebugEnabled) {
    return createEditorRuntimeController(surfaceId, "preview", launchProfile);
  }
  return initializeEditorRuntimeController({
    surfaceId,
    launchProfile,
    consumer: "preview",
    surface: editorRuntimeSurface,
    frame: editorRuntimeFrame,
  });
}

function createEditorRuntimeController(surfaceId, consumer, launchProfile) {
  const surface = document.createElement("div");
  surface.className = "editor-runtime-surface";
  surface.tabIndex = -1;
  const frame = document.createElement("iframe");
  frame.className = "editor-runtime-frame";
  frame.tabIndex = -1;
  frame.title = "Editor runtime preview";
  frame.setAttribute("sandbox", "allow-scripts allow-same-origin");
  frame.setAttribute("allow", "autoplay");
  frame.setAttribute("scrolling", "no");
  surface.append(frame);
  return initializeEditorRuntimeController({
    surfaceId,
    consumer,
    launchProfile,
    surface,
    frame,
  });
}

function installEditorRuntimeResizeObserver(controller) {
  if (typeof ResizeObserver !== "function") {
    throw new Error("Editor runtime surface requires ResizeObserver.");
  }
  controller.resizeObserver = new ResizeObserver(() => {
    const previous = controller.lastFrameRect;
    if (!previous) {
      return;
    }
    const current = controller.frame.getBoundingClientRect();
    if (current.width === previous.width && current.height === previous.height) {
      return;
    }
    controller.lastFrameRect = current;
    editorRuntimeCommittedFrames.delete(controller.surfaceId);
    delete controller.surface.dataset.frameRevision;
  });
  controller.resizeObserver.observe(controller.surface);
}

function editorRuntimeController(surfaceId, consumer, launchProfile) {
  const controller = editorRuntimeControllers.get(surfaceId)
    || createEditorRuntimeController(surfaceId, consumer, launchProfile);
  if (controller.launchProfile !== launchProfile) {
    throw new Error(`Editor runtime surface ${surfaceId} cannot change launch profile.`);
  }
  controller.consumer = consumer;
  controller.surface.dataset.consumer = consumer;
  return controller;
}

function sendPendingEditorRuntimeGeneration(controller) {
  const pending = controller.pendingGeneration;
  if (!controller.hostReady || !pending || pending.sent || !controller.frame.contentWindow) {
    return false;
  }
  pending.sent = true;
  controller.frame.contentWindow.postMessage({
    type: "PuzzleStudioLoadRuntimeGeneration",
    runtimeGenerationId: pending.generationId,
    runtimeExportJson: pending.runtime.runtimeExportJson,
    progressIdentityKey: pending.runtime.progressIdentityKey,
    playerArtifact: pending.runtime.playerArtifact,
    traceProvenance: pending.runtime.traceProvenance,
  }, window.location.origin);
  return true;
}

function markEditorRuntimeHostLoaded(controller) {
  if (!controller.hostReady) {
    controller.hostReady = true;
    controller.resolveHostReady?.(true);
    controller.resolveHostReady = null;
  }
  sendPendingEditorRuntimeGeneration(controller);
}

function loadEditorRuntimeController(controller, runtime, buildId) {
  const launch = editorRuntimeLaunchPayload(runtime, controller.launchProfile);
  const generation = checkedIdentityIncrement(
    controller.generationId,
    U64_MAX_IDENTITY,
    "Editor runtime generation",
  );
  controller.pendingGeneration?.resolveLoaded(false);
  controller.ready = false;
  controller.loadingBuildId = buildId;
  controller.displayKey = "";
  controller.visualDraftKey = "";
  controller.pending = null;
  controller.resolveReady?.(false);
  controller.resolveLoaded?.(false);
  controller.readyPromise = new Promise((resolve) => {
    controller.resolveReady = resolve;
  });
  let resolveLoaded = null;
  controller.loadedPromise = new Promise((resolve) => {
    resolveLoaded = resolve;
    controller.resolveLoaded = resolve;
  });
  controller.generationId = generation;
  controller.pendingGeneration = {
    generationId: generation.toString(),
    buildId,
    runtime: launch,
    sent: false,
    resolveLoaded,
  };
  editorRuntimeCommittedFrames.delete(controller.surfaceId);
  delete controller.surface.dataset.frameRevision;
  sendPendingEditorRuntimeGeneration(controller);
}

function stopEditorRuntimeController(controller) {
  controller.pendingGeneration?.resolveLoaded(false);
  controller.resolveReady?.(false);
  controller.resolveLoaded?.(false);
  const generationId = controller.pendingGeneration?.generationId
    || controller.activeGenerationId;
  if (generationId && controller.frame.contentWindow) {
    controller.frame.contentWindow.postMessage({
      type: "PuzzleStudioStopRuntimeGeneration",
      runtimeGenerationId: generationId,
    }, window.location.origin);
  }
  controller.pendingGeneration = null;
  controller.loadingBuildId = "";
  controller.ready = false;
  controller.readyPromise = Promise.resolve(false);
  controller.loadedPromise = Promise.resolve(false);
  controller.resolveReady = null;
  controller.resolveLoaded = null;
  controller.pending = null;
  controller.surface.dataset.runtimeReady = "false";
  controller.surface.hidden = true;
}

function canonicalU64Identity(value) {
  if (
    typeof value !== "string"
    || value.length === 0
    || value.length > 20
    || !/^(0|[1-9][0-9]*)$/.test(value)
  ) {
    return null;
  }
  return BigInt(value) <= U64_MAX_IDENTITY ? value : null;
}

function acceptEditorRuntimeReady(controller, source, message) {
  const generationId = canonicalU64Identity(String(message.runtimeGenerationId || ""));
  const pending = controller.pendingGeneration;
  const pendingGeneration = Boolean(pending && generationId === pending.generationId);
  const publishedGeneration = Boolean(
    !pending && generationId === controller.activeGenerationId,
  );
  if (
    source !== controller.frame.contentWindow
    || generationId === null
    || (!pendingGeneration && !publishedGeneration)
  ) {
    return false;
  }
  controller.ready = true;
  if (pendingGeneration) {
    controller.activeGenerationId = generationId;
    controller.presentedBuildId = pending.buildId;
    controller.buildId = pending.buildId;
    controller.loadingBuildId = "";
    controller.pendingGeneration = null;
    controller.frame.removeAttribute("aria-hidden");
    controller.frame.style.visibility = "visible";
    controller.lastFrameRect = controller.frame.getBoundingClientRect();
    if (controller.resolveLoaded === pending.resolveLoaded) controller.resolveLoaded = null;
    pending.resolveLoaded(true);
  }
  controller.surface.dataset.runtimeReady = "true";
  controller.resolveReady?.(true);
  controller.resolveReady = null;
  if (controller.consumer === "preview" && controller === previewEditorRuntimeController()) {
    previewRuntimeReady = true;
  }
  return true;
}

function rejectPendingEditorRuntimeGeneration(controller, source, message) {
  const generationId = canonicalU64Identity(String(message.runtimeGenerationId || ""));
  const pending = controller.pendingGeneration;
  if (
    source !== controller.frame.contentWindow
    || generationId === null
    || !pending
    || generationId !== pending.generationId
  ) {
    return false;
  }
  controller.pendingGeneration = null;
  controller.loadingBuildId = "";
  controller.ready = false;
  controller.readyPromise = Promise.resolve(false);
  controller.resolveReady?.(false);
  controller.resolveReady = null;
  pending.resolveLoaded(false);
  if (controller.resolveLoaded === pending.resolveLoaded) controller.resolveLoaded = null;
  controller.loadedPromise = Promise.resolve(false);
  controller.pending = null;
  controller.surface.dataset.runtimeReady = "false";
  if (controller.consumer === "preview" && controller === previewEditorRuntimeController()) {
    previewRuntimeReady = false;
  }
  return true;
}

function levelEditorAuthoringRuntimeBuild() {
  return Object.freeze({
    id: "editor-level-authoring-runtime",
    runtime: Object.freeze({
      runtimeExportJson: null,
      progressIdentityKey: null,
      playerArtifact: null,
    }),
  });
}

async function ensureEditorRuntimeController(
  host,
  surfaceId,
  consumer,
  launchProfile,
  runtime = null,
) {
  const launch = runtime?.runtime ?? previewBuild?.runtime;
  const buildId = String(runtime?.id ?? previewBuild?.id ?? "");
  if (!host || !launch || !buildId) {
    throw new Error("Editor Bevy runtime surface requires an explicit runtime build and host.");
  }
  const controller = editorRuntimeController(surfaceId, consumer, launchProfile);
  if (controller.surface.parentElement !== host) {
    host.replaceChildren(controller.surface);
  }
  controller.surface.hidden = false;
  host.dataset.runtimeEmpty = "false";
  if (controller.buildId !== buildId && controller.loadingBuildId !== buildId) {
    loadEditorRuntimeController(controller, launch, buildId);
  }
  if (controller.buildId !== buildId || !controller.ready) {
    const ready = await controller.readyPromise;
    if (!ready) {
      throw new Error(`Editor Bevy runtime ${surfaceId} did not become ready.`);
    }
  }
  return controller;
}

function hideEditorRuntimeSurface(host, surfaceId = "solver-observation") {
  const controller = editorRuntimeControllers.get(surfaceId);
  if (controller) {
    const requestId = checkedIdentityIncrement(
      controller.requestId,
      U64_MAX_IDENTITY,
      "Editor runtime request",
    );
    controller.requestId = requestId;
    controller.displayKey = "";
    controller.pending = null;
    controller.surface.hidden = true;
  }
  if (host) {
    host.dataset.runtimeEmpty = "true";
  }
}

function editorRuntimeHasPresentedGeneration(controller) {
  return controller.ready
    && !controller.pendingGeneration
    && Boolean(controller.activeGenerationId)
    && controller.frame?.isConnected
    && controller.frame.parentElement === controller.surface;
}

function markPreviewPresentationReady(controller) {
  const buildId = String(previewBuild?.id || "");
  if (
    controller.consumer !== "preview"
    || controller !== previewEditorRuntimeController()
    || !controller.ready
    || controller.pendingGeneration
    || !editorRuntimeHasPresentedGeneration(controller)
    || !buildId
    || String(controller.presentedBuildId || "") !== buildId
  ) {
    return false;
  }
  if (controller.surface.dataset.previewReadyBuildId === buildId) {
    return true;
  }
  controller.surface.dataset.previewReadyBuildId = buildId;
  appendPreviewLog("system", "Preview ready", { source: "runtime" });
  setStatus("Preview ready", "is-ok");
  return true;
}

function queueEditorRuntimeDisplay({
  host,
  consumer,
  surfaceId,
  launchProfile,
  key,
  onError,
  runtime,
  selectCommand,
  dispatch,
}) {
  const existingController = editorRuntimeControllers.get(surfaceId);
  const request = checkedIdentityIncrement(
    existingController?.requestId ?? 0n,
    U64_MAX_IDENTITY,
    "Editor runtime request",
  );
  const controller = editorRuntimeController(surfaceId, consumer, launchProfile);
  controller.requestId = request;
  Promise.resolve(runtime)
    .then((resolvedRuntime) => ensureEditorRuntimeController(
      host,
      surfaceId,
      consumer,
      launchProfile,
      resolvedRuntime,
    ))
    .then(async (activeController) => {
      if (request !== activeController.requestId) {
        return;
      }
      const command = await (selectCommand?.(activeController) || { key, dispatch });
      if (request !== activeController.requestId) {
        return;
      }
      if (!command.key || typeof command.dispatch !== "function") {
        throw new Error("Editor Bevy runtime command selection is invalid.");
      }
      if (
        activeController.displayKey === command.key
        || activeController.pending?.key === command.key
      ) {
        return;
      }
      const commandId = command.dispatch(activeController.frame);
      if (!commandId) {
        throw new Error("Editor Bevy runtime command transport is unavailable.");
      }
      activeController.pending = { commandId, key: command.key };
      editorRuntimeCommands.set(commandId, {
        key: command.key,
        consumer,
        surfaceId,
        kind: command.context?.kind || "display",
        controller: activeController,
        ...command.context,
      });
      activeController.surface.dataset.pendingCommandId = String(commandId);
    })
    .catch((error) => {
      if (request !== controller.requestId) {
        return;
      }
      const retainPublishedFrame = editorRuntimeHasPresentedGeneration(controller);
      controller.surface.hidden = !retainPublishedFrame;
      host.dataset.runtimeEmpty = retainPublishedFrame ? "false" : "true";
      onError?.(error);
    });
}

function editorModelName(exportData = currentPreviewExportData()) {
  const model = String(exportData?.modelName || "").trim();
  if (!model) {
    throw new Error("Editor runtime display is missing its compiled model identity.");
  }
  return model;
}

function editorDraftSurface(interaction, surfaceId) {
  return { surfaceId, interaction };
}

function editorRuntimePresentationForState(
  state,
  surfaceId,
  interaction,
  exportData = currentPreviewExportData(),
) {
  if (state?.kind === "2d") {
    return {
      surface: editorDraftSurface(interaction, surfaceId),
      renderer: { kind: "grid2d" },
    };
  }
  if (state?.kind === "3d") {
    if (typeof level3dEditorRendererStrategy !== "function") {
      throw new Error("The compiled 3D editor renderer presentation is unavailable.");
    }
    return {
      surface: editorDraftSurface(interaction, surfaceId),
      renderer: level3dEditorRendererStrategy(exportData),
    };
  }
  throw new Error("Editor runtime state must declare kind 2d or 3d.");
}

function queueLevelAuthoringRuntime() {
  if (!levelBoard || levelBuilder.hidden) {
    return false;
  }
  const exportData = currentLevelExportData();
  if (!levelEditorAssistanceReady(exportData)) {
    return false;
  }
  const interaction = {
    kind: "paint",
    operation: level.selectedObjectId ? "replace" : "erase",
  };
  const surfaceId = levelPlaytestActive ? "level-play" : "level-authoring";
  const levelIndex = currentEditableLevelIndex(exportData);
  const presentation = {
    surface: editorDraftSurface(interaction, surfaceId),
    renderer: { kind: "grid2d" },
  };
  queueEditorRuntimeDisplay({
    host: levelBoard,
    consumer: "authoring",
    surfaceId,
    launchProfile: levelPlaytestActive
      ? EDITOR_PLAYER_LAUNCH_PROFILE
      : EDITOR_AUTHORING_LAUNCH_PROFILE,
    selectCommand: async (controller) => {
      const state = levelPlaytestActive
        ? await exportData.session.draftState(levelIndex)
        : await exportData.session.authoringState(levelIndex);
      const payload = levelPlaytestActive
        ? { model: exportData.modelName, levelIndex, state }
        : {
          model: exportData.authoringModelProjection,
          levelIndex,
          state,
          presentation,
        };
      return {
        key: `draft:${surfaceId}:${nextEditorPreviewCommandId}`,
        dispatch: (targetFrame) => levelPlaytestActive
          ? postEditorModelState(payload, targetFrame)
          : postEditorAuthoringProjection(payload, targetFrame),
      };
    },
    onError: (error) => setPaneStatus("level", `Level display failed: ${userFacingRuntimeError(error)}`, "is-error"),
    runtime: levelPlaytestActive ? undefined : levelEditorAuthoringRuntimeBuild(),
  });
  return true;
}

function editorPointerGesture(gesture) {
  return gesture === "press"
    ? "press"
    : gesture === "release"
      ? "release"
      : gesture === "leave"
        ? "leave"
        : "move";
}

function editorPointerEraseIntent(event) {
  return Number(event?.button) === 2;
}

async function dispatchEditorAuthoringPointer(controller, eventData) {
  if (controller?.consumer !== "authoring") {
    return;
  }
  const frameRevision = editorRuntimeCommittedFrames.get(controller.surfaceId);
  if (canonicalU64Identity(frameRevision) === null) {
    return;
  }
  if (controller.pointerCommandPending || controller.pending) {
    controller.queuedAuthoringPointer = {
      eventData: { ...eventData },
    };
    return;
  }
  const gesture = editorPointerGesture(eventData.gesture);
  if (gesture === "press" && Number(eventData.button) !== 0 && !editorPointerEraseIntent(eventData)) {
    return;
  }
  const mutate = gesture === "press" || (gesture === "move" && controller.pointerPressed === true);
  if (gesture === "press") {
    controller.pointerPressed = true;
    controller.pointerErase = editorPointerEraseIntent(eventData);
    const kind = controller.surfaceId === "level-authoring" ? "level" : "level3d";
    const transition = kind === "level"
      ? await dispatchLevelSessionCommand({ type: "beginEdit" }, { render: false })
      : await dispatchLevel3dSessionCommand({ type: "beginEdit" }, { render: false });
    if (!transition) {
      controller.pointerPressed = false;
      return;
    }
    controller.authoringHistory = { kind };
  } else if (gesture === "release" || gesture === "leave") {
    controller.pointerPressed = false;
  }
  const erase = controller.pointerErase === true;
  const commandId = postEditorPointerInput({
    surfaceId: controller.surfaceId,
    committedFrameRevision: frameRevision,
    xCss: Number(eventData.xCss),
    yCss: Number(eventData.yCss),
    gesture,
    operation: erase ? "erase" : null,
  }, controller.frame);
  if (commandId) {
    controller.pointerCommandPending = true;
    editorRuntimeCommands.set(commandId, {
      kind: "editorPointer",
      consumer: "authoring",
      surfaceId: controller.surfaceId,
      frameRevision,
      gesture,
      controller,
      mutate,
      erase,
    });
  }
  if (gesture === "release" || gesture === "leave") {
    controller.pointerErase = false;
  }
}

function completeEditorAuthoringPointer(controller) {
  controller.pointerCommandPending = false;
  flushQueuedEditorAuthoringPointer(controller);
}

function flushQueuedEditorAuthoringPointer(controller) {
  if (controller.pointerCommandPending || controller.pending) {
    return;
  }
  const queued = controller.queuedAuthoringPointer;
  controller.queuedAuthoringPointer = null;
  if (!queued) {
    return;
  }
  window.setTimeout(() => {
    dispatchEditorAuthoringPointer(
      controller,
      queued.eventData,
    );
  }, 0);
}

function editorGridPosition(hit) {
  const tagged = hit?.position;
  return tagged?.position && (tagged.kind === "grid2d" || tagged.kind === "grid3d")
    ? tagged
    : null;
}

async function applyEditorAuthoringHit(hit, surfaceId, options = {}) {
  if (!hit) {
    return;
  }
  const erase = options.erase === true;
  const tagged = editorGridPosition(hit);
  if (tagged?.kind === "grid2d" && surfaceId === "level-authoring") {
    const x = Math.trunc(Number(tagged.position.x));
    const y = Math.trunc(Number(tagged.position.y));
    const index = y * level.width + x;
    return levelBucketActive && !erase
      ? await bucketFillLevelFromIndex(index)
      : await paintLevelCellAtIndex(index, erase ? null : level.selectedObjectId);
  }
  if (tagged?.kind === "grid3d" && surfaceId.startsWith("level-authoring")) {
    const selected = typeof level3dSelectedEntry === "function" ? level3dSelectedEntry() : null;
    const symbol = erase
      ? level3dEmptyChar()
      : selected?.char || level3d.selectedChar || level3dEmptyChar();
    return level3d.layerFillActive && !erase
      ? await bucketFillLevel3dLayerFromPosition(tagged.position)
      : await paintLevel3dCellAtPosition(tagged.position, symbol);
  }
  if (erase) {
    return false;
  }
  if (hit.kind !== "resize") {
    return;
  }
  const mode = hit.mode === "shrink" ? "shrink" : "expand";
  const side = hit.side === "min" ? "min" : "max";
  if (surfaceId === "level-authoring") {
    const edge = hit.axis === "x"
      ? (side === "min" ? "left" : "right")
      : (side === "min" ? "top" : "bottom");
    await resizeLevelEdge(edge, mode);
    return true;
  }
  if (surfaceId.startsWith("level-authoring")) {
    const delta = mode === "shrink" ? -1 : 1;
    if (hit.axis === "x") {
      await resizeLevel3dWidth(level3d.width + delta, { edge: side === "min" ? "left" : "right" });
    } else if (hit.axis === "y") {
      await resizeLevel3dDepth(level3d.depth + delta, { edge: side === "min" ? "front" : "back" });
    } else {
      await resizeLevel3dHeight(level3d.height + delta, { edge: side === "min" ? "bottom" : "top" });
    }
    return true;
  }
  return false;
}

function updatePreviewFrameLayout(layout) {
  void layout;
  syncPreviewViewportGeometry();
}

function setPreviewViewportAspect(aspectRatio) {
  const width = Number(aspectRatio?.width);
  const height = Number(aspectRatio?.height);
  const next = Number.isFinite(width) && Number.isFinite(height) && width > 0 && height > 0
    ? width / height
    : previewDefaultLogicalWidth / previewDefaultLogicalHeight;
  if (Math.abs(next - previewViewportAspect) < 0.0001) {
    return;
  }
  previewViewportAspect = next;
  schedulePreviewViewportGeometrySync(2);
}

function syncPreviewViewportGeometry() {
  if (!previewFrameWrap || !previewViewport) {
    return;
  }
  if (previewFrameWrap.getClientRects().length === 0 || previewViewport.getClientRects().length === 0) {
    return;
  }
  const available = editorFrameAvailableSize(previewFrameWrap, {
    container: playPreview,
    reservedBlock: previewLogReservedBlockSize(),
  });
  const viewportSize = fitEditorAspectFrame(available, previewViewportAspect);
  const viewportWidth = viewportSize.width;
  const viewportHeight = viewportSize.height;
  const framePaddingAndBorder = 0;
  previewFrameWrap.style.setProperty("--preview-viewport-width", `${viewportWidth}px`);
  previewFrameWrap.style.setProperty("--preview-viewport-height", `${viewportHeight}px`);
  previewFrameWrap.style.setProperty("--preview-frame-height", `${viewportHeight + framePaddingAndBorder}px`);
  syncPreviewAutoLogHeight(viewportHeight + framePaddingAndBorder);
}

function schedulePreviewViewportGeometrySync(passes = 2) {
  previewViewportGeometrySyncPasses = Math.max(
    previewViewportGeometrySyncPasses,
    Math.max(1, Math.trunc(Number(passes) || 1)),
  );
  if (previewViewportGeometrySyncFrame) {
    return;
  }
  const tick = () => {
    previewViewportGeometrySyncFrame = 0;
    syncPreviewViewportGeometry();
    previewViewportGeometrySyncPasses -= 1;
    if (previewViewportGeometrySyncPasses > 0) {
      previewViewportGeometrySyncFrame = requestAnimationFrame(tick);
    }
  };
  previewViewportGeometrySyncFrame = requestAnimationFrame(tick);
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



function normalizeEditorDimension(dimension) {
  return String(dimension || "").toLowerCase() === "3d" ? "3d" : "2d";
}

function editorDimensionLabel(dimension = currentEditorDimension) {
  return normalizeEditorDimension(dimension) === "3d" ? "3D" : "2D";
}

function levelModeForEditorDimension(dimension = currentEditorDimension) {
  return normalizeEditorDimension(dimension) === "3d" ? "level3d" : "edit";
}

function visualModeForEditorDimension(dimension = currentEditorDimension) {
  return normalizeEditorDimension(dimension) === "3d" ? "visual3d" : "visual";
}

function focusedPuzzleSourceContext(document = activeDocument()) {
  if (!isPuzzleDocument(document) || !isTextDocument(document)) {
    return null;
  }
  const source = sourceForDocument(document);
  return { document, source };
}

async function focusedPuzzleSourceContextWithEntries(context = focusedPuzzleSourceContext()) {
  if (!context?.document) {
    return null;
  }
  const documentId = context.document.id || "";
  await loadSurfaceEntriesForSource(context.source, { reportUnavailable: true });
  const current = focusedPuzzleSourceContext();
  if (
    !current?.document
    || (current.document.id || "") !== documentId
    || current.source !== context.source
  ) {
    return null;
  }
  return current;
}

function firstFocusedPuzzleEntry(kind, context = focusedPuzzleSourceContext()) {
  return focusedPuzzleEntries(kind, context)[0] || null;
}

function focusedPuzzleEntries(kind, context = focusedPuzzleSourceContext()) {
  if (!context?.document) {
    return [];
  }
  return uniqueFocusedPuzzleEntries(focusedPuzzleSurfaceEntriesByKind(kind, context))
    .map((entry) => {
      if (entry.dimension !== "2d" && entry.dimension !== "3d") {
        throw new Error(`Source ${kind} entry is missing its canonical dimension.`);
      }
      return {
        dimension: entry.dimension,
        target: { ...entry, document: context.document },
      };
    })
    .sort((left, right) => left.target.start - right.target.start);
}

function firstFocusedPuzzleEntryForDimension(kind, dimension, context = focusedPuzzleSourceContext()) {
  return focusedPuzzleEntriesForDimension(kind, dimension, context)[0]?.target || null;
}

function focusedPuzzleEntriesForDimension(kind, dimension, context = focusedPuzzleSourceContext()) {
  const normalized = normalizeEditorDimension(dimension);
  return focusedPuzzleEntries(kind, context)
    .filter((item) => item.dimension === normalized);
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
  return kind === "visual"
    ? visualModeForEditorDimension(dimension)
    : levelModeForEditorDimension(dimension);
}

async function syncPaneModesFromFocusedPuzzleSource(options = {}) {
  const context = focusedPuzzleSourceContext();
  if (!context?.document) {
    return null;
  }
  const documentId = context.document.id;
  await loadSurfaceEntriesForSource(context.source, { reportUnavailable: true });
  const currentContext = focusedPuzzleSourceContext();
  if (currentContext?.document?.id !== documentId || currentContext.source !== context.source) {
    return null;
  }
  const firstLevel = firstFocusedPuzzleEntry("level", context);
  const firstVisual = firstFocusedPuzzleEntry("visual", context);
  const levelMode = modeForFocusedPuzzleEntry("level", context);
  const visualMode = modeForFocusedPuzzleEntry("visual", context);
  if (levelMode) {
    currentLevelPaneMode = levelMode;
  } else if (currentPreviewMode === "edit" || currentPreviewMode === "level3d") {
    currentLevelPaneMode = "none";
  }
  if (visualMode) {
    currentVisualPaneMode = visualMode;
  } else if (currentPreviewMode === "visual" || currentPreviewMode === "visual3d") {
    currentVisualPaneMode = "none";
  }

  let nextMode = null;
  if (currentPreviewMode === "edit" || currentPreviewMode === "level3d") {
    nextMode = levelMode;
  } else if (currentPreviewMode === "visual" || currentPreviewMode === "visual3d") {
    nextMode = visualMode;
  }

  if (nextMode && options.switchOpenPane !== false && nextMode !== currentPreviewMode) {
    setPreviewMode(nextMode);
  } else {
    const inferredDimension = editorDimensionForPreviewMode(nextMode || levelMode || visualMode || currentLevelPaneMode || currentVisualPaneMode);
    currentEditorDimension = normalizeEditorDimension(inferredDimension);
    syncPreviewModeButtonState();
  }
  if (options.loadFirst !== false) {
    if (currentPreviewMode === "edit" || currentPreviewMode === "level3d") {
      await loadFocusedPuzzleEntry("level", firstLevel, { silent: true, recordHistory: false });
    } else if (currentPreviewMode === "visual" || currentPreviewMode === "visual3d") {
      await loadFocusedPuzzleEntry("visual", firstVisual, { silent: true, recordHistory: false });
    }
  }
  if (
    ((currentPreviewMode === "edit" || currentPreviewMode === "level3d") && !firstLevel)
    || ((currentPreviewMode === "visual" || currentPreviewMode === "visual3d") && !firstVisual)
  ) {
    applyPaneVisibility();
  }
  return nextMode || null;
}

async function loadSurfaceEntriesForSource(source, options = {}) {
  const text = String(source || "");
  if (!text) {
    surfaceEntriesCache = { source: text, entries: [] };
    return surfaceEntriesCache.entries;
  }
  if (surfaceEntriesCache?.source === text) {
    return surfaceEntriesCache.entries;
  }
  if (surfaceEntriesRequest?.source === text) {
    return surfaceEntriesRequest.promise;
  }
  if (typeof window.PuzzleStudioRuntime?.sourceEntryInfo !== "function") {
    const message = "Source entries unavailable: editor analysis worker is not loaded.";
    if (options.reportUnavailable !== false) {
      setStatus(message, "is-error");
    }
    throw new Error(message);
  }
  const request = {};
  request.source = text;
  request.promise = window.PuzzleStudioRuntime.sourceEntryInfo(text)
    .then((entryInfo) => {
      const entries = normalizeResolvedSourceTargets(text, entryInfo?.entries);
      if (surfaceEntriesRequest === request) {
        surfaceEntriesCache = { source: text, entries };
        surfaceEntriesRequest = null;
      }
      return entries;
    })
    .catch((error) => {
      const currentRequest = surfaceEntriesRequest === request;
      if (currentRequest) {
        surfaceEntriesRequest = null;
      }
      const message = `Source entries unavailable: ${userFacingRuntimeError(error)}`;
      const activeContext = focusedPuzzleSourceContext();
      if (
        options.reportUnavailable !== false
        && currentRequest
        && activeContext?.source === text
      ) {
        setStatus(message, "is-error");
      }
      throw new Error(message);
    });
  surfaceEntriesRequest = request;
  return request.promise;
}

function refreshSurfaceEntriesForActiveSource(source) {
  const text = String(source || "");
  surfaceEntriesCache = null;
  return loadSurfaceEntriesForSource(text, { reportUnavailable: true });
}

function surfaceEntriesForSource(source, options = {}) {
  const text = String(source || "");
  if (!text) {
    return [];
  }
  if (surfaceEntriesCache?.source === text) {
    return surfaceEntriesCache.entries;
  }
  const activeSource = focusedPuzzleSourceContext()?.source;
  if (text === activeSource) {
    const message = "Source entries are not ready for the active editor revision.";
    if (options.reportUnavailable !== false) {
      setStatus(message, "is-error");
    }
    throw new Error(message);
  }
  const message = "Source entries are available only for the active analyzed revision.";
  if (options.reportUnavailable !== false) {
    setStatus(message, "is-error");
  }
  throw new Error(message);
}

function focusedPuzzleSurfaceEntries(context = focusedPuzzleSourceContext()) {
  try {
    return surfaceEntriesForSource(context?.source || "", { reportUnavailable: true });
  } catch (error) {
    console.warn("Focused source entries unavailable", error);
    return [];
  }
}

function sourceTargetMatches(target, kind, dimension = "") {
  return target?.kind === kind
    && (!dimension || target.dimension === normalizeEditorDimension(dimension));
}

function focusedPuzzleSurfaceEntriesByKind(kind, context = focusedPuzzleSourceContext(), dimension = "") {
  return focusedPuzzleSurfaceEntries(context)
    .filter((entry) => sourceTargetMatches(entry, kind, dimension));
}

function firstFocusedPuzzleLevel2dEntry(source, document) {
  return focusedPuzzleLevel2dEntries(source, document)[0] || null;
}

function focusedPuzzleLevel2dEntries(source, document) {
  return uniqueFocusedPuzzleEntries(
    focusedPuzzleSurfaceEntriesByKind("level", { document, source }, "2d")
  );
}

function firstFocusedPuzzleLevel2dStart(source, document) {
  return firstFocusedPuzzleLevel2dEntry(source, document)?.start ?? null;
}

function firstFocusedPuzzleLevel3dEntry(source) {
  return focusedPuzzleLevel3dEntries(source)[0] || null;
}

function focusedPuzzleLevel3dEntries(source) {
  return uniqueFocusedPuzzleEntries(
    focusedPuzzleSurfaceEntriesByKind("level", { document: activeDocument(), source }, "3d")
  );
}

function firstFocusedPuzzleLevel3dStart(source) {
  return firstFocusedPuzzleLevel3dEntry(source)?.start ?? null;
}

function firstFocusedPuzzleVisual2dEntry(source) {
  return focusedPuzzleVisual2dEntries(source)[0] || null;
}

function focusedPuzzleVisual2dEntries(source) {
  return uniqueFocusedPuzzleEntries(
    focusedPuzzleSurfaceEntriesByKind("visual", { document: activeDocument(), source }, "2d")
  );
}

function firstFocusedPuzzleVisual2dStart(source) {
  return firstFocusedPuzzleVisual2dEntry(source)?.start ?? null;
}

function firstFocusedPuzzleVisual3dEntry(source) {
  return focusedPuzzleVisual3dEntries(source)[0] || null;
}

function focusedPuzzleVisual3dEntries(source) {
  return uniqueFocusedPuzzleEntries(
    focusedPuzzleSurfaceEntriesByKind("visual", { document: activeDocument(), source }, "3d")
  );
}

function firstFocusedPuzzleVisual3dStart(source) {
  return firstFocusedPuzzleVisual3dEntry(source)?.start ?? null;
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
  const entries = focusedPuzzleEntriesForDimension("level", "2d", context);
  if (!entries.length) {
    return null;
  }
  const levelIndex = currentEditableLevelIndex();
  const levelName = levelEditorLevels(currentPreviewExportData())[levelIndex]?.name || "";
  const selected = (
    levelName
      ? entries.find((entry) => sourceTitleMatches(entry.target?.name, levelName))
      : null
  ) || entries[levelIndex] || entries[0];
  if (!selected?.target) {
    return null;
  }
  return {
    dimension: "2d",
    target: {
      ...selected.target,
      document: context.document,
      name: levelName || selected.target.name || "",
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
  return loadFocusedLevelPaneEntry(entry, options);
}

function loadFirstLevelPaneEntry(context = focusedPuzzleSourceContext(), options = {}) {
  return loadFocusedLevelPaneEntry(firstFocusedPuzzleEntry("level", context), options);
}

function loadAvailableLevelPaneEntry(context = focusedPuzzleSourceContext(), options = {}) {
  const requestedMode = ["edit", "level3d"].includes(options.mode) ? options.mode : currentLevelPaneMode;
  return loadLevelPaneEntryForMode(requestedMode, context, options)
    || loadFirstLevelPaneEntry(context, options);
}

function loadFocusedLevelPaneEntry(entry, options = {}) {
  if (!entry) {
    return false;
  }
  const targetMode = levelModeForEditorDimension(entry.dimension);
  if (options.openPane !== false) {
    openPreviewModePane(targetMode);
  }
  return loadFocusedPuzzleEntry("level", entry, {
    silent: options.silent !== false,
    recordHistory: Boolean(options.recordHistory),
    openPane: options.openPane !== false,
  });
}

function loadFocusedPuzzleEntry(kind, entry, options = {}) {
  if (!entry?.target || entry.target.document?.id !== activeDocument()?.id) {
    return false;
  }
  const dimension = normalizeEditorDimension(entry.dimension);
  const target = entry.target;
  const mode = kind === "visual"
    ? visualModeForEditorDimension(dimension)
    : levelModeForEditorDimension(dimension);
  if ((mode === "edit" || mode === "level3d") && kind !== "level") {
    return false;
  }
  if ((mode === "visual" || mode === "visual3d") && kind !== "visual") {
    return false;
  }
  if (mode === "edit" && dimension === "2d") {
    currentLevelPaneMode = "edit";
    return finishFocusedPuzzleEntryLoad(loadLevelSourceTarget(target, {
      silent: options.silent !== false,
      recordHistory: Boolean(options.recordHistory),
      openPane: options.openPane !== false,
    }));
  }
  if (mode === "level3d" && dimension === "3d" && typeof loadLevel3dSourceTarget === "function") {
    currentLevelPaneMode = "level3d";
    return finishFocusedPuzzleEntryLoad(loadLevel3dSourceTarget(target, {
      silent: options.silent !== false,
      recordHistory: Boolean(options.recordHistory),
      switchMode: options.openPane !== false,
    }));
  }
  if (mode === "visual" && dimension === "2d" && typeof loadVisualSourceTarget === "function") {
    currentVisualPaneMode = "visual";
    return finishFocusedPuzzleEntryLoad(loadFocusedVisualPuzzleEntry(entry, options));
  }
  if (mode === "visual3d" && dimension === "3d" && typeof loadVisual3dSourceTarget === "function") {
    currentVisualPaneMode = "visual3d";
    return finishFocusedPuzzleEntryLoad(loadFocusedVisualPuzzleEntry(entry, options));
  }
  return false;
}

async function loadFocusedVisualPuzzleEntry(entry, options = {}) {
  const document = entry?.target?.document;
  const source = puzzleTextDocumentSource(document);
  const documentId = document?.id || "";
  const resolved = await resolveSourceTargetFromWasm(source, entry?.target?.start);
  const current = focusedPuzzleSourceContext();
  if (
    !current?.document
    || (current.document.id || "") !== documentId
    || current.source !== source
  ) {
    throw new Error("Visual source changed while its editing contract was being resolved.");
  }
  if (!sourceTargetMatches(resolved, "visual", entry.dimension) || !resolved.sourceVisual) {
    throw new Error("Resolved visual source contract is unavailable.");
  }
  return loadResolvedSourceTarget({
    ...resolved,
    document: current.document,
  }, {
    silent: options.silent !== false,
    recordHistory: Boolean(options.recordHistory),
    switchMode: true,
  });
}

function finishFocusedPuzzleEntryLoad(result) {
  if (result && typeof result.then === "function") {
    return result.then((loaded) => finishFocusedPuzzleEntryLoad(loaded));
  }
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

function puzzleTextDocumentSource(document = focusedPuzzleTextDocument()) {
  return document?.id === activeDocument()?.id
    ? sourceEditorDocumentValue()
    : document?.source || "";
}

function applyPuzzleSourceChange(document, source) {
  document.source = source;
  if (document.id === activeDocument()?.id) {
    setSourceEditorValue(source, { resetUndo: false });
  }
  scheduleLocalSave();
  schedulePreview();
}

function applyPuzzleSourceMutation(document, requestedSource, nextSource) {
  if (puzzleTextDocumentSource(document) !== requestedSource) {
    return false;
  }
  applyPuzzleSourceChange(document, nextSource);
  return true;
}

function defaultEmptyLevel2dSourceData() {
  const rows = Array.from({ length: 5 }, () => ".....");
  return { rows, localLegends: [] };
}

async function levelSourceRequest(source, request) {
  const runtime = window.PuzzleStudioRuntime;
  if (typeof runtime?.levelSourceRequest !== "function") {
    throw new Error("Rust level source editing is unavailable.");
  }
  return runtime.levelSourceRequest(source, request);
}

function performLevelSourceAction(options) {
  return levelShared.performSourceAction({
    ...options,
    executeRequest: levelSourceRequest,
    applyMutation: applyPuzzleSourceMutation,
  });
}

async function insertLevelWithDefaultBlock(source, name, levelData, namespace = "") {
  return levelSourceRequest(source, {
    operation: "insert",
    name,
    namespace: sanitizeLevelNamespace(namespace),
    rows: levelData?.rows || [],
    localLegends: levelSourceLegendDrafts(levelData?.localLegends),
    cursor: sourceEditor.selection().from,
    createContainer: true,
  });
}

async function addEmptyLevel2dToFocusedSource() {
  const document = focusedPuzzleTextDocument();
  if (!document) {
    setStatus("No puzzle source for level", "is-error");
    return false;
  }
  const name = "";
  const sourceData = defaultEmptyLevel2dSourceData();
  const source = puzzleTextDocumentSource(document);
  try {
    const result = await insertLevelWithDefaultBlock(source, name, sourceData, "");
    if (!applyPuzzleSourceMutation(document, source, result.source)) {
      setStatus("Level source changed while the edit was being prepared; retry the edit.", "is-error");
      return false;
    }
  } catch (error) {
    setStatus(`Could not add 2D level: ${error?.message || error}`, "is-error");
    return false;
  }
  currentLevelPaneMode = "edit";
  openPreviewModePane("edit");
  applyPaneVisibility();
  setPaneStatus("level", "Added 2D level; waiting for Rust source analysis.", "");
  hideEditorHoverTooltip();
  return true;
}

function defaultEmptyLevel3dSourceData() {
  return { rows: [LEVEL3D_EMPTY_CHAR], unknownCells: 0 };
}

async function addEmptyLevel3dToFocusedSource() {
  const document = focusedPuzzleTextDocument();
  if (!document) {
    setPaneStatus("level", "No puzzle source for 3D level", "is-error");
    return false;
  }
  const name = "level 1";
  const bundle = "levels";
  const sourceData = defaultEmptyLevel3dSourceData();
  const source = puzzleTextDocumentSource(document);
  try {
    const result = await levelSourceRequest(source, {
      operation: "insert",
      name,
      namespace: bundle,
      rows: sourceData.rows,
      localLegends: [],
      cursor: sourceEditor.selection().from,
      createContainer: true,
    });
    if (!applyPuzzleSourceMutation(document, source, result.source)) {
      setPaneStatus("level", "3D level source changed while the edit was being prepared; retry the edit.", "is-error");
      return false;
    }
  } catch (error) {
    setPaneStatus("level", `Could not add 3D level: ${error?.message || error}`, "is-error");
    return false;
  }
  currentLevelPaneMode = "level3d";
  openPreviewModePane("level3d");
  applyPaneVisibility();
  setPaneStatus("level", "Added 3D level; waiting for Rust source analysis.", "");
  hideEditorHoverTooltip();
  return true;
}

async function openLevelPaneForCurrentPreviewLevel() {
  const build = previewBuild;
  if (!build || previewSession?.buildId !== build.id) {
    setPaneStatus("level", "No compiled preview level", "is-error");
    return false;
  }
  const modelName = String(previewSessionState()?.activeModel || "").trim();
  const exportData = await loadPreviewSourceProjection(modelName);
  if (!levelEditorLevels(exportData).length) {
    setPaneStatus("level", "No source level for the active preview model", "is-error");
    return false;
  }
  const levelIndex = currentPreviewRuntimeLevelIndex(exportData);
  if (levelIndex === null) {
    requestFocusedPreviewState();
    setPaneStatus("level", "Current preview level is not ready", "is-error");
    return false;
  }
  const targetMode = isPuzzle3dExport(exportData) ? "level3d" : "edit";
  currentEditorDimension = editorDimensionForPreviewMode(targetMode);
  currentLevelPaneMode = targetMode;
  setActiveLevelIndex(levelIndex, exportData);
  const target = targetMode === "level3d"
    ? currentLevel3dSourceLocationForIndex(levelIndex, exportData, { build })
    : currentLevelSourceLocation({ build, exportData, levelIndex });
  if (!target) {
    openPreviewModePane(targetMode);
    setPaneStatus("level", `No source for preview level ${levelIndex + 1}`, "is-error");
    applyPaneVisibility();
    hideEditorHoverTooltip();
    return false;
  }
  if (currentSourceForDocument(target.document) !== target.sourceSnapshot) {
    openPreviewModePane(targetMode);
    setPaneStatus("level", "Preview source changed. Run Preview before editing this level.", "is-error");
    applyPaneVisibility();
    hideEditorHoverTooltip();
    return false;
  }
  if (target.document?.id !== activeDocument()?.id) {
    if (!revealSourceLocation(target, { recordHistory: false, revealPane: true })) {
      setPaneStatus("level", `Could not open source for preview level ${levelIndex + 1}`, "is-error");
      return false;
    }
  }
  const loaded = await loadResolvedSourceTarget({
    ...target,
    kind: "level",
    dimension: targetMode === "level3d" ? "3d" : "2d",
  }, {
    silent: true,
    recordHistory: false,
    document: target.document,
  });
  if (!loaded) {
    openPreviewModePane(targetMode);
    setPaneStatus("level", `Could not load preview level ${levelIndex + 1}`, "is-error");
    applyPaneVisibility();
    hideEditorHoverTooltip();
    return false;
  }
  applyPaneVisibility();
  hideEditorHoverTooltip();
  return true;
}

async function openLevelPaneForCurrentDimension(options = {}) {
  const requestedMode = ["edit", "level3d"].includes(options.mode)
    ? options.mode
    : levelModeForEditorDimension(currentEditorDimension);
  openPreviewModePane(requestedMode);
  setPaneStatus("level", "Level editing is waiting for source analysis.", "");
  const context = await focusedPuzzleSourceContextWithEntries();
  if (!context) {
    return false;
  }
  const loaded = await loadAvailableLevelPaneEntry(context, {
    mode: requestedMode,
    silent: true,
    recordHistory: false,
  });
  if (!loaded) {
    currentLevelPaneMode = requestedMode;
    if (requestedMode === "edit") {
      resetLevelBuilderFromSource(true);
    } else if (typeof renderLevel3dBuilder === "function") {
      renderLevel3dBuilder();
    }
    setPaneStatus("level", "No level in active source", "is-error");
    applyPaneVisibility();
    hideEditorHoverTooltip();
  }
  return Boolean(loaded);
}

async function openVisualPaneForCurrentDimension() {
  const pendingMode = visualModeForEditorDimension(currentEditorDimension);
  openPreviewModePane(pendingMode);
  setPaneStatus("visual", "Visual editing is waiting for source analysis.", "");
  const context = await focusedPuzzleSourceContextWithEntries();
  if (!context) {
    return null;
  }
  const first = firstFocusedPuzzleEntry("visual", context);
  const mode = first
    ? visualModeForEditorDimension(first.dimension)
    : visualModeForEditorDimension(currentEditorDimension);
  openPreviewModePane(mode);
  if (first) {
    await loadFocusedPuzzleEntry("visual", first, { silent: true, recordHistory: false });
  }
  return mode;
}

function editorDimensionForPreviewMode(mode) {
  if (mode === "level3d" || mode === "visual3d") {
    return "3d";
  }
  if (mode === "edit" || mode === "visual") {
    return "2d";
  }
  return currentEditorDimension;
}

function setEditorDimensionMode(dimension) {
  currentEditorDimension = normalizeEditorDimension(dimension);
  currentLevelPaneMode = levelModeForEditorDimension(currentEditorDimension);
  currentVisualPaneMode = visualModeForEditorDimension(currentEditorDimension);

  if (currentPreviewMode === "edit" || currentPreviewMode === "level3d") {
    setPreviewMode(currentLevelPaneMode);
    return currentLevelPaneMode;
  }
  if (currentPreviewMode === "visual" || currentPreviewMode === "visual3d") {
    setPreviewMode(currentVisualPaneMode);
    return currentVisualPaneMode;
  }
  applyPaneVisibility();
  if (isPaneVisible("level")) {
    if (currentLevelPaneMode === "level3d") {
      renderLevel3dBuilder();
    } else {
      renderLevelBoard();
    }
  }
  if (isPaneVisible("visual")) {
    if (currentVisualPaneMode === "visual3d") {
      renderVisual3dBuilder();
    } else {
      renderVisualBuilder();
    }
  }
  return currentPreviewMode;
}

function levelPaneBindLabel() {
  if (currentLevelPaneMode === "edit") {
    return "2D";
  }
  if (currentLevelPaneMode === "level3d") {
    return "3D";
  }
  return "none";
}

function syncPaneBindLabels() {
  if (levelPaneModeSwitch) {
    levelPaneModeSwitch.textContent = levelPaneBindLabel();
  }
}

function syncPreviewModeButtonState() {
  const previewMode = normalizePreviewMode(currentPreviewMode);
  const paneVisible = isPaneVisible(workPaneIdForPreviewMode(previewMode));
  const visualPaneVisible = isPaneVisible("visual");
  const dimensionLabel = editorDimensionLabel();
  playModeButton.classList.toggle("is-active", paneVisible && previewMode === "play");
  editModeButton.classList.toggle("is-active", isPaneVisible("level"));
  solverModeButton.classList.toggle("is-active", paneVisible && previewMode === "solver");
  visualModeButton.classList.toggle("is-active", visualPaneVisible && !visual.animationMode);
  visual3dModeButton?.classList.toggle("is-active", visualPaneVisible && currentVisualPaneMode === "visual3d");
  editModeButton.title = `Open ${dimensionLabel} level editor`;
  editModeButton.setAttribute("aria-label", `Open ${dimensionLabel} level editor`);
  visualModeButton.title = `Open ${dimensionLabel} visual editor`;
  visualModeButton.setAttribute("aria-label", `Open ${dimensionLabel} visual editor`);
  const visualAnimationActive = currentVisualPaneMode === "visual3d" ? Boolean(visual3d.animationMode) : Boolean(visual.animationMode);
  visualAnimateModeButton?.classList.toggle("is-active", visualPaneVisible && visualAnimationActive);
  visualAnimateModeButton?.setAttribute("aria-pressed", String(visualPaneVisible && visualAnimationActive));
  if (visualSourceActionBank) {
    visualSourceActionBank.hidden = !visualPaneVisible || currentVisualPaneMode !== "visual";
  }
  if (visual3dSourceActionBank) {
    visual3dSourceActionBank.hidden = !visualPaneVisible || currentVisualPaneMode !== "visual3d";
  }
  for (const button of visualDimensionButtons) {
    const active = normalizeEditorDimension(button.dataset.visualDimension) === currentEditorDimension;
    button.classList.toggle("is-active", active);
    button.setAttribute("aria-pressed", String(active));
  }
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
  for (const button of visualPaneModeButtons) {
    const active = visualPaneVisible && button.dataset.visualPaneMode === currentVisualPaneMode;
    button.classList.toggle("is-active", active);
    button.setAttribute("aria-pressed", String(active));
  }
  soundsTopbarButton.classList.toggle("is-active", paneVisible && previewMode === "sounds");
  psImportTopbarButton?.classList.toggle("is-active", paneVisible && previewMode === "psimport");
  docsTopbarButton?.classList.toggle("is-active", paneVisible && previewMode === "docs");
}

function setPreviewMode(mode, options = {}) {
  const wasVisualMode = currentPreviewMode === "visual";
  const previewMode = normalizePreviewMode(mode);
  if (currentPreviewMode === "solver" && previewMode !== "solver") {
    setEditorSolverDisplayedArtifact("");
  }
  hideEditorHoverTooltip();
  if (previewMode !== "edit" && levelPlaytestActive) {
    stopLevelPlaytest({ syncPreview: false });
  }
  if (previewMode !== "play" && !(previewMode === "edit" && levelPlaytestActive)) {
    stopEditorRuntimeController(previewEditorRuntimeController());
  }
  if (wasVisualMode && previewMode !== "visual" && visual) {
    visual.shapeTagPickerOpen = false;
    if (typeof stopVisualAnimationPlayback === "function") {
      stopVisualAnimationPlayback();
    }
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
  const visualMode = previewMode === "visual";
  const visual3dMode = previewMode === "visual3d";
  const soundsMode = previewMode === "sounds";
  const psImportMode = previewMode === "psimport";
  if (editMode || level3dMode) {
    currentEditorDimension = editorDimensionForPreviewMode(previewMode);
    currentLevelPaneMode = levelModeForEditorDimension(currentEditorDimension);
  } else if (visualMode || visual3dMode) {
    currentEditorDimension = editorDimensionForPreviewMode(previewMode);
    currentVisualPaneMode = visualModeForEditorDimension(currentEditorDimension);
  }
  if (levelPaneModeSwitch) {
    levelPaneModeSwitch.hidden = !isPaneVisible("level");
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
  syncSolverPaneModeControls();
  ensureLevel3dPaneFrameWidth();
  syncPreviewViewportGeometry();
  scheduleBoardScaleSync(3);
  if (!isPaneVisible("sounds")) {
    stopSoundPlayback();
  }
  if (editMode) {
    if (levelSolutionPreview) {
      clearSolutionPreview();
    }
    renderLevelBoard();
  }
  if (solverMode) {
    syncSolverLevelSelector();
    syncSolverTaskReadout();
    renderSolverBoard();
    updateSolutionControls();
  }
  if (visualMode) {
    renderVisualBuilder();
  }
  if (visual3dMode) {
    renderVisual3dBuilder();
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
  return postEditorSnapshotRequest();
}

function currentPreviewRuntimeLevelIndex(exportData = currentPreviewExportData()) {
  const state = previewSessionState();
  if (!state || state.screenHasPuzzle === false || !Number.isInteger(Number(state.levelIndex))) {
    return null;
  }
  const levelCount = Number.isInteger(Number(state.levelCount))
    ? Math.max(0, Math.trunc(Number(state.levelCount)))
    : levelEditorLevels(exportData).length;
  if (!levelCount) {
    return null;
  }
  return Math.max(0, Math.min(levelCount - 1, Math.trunc(Number(state.levelIndex))));
}

function restoreCompiledGamePreview(options = {}) {
  const runtime = previewBuild?.runtime;
  if (!runtime || !previewFrame) {
    return;
  }
  if (
    !options.force
    && !previewFrameHasEditorLevelState
    && previewFrameHasCurrentCompiledPreview
  ) {
    return;
  }
  previewFrameHasEditorLevelState = false;
  const session = ensurePreviewSession();
  session.state = null;
  session.runtimeStatus = null;
  setPreviewDocumentLoaded(false);
  setPreviewRuntime(runtime, { markDocumentLoaded: true });
  syncPreviewLevelActionButtons();
}

function activePreviewModeAcceptsLevelState() {
  return currentPreviewMode === "edit" && levelPlaytestActive;
}

function levelEditorAssistanceReady(
  exportData = currentLevelExportData(),
) {
  return Boolean(
    exportData?.manifest
    && exportData?.session
  );
}

function deferLevelEditorAssistance(exportData = currentLevelExportData()) {
  if (exportData) {
    setPaneStatus("level", "Level editing is waiting for source analysis.", "");
  }
  return false;
}

function resetLevelBuilderFromSource() {
  const exportData = currentLevelExportData();
  if (!levelEditorAssistanceReady(exportData)) {
    return deferLevelEditorAssistance(exportData);
  }
  clearLevelEditSource();
  levelDisplayCells = null;
  level.exportData = exportData;
  level.palette = levelPaletteFromExport(levelReferenceSource(exportData), exportData);
  level.activeLayer = normalizedLevelActiveLayer(level.activeLayer, exportData);
  const levelIndex = currentEditableLevelIndex(exportData);
  applyLevelSessionSnapshot(exportData.session.levelSnapshot(levelIndex), exportData);
  ensureLevelLayerMaps(exportData);
  if (!level.palette.some((entry) => entry.id === level.selectedObjectId)) {
    level.selectedObjectId = level.palette[0]?.id ?? 0;
  }
  updateLevelSizeLabel();
  renderLevelPalette();
  renderLevelBoard();
  return true;
}

function resetLevelBuilderFromPreviewSource() {
  return resetLevelBuilderFromSource();
}

function titleLabel(value) {
  return String(value || "Tile")
    .replace(/[:_-]+/g, " ")
    .replace(/\s+/g, " ")
    .trim()
    .replace(/\b\w/g, (letter) => letter.toUpperCase());
}

function levelPaletteFromExport(source, exportData = currentLevelExportData()) {
  const placeableObjects = sourcePlaceableObjectNames(source, exportData);
  const objects = engineObjects(exportData).filter((object) => placeableObjects.has(object.name));
  return [
    { id: 0, name: "Eraser", layer: null, visual: "eraser" },
    ...objects,
  ];
}

function levelReferenceSource(exportData = currentLevelExportData()) {
  return exportData?.source || activePreviewSource();
}

function currentPreviewExportData() {
  const buildId = typeof previewBuild?.id === "string" ? previewBuild.id : "";
  if (!buildId || !previewSourceProjection || previewSourceProjection.buildId !== buildId) {
    return null;
  }
  return previewSourceProjection.exportData || null;
}

function currentLevelExportData(exportData = null) {
  return exportData || level.exportData || currentPreviewExportData() || null;
}

function levelEditorManifest(exportData = currentLevelExportData()) {
  return exportData?.manifest || null;
}

function levelEditorLevels(exportData = currentLevelExportData()) {
  return levelEditorManifest(exportData)?.levels || [];
}

function levelEditorObjects(exportData = currentLevelExportData()) {
  return levelEditorManifest(exportData)?.objects || [];
}

function sourcePlaceableObjectNames(source, exportData = currentLevelExportData()) {
  return new Set(sourceCharEntries(source, exportData)
    .filter((entry) => entry.objects.length === 1)
    .map((entry) => entry.objects[0]));
}

function engineObjects(exportData = currentLevelExportData()) {
  return [...levelEditorObjects(exportData)]
    .sort((left, right) => left.layer - right.layer || left.name.localeCompare(right.name));
}

function engineObjectById(objectId, exportData = currentLevelExportData()) {
  return levelEditorObjects(exportData).find((object) => object.id === objectId) || null;
}

function layerCount(exportData = currentLevelExportData()) {
  const levels = levelEditorLevels(exportData);
  const levelIndex = levels.length ? currentEditableLevelIndex(exportData) : 0;
  return levels[levelIndex]?.layerCount || null;
}

function initialLevelSize(exportData = currentPreviewExportData()) {
  const manifestLevel = levelEditorLevels(exportData)[currentEditableLevelIndex(exportData)];
  if (manifestLevel) {
    return { width: manifestLevel.width, height: manifestLevel.height };
  }
  return { width: 9, height: 5 };
}

function currentEditableLevelIndex(exportData = currentPreviewExportData()) {
  return setActiveLevelIndex(activeLevelIndex, exportData);
}

function setActiveLevelIndex(index, exportData = currentPreviewExportData()) {
  const levels = levelEditorLevels(exportData);
  if (!levels.length) {
    activeLevelIndex = 0;
    return 0;
  }
  const rawIndex = index ?? 0;
  activeLevelIndex = Math.max(0, Math.min(levels.length - 1, Math.trunc(Number(rawIndex) || 0)));
  return activeLevelIndex;
}

function normalizedLevelIndex(index, exportData = currentPreviewExportData()) {
  const levels = levelEditorLevels(exportData);
  if (!levels.length) {
    return 0;
  }
  const rawIndex = index ?? 0;
  return Math.max(0, Math.min(levels.length - 1, Math.trunc(Number(rawIndex) || 0)));
}

function cloneJson(value) {
  return value == null ? null : JSON.parse(JSON.stringify(value));
}

function solverLevelDescriptor(levels, levelIndex) {
  const levelEntry = levels?.[levelIndex];
  if (!levelEntry) {
    return null;
  }
  return {
    index: levelIndex,
    levelName: levelEntry.name || "",
  };
}

function solverCompileId(solverBuild) {
  return solverBuild?.solverPrepared?.artifactId || "";
}

function clearSolverTask() {
  activeSolverTask = null;
  if (solverPaneMode === "custom-goal") {
    customGoalSolverTask = null;
  } else {
    levelGoalSolverTask = null;
  }
  syncSolverLevelSelector();
  syncSolverTaskReadout();
  setSolveLevelButtonState(Boolean(activeLevelSolveRequest));
}

function setActiveSolverTask(task) {
  activeSolverTask = task ? cloneJson(task) : null;
  if (solverPaneMode === "custom-goal") {
    customGoalSolverTask = activeSolverTask;
  } else {
    levelGoalSolverTask = activeSolverTask;
  }
  syncSolverLevelSelector();
  syncSolverTaskReadout();
  setSolveLevelButtonState(Boolean(activeLevelSolveRequest));
  return activeSolverTask;
}

function solverTaskBaseKey(task = activeSolverTask) {
  if (!task) {
    return "";
  }
  return JSON.stringify({
    producer: task.producer || "",
    rules: {
      compileId: task.rules?.compileId || "",
      documentId: task.rules?.documentId || "",
      modelName: task.rules?.modelName || "",
      modelKind: task.rules?.modelKind || "",
    },
    level: task.level || null,
    state: task.state || null,
  });
}

function customGoalConstraints(task = activeSolverTask) {
  const key = solverTaskBaseKey(task);
  return key ? customGoalConstraintsByTask.get(key) || [] : [];
}

function solverTaskRunKey(task = activeSolverTask) {
  const base = solverTaskBaseKey(task);
  if (!base) {
    return "";
  }
  return JSON.stringify({
    base,
    objective: solverPaneMode === "custom-goal"
      ? { kind: "custom", constraints: customGoalConstraints(task) }
      : { kind: "level_completion" },
  });
}

function isSolverTaskComplete(task = activeSolverTask) {
  const key = solverTaskRunKey(task);
  return Boolean(key && completedSolverTaskKey === key);
}

function markActiveSolverTaskComplete() {
  completedSolverTaskKey = solverTaskRunKey(activeSolverTask);
}

function solverTaskLevelIndex(task = activeSolverTask) {
  return Number.isInteger(task?.level?.index) ? task.level.index : null;
}

function solverTaskLevelLabel(task = activeSolverTask) {
  if (!task) {
    return "Choose a level";
  }
  if (task.producer === "level-editor") {
    return "Current board";
  }
  const level = task.level?.levelName || `Level ${(task.level?.index ?? 0) + 1}`;
  return level;
}

function syncSolverTaskReadout() {
  if (solverTargetName) {
    solverTargetName.textContent = solverTaskLevelLabel();
    solverTargetName.title = solverTargetName.textContent;
  }
}

function solverLevelOptionLabel(level, index) {
  return level?.name || `Level ${index + 1}`;
}

function syncSolverLevelSelector(exportData = currentPreviewExportData()) {
  if (!solverLevelSelect) {
    return;
  }
  const preparedLevels = preparedPreviewSolverBuild()?.solverPrepared?.levels;
  const levels = Array.isArray(preparedLevels)
    ? preparedLevels
    : levelEditorLevels(exportData);
  const nextSignature = levels
    .map((level, index) => `${index}:${solverLevelOptionLabel(level, index)}`)
    .join("\n");
  if (solverLevelSelect.dataset.levelSignature !== nextSignature) {
    const options = [];
    const placeholder = document.createElement("option");
    placeholder.value = "";
    placeholder.textContent = levels.length ? "Load" : "No level to solve";
    placeholder.disabled = true;
    options.push(placeholder);
    options.push(...levels.map((level, index) => {
      const option = document.createElement("option");
      option.value = String(index);
      option.textContent = solverLevelOptionLabel(level, index);
      return option;
    }));
    solverLevelSelect.replaceChildren(...options);
    solverLevelSelect.dataset.levelSignature = nextSignature;
  }
  solverLevelSelect.disabled = !levels.length || activeLevelSolveRequest !== null;
  solverLevelSelect.value = "";
}

function selectSolverLevel(index, solverBuild = preparedPreviewSolverBuild()) {
  const exportData = solverBuild?.exportData;
  if (activeLevelSolveRequest) {
    syncSolverLevelSelector(exportData);
    return false;
  }
  const preparedLevels = solverBuild?.solverPrepared?.levels;
  const levels = Array.isArray(preparedLevels)
    ? preparedLevels
    : levelEditorLevels(exportData);
  if (!levels.length) {
    clearSolverTask();
    renderSolverBoard();
    return false;
  }
  const levelIndex = Math.max(0, Math.min(levels.length - 1, Math.trunc(Number(index) || 0)));
  solverSelectedLevelIndex = levelIndex;
  const task = createPreviewSolverTask(solverBuild, levelIndex);
  if (!task) {
    clearSolverTask();
    renderSolverBoard();
    setLevelSolveStatus("No level to solve", "is-error");
    return false;
  }
  setActiveSolverTask(task);
  clearSolutionPreview({ preserveSolverTask: true });
  setLevelSolveStatus("");
  renderSolverBoard();
  return true;
}

function createSolverTask({ producer, solverBuild, exportData, levelIndex, stateKind, lifecycle, stateData, scene = null } = {}) {
  const prepared = solverBuild?.solverPrepared;
  const preparedLevels = Array.isArray(prepared?.levels) ? prepared.levels : [];
  const sourceLevels = producer === "preview-level" ? preparedLevels : levelEditorLevels(exportData);
  const targetIndex = sourceLevels?.length
    ? Math.max(0, Math.min(sourceLevels.length - 1, Math.trunc(Number(levelIndex) || 0)))
    : 0;
  const levelInfo = solverLevelDescriptor(sourceLevels, targetIndex);
  const modelKind = prepared?.modelKind;
  const exportModelKind = exportData ? (isPuzzle3dExport(exportData) ? "3d" : "2d") : modelKind;
  if (!levelInfo || !stateData || !prepared?.artifactId
    || (modelKind !== "2d" && modelKind !== "3d") || exportModelKind !== modelKind) {
    return null;
  }
  return {
    producer,
    rules: {
      compileId: solverCompileId(solverBuild),
      documentId: solverBuild.documentId || "",
      modelName: prepared.modelName || "",
      modelKind,
    },
    level: levelInfo,
    state: {
      kind: stateKind,
      lifecycle,
      data: cloneJson(stateData),
    },
    goalObjects: Array.isArray(prepared.objects)
      ? prepared.objects.map((object) => String(object?.name || "")).filter(Boolean)
      : [],
    scene: cloneJson(scene),
  };
}

function createPreviewSolverTask(build, levelIndex) {
  const prepared = build?.solverPrepared;
  const levels = Array.isArray(prepared?.levels) ? prepared.levels : [];
  const targetIndex = levels.length
    ? Math.max(0, Math.min(levels.length - 1, Math.trunc(Number(levelIndex) || 0)))
    : 0;
  return createSolverTask({
    producer: "preview-level",
    solverBuild: build,
    exportData: null,
    levelIndex: targetIndex,
    stateKind: "compiled-start",
    lifecycle: "playable-start",
    stateData: levels[targetIndex]?.initialState || null,
  });
}

function createEditorSolverTask({ solverBuild, exportData, levelIndex, stateData, scene = null } = {}) {
  return createSolverTask({
    producer: "level-editor",
    solverBuild,
    exportData,
    levelIndex,
    stateKind: "editor-staged",
    lifecycle: "playable-start",
    stateData,
    scene,
  });
}

function previewSolverTaskLevelIndex(exportData = currentPreviewExportData()) {
  const state = previewSessionState();
  const preparedLevels = preparedPreviewSolverBuild()?.solverPrepared?.levels;
  const levelCount = Array.isArray(preparedLevels)
    ? preparedLevels.length
    : levelEditorLevels(exportData).length;
  const normalize = (index) => levelCount
    ? Math.max(0, Math.min(levelCount - 1, Math.trunc(Number(index) || 0)))
    : 0;
  if (
    state
    && state.screenHasPuzzle !== false
    && Number.isInteger(Number(state.levelIndex))
  ) {
    return normalize(state.levelIndex);
  }
  if (Number.isInteger(solverSelectedLevelIndex)) {
    return normalize(solverSelectedLevelIndex);
  }
  return null;
}

function solverPreparedBuildKey(buildId, modelName) {
  return `${String(buildId || "")}\u0000${String(modelName || "").trim()}`;
}

function previewSolverModelName(build = previewBuild) {
  const projectedModel = build === previewBuild
    ? String(currentPreviewExportData()?.modelName || "").trim()
    : String(build?.exportData?.modelName || "").trim();
  if (projectedModel) {
    return projectedModel;
  }
  if (build !== previewBuild) {
    return "";
  }
  return String(previewSessionState()?.activeModel || solverSelectedPuzzleName).trim();
}

async function resolvePreviewSolverModelName(build = previewBuild) {
  const selectedModel = previewSolverModelName(build);
  if (selectedModel) {
    return selectedModel;
  }
  const context = await focusedPuzzleSourceContextWithEntries();
  if (!context?.document || context.document.id !== build?.documentId) {
    return "";
  }
  const modelNames = new Set(
    focusedPuzzleEntries("level", context)
      .map((entry) => String(entry.target?.params?.model || "").trim())
      .filter(Boolean),
  );
  return modelNames.size === 1 ? [...modelNames][0] : "";
}

function preparedPreviewSolverBuild(build = previewBuild) {
  const prepared = solverPreparedByBuildId.get(
    solverPreparedBuildKey(build?.id, previewSolverModelName(build)),
  );
  return build && prepared?.artifactId
    ? { ...build, exportData: currentPreviewExportData(), solverPrepared: prepared }
    : null;
}

function setPreviewSolverTaskFromActiveLevel(solverBuild = preparedPreviewSolverBuild()) {
  const exportData = solverBuild?.exportData;
  const levelIndex = previewSolverTaskLevelIndex(exportData);
  if (levelIndex === null) {
    clearSolverTask();
    return false;
  }
  const task = createPreviewSolverTask(solverBuild, levelIndex);
  if (!task) {
    clearSolverTask();
    return false;
  }
  setActiveSolverTask(task);
  clearSolutionPreview({ preserveSolverTask: true });
  setLevelSolveStatus("");
  return true;
}

function refreshVisiblePreviewSolverTask(solverBuild = preparedPreviewSolverBuild()) {
  if (currentPreviewMode !== "solver") {
    return false;
  }
  if (setPreviewSolverTaskFromActiveLevel(solverBuild)) {
    return true;
  }
  if (!levelSolveStatus?.textContent?.trim()) {
    setLevelSolveStatus("No level to solve", "is-error");
  }
  return false;
}

async function prepareSolverBuild(build, modelName, status = setLevelSolveStatus) {
  if (!build?.id || !Array.isArray(build.documents) || !build.documents.length) {
    status("No solver source snapshot", "is-error");
    return null;
  }
  const selectedModel = String(modelName || "").trim();
  if (!selectedModel) {
    status("No selected solver model", "is-error");
    return null;
  }
  const cacheKey = solverPreparedBuildKey(build.id, selectedModel);
  const cached = solverPreparedByBuildId.get(cacheKey);
  if (cached?.artifactId) {
    return { ...build, solverPrepared: cached };
  }
  status("Preparing solver", "");
  try {
    const prepared = await prepareEditorSolverArtifact({
      documents: compilerDocumentsForSnapshot(build.documents),
      puzzlePath: build.puzzlePath,
      modelName: selectedModel,
      documentId: build.documentId,
    });
    solverPreparedByBuildId.set(cacheKey, prepared);
    status("Solver ready", "");
    return { ...build, solverPrepared: prepared };
  } catch (error) {
    status(`Solver prepare failed: ${userFacingRuntimeError(error)}`, "is-error");
    return null;
  }
}

async function ensurePreviewSolverBuild() {
  while (currentPreviewMode === "solver") {
    const build = previewBuild;
    if (!build || previewSession?.buildId !== build.id) {
      setLevelSolveStatus("No active preview", "is-error");
      return null;
    }
    const modelName = await resolvePreviewSolverModelName(build);
    if (previewBuild?.id !== build.id) {
      setLevelSolveStatus("Preparing updated preview", "");
      continue;
    }
    solverSelectedPuzzleName = modelName;
    const exportData = await loadPreviewSourceProjection(modelName);
    const preparedBuild = await prepareSolverBuild({ ...build, exportData }, modelName);
    if (!preparedBuild) {
      return null;
    }
    if (previewBuild?.id === build.id) {
      return preparedBuild;
    }
    setLevelSolveStatus("Preparing updated preview", "");
  }
  return null;
}

async function prepareCurrentDraftSolverBuild() {
  const document = activePreviewDocument();
  if (!isPuzzleDocument(document)) {
    setLevelSolveStatus("No rule model for edited level", "is-error");
    return null;
  }
  const presentationManifest = await ensurePreviewDocumentsLoaded(document);
  const build = capturePreviewBuildInput(document, presentationManifest);
  return prepareSolverBuild(build, currentLevelExportData()?.modelName);
}

async function openSolverPaneForCurrentLevel() {
  openPreviewModePane("solver");
  solverSelectedPuzzleName = "";
  solverSelectedLevelIndex = null;
  const build = await ensurePreviewSolverBuild();
  if (!build) {
    clearSolverTask();
    renderSolverBoard();
    return false;
  }
  refreshVisiblePreviewSolverTask(build);
  requestFocusedPreviewState();
  renderSolverBoard();
  return Boolean(activeSolverTask);
}

async function solvePreviewPaneCurrentLevel() {
  if (activeLevelSolveRequest) {
    await solveLevel();
    return;
  }
  const ready = await openSolverPaneForCurrentLevel();
  if (!ready) {
    if (!levelSolveStatus?.textContent?.trim()) {
      setLevelSolveStatus("No level to solve", "is-error");
    }
    return;
  }
  await solveLevel();
}

async function loadLevelFromSourcePosition(position, options = {}) {
  if (!isPuzzleDocument(activeDocument()) || !isTextDocument(activeDocument())) {
    return null;
  }
  const source = sourceEditorDocumentValue();
  if (typeof resolveSourceTargetFromWasm !== "function") {
    return null;
  }
  const target = await resolveSourceTargetFromWasm(source, position);
  if (!sourceTargetMatches(target, "level", "2d")) {
    return null;
  }
  return loadLevelSourceTarget(target, options);
}

async function resolveSourceTargetFromWasm(source, position) {
  if (typeof window.PuzzleStudioRuntime?.resolveSourceTarget !== "function") {
    return null;
  }
  const raw = await window.PuzzleStudioRuntime.resolveSourceTarget(source, position);
  const payload = JSON.parse(raw || "{}");
  return normalizeResolvedSourceTarget(source, payload?.target || null, position, true);
}

function normalizeResolvedSourceTarget(source, target, position = null, utf16Offsets = false) {
  void position;
  if (!target || typeof target !== "object") {
    return null;
  }
  const normalized = { ...target };
  if (!utf16Offsets) {
    for (const key of ["start", "end", "bodyStart", "bodyEnd"]) {
      if (Number.isInteger(normalized[key])) {
        normalized[key] = sourceUtf16OffsetFromByteOffset(source, normalized[key]);
      }
    }
  }
  return normalized;
}

function normalizeResolvedSourceTargets(source, targets) {
  if (!Array.isArray(targets)) {
    return [];
  }
  const keys = ["start", "end", "bodyStart", "bodyEnd"];
  const byteOffsets = new Set();
  for (const target of targets) {
    for (const key of keys) {
      if (Number.isInteger(target?.[key])) {
        byteOffsets.add(target[key]);
      }
    }
  }
  const pending = Array.from(byteOffsets).sort((left, right) => left - right);
  const utf16ByByte = new Map();
  let pendingIndex = 0;
  let byteOffset = 0;
  let utf16Offset = 0;
  while (pendingIndex < pending.length) {
    const targetByte = pending[pendingIndex];
    while (byteOffset < targetByte && utf16Offset < source.length) {
      const codePoint = source.codePointAt(utf16Offset);
      byteOffset += codePoint <= 0x7f ? 1 : codePoint <= 0x7ff ? 2 : codePoint <= 0xffff ? 3 : 4;
      utf16Offset += codePoint > 0xffff ? 2 : 1;
    }
    utf16ByByte.set(targetByte, utf16Offset);
    pendingIndex += 1;
  }
  return targets.map((target) => {
    if (!target || typeof target !== "object") {
      return null;
    }
    const normalized = { ...target };
    for (const key of keys) {
      if (Number.isInteger(normalized[key])) {
        normalized[key] = utf16ByByte.get(normalized[key]);
      }
    }
    return normalized;
  }).filter(Boolean);
}

async function loadLevelSourceTarget(target, options = {}) {
  if (!isPuzzleDocument(activeDocument()) || !isTextDocument(activeDocument())) {
    return null;
  }
  const document = activeDocument();
  const source = sourceEditorDocumentValue();
  const sourceEntry = sourceEditableEntryFromTarget(source, target, { defaultName: "" });
  return loadLevelSourceEntry(source, sourceEntry, { ...options, document });
}

async function loadLevelSourceEntry(source, entry, options = {}) {
  const document = options.document || activeDocument();
  if (!isPuzzleDocument(document) || !isTextDocument(document)) {
    reportLevelSourceLoadFailure("No level source document is active.", options);
    return null;
  }
  const loadOptions = { ...options, document };
  if (options.recordHistory) {
    pushSourceNavigationHistory();
  }
  if (options.openPane !== false) {
    openPreviewModePane("edit");
  }
  const modelName = String(entry?.params?.model || "").trim();
  if (!modelName) {
    reportLevelSourceLoadFailure("Level source target is missing its compiled model identity.", options);
    return null;
  }
  let exportData = null;
  try {
    exportData = await levelEditorSourceExportData(source, modelName);
  } catch (error) {
    reportLevelSourceLoadFailure(
      `Could not load level editor source contract: ${userFacingRuntimeError(error)}`,
      options,
    );
    return null;
  }
  return loadLevelSourceEntryWithExportData(source, entry, exportData, loadOptions);
}

async function levelEditorSourceExportData(source, modelName) {
  if (typeof window.PuzzleStudioRuntime?.levelEditorSourceSession !== "function") {
    throw new Error("Editor WASM function is missing: levelEditorSourceSession");
  }
  const requestedModel = String(modelName || "").trim();
  if (!requestedModel) {
    throw new Error("Level editor source contract requires an explicit model identity");
  }
  const session = await window.PuzzleStudioRuntime.levelEditorSourceSession(source, requestedModel);
  const contract = session.manifest();
  if (contract?.version !== 3 || contract?.kind !== "puzzle-level-editor") {
    throw new Error(`Unsupported level editor source contract version: ${contract?.version ?? "missing"}`);
  }
  if (contract.dimension !== "2d" && contract.dimension !== "3d") {
    throw new Error("Level editor source contract has an invalid dimension");
  }
  if (contract.dimension === "3d" && (!contract.camera || typeof contract.camera !== "object")) {
    throw new Error("3D level editor source contract is missing its typed camera");
  }
  if (String(contract.model || "") !== requestedModel) {
    throw new Error(`Level editor source contract returned model ${JSON.stringify(contract.model)} for ${JSON.stringify(requestedModel)}`);
  }
  if (!Array.isArray(contract.objects)) {
    throw new Error("Level editor source contract has invalid objects");
  }
  if (contract.objects.some((object) => (
    typeof object?.name !== "string"
    || !object.name
    || !Number.isSafeInteger(object.id)
    || object.id <= 0
    || !Number.isSafeInteger(object.layer)
    || object.layer < 0
  )) || new Set(contract.objects.map((object) => object.id)).size !== contract.objects.length) {
    throw new Error("Level editor source contract contains invalid object identities");
  }
  if (!Array.isArray(contract.levels) || contract.levels.some((level) => (
    typeof level?.name !== "string"
    || !Number.isSafeInteger(level.levelIndex)
    || !Number.isSafeInteger(level.sourceLevelIndex)
    || !Number.isSafeInteger(level.width)
    || level.width <= 0
    || !Number.isSafeInteger(level.depth)
    || level.depth <= 0
    || !Number.isSafeInteger(level.height)
    || level.height <= 0
    || !Number.isSafeInteger(level.layerCount)
    || level.layerCount <= 0
    || !Number.isSafeInteger(level.authoredLayerCount)
    || level.authoredLayerCount < 0
    || !Array.isArray(level.regions)
    || !Array.isArray(level.legend)
  ))) {
    throw new Error("Level editor source contract contains invalid levels");
  }
  if (!Array.isArray(contract.initialVariables)
    || contract.initialVariables.some((value) => !Number.isSafeInteger(value))) {
    throw new Error("Level editor source contract contains invalid initial variables");
  }
  const visuals = applyLevelEditorContractVisuals(session, contract.objects, contract.visualOrder);
  const authoringModelProjection = session.authoringModelProjection();
  return {
    modelName: requestedModel,
    source,
    manifest: contract,
    session,
    authoringModelProjection,
    visuals,
  };
}

function applyLevelEditorContractVisuals(session, objects, visualOrder) {
  const aliases = {};
  const entries = {};
  const rendererVisuals = {};
  for (const object of objects) {
    const payload = session.visual(object.id);
    if (!payload) {
      continue;
    }
    const visualName = `object:${object.id}`;
    aliases[object.name] = visualName;
    entries[visualName] = payload;
    rendererVisuals[object.name] = payload;
  }
  window.GameVisuals = {
    aliases: { ...aliases },
    entries: { ...entries },
    order: {
      direction_priority: [...(visualOrder?.direction_priority || [])],
      priorities: [...(visualOrder?.priorities || [])],
    },
  };
  return rendererVisuals;
}

function loadLevelSourceEntryWithExportData(source, entry, exportData, options = {}) {
  const levels = levelEditorLevels(exportData);
  let levelIndex = levels.length
    ? previewLevelIndexForSourceEntry(entry, exportData)
    : Math.max(0, Math.trunc(Number(options.levelIndex) || 0));
  if (levels.length && !levels[levelIndex]) {
    reportLevelSourceLoadFailure("No matching compiled level to edit", options);
    return null;
  }
  if (levels.length) {
    levelIndex = setActiveLevelIndex(levelIndex, exportData);
  } else {
    activeLevelIndex = levelIndex;
  }
  const levelName = levels[levelIndex]?.name || entry?.name || `level_${levelIndex + 1}`;
  if (!loadLevelFromSourceEntry(source, entry, { ...options, exportData, levelIndex, levelName })) {
    reportLevelSourceLoadFailure(`Could not load level ${levelName}`, options);
    return null;
  }
  setLevelEditSource(entry, options.document || activeDocument());
  setLevelNameInputs(editableLevelNameForSourceEntry(entry, levelName));
  const integrationDiagnostics = Array.isArray(exportData.manifest?.diagnostics)
    ? exportData.manifest.diagnostics.filter((diagnostic) => typeof diagnostic === "string" && diagnostic)
    : [];
  if (integrationDiagnostics.length) {
    const message = `Level editor loaded with source diagnostics: ${integrationDiagnostics[0]}`;
    setPaneStatus("level", message, "is-error");
    if (!options.silent) {
      setStatus(message, "is-error");
    }
  } else {
    setPaneStatus("level", "", "");
    if (!options.silent) {
      setStatus(`Loaded level ${levelName}`, "is-ok");
    }
  }
  return `level:${levelIndex}:${levelName}`;
}

function reportLevelSourceLoadFailure(message, options = {}) {
  setPaneStatus("level", message, "is-error");
  if (!options.silent) {
    setStatus(message, "is-error");
  }
}

function loadLevelFromSourceEntry(source, entry, options = {}) {
  const exportData = options.exportData || currentLevelExportData();
  const referenceSource = levelReferenceSource(exportData);
  const state = sourceLevelStateFromEntry(source, entry, exportData, { ...options, referenceSource });
  if (!state) {
    return false;
  }
  clearSolutionPreview();
  stopLevelPlaytest({ syncPreview: false });
  levelDisplayCells = null;
  applyLevelSessionSnapshot(state, exportData);
  level.exportData = exportData;
  level.palette = levelPaletteFromExport(referenceSource, exportData);
  level.activeLayer = normalizedLevelActiveLayer(level.activeLayer, exportData);
  if (!level.palette.some((entry) => entry.id === level.selectedObjectId)) {
    level.selectedObjectId = level.palette[0]?.id ?? 0;
  }
  renderLevelPalette();
  renderLevelBoard();
  if (levelPlaytestActive && !previewBuildIsStale && exportData === currentPreviewExportData()) {
    sendLevelDraftToPreview(options.levelIndex ?? currentEditableLevelIndex(exportData));
  }
  return true;
}

function sourceLevelStateFromEntry(_source, entry, exportData = currentLevelExportData(), _options = {}) {
  if (!entry || !levelEditorObjects(exportData).length) {
    return null;
  }
  if (!exportData.manifest) {
    throw new Error("Compiled level editor source contract is unavailable.");
  }
  const levelIndex = previewLevelIndexForSourceEntry(entry, exportData);
  const session = exportData.session;
  if (!levelEditorLevels(exportData)[levelIndex] || !session) {
    return null;
  }
  return session.levelSnapshot(levelIndex);
}

function levelObjectIdsToSlots(objectIds, exportData = currentLevelExportData()) {
  const slots = makeEmptyCell(exportData);
  for (const rawId of objectIds || []) {
    const object = engineObjectById(Number(rawId), exportData);
    if (!object || !Number.isInteger(object.layer) || object.layer < 0 || object.layer >= slots.length) {
      throw new Error(`Rust level snapshot contains invalid object identity ${rawId}`);
    }
    slots[object.layer] = object.id;
  }
  return slots;
}

function applyLevelSessionSnapshot(snapshot, exportData = currentLevelExportData()) {
  if (snapshot?.dimension !== "2d") {
    throw new Error("2D level editor received a non-2D Rust snapshot");
  }
  const width = Math.max(1, Math.trunc(Number(snapshot.width) || 0));
  const height = Math.max(1, Math.trunc(Number(snapshot.height) || 0));
  const cellCount = width * height;
  if (!Array.isArray(snapshot.cells) || snapshot.cells.length !== cellCount) {
    throw new Error("Rust level snapshot cell count does not match its dimensions");
  }
  if (!Array.isArray(snapshot.layers) || !snapshot.layers.length
    || snapshot.layers.some((layerCells) => !Array.isArray(layerCells) || layerCells.length !== cellCount)) {
    throw new Error("Rust level snapshot authored layers do not match its dimensions");
  }
  level.width = width;
  level.height = height;
  level.regions = normalizedLevelRegions(snapshot.regions, width, height);
  level.layers = snapshot.layers.map((layerCells) => (
    layerCells.map((objectIds) => levelObjectIdsToSlots(objectIds, exportData))
  ));
  level.cells = snapshot.cells.map((objectIds) => levelObjectIdsToSlots(objectIds, exportData));
  level.activeLayer = Math.max(0, Math.min(level.layers.length - 1, Math.trunc(Number(snapshot.activeLayer) || 0)));
  level.hasLocalDraft = snapshot.hasLocalDraft === true;
}

async function dispatchLevelSessionCommand(command, options = {}) {
  const exportData = options.exportData || currentLevelExportData();
  const session = exportData?.session;
  const levelIndex = options.levelIndex ?? currentEditableLevelIndex(exportData);
  if (!session || typeof session.dispatchLevel !== "function") {
    setPaneStatus("level", "Rust level edit session is unavailable.", "is-error");
    return null;
  }
  const sourceBefore = activeLevelEditSource();
  let transition;
  try {
    transition = await session.dispatchLevel(levelIndex, command);
    applyLevelSessionSnapshot(transition.snapshot, exportData);
    if (transition.sourceUpdate) {
      const document = activeLevelEditDocument();
      if (!document || !applyPuzzleSourceMutation(document, sourceBefore, transition.sourceUpdate.source)) {
        throw new Error("Level source changed while the Rust edit was being applied.");
      }
      setLevelEditSource({
        start: transition.sourceUpdate.start,
        end: transition.sourceUpdate.end,
        name: levelNameInput?.value || level.editSourceName,
      }, document);
    }
  } catch (error) {
    setPaneStatus("level", `Level edit failed: ${userFacingRuntimeError(error)}`, "is-error");
    return null;
  }
  clearSolutionPreview();
  levelDisplayCells = null;
  setLevelSolveStatus("");
  if (options.render !== false) {
    renderLevelPalette();
    renderLevelBoard();
  }
  scheduleLocalSave();
  return transition;
}

function previewLevelIndexForSourceEntry(entry, exportData = currentPreviewExportData()) {
  const levels = levelEditorLevels(exportData);
  const requestedName = String(entry?.name || "").trim();
  const rawIndex = Number.isInteger(entry?.levelIndex) ? entry.levelIndex : -1;
  const sourceIndexed = levels.findIndex((level) => Number(level?.sourceLevelIndex) === rawIndex);
  if (sourceIndexed >= 0 && (!requestedName || sourceTitleMatches(requestedName, levels[sourceIndexed].name))) {
    return sourceIndexed;
  }
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
  if (sourceTargetMatches(target, "level", "3d") && typeof loadLevel3dSourceTarget === "function") {
    return loadLevel3dSourceTarget(target, options);
  }
  if (sourceTargetMatches(target, "level", "2d")) {
    return loadLevelSourceTarget(target, options);
  }
  if (sourceTargetMatches(target, "visual", "2d") && typeof loadVisualSourceTarget === "function") {
    return loadVisualSourceTarget(target, options);
  }
  if (sourceTargetMatches(target, "visual", "3d") && typeof loadVisual3dSourceTarget === "function") {
    return loadVisual3dSourceTarget(target, options);
  }
  if (target.kind === "sounds" && typeof loadSoundSourceTarget === "function") {
    return loadSoundSourceTarget(target, options);
  }
  return null;
}

function previewModeForSourceTarget(target) {
  if (target?.kind === "sounds") return "sounds";
  if (target?.kind === "level") return levelModeForEditorDimension(target.dimension);
  if (target?.kind === "visual") return visualModeForEditorDimension(target.dimension);
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

let sourceCursorPreviewSyncTimer = 0;

// Caret-follow sync runs on every input and selectionchange. Debounce those
// high-frequency triggers even though Rust reuses the active source analysis.
function scheduleSourceCursorPreviewSync(options = {}) {
  window.clearTimeout(sourceCursorPreviewSyncTimer);
  sourceCursorPreviewSyncTimer = window.setTimeout(() => {
    sourceCursorPreviewSyncTimer = 0;
    syncPreviewModeFromSourceCursor(options);
  }, 150);
}

function syncPreviewModeFromSourceCursor(options = {}) {
  window.clearTimeout(sourceCursorPreviewSyncTimer);
  sourceCursorPreviewSyncTimer = 0;
  const document = activeDocument();
  sourceTargetRequestId += 1;
  const requestId = sourceTargetRequestId;
  if (!isPuzzleDocument(document) || !isTextDocument(document)) {
    sourceCursorPreviewKey = "";
    return false;
  }
  if (!options.allowInactiveMode && !["edit", "level3d", "visual", "visual3d", "sounds"].includes(currentPreviewMode)) {
    sourceCursorPreviewKey = "";
    return false;
  }
  const source = sourceEditorDocumentValue();
  const documentId = document.id || "";
  const position = Math.max(
    0,
    Math.min(source.length, Math.trunc(Number(
      options.position ?? sourceEditor.selection().from,
    ) || 0)),
  );
  const resolvedMode = previewModeForSourceTarget(sourceCursorResolveRegion);
  // The source structure (which block the caret sits in) only changes when the
  // text changes. While the text is unchanged and the caret is still inside the
  // last resolved target's range, the target is identical and the preview is
  // already in sync, so skip the revision-local target query. This covers
  // cursor navigation (arrows / click / selectionchange) without worker traffic.
  if (
    options.force !== true
    && sourceCursorResolveRegion
    && sourceCursorResolveRegion.source === source
    && position >= sourceCursorResolveRegion.start
    && position <= sourceCursorResolveRegion.end
    && currentPreviewMode === resolvedMode
    && isPaneVisible(workPaneIdForPreviewMode(resolvedMode))
  ) {
    return false;
  }
  // input + selectionchange + arrow keyup each fire this for the same edit.
  // Coalesce identical target queries against the worker's active revision.
  const activePaneSignature = `${currentPreviewMode}:${isPaneVisible(workPaneIdForPreviewMode(currentPreviewMode))}`;
  const resolveSignature = `${position}\u0000${activePaneSignature}\u0000${source}`;
  if (options.force !== true && resolveSignature === sourceCursorResolveSignature) {
    return false;
  }
  sourceCursorResolveSignature = resolveSignature;
  const loadOptions = {
    silent: true,
    switchMode: true,
    recordHistory: options.recordHistory === true,
  };
  resolveSourceTargetFromWasm(source, position)
    .then(async (target) => {
      if (
        requestId !== sourceTargetRequestId
        || documentId !== (activeDocument()?.id || "")
        || source !== sourceEditorDocumentValue()
      ) {
        return false;
      }
      sourceCursorResolveRegion = target && Number.isInteger(target.start) && Number.isInteger(target.end)
        ? { source, kind: target.kind, dimension: target.dimension, start: target.start, end: target.end }
        : null;
      const key = target ? await loadResolvedSourceTarget(target, loadOptions) || "" : "";
      return finishSourceTargetSync(key, options);
    })
    .catch((error) => {
      if (
        requestId !== sourceTargetRequestId
        || documentId !== (activeDocument()?.id || "")
        || source !== sourceEditorDocumentValue()
      ) {
        return false;
      }
      sourceCursorPreviewKey = "";
      setStatus(`Source target sync failed: ${userFacingRuntimeError(error)}`, "is-error");
      return false;
    });
  return false;
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
    return currentLevelSourceLocation({ sourceScope: "workspace" });
  }
  if (mode === "level3d") {
    return currentLevel3dSourceLocation();
  }
  if (mode === "visual") {
    return currentVisualSourceLocation();
  }
  if (mode === "visual3d") {
    return currentVisual3dSourceLocation();
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
  const source = sourceEditorDocumentValue();
  const sourceStart = Math.max(0, Math.min(source.length, target.start || 0));
  const start = sourceStart;
  sourceEditor.setSelection(start, start);
  scrollSourceOffsetIntoView(start, options.scrollAlignment);
  if (typeof updateSourceMeta === "function") {
    updateSourceMeta();
  }
  return true;
}

function sourceDocumentsForPreviewBuild(build) {
  if (!build?.documents?.length) {
    throw new Error("Preview build is missing its source snapshot.");
  }
  return build.documents.map((snapshot) => ({
    document: documents.find((document) =>
      document.id === snapshot.documentId
      || (
        normalizePath(document.puzzlePath || document.name) === normalizePath(snapshot.path)
        && normalizePath(document.workspaceRoot || workspaceRoot || "") === normalizePath(build.workspaceRoot || "")
      )
    ) || null,
    source: snapshot.source,
  })).filter((entry, index, entries) => (
    entry.document
    && entries.findIndex((candidate) => candidate.document?.id === entry.document.id) === index
  ));
}

function sourceDocumentsForLevelLocation(options) {
  if (options.build) {
    return sourceDocumentsForPreviewBuild(options.build);
  }
  if (options.sourceScope === "workspace") {
    return puzzleTextDocuments().map((document) => ({
      document,
      source: sourceForDocument(document),
    }));
  }
  throw new Error("Level source lookup requires a preview build or workspace scope.");
}

function currentLevelSourceLocation(options = {}) {
  const exportData = options.exportData || currentPreviewExportData();
  const levelIndex = Number.isInteger(options.levelIndex)
    ? normalizedLevelIndex(options.levelIndex, exportData)
    : currentEditableLevelIndex(exportData);
  const levelName = levelEditorLevels(exportData)[levelIndex]?.name || "";
  const sourceDocuments = sourceDocumentsForLevelLocation(options);
  const allEntries = [];
  for (const { document, source } of sourceDocuments) {
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
        sourceSnapshot: source,
        levelIndex,
        key: `${entry.document.id}:level:${levelIndex}:${levelName}:${entry.start}`,
      };
    }
  }
  const positionalEntry = levelName ? null : allEntries[levelIndex] || null;
  if (positionalEntry) {
    return {
      document: positionalEntry.document,
      start: positionalEntry.start,
      end: positionalEntry.end,
      sourceSnapshot: sourceDocuments
        .find((entry) => entry.document?.id === positionalEntry.document?.id)?.source || "",
      levelIndex,
      key: `${positionalEntry.document.id}:level:${levelIndex}::${positionalEntry.start}`,
    };
  }
  return null;
}

function currentLevel3dSourceLocationForIndex(levelIndex, exportData = currentPreviewExportData(), options = {}) {
  const targetIndex = normalizedLevelIndex(levelIndex, exportData);
  const levelName = levelEditorLevels(exportData)[targetIndex]?.name || "";
  const allEntries = [];
  for (const { document, source } of sourceDocumentsForPreviewBuild(options.build)) {
    for (const entry of surfaceEntriesForSource(source).filter((candidate) => sourceTargetMatches(candidate, "level", "3d"))) {
      const entryIndex = allEntries.length;
      const target = {
        document,
        start: entry.start,
        end: entry.end,
        bodyStart: entry.bodyStart,
        bodyEnd: entry.bodyEnd,
        sourceSnapshot: source,
        name: entry.name || "",
        bundle: entry.params?.bundle || "",
        model: entry.params?.model || "",
        levelIndex: entryIndex,
        key: `${document.id}:level3d:${entryIndex}:${entry.name || ""}:${entry.start}`,
      };
      allEntries.push(target);
      if (levelName && sourceTitleMatches(entry.name, levelName)) {
        return target;
      }
    }
  }
  if (levelName) {
    return null;
  }
  return allEntries[targetIndex] || null;
}

function findLevelSourceEntries(source, document) {
  const entries = [];
  const seen = new Set();
  for (const entry of surfaceEntriesForSource(source).filter((candidate) => sourceTargetMatches(candidate, "level", "2d"))) {
    const key = `${entry.start}:${entry.end}`;
    if (seen.has(key)) {
      continue;
    }
    seen.add(key);
    entries.push({
      document,
      name: entry.name || "",
      sourceName: Object.prototype.hasOwnProperty.call(entry, "sourceName") ? entry.sourceName : entry.name || "",
      namespace: entry.params?.namespace || "",
      start: entry.start,
      end: entry.end,
      levelIndex: entry.levelIndex,
    });
  }
  return entries;
}

function currentVisualSourceLocation() {
  if (!Number.isInteger(visual.editSourceStart) || !Number.isInteger(visual.editSourceEnd)) {
    return null;
  }
  for (const document of puzzleTextDocuments()) {
    if (document.id === visual.editDocumentId) {
      return {
        document,
        start: visual.editSourceStart,
        end: visual.editSourceEnd,
        key: `${document.id}:visual:${visual.editSourceName || ""}:${visual.editSourceStart}`,
      };
    }
  }
  return null;
}

function currentVisual3dSourceLocation() {
  if (!Number.isInteger(visual3d.editSourceStart) || !Number.isInteger(visual3d.editSourceEnd)) {
    return null;
  }
  for (const document of puzzleTextDocuments()) {
    if (document.id === visual3d.editDocumentId) {
      return {
        document,
        start: visual3d.editSourceStart,
        end: visual3d.editSourceEnd,
        key: `${document.id}:visual3d:${visual3d.editSourceName || ""}:${visual3d.editSourceStart}`,
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
    ? sourceEditorDocumentValue()
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

function makeEmptyCells(width, height, exportData = currentLevelExportData()) {
  return Array.from({ length: width * height }, () => makeEmptyCell(exportData));
}

function makeEmptyLevelLayer(width = level.width, height = level.height, exportData = currentLevelExportData()) {
  return makeEmptyCells(width, height, exportData);
}

function makeEmptyCell(exportData = currentLevelExportData()) {
  return Array.from({ length: layerCount(exportData) }, () => 0);
}

function cloneCellSlots(slots, exportData = currentLevelExportData()) {
  const next = makeEmptyCell(exportData);
  if (Array.isArray(slots)) {
    for (let index = 0; index < Math.min(slots.length, next.length); index += 1) {
      next[index] = Number(slots[index]) || 0;
    }
  }
  return next;
}

function normalizeLevelLayerMap(cells, exportData = currentLevelExportData()) {
  const size = Math.max(1, level.width) * Math.max(1, level.height);
  const next = makeEmptyLevelLayer(level.width, level.height, exportData);
  if (!Array.isArray(cells)) {
    return next;
  }
  for (let index = 0; index < Math.min(size, cells.length); index += 1) {
    next[index] = cloneCellSlots(cells[index], exportData);
  }
  return next;
}

function ensureLevelLayerMaps(exportData = currentLevelExportData()) {
  const cellCount = Math.max(1, level.width) * Math.max(1, level.height);
  const expectedSlots = layerCount(exportData);
  const invalidLayer = !Array.isArray(level.layers) || !level.layers.length
    ? -1
    : level.layers.findIndex((layerCells) => !Array.isArray(layerCells)
      || layerCells.length !== cellCount
      || layerCells.some((cell) => !Array.isArray(cell) || cell.length !== expectedSlots));
  if (invalidLayer >= 0 || !Array.isArray(level.layers) || !level.layers.length) {
    const layerCells = invalidLayer >= 0 ? level.layers[invalidLayer] : null;
    const actualCells = Array.isArray(layerCells) ? layerCells.length : null;
    const actualSlots = Array.isArray(layerCells?.[0]) ? layerCells[0].length : null;
    throw new Error(
      `Rust level layer projection is inconsistent with the active model: expected ${cellCount} cells × ${expectedSlots} slots; layer ${invalidLayer} has ${actualCells} cells × ${actualSlots} slots`,
    );
  }
  return level.layers;
}

function levelLayerCount2d() {
  return Math.max(1, Array.isArray(level.layers) ? level.layers.length : 0);
}

function levelLayerCells(layerIndex = level.activeLayer, exportData = currentLevelExportData()) {
  ensureLevelLayerMaps(exportData);
  const index = normalizedLevelActiveLayer(layerIndex);
  return level.layers[index] || level.layers[0];
}

function levelCompositeCells(options = {}) {
  const exportData = options.exportData || currentLevelExportData();
  void options;
  const layers = Array.isArray(level.layers) && level.layers.length ? level.layers : [];
  const composite = makeEmptyCells(level.width, level.height, exportData);
  for (const [layerIndex, layerCells] of layers.entries()) {
    void layerIndex;
    const normalizedCells = normalizeLevelLayerMap(layerCells, exportData);
    for (let cellIndex = 0; cellIndex < composite.length; cellIndex += 1) {
      const target = composite[cellIndex];
      const source = normalizedCells[cellIndex];
      for (let slotIndex = 0; slotIndex < target.length; slotIndex += 1) {
        if (source[slotIndex]) {
          target[slotIndex] = source[slotIndex];
        }
      }
    }
  }
  return composite;
}

function renderLevelPalette() {
  ensureLevelLayerMaps();
  const eraserButton = renderLevelEraserButton();
  levelPalette.replaceChildren(...[levelFillButton, eraserButton].filter(Boolean));
  levelPalette.classList.add("is-visual-only");
  const objects = level.palette.filter((object) => object.id !== 0);
  renderLevelPaletteGroup("", objects);
  levelPalette.append(renderLevelAddLegendButton());
  renderLevelLayerControls();
  renderLevelLayerPreviews();
  updateLevelPlaytestControls();
}

function renderLevelLayerControls() {
  if (!levelLayerControls) {
    return;
  }
  const controls = [];
  controls.push(levelLayersModeButton());
  if (level.layerMode) {
    controls.push(
      levelLayerAddButton(),
      levelLayerRemoveButton(),
      levelCompositeLayersButton(),
    );
  }
  levelLayerControls.replaceChildren(...controls);
}

function levelLayersModeButton() {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "icon-button visual-icon-button level-layers-enable-button";
  button.classList.toggle("is-enabled", level.layerMode);
  button.setAttribute("aria-label", "Toggle level layer mode");
  button.setAttribute("aria-pressed", String(level.layerMode));
  button.title = "Level layers";
  button.dataset.tooltip = button.title;
  button.disabled = levelPlaytestActive;
  button.innerHTML = editorIconSvg("layers");
  button.addEventListener("click", () => {
    setLevelLayerMode(!level.layerMode);
  });
  return button;
}

function levelLayerAddButton() {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "icon-button visual-icon-button level-layer-add-button";
  button.setAttribute("aria-label", "Add level layer");
  button.classList.toggle("is-selected", levelLayerInsertMode);
  button.setAttribute("aria-pressed", String(levelLayerInsertMode));
  button.title = levelLayerInsertMode ? "Cancel add layer" : "Add layer";
  button.dataset.tooltip = "Add layer";
  button.disabled = levelPlaytestActive;
  button.innerHTML = editorIconSvg("layers-plus");
  button.addEventListener("click", toggleLevelLayerInsertMode);
  return button;
}

function levelLayerRemoveButton() {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "icon-button visual-icon-button level-layer-remove-button";
  button.setAttribute("aria-label", "Remove current level layer");
  button.classList.toggle("is-selected", levelLayerRemoveMode);
  button.setAttribute("aria-pressed", String(levelLayerRemoveMode));
  button.title = levelLayerRemoveMode ? "Cancel remove layer" : "Remove layer";
  button.dataset.tooltip = "Remove layer";
  button.disabled = levelPlaytestActive || levelLayerCount2d() <= 1;
  button.innerHTML = editorIconSvg("layers-minus");
  button.addEventListener("click", toggleLevelLayerRemoveMode);
  return button;
}

async function setLevelLayer(layerIndex) {
  const transition = await dispatchLevelSessionCommand({
    type: "selectLayer",
    layerIndex: normalizedLevelActiveLayer(layerIndex),
  }, { render: false });
  if (!transition) {
    return false;
  }
  level.showCompositeLayers = false;
  levelLayerInsertMode = false;
  levelLayerRemoveMode = false;
  renderLevelPalette();
  renderLevelBoard();
  setStatus(`Editing layer ${level.activeLayer + 1}`, "is-ok");
  return true;
}

function setLevelLayerMode(enabled) {
  level.layerMode = Boolean(enabled);
  level.showCompositeLayers = false;
  levelLayerInsertMode = false;
  levelLayerRemoveMode = false;
  renderLevelPalette();
  renderLevelBoard();
}

function toggleLevelLayerInsertMode() {
  if (levelPlaytestActive) {
    return;
  }
  levelLayerRemoveMode = false;
  levelLayerInsertMode = !levelLayerInsertMode;
  renderLevelLayerControls();
  renderLevelLayerPreviews();
  setStatus(levelLayerInsertMode ? "Choose a layer gap to add a layer" : "Add layer canceled", "is-ok");
}

function toggleLevelLayerRemoveMode() {
  if (levelPlaytestActive || levelLayerCount2d() <= 1) {
    return;
  }
  levelLayerInsertMode = false;
  levelLayerRemoveMode = !levelLayerRemoveMode;
  renderLevelLayerControls();
  renderLevelLayerPreviews();
  setStatus(levelLayerRemoveMode ? "Choose a layer to remove" : "Remove layer canceled", "is-ok");
}

async function insertLevelLayerAt(index) {
  if (levelPlaytestActive) {
    return false;
  }
  const insertAt = Math.max(0, Math.min(levelLayerCount2d(), Math.trunc(Number(index) || 0)));
  const transition = await dispatchLevelSessionCommand({ type: "insertLayer", layerIndex: insertAt }, { render: false });
  if (!transition) {
    return false;
  }
  level.showCompositeLayers = false;
  levelLayerInsertMode = false;
  renderLevelPalette();
  renderLevelBoard();
  setStatus(`Added layer ${level.activeLayer + 1}`, "is-ok");
  return true;
}

async function removeLevelLayerAt(index) {
  if (levelPlaytestActive || levelLayerCount2d() <= 1) {
    return false;
  }
  const removeAt = Math.max(0, Math.min(levelLayerCount2d() - 1, Math.trunc(Number(index) || 0)));
  const transition = await dispatchLevelSessionCommand({ type: "removeLayer", layerIndex: removeAt }, { render: false });
  if (!transition) {
    return false;
  }
  level.showCompositeLayers = false;
  levelLayerRemoveMode = false;
  renderLevelPalette();
  renderLevelBoard();
  setStatus(`Removed layer ${removeAt + 1}`, "is-ok");
  return true;
}

function renderLevelLayerPreviews() {
  if (!levelLayerPreviewPanel || !levelLayerPreviewStrip) {
    return;
  }
  const show = level.layerMode && !levelPlaytestActive;
  levelLayerPreviewPanel.hidden = !show;
  if (!show) {
    levelLayerPreviewStrip.replaceChildren();
    return;
  }
  ensureLevelLayerMaps();
  const canInsert = levelLayerInsertMode;
  const canRemove = levelLayerRemoveMode && levelLayerCount2d() > 1;
  levelLayerPreviewStrip.classList.toggle("is-insert-mode", canInsert);
  levelLayerPreviewStrip.classList.toggle("is-remove-mode", canRemove);
  const exportData = currentLevelExportData();
  const fragment = document.createDocumentFragment();
  for (let index = 0; index < levelLayerCount2d(); index += 1) {
    if (canInsert) {
      fragment.append(levelLayerInsertTargetButton(index));
    }
    const button = document.createElement("button");
    button.type = "button";
    button.className = "level-layer-preview-button";
    button.classList.toggle("is-active", index === normalizedLevelActiveLayer());
    button.setAttribute("aria-label", canRemove ? `Remove level layer ${index + 1}` : `Edit level layer ${index + 1}`);
    button.title = canRemove ? "Remove layer" : `Layer ${index + 1}`;
    const view = document.createElement("span");
    view.className = "level-layer-preview-view game-preview-scope board";
    view.setAttribute("aria-hidden", "true");
    if (window.PuzzleAuthoringRenderer) {
      new window.PuzzleAuthoringRenderer(view).render(levelScene(levelLayerCells(index, exportData), exportData));
    }
    const label = document.createElement("span");
    label.className = "level-layer-preview-index";
    label.textContent = `Layer ${index + 1}`;
    button.append(view, label);
    button.addEventListener("click", () => {
      if (canRemove) {
        removeLevelLayerAt(index);
      } else {
        setLevelLayer(index);
      }
    });
    fragment.append(button);
  }
  if (canInsert) {
    fragment.append(levelLayerInsertTargetButton(levelLayerCount2d()));
  }
  levelLayerPreviewStrip.replaceChildren(fragment);
}

function levelLayerInsertTargetButton(index) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "level-layer-insert-target";
  button.setAttribute("aria-label", `Insert level layer at position ${index + 1}`);
  button.title = "Add layer";
  button.addEventListener("click", () => insertLevelLayerAt(index));
  return button;
}

function renderLevelEraserButton() {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "icon-button level-palette-tool-button visual-icon-button level-eraser-button";
  button.classList.toggle("is-active", level.selectedObjectId === 0);
  button.setAttribute("aria-label", "Paint Eraser");
  button.setAttribute("aria-pressed", String(level.selectedObjectId === 0));
  button.title = "Eraser";
  button.dataset.tooltip = "Eraser";
  button.append(renderLevelEraserIcon());
  button.addEventListener("click", selectLevelEraser);
  return button;
}

function selectLevelEraser() {
  if (levelPlaytestActive) return false;
  level.selectedObjectId = 0;
  setLevelActiveLayerForObject(0);
  renderLevelPalette();
  return true;
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
    button.dataset.tooltip = object.name;
    button.setAttribute("aria-label", `Paint ${object.name}`);
    button.append(renderObjectPreview(object));

    const label = document.createElement("span");
    label.className = "tile-label";
    label.textContent = object.name;
    button.append(label);

    button.addEventListener("click", () => selectLevelPaletteObject(object.id));
    group.append(button);
  }
  levelPalette.append(group);
}

function selectLevelPaletteObject(objectId) {
  const object = level.palette.find((candidate) => candidate.id === objectId && candidate.id !== 0);
  if (!object || levelPlaytestActive) return false;
  level.selectedObjectId = object.id;
  setLevelActiveLayerForObject(object.id);
  renderLevelPalette();
  return true;
}

function renderLevelAddLegendButton() {
  const wrap = document.createElement("span");
  wrap.className = "level-palette-add-wrap";
  const candidates = levelPaletteAddCandidates();
  const button = document.createElement("button");
  button.type = "button";
  button.className = "icon-button level-palette-tool-button visual-icon-button level-add-legend-button";
  button.disabled = !candidates.length;
  button.setAttribute("aria-label", "Add tile legend");
  button.setAttribute("aria-expanded", String(level.addPaletteOpen && candidates.length > 0));
  button.title = candidates.length ? "Add tile legend" : "No unlisted objects";
  button.dataset.tooltip = button.title;
  button.innerHTML = editorIconSvg("plus");
  button.addEventListener("click", () => {
    if (!candidates.length) {
      return;
    }
    level.addPaletteOpen = !level.addPaletteOpen;
    renderLevelPalette();
  });
  wrap.append(button);
  if (level.addPaletteOpen && candidates.length) {
    const menu = document.createElement("div");
    menu.className = "level-palette-add-menu";
    menu.setAttribute("role", "menu");
    for (const object of candidates) {
      const item = document.createElement("button");
      item.type = "button";
      item.className = "option-button level-palette-add-menu-item";
      item.setAttribute("role", "menuitem");
      item.textContent = object.name;
      item.title = object.name;
      item.addEventListener("click", () => {
        addLevelPaletteObjectToLegend(object);
      });
      menu.append(item);
    }
    wrap.append(menu);
  }
  return wrap;
}

function levelPaletteAddCandidates(source = currentLevelAuthoringSource(), exportData = currentLevelExportData()) {
  if (!levelEditorObjects(exportData).length) {
    return [];
  }
  const placeable = sourcePlaceableObjectNames(source, exportData);
  return engineObjects(exportData).filter((object) => (
    object.id !== 0
    && !String(object.name || "").startsWith("@")
    && !placeable.has(object.name)
  ));
}

function renderLevelBoard() {
  updateLevelSizeLabel();
  syncLevelResizeControls();
  const exportData = currentLevelExportData();
  if (!levelEditorAssistanceReady(exportData)) {
    levelBoard.replaceChildren();
    renderLevelLayerPreviews();
    scheduleBoardScaleSync();
    return;
  }
  const cells = displayedLevelCells();
  if (queueLevelAuthoringRuntime()) {
    renderLevelLayerPreviews();
    scheduleBoardScaleSync();
    return;
  }
  if (!levelRenderer) {
    levelBoard.replaceChildren();
    setPaneStatus("level", "Level renderer unavailable", "is-error");
    syncLevelBoardScale();
    scheduleBoardScaleSync();
    renderSolverBoard();
    return;
  }
  levelRenderer.render(levelScene(cells, exportData));
  renderLevelLayerPreviews();
  syncLevelGridVisibility();
  levelBoard.querySelectorAll(".cell").forEach((cell, index) => {
    cell.dataset.index = String(index);
    cell.setAttribute("aria-label", cellLabel(cells[index], exportData));
    cell.setAttribute("role", "button");
    cell.tabIndex = 0;
  });
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
  if (!solverBoard || solverPanel.hidden) {
    return;
  }
  const agentCandidate = solverPaneMode === "custom-goal" && solverObservationPreview?.source === "agent"
    ? selectedSolverMaterializedCandidate()
    : null;
  if (agentCandidate
    && Number.isInteger(agentCandidate.levelIndex)
    && activeSolverTask?.level?.index !== agentCandidate.levelIndex) {
    const observation = solverObservationPreview;
    solverObservationPreview = null;
    const selected = selectSolverLevel(agentCandidate.levelIndex);
    solverObservationPreview = observation;
    customGoalSolverObservationPreview = observation;
    if (selected) {
      renderSolverObservationPanel();
    }
  }
  const display = solverRuntimeDisplayState();
  if (!display) {
    hideEditorRuntimeSurface(solverBoard);
    return;
  }
  queueSolverRuntimeState(display);
}

function solverRuntimeDisplayState() {
  const observationCandidate = selectedSolverObservationCandidate();
  const materializedCandidate = selectedSolverMaterializedCandidate();
  if (solverObservationPreview?.displayCandidate && materializedCandidate?.state) {
    return {
      state: materializedCandidate.state,
      levelIndex: Number.isInteger(materializedCandidate.levelIndex)
        ? materializedCandidate.levelIndex
        : solverTaskLevelIndex(),
      materializeLevelStart: false,
      key: `observation:${solverTaskRunKey()}:${observationCandidate.candidateId}:${materializedCandidate.stateHash}`,
    };
  }
  if (solverPaneMode === "level-goal" && Array.isArray(levelSolutionPreview?.steps)) {
    const step = levelSolutionPreview.steps[levelSolutionPreview.index];
    if (step?.state) {
      return {
        state: step.state,
        levelIndex: solverTaskLevelIndex(),
        materializeLevelStart: false,
        key: `solution:${levelSolutionPreview.index}:${JSON.stringify(step.state)}`,
      };
    }
  }
  if (materializedCandidate?.state) {
    return {
      state: materializedCandidate.state,
      levelIndex: Number.isInteger(materializedCandidate.levelIndex)
        ? materializedCandidate.levelIndex
        : solverTaskLevelIndex(),
      materializeLevelStart: false,
      key: `observation:${solverTaskRunKey()}:${observationCandidate.candidateId}:${materializedCandidate.stateHash}`,
    };
  }
  if (activeSolverTask?.state?.data) {
    return {
      state: activeSolverTask.state.data,
      levelIndex: solverTaskLevelIndex(),
      materializeLevelStart: activeSolverTask.state.lifecycle === "playable-start",
      key: `task:${solverTaskRunKey(activeSolverTask)}`,
    };
  }
  return null;
}

function queueSolverRuntimeState(display) {
  const levelIndex = Number(display?.levelIndex);
  if (!display?.state || !Number.isInteger(levelIndex) || levelIndex < 0) {
    hideEditorRuntimeSurface(solverBoard);
    setLevelSolveStatus("Solver runtime state is incomplete.", "is-error");
    return;
  }
  queueEditorRuntimeDisplay({
    host: solverBoard,
    consumer: "solver",
    surfaceId: "solver-observation",
    launchProfile: EDITOR_PLAYER_LAUNCH_PROFILE,
    dispatch: (targetFrame) => postEditorModelState({
      model: activeSolverTask?.rules?.modelName || "",
      state: display.state,
      levelIndex,
      materializeLevelStart: display.materializeLevelStart === true,
      presentation: editorRuntimePresentationForState(
        display.state,
        "solver-observation",
        { kind: "observe" },
        currentPreviewExportData(),
      ),
    }, targetFrame),
    key: display.key,
    onError: (error) => setLevelSolveStatus(`Solver display failed: ${userFacingRuntimeError(error)}`, "is-error"),
  });
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
  // The Bevy iframe renders at the final solver viewport size.
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
  for (const visual of board.querySelectorAll(".visual")) {
    const style = window.getComputedStyle(visual);
    const cols = Math.max(1, Math.trunc(Number(style.getPropertyValue("--visual-cols")) || 1));
    const rows = Math.max(1, Math.trunc(Number(style.getPropertyValue("--visual-rows")) || 1));
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
  const exportData = currentPreviewExportData();
  const levelIndex = currentEditableLevelIndex(exportData);
  const manifestLevel = levelEditorLevels(exportData)[levelIndex];
  if (!manifestLevel) {
    return false;
  }
  const loaded = loadLevelFromSourceEntry(exportData.source, {
    levelIndex: manifestLevel.sourceLevelIndex,
    name: manifestLevel.name,
  }, { exportData, levelIndex });
  if (!loaded) {
    return false;
  }
  setLevelNameInputs(manifestLevel.name);
  if (requestRender) {
    sendLevelDraftToPreview(levelIndex);
  }
  return true;
}

function sceneFromStateData(state, options = {}) {
  if (!state?.width || !state?.height || !state?.layerCount || !Array.isArray(state.slots)) {
    return null;
  }
  const exportData = options.exportData || currentPreviewExportData();
  const objectsById = new Map(levelEditorObjects(exportData).map((object) => [object.id, object]));
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
            visual: object.name,
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

function cellSlotsFromLayers(layers, exportData = currentPreviewExportData()) {
  const slots = makeEmptyCell(exportData);
  for (const layer of layers) {
    if (Number.isInteger(layer.layer) && layer.layer >= 0 && layer.layer < slots.length) {
      slots[layer.layer] = objectIdForLayer(layer, exportData);
    }
  }
  return slots;
}

function objectIdForLayer(layer, exportData = currentPreviewExportData()) {
  const explicit = Number(layer?.objectId) || 0;
  if (explicit) {
    return explicit;
  }
  const name = layer?.object || "";
  const object = levelEditorObjects(exportData).find((entry) =>
    name && entry.name === name
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
  if (window.PuzzleAuthoringRenderer) {
    new window.PuzzleAuthoringRenderer(root).render(objectScene(object));
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
  return editorIconElement("eraser", { className: "level-token-eraser" });
}

function levelListFilterIconSvg() {
  return editorIconSvg("list-filter", { className: "level-layer-visibility-icon" });
}

function levelCompositeLayersButton() {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "icon-button visual-icon-button level-composite-layers-button";
  button.classList.toggle("is-selected", level.showCompositeLayers);
  button.setAttribute("aria-label", "Show composite level layers");
  button.setAttribute("aria-pressed", String(level.showCompositeLayers));
  button.title = level.showCompositeLayers ? "Show active layer" : "Show composite";
  button.dataset.tooltip = button.title;
  button.disabled = levelPlaytestActive;
  button.innerHTML = editorIconSvg("eye");
  button.addEventListener("click", () => {
    level.showCompositeLayers = !level.showCompositeLayers;
    renderLevelPalette();
    renderLevelBoard();
    setStatus(level.showCompositeLayers ? "Showing composite level layers" : `Editing layer ${level.activeLayer + 1}`, "is-ok");
  });
  return button;
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

function levelScene(sourceCells = level.cells, exportData = currentLevelExportData()) {
  return sceneFromCellSlots(sourceCells, {
    width: level.width,
    height: level.height,
    regions: levelRegions(),
    exportData,
  });
}

function sceneFromCellSlots(sourceCells, options = {}) {
  const width = Math.max(1, Number(options.width || level.width || 1));
  const height = Math.max(1, Number(options.height || level.height || 1));
  const exportData = options.exportData || currentLevelExportData();
  const cells = sourceCells.map((slots, index) => ({
    x: index % width,
    y: Math.floor(index / width),
    layers: layersForSlots(normalizedCellSlots(slots, exportData), exportData),
  }));
  return {
    width,
    height,
    layerCount: layerCount(exportData),
    regions: options.regions || [],
    cells,
  };
}

function normalizedCellSlots(slots, exportData = currentLevelExportData()) {
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
  if (levelPlaytestActive) {
    if (levelDisplayCells?.length === level.cells.length) {
      return levelDisplayCells;
    }
    ensureLevelLayerMaps();
    return levelCompositeCells();
  }
  ensureLevelLayerMaps();
  return level.showCompositeLayers ? levelCompositeCells() : levelLayerCells();
}

function layersForSlots(slots, exportData = currentLevelExportData()) {
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
    visual: object.name,
  };
}

function cellLabel(slots, exportData = currentLevelExportData()) {
  const names = layersForSlots(slots, exportData).map((layer) => layer.object);
  return names.length ? names.join(", ") : "Empty";
}

function addLevelEdge(edge) {
  resizeLevelEdge(edge, "expand");
}

function shrinkLevelEdge(edge) {
  resizeLevelEdge(edge, "shrink");
}

async function resizeLevelEdge(edge, mode = levelResizeMode || "expand") {
  const normalizedMode = mode === "shrink" ? "shrink" : "expand";
  if (levelPlaytestActive) {
    return false;
  }
  stopLevelPlaytest({ syncPreview: false });
  const transition = await dispatchLevelSessionCommand({ type: "resize", edge, mode: normalizedMode });
  if (!transition) {
    return false;
  }
  setStatus(normalizedMode === "shrink" ? "Level shrunk" : "Level expanded", "is-ok");
  return true;
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

async function transformLevelCells(transform, message) {
  if (levelPlaytestActive) {
    return false;
  }
  stopLevelPlaytest({ syncPreview: false });
  const transition = await dispatchLevelSessionCommand({ type: "transform", transform });
  if (!transition) {
    return false;
  }
  setStatus(message, "is-ok");
  return true;
}

function rotateLevelLeft() {
  return transformLevelCells("rotateLeft", "Rotated level left");
}

function rotateLevelRight() {
  return transformLevelCells("rotateRight", "Rotated level right");
}

function flipLevelHorizontal() {
  return transformLevelCells("flipHorizontal", "Flipped level horizontal");
}

function flipLevelVertical() {
  return transformLevelCells("flipVertical", "Flipped level vertical");
}

function updateLevelSizeLabel() {
  levelSizeLabel.textContent = `${level.width} × ${level.height}`;
  levelBoard.style.setProperty("--cols", String(Math.max(1, level.width)));
  levelBoard.style.setProperty("--rows", String(Math.max(1, level.height)));
}

function normalizedLevelActiveLayer(layer = level.activeLayer, exportData = currentLevelExportData()) {
  void exportData;
  const count = levelLayerCount2d();
  return Math.max(0, Math.min(count - 1, Math.trunc(Number(layer) || 0)));
}

function setLevelActiveLayerForObject(objectId) {
  void objectId;
  level.activeLayer = normalizedLevelActiveLayer(level.activeLayer);
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

async function bucketFillLevelFromIndex(index) {
  if (levelPlaytestActive || !Number.isInteger(index) || index < 0 || index >= level.cells.length) {
    return false;
  }
  const transition = await dispatchLevelSessionCommand({
    type: "fill2d",
    index,
    objectId: level.selectedObjectId || null,
  });
  if (!transition) {
    return false;
  }
  if (!transition.changed) {
    setStatus("Connected area already has that tile", "is-ok");
    return true;
  }
  setStatus(level.selectedObjectId ? "Filled connected area" : "Erased connected area", "is-ok");
  return true;
}

async function paintLevelCellAtIndex(index, objectId, options = {}) {
  void options;
  if (levelPlaytestActive) {
    return false;
  }
  if (!Number.isInteger(index) || index < 0 || index >= level.cells.length) {
    return false;
  }
  return Boolean((await dispatchLevelSessionCommand({
    type: "paint2d",
    index,
    objectId: objectId || null,
  }))?.changed);
}

function paintLevelCellFromPoint(clientX, clientY, objectId) {
  return paintLevelCellAtIndex(
    levelCellIndexFromElement(document.elementFromPoint(clientX, clientY)),
    objectId,
  );
}

async function startLevelPaint(event) {
  if (levelPlaytestActive) {
    focusLevelInputTarget();
    event.preventDefault();
    return;
  }
  const erase = editorPointerEraseIntent(event);
  if (event.button !== 0 && !erase) {
    return;
  }
  const objectId = erase ? null : level.selectedObjectId;
  const index = levelCellIndexFromElement(document.elementFromPoint(event.clientX, event.clientY));
  if (!Number.isInteger(index) || index < 0) {
    return;
  }
  event.preventDefault();
  if (levelBucketActive && !erase) {
    await bucketFillLevelFromIndex(index);
    return;
  }
  if (!await dispatchLevelSessionCommand({ type: "beginEdit" }, { render: false })) {
    return;
  }
  levelPaintDrag = levelShared.beginPointerPaint({
    target: levelBoard,
    pointerId: event.pointerId,
    beforeSnapshot: null,
    brush: objectId,
  });
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

async function stopLevelPaint(event) {
  if (!levelPaintDrag || levelPaintDrag.pointerId !== event.pointerId) {
    return;
  }
  levelShared.finishPointerPaint(levelPaintDrag, event.pointerId, () => {});
  await dispatchLevelSessionCommand({ type: "commitEdit" }, { render: false });
  levelPaintDrag = null;
}

function paintLevelDragIndex(index) {
  if (!levelPaintDrag || !Number.isInteger(index) || index < 0) {
    return;
  }
  levelShared.applyPointerPaint(
    levelPaintDrag,
    index,
    (objectId) => paintLevelCellAtIndex(index, objectId),
  );
}

function levelPlaytestLifecycle() {
  return {
    isActive: () => levelPlaytestActive,
    setActive: (active) => { levelPlaytestActive = active; },
    prepare: () => ensureCompiledPreviewForLevelPlaytest({
      noDocumentMessage: "No level to play",
      compilingMessage: "Compiling preview for play",
      failureMessage: "Preview compile failed",
    }),
    validate: (exportData) => {
      const levelIndex = currentEditableLevelIndex(exportData);
      if (exportData?.session?.levelSnapshot(levelIndex)?.cells?.length) {
        return true;
      }
      setStatus("No level to play", "is-error");
      return false;
    },
    beforeStart: () => {
      clearSolutionPreview();
      levelDisplayCells = null;
    },
    hasTransient: () => Boolean(levelDisplayCells),
    clearTransient: () => {
      levelDisplayCells = null;
      if (levelPaintDrag?.target?.hasPointerCapture?.(levelPaintDrag.pointerId)) {
        levelPaintDrag.target.releasePointerCapture(levelPaintDrag.pointerId);
      }
      levelPaintDrag = null;
    },
    updateControls: updateLevelPlaytestControls,
    render: renderLevelBoard,
    focus: focusLevelInputTarget,
    failed: (error) => setStatus(`Play failed: ${userFacingRuntimeError(error)}`, "is-error"),
  };
}

async function startLevelPlaytest() {
  return levelShared.startPlaytest(levelPlaytestLifecycle());
}

function stopLevelPlaytest(options = {}) {
  return levelShared.stopPlaytest(levelPlaytestLifecycle(), options);
}

function focusLevelInputTarget() {
  editorRuntimeControllers.get("level-play")?.surface.focus?.({ preventScroll: true });
}

function sendLevelPlaytestKey(event) {
  return levelShared.sendPlaytestKey({
    active: levelPlaytestActive,
    controller: editorRuntimeControllers.get("level-play"),
    event,
    send: postEditorKeyInput,
  });
}

function toggleLevelPlaytest() {
  levelShared.togglePlaytest(levelPlaytestLifecycle());
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
    levelExpandButton,
    levelShrinkButton,
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
  levelLayerControls?.querySelectorAll("button").forEach((button) => {
    button.disabled = levelPlaytestActive
      || ((button.classList.contains("level-layer-step-button") || button.classList.contains("level-layer-remove-button")) && levelLayerCount2d() <= 1);
  });
  levelLayerControls?.querySelectorAll("input").forEach((input) => {
    input.disabled = levelPlaytestActive;
  });
  levelPalette?.querySelectorAll("button").forEach((button) => {
    button.disabled = levelPlaytestActive;
  });
  levelEdgeButtons.forEach((button) => {
    button.disabled = levelPlaytestActive;
  });
}

async function sendLevelDraftToPreview(levelIndex = currentEditableLevelIndex()) {
  if (!activePreviewModeAcceptsLevelState()) {
    return;
  }
  const exportData = currentLevelExportData();
  previewFrameHasEditorLevelState = true;
  const sourceSession = exportData?.session;
  if (typeof sourceSession?.draftState !== "function") {
    throw new Error("Rust level draft projection is unavailable.");
  }
  const state = await sourceSession.draftState(levelIndex);
  const playerState = {
    model: editorModelName(exportData),
    levelIndex,
    state,
    materializeLevelStart: false,
  };
  postEditorModelState(playerState);
}

async function solveLevel(options = {}) {
  if (activeLevelSolveRequest) {
    cancelLevelSolve();
    return;
  }
  stopSolverObservationPlayback();
  if (!activeSolverTask && currentPreviewMode === "solver") {
    refreshVisiblePreviewSolverTask();
  }
  const task = activeSolverTask;
  if (!task) {
    setLevelSolveStatus("No level to solve", "is-error");
    return;
  }
  if (isSolverTaskComplete(task)) {
    setSolveLevelButtonState(false);
    setLevelSolveStatus("This level has already been solved", "is-error");
    return;
  }
  let solveRequest = null;
  try {
    solveRequest = solverRequestForTask(task);
  } catch (error) {
    setLevelSolveStatus(userFacingRuntimeError(error), "is-error");
    return;
  }

  clearSolutionPreview({ preserveSolverTask: true });
  renderSolverBoard();
  const requestId = createDocumentId();
  let worker = null;
  try {
    worker = createWasmSolveWorker();
  } catch (error) {
    setSolveLevelButtonState(false);
    stopLevelSolveFeedback();
    setLevelSolveStatus(`Solver worker failed: ${userFacingWorkerError(error)}`, "is-error");
    return;
  }
  activeLevelSolveRequest = {
    id: requestId,
    backend: "wasm-worker",
    worker,
    progressCount: 0,
    mode: solverPaneMode,
    request: solveRequest,
  };
  registerEditorSolverWorkerRequest(requestId, (message) => {
    if (message.type === "progress") {
      handleLevelSolveProgress({ requestId, observation: message.observation });
      return false;
    }
    if (message.type === "materialized-candidate") {
      handleSolverCandidateMaterialized(message);
      return false;
    }
    if (message.type === "materialization-error") {
      handleSolverCandidateMaterializationError(message);
      return false;
    }
    if (message.type === "result") {
      handleLevelSolveResult({ requestId, solution: message.solution });
      return true;
    }
    if (message.type === "error") {
      handleLevelSolveResult({
        requestId,
        error: `Solver worker failed: ${userFacingWorkerError(message.error)}`,
      });
      return true;
    }
    return false;
  }, (error) => {
    handleLevelSolveResult({
      requestId,
      error: `Solver worker failed: ${userFacingWorkerError(error)}`,
    });
  });
  setSolveLevelButtonState(true);
  startLevelSolveFeedback("Solving");
  try {
    worker.postMessage({
      type: "solve",
      requestId,
      wasm: wasmSolverWorkerConfig(),
      artifactId: task.rules.compileId,
      modelKind: task.rules.modelKind,
      request: solveRequest,
    });
  } catch (error) {
    editorSolverWorkerRequests.delete(requestId);
    handleLevelSolveResult({
      requestId,
      error: `Solver worker failed: ${userFacingWorkerError(error)}`,
    });
  }
}

function solverRequestForTask(task) {
  if (!task?.rules?.modelKind || !task?.state?.data || !task?.level) {
    throw new Error("Solver task is incomplete.");
  }
  const objective = solverPaneMode === "custom-goal"
    ? { kind: "custom", constraints: cloneJson(customGoalConstraints(task)) }
    : { kind: "level_completion" };
  if (objective.kind === "custom" && !objective.constraints.length) {
    throw new Error("Add at least one goal condition before starting the solver.");
  }
  return {
    levelIndex: task.level.index,
    state: task.state.data,
    materializeLevelStart: task.state.lifecycle === "playable-start",
    objective,
    maxDepth: 512,
    maxStoredNodes: 5_000_000,
    observationCandidateLimit: 8,
  };
}

async function solveEditedLevelFromEditor() {
  const exportData = currentLevelExportData();
  if (!exportData) {
    setLevelSolveStatus("No rule model for edited level", "is-error");
    return;
  }
  const solverBuild = await prepareCurrentDraftSolverBuild();
  if (!solverBuild) return;
  const levelIndex = currentEditableLevelIndex(exportData);
  const sourceSession = exportData.session;
  if (typeof sourceSession?.draftState !== "function") {
    setLevelSolveStatus("No typed level draft is available", "is-error");
    return;
  }
  let stateData;
  try {
    stateData = await sourceSession.draftState(levelIndex);
  } catch (error) {
    setLevelSolveStatus(`Level draft is invalid: ${userFacingRuntimeError(error)}`, "is-error");
    return;
  }
  if (isPuzzle3dExport(exportData)) {
    if (!stateData) {
      setLevelSolveStatus("No 3D level state", "is-error");
      return;
    }
    setActiveSolverTask(createEditorSolverTask({
      solverBuild,
      exportData,
      levelIndex,
      stateData,
    }), exportData);
  } else {
    const scene = stateData ? sceneFromStateData(stateData, {
      regions: levelRegions(),
      exportData,
    }) : null;
    if (!stateData || !scene) {
      setLevelSolveStatus("No level state", "is-error");
      return;
    }
    setActiveSolverTask(createEditorSolverTask({
      solverBuild,
      exportData,
      levelIndex,
      stateData,
      scene,
    }), exportData);
  }
  openPreviewModePane("solver");
  syncSourceFromPreviewPane(isPuzzle3dExport(exportData) ? "level3d" : "solver");
  renderSolverBoard();
  solveLevel();
}

function cancelLevelSolve() {
  if (!activeLevelSolveRequest) {
    return;
  }
  if (
    activeLevelSolveRequest.backend !== "wasm-worker"
    || !activeLevelSolveRequest.worker
  ) {
    const error = new Error("Active solver request is not owned by the WASM worker backend.");
    setLevelSolveStatus(error.message, "is-error");
    console.error(error);
    return;
  }
  activeLevelSolveRequest.worker.postMessage({
    type: "cancel",
    requestId: activeLevelSolveRequest.id,
    wasm: wasmSolverWorkerConfig(),
  });
  setLevelSolveStatus("Cancelling", "");
}

function setSolveLevelButtonState(isSolving) {
  const taskComplete = !isSolving && isSolverTaskComplete();
  const customGoalMissing = !isSolving
    && solverPaneMode === "custom-goal"
    && customGoalConstraints().length === 0;
  const label = isSolving ? "Cancel" : "Solve";
  const visibleLabel = label;
  const title = taskComplete
    ? "This goal has already been solved"
    : customGoalMissing ? "Add a goal condition" : visibleLabel;
  syncSolverLevelSelector();
  for (const button of [solveLevelButton, previewSolveButton, levelSolveShortcutButton, level3dSolveShortcutButton]) {
    if (!button) {
      continue;
    }
    button.classList.toggle("is-solving", Boolean(isSolving));
    const previewHasNoLevel = button === previewSolveButton && !isSolving && !previewHasCurrentLevel();
    button.disabled = taskComplete || customGoalMissing || previewHasNoLevel;
    button.setAttribute("aria-label", label);
    button.title = title;
    button.dataset.tooltip = title;
    const labelElement = button.querySelector(".solve-button-label");
    if (labelElement) {
      labelElement.textContent = visibleLabel;
    }
  }
  solverBoardViewport?.classList.toggle("is-solving", Boolean(isSolving));
  solverBoardViewport?.closest(".solver-board-wrap")?.classList.toggle("is-solving", Boolean(isSolving));
  syncSolverTaskReadout();
  renderCustomGoalEditor();
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
  if ((activeLevelSolveRequest.progressCount || 0) > 0) {
    return;
  }
  const elapsedMs = Date.now() - levelSolveStartedAt;
  setLevelSolveStatus(`Solving: starting search, ${formatSeconds(elapsedMs)}`, "");
}

function handleLevelSolveProgress(message) {
  if (!activeLevelSolveRequest || message.requestId !== activeLevelSolveRequest.id) {
    return;
  }
  activeLevelSolveRequest.progressCount += 1;
  showSolverObservation({
    ...message.observation,
    source: activeLevelSolveRequest.mode === "custom-goal" ? "custom" : "wasm",
  });
}

function handleLevelSolveResult(message) {
  if (!activeLevelSolveRequest || message.requestId !== activeLevelSolveRequest.id) {
    return;
  }
  const hadLiveProgress = (activeLevelSolveRequest.progressCount || 0) > 0;
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

  if (!hadLiveProgress && Array.isArray(solution.observations) && solution.observations.length) {
    playSolverObservations(solution.observations, () => finishLevelSolveResult(solution));
    return;
  }
  finishLevelSolveResult(solution);
}

function finishLevelSolveResult(solution) {
  if (solution.result === "solved") {
    markActiveSolverTaskComplete();
    setSolveLevelButtonState(false);
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

function playSolverObservations(observations, onComplete) {
  stopSolverObservationPlayback();
  const frames = observations.filter(
    (observation) => Array.isArray(observation?.candidates)
      && observation.candidates.length > 0,
  );
  if (!frames.length) {
    onComplete?.();
    return;
  }
  let index = 0;
  const stepMs = Math.max(
    solverObservationPlaybackMinStepMs,
    Math.min(
      solverObservationPlaybackMaxStepMs,
      Math.floor(solverObservationPlaybackMaxMs / frames.length),
    ),
  );
  const show = () => {
    const frame = frames[index];
    showSolverObservation(frame);
    index += 1;
    if (index >= frames.length) {
      stopSolverObservationPlayback();
      onComplete?.();
    }
  };
  show();
  if (index < frames.length) {
    solverObservationTimer = window.setInterval(show, stepMs);
  }
}

function stopSolverObservationPlayback() {
  if (solverObservationTimer) {
    window.clearInterval(solverObservationTimer);
    solverObservationTimer = 0;
  }
}

function showSolverObservation(observation) {
  const candidates = Array.isArray(observation?.candidates)
    ? observation.candidates.filter((candidate) => candidate?.candidateId)
    : [];
  if (!candidates.length) {
    return;
  }
  const source = observation.source || "wasm";
  const customSource = source === "agent" || source === "custom";
  const previousPreview = customSource
    ? customGoalSolverObservationPreview
    : levelGoalSolverObservationPreview;
  const sameObservationStream = previousPreview?.source === source
    && (source !== "agent" || (
      previousPreview.sessionId === observation.sessionId
      && previousPreview.searchId === observation.searchId
    ));
  const previousCandidateId = sameObservationStream
    ? previousPreview?.selectedCandidateId
    : "";
  const selectedCandidateId = candidates.some(
    (candidate) => candidate.candidateId === previousCandidateId,
  )
    ? previousCandidateId
    : candidates[0].candidateId;
  const materializedCandidates = sameObservationStream
    ? previousPreview?.materializedCandidates || new Map()
    : new Map();
  const selectedCandidate = candidates.find(
    (candidate) => candidate.candidateId === selectedCandidateId,
  );
  const preservePending = sameObservationStream
    && previousPreview?.pendingCandidateId === selectedCandidateId
    && previousPreview?.pendingCandidateStateHash === selectedCandidate?.stateHash;
  const nextPreview = {
    source,
    sessionId: observation.sessionId || "",
    searchId: observation.searchId || "",
    revision: Number(observation.revision) || 0,
    progress: cloneJson(observation.progress || null),
    candidates: cloneJson(candidates),
    selectedCandidateId,
    materializedCandidates,
    pendingMaterializationId: preservePending
      ? previousPreview.pendingMaterializationId
      : "",
    pendingCandidateId: preservePending ? previousPreview.pendingCandidateId : "",
    pendingCandidateStateHash: preservePending
      ? previousPreview.pendingCandidateStateHash
      : "",
    displayCandidate: true,
    maxStoredNodes: observation.maxStoredNodes
      || activeLevelSolveRequest?.request?.maxStoredNodes
      || null,
  };
  if (customSource) {
    customGoalSolverObservationPreview = nextPreview;
  } else {
    levelGoalSolverObservationPreview = nextPreview;
  }
  const activeMode = customSource ? "custom-goal" : "level-goal";
  solverPaneStatuses[activeMode] = {
    text: solverObservationStatus(nextPreview),
    className: "",
  };
  if (solverPaneMode !== activeMode) {
    return;
  }
  solverObservationPreview = nextPreview;
  levelSolutionPreview = null;
  if (activeMode === "level-goal") {
    levelGoalSolutionPreview = null;
    levelGoalSolveSummaryText = "";
  } else {
    customGoalSolutionPreview = null;
    customGoalSolveSummaryText = "";
  }
  renderSolverObservationPanel();
  renderSolverBoard();
  requestSelectedSolverCandidateMaterialization();
  updateSolutionControls();
  setLevelSolveStatus(solverPaneStatuses[activeMode].text, "");
}

function selectedSolverObservationCandidate() {
  const candidates = solverObservationPreview?.candidates;
  if (!Array.isArray(candidates) || !candidates.length) {
    return null;
  }
  return candidates.find(
    (candidate) => candidate.candidateId === solverObservationPreview.selectedCandidateId,
  ) || candidates[0];
}

function selectedSolverMaterializedCandidate() {
  const candidate = selectedSolverObservationCandidate();
  if (!candidate) {
    return null;
  }
  const materialized = solverObservationPreview?.materializedCandidates?.get(candidate.candidateId);
  return materialized?.candidateStateHash === candidate.stateHash ? materialized : null;
}

function selectSolverObservationCandidate(candidateId) {
  if (!solverObservationPreview?.candidates?.some(
    (candidate) => candidate.candidateId === candidateId,
  )) {
    return;
  }
  solverObservationPreview.selectedCandidateId = candidateId;
  solverObservationPreview.displayCandidate = true;
  renderSolverObservationPanel();
  renderSolverBoard();
  requestSelectedSolverCandidateMaterialization();
}

function requestSelectedSolverCandidateMaterialization() {
  const candidate = selectedSolverObservationCandidate();
  if (solverObservationPreview?.source === "agent") {
    requestAgentCandidateMaterialization(candidate);
    return;
  }
  const request = activeLevelSolveRequest;
  if (!candidate || !request?.worker || selectedSolverMaterializedCandidate()) {
    return;
  }
  if (solverObservationPreview.pendingCandidateId === candidate.candidateId
    && solverObservationPreview.pendingCandidateStateHash === candidate.stateHash) {
    return;
  }
  const materializationId = createDocumentId();
  solverObservationPreview.pendingMaterializationId = materializationId;
  solverObservationPreview.pendingCandidateId = candidate.candidateId;
  solverObservationPreview.pendingCandidateStateHash = candidate.stateHash;
  request.worker.postMessage({
    type: "materialize-candidate",
    requestId: request.id,
    materializationId,
    candidateId: candidate.candidateId,
    wasm: wasmSolverWorkerConfig(),
  });
}

function requestAgentCandidateMaterialization(candidate) {
  if (!candidate || selectedSolverMaterializedCandidate()
    || !solverObservationPreview?.sessionId || !solverObservationPreview?.searchId) {
    return;
  }
  if (solverObservationPreview.pendingCandidateId === candidate.candidateId
    && solverObservationPreview.pendingCandidateStateHash === candidate.stateHash) {
    return;
  }
  const materializationId = createDocumentId();
  solverObservationPreview.pendingMaterializationId = materializationId;
  solverObservationPreview.pendingCandidateId = candidate.candidateId;
  solverObservationPreview.pendingCandidateStateHash = candidate.stateHash;
  postAgentInvestigationRequest({
    version: 3,
    requestId: materializationId,
    op: "materialize_search_candidate",
    sessionId: solverObservationPreview.sessionId,
    searchId: solverObservationPreview.searchId,
    candidateId: candidate.candidateId,
  }).catch((error) => {
    if (solverObservationPreview?.pendingMaterializationId === materializationId) {
      solverObservationPreview.pendingMaterializationId = "";
      solverObservationPreview.pendingCandidateId = "";
      solverObservationPreview.pendingCandidateStateHash = "";
    }
    setLevelSolveStatus(`Candidate preview unavailable: ${userFacingRuntimeError(error)}`, "is-error");
  });
}

async function postAgentInvestigationRequest(request) {
  const response = await fetch("/api/agent", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(request),
  });
  if (!response.ok) {
    throw new Error(await response.text() || `Agent request failed (${response.status})`);
  }
  return response.json();
}

function applyAgentObservationEvent(event) {
  agentInvestigation.events.push(cloneJson(event));
  if (agentInvestigation.events.length > 40) {
    agentInvestigation.events.splice(0, agentInvestigation.events.length - 40);
  }
  if (event.search) {
    agentInvestigation.search = cloneJson(event.search);
    const stats = event.search.stats || {};
    const candidates = Array.isArray(event.search.candidates)
      ? event.search.candidates.map((candidate) => ({
        ...candidate,
        moves: Array.isArray(candidate.inputs)
          ? candidate.inputs.map((name) => ({ name }))
          : [],
      }))
      : [];
    showSolverObservation({
      source: "agent",
      revision: event.sequence,
      sessionId: event.search.sessionId,
      searchId: event.search.searchId,
      maxStoredNodes: event.search.limits?.maxStoredNodes,
      progress: {
        visited: stats.visited || 0,
        expanded: stats.expanded || 0,
        frontier: stats.frontier || 0,
        advisoryCutoffs: stats.advisoryCutoffs || 0,
        maxDepthReached: stats.maxDepthReached || 0,
        depth: candidates[0]?.depth || 0,
      },
      candidates,
    });
  }
  if (event.materializedCandidate && customGoalSolverObservationPreview?.source === "agent") {
    const preview = customGoalSolverObservationPreview;
    const candidate = preview.candidates.find(
      (entry) => entry.candidateId === event.materializedCandidate.candidateId,
    );
    if (candidate) {
      const levelIndex = event.materializedCandidate.levelIndex;
      if (solverPaneMode === "custom-goal" && activeSolverTask?.level?.index !== levelIndex) {
        const observation = preview;
        solverObservationPreview = null;
        selectSolverLevel(levelIndex);
        solverObservationPreview = observation;
        customGoalSolverObservationPreview = observation;
        renderSolverObservationPanel();
      }
      preview.pendingMaterializationId = "";
      preview.pendingCandidateId = "";
      preview.pendingCandidateStateHash = "";
      preview.materializedCandidates.set(candidate.candidateId, {
        candidateId: candidate.candidateId,
        candidateStateHash: candidate.stateHash,
        stateHash: event.materializedCandidate.terminalHash,
        state: event.materializedCandidate.runtimeState,
        levelIndex: event.materializedCandidate.levelIndex,
      });
      if (solverPaneMode === "custom-goal") {
        solverObservationPreview = preview;
        renderSolverBoard();
      }
    }
  }
  renderAgentInvestigationPanel();
}

function renderAgentInvestigationPanel() {
  if (!agentInvestigationPanel) {
    return;
  }
  const events = agentInvestigation.events;
  agentInvestigationPanel.hidden = solverPaneMode !== "custom-goal" || events.length === 0;
  if (!events.length) {
    return;
  }
  const latest = events[events.length - 1];
  const search = agentInvestigation.search;
  agentInvestigationIdentity.textContent = [
    search?.sessionId || latest.sessionId,
    search?.searchId || latest.searchId,
  ].filter(Boolean).join(" / ");
  const summaries = events.slice(-6).map((event) => {
    const row = document.createElement("div");
    row.className = "agent-investigation-event";
    if (event.search) {
      row.textContent = `${event.search.status}: ${event.search.stats.expanded} expanded, ${event.search.stats.frontier} frontier, ${event.search.candidates.length} candidates`;
    } else if (event.materializedCandidate) {
      row.textContent = `${event.materializedCandidate.candidateId} materialized as ${event.materializedCandidate.terminalStateId}`;
    } else if (event.issue) {
      row.textContent = `${event.operation || "request"}: ${event.issue.message || event.issue.code}`;
    } else {
      row.textContent = `${event.operation || "request"}: complete`;
    }
    return row;
  });
  agentInvestigationEvents.replaceChildren(...summaries);
  const issues = events.filter((event) => event.issue).slice(-3);
  agentInvestigationIssues.hidden = issues.length === 0;
  agentInvestigationIssues.textContent = issues
    .map((event) => `${event.issue.code || "error"}: ${event.issue.message || "Agent request failed"}`)
    .join("\n");
  agentInvestigationRaw.textContent = events
    .slice(-12)
    .map((event) => `${event.raw?.request || ""}\n${event.raw?.response || ""}`)
    .join("\n\n");
}

function solverTaskDimensions(task = activeSolverTask) {
  const state = task?.state?.data;
  if (task?.rules?.modelKind === "3d") {
    return [state?.width, state?.depth, state?.height].map((value) => Number(value) || 0);
  }
  return [state?.width, state?.height].map((value) => Number(value) || 0);
}

function renderCustomGoalObjectOptions() {
  if (!solverCustomGoalObject) {
    return;
  }
  const previous = solverCustomGoalObject.value;
  const exact = solverCustomGoalPredicate?.value === "exact";
  const options = [];
  if (exact) {
    const empty = document.createElement("option");
    empty.value = "";
    empty.textContent = "Empty cell";
    options.push(empty);
  } else {
    const placeholder = document.createElement("option");
    placeholder.value = "";
    placeholder.textContent = "Choose object";
    placeholder.disabled = true;
    options.push(placeholder);
  }
  for (const name of activeSolverTask?.goalObjects || []) {
    const option = document.createElement("option");
    option.value = name;
    option.textContent = name;
    options.push(option);
  }
  solverCustomGoalObject.replaceChildren(...options);
  if (options.some((option) => option.value === previous)) {
    solverCustomGoalObject.value = previous;
  }
}

function renderCustomGoalEditor() {
  if (!solverCustomGoalEditor) {
    return;
  }
  const visible = solverPaneMode === "custom-goal";
  solverCustomGoalEditor.hidden = !visible;
  if (!visible) {
    return;
  }
  const dimensions = solverTaskDimensions();
  const coordinateInputs = [solverCustomGoalX, solverCustomGoalY, solverCustomGoalZ];
  coordinateInputs.forEach((input, index) => {
    if (!input) return;
    const bound = dimensions[index] || 0;
    input.max = String(Math.max(0, bound - 1));
    input.disabled = bound === 0 || Boolean(activeLevelSolveRequest);
    if (Number(input.value) >= bound) {
      input.value = String(Math.max(0, bound - 1));
    }
  });
  solverCustomGoalZField.hidden = dimensions.length !== 3;
  renderCustomGoalObjectOptions();
  solverCustomGoalPredicate.disabled = !activeSolverTask || Boolean(activeLevelSolveRequest);
  solverCustomGoalObject.disabled = !activeSolverTask || Boolean(activeLevelSolveRequest);
  const goalObjectReady = solverCustomGoalPredicate.value === "exact"
    || Boolean(solverCustomGoalObject.value);
  solverCustomGoalAddButton.disabled = !activeSolverTask
    || !goalObjectReady
    || Boolean(activeLevelSolveRequest);

  const constraints = customGoalConstraints();
  if (!constraints.length) {
    const empty = document.createElement("span");
    empty.className = "solver-custom-goal-empty";
    empty.textContent = activeSolverTask
      ? "Add at least one condition, then start the solver."
      : "Choose a level to define its goal.";
    solverCustomGoalList.replaceChildren(empty);
    return;
  }
  const labels = { contains: "Contains", excludes: "Excludes", exact: "Exactly" };
  const rows = constraints.map((constraint, index) => {
    const row = document.createElement("div");
    row.className = "solver-custom-goal-condition";
    const description = document.createElement("span");
    const objects = constraint.objects.length ? constraint.objects.join(" + ") : "empty";
    description.textContent = `${labels[constraint.predicate]} ${objects} at (${constraint.position.join(", ")})`;
    const remove = document.createElement("button");
    remove.type = "button";
    remove.className = "icon-button";
    remove.title = "Remove goal condition";
    remove.setAttribute("aria-label", `Remove goal condition ${index + 1}`);
    remove.disabled = Boolean(activeLevelSolveRequest);
    remove.append(window.editorIconElement("trash-2"));
    remove.addEventListener("click", () => removeCustomGoalConstraint(index));
    row.append(description, remove);
    return row;
  });
  solverCustomGoalList.replaceChildren(...rows);
}

function addCustomGoalConstraint() {
  const key = solverTaskBaseKey();
  const dimensions = solverTaskDimensions();
  if (!key || !dimensions.length) {
    setLevelSolveStatus("Choose a level before defining a custom goal", "is-error");
    return;
  }
  const inputs = [solverCustomGoalX, solverCustomGoalY, solverCustomGoalZ];
  const position = dimensions.map((bound, index) => Math.trunc(Number(inputs[index]?.value)));
  if (position.some((coordinate, index) => (
    !Number.isInteger(coordinate) || coordinate < 0 || coordinate >= dimensions[index]
  ))) {
    setLevelSolveStatus("Goal coordinates are outside the selected level", "is-error");
    return;
  }
  const predicate = solverCustomGoalPredicate.value;
  const object = solverCustomGoalObject.value;
  if (predicate !== "exact" && !object) {
    setLevelSolveStatus("Contains and excludes require an object", "is-error");
    return;
  }
  const constraints = customGoalConstraints().map((constraint) => cloneJson(constraint));
  const exactIndex = predicate === "exact"
    ? constraints.findIndex((constraint) => (
      constraint.predicate === "exact"
      && constraint.position.length === position.length
      && constraint.position.every((coordinate, index) => coordinate === position[index])
    ))
    : -1;
  if (exactIndex >= 0) {
    constraints[exactIndex].objects = object
      ? [...new Set([...constraints[exactIndex].objects, object])]
      : [];
  } else {
    constraints.push({ position, predicate, objects: object ? [object] : [] });
  }
  customGoalConstraintsByTask.set(key, constraints);
  completedSolverTaskKey = "";
  clearSolutionPreview({ preserveSolverTask: true });
  solverPaneStatuses["custom-goal"] = { text: "Ready to solve custom goal", className: "" };
  setLevelSolveStatus("Ready to solve custom goal", "");
  renderCustomGoalEditor();
}

function removeCustomGoalConstraint(index) {
  const key = solverTaskBaseKey();
  if (!key) return;
  const constraints = customGoalConstraints().filter((_, candidate) => candidate !== index);
  customGoalConstraintsByTask.set(key, constraints);
  completedSolverTaskKey = "";
  clearSolutionPreview({ preserveSolverTask: true });
  const text = constraints.length ? "Ready to solve custom goal" : "Add a goal condition";
  solverPaneStatuses["custom-goal"] = { text, className: "" };
  setLevelSolveStatus(text, "");
  renderCustomGoalEditor();
}

function solverObservationStatus(observation) {
  const progress = observation?.progress || {};
  const visited = Number.isFinite(progress.visited) ? progress.visited : null;
  const depth = Number.isFinite(progress.depth) ? progress.depth : null;
  const parts = [];
  if (visited !== null) {
    parts.push(`${visited} states`);
  }
  if (depth !== null) {
    parts.push(`depth ${depth}`);
  }
  return parts.length ? `Searching: ${parts.join(", ")}` : "Searching";
}

function syncSolverPaneModeControls() {
  const customGoal = solverPaneMode === "custom-goal";
  solverCustomGoalToggleButton?.setAttribute("aria-pressed", String(customGoal));
  renderCustomGoalEditor();
  renderAgentInvestigationPanel();
}

function setSolverPaneMode(mode) {
  const nextMode = mode === "custom-goal" ? "custom-goal" : "level-goal";
  if (nextMode === solverPaneMode) {
    syncSolverPaneModeControls();
    return;
  }
  stopSolutionPlayback();
  stopSolverObservationPlayback();
  if (solverPaneMode === "level-goal") {
    levelGoalSolverTask = activeSolverTask;
    levelGoalSolutionPreview = levelSolutionPreview;
    levelGoalSolveSummaryText = levelSolveSummaryText;
    customGoalSolverTask ||= cloneJson(activeSolverTask);
  } else {
    customGoalSolverTask = activeSolverTask;
    customGoalSolutionPreview = levelSolutionPreview;
    customGoalSolveSummaryText = levelSolveSummaryText;
  }
  solverPaneMode = nextMode;
  if (nextMode === "level-goal") {
    activeSolverTask = levelGoalSolverTask;
    levelSolutionPreview = levelGoalSolutionPreview;
    levelSolveSummaryText = levelGoalSolveSummaryText;
    solverObservationPreview = levelGoalSolverObservationPreview;
  } else {
    activeSolverTask = customGoalSolverTask;
    levelSolutionPreview = customGoalSolutionPreview;
    levelSolveSummaryText = customGoalSolveSummaryText;
    solverObservationPreview = customGoalSolverObservationPreview;
  }
  syncSolverPaneModeControls();
  syncSolverLevelSelector();
  syncSolverTaskReadout();
  setSolveLevelButtonState(Boolean(activeLevelSolveRequest));
  renderSolverObservationPanel();
  renderSolverBoard();
  updateSolutionControls();
  if (nextMode === "custom-goal" && solverObservationPreview) {
    requestSelectedSolverCandidateMaterialization();
  }
  const status = solverPaneStatuses[nextMode];
  setLevelSolveStatus(status?.text || "", status?.className || "");
}

async function pollAgentObservations() {
  if (window.PuzzleStudioHost.mode() !== "server" || agentObservationPolling) {
    return;
  }
  agentObservationPolling = true;
  try {
    const response = await fetch(`/api/agent-observations?after=${agentObservationCursor}`, {
      cache: "no-store",
    });
    if (!response.ok) {
      return;
    }
    const payload = await response.json();
    for (const event of Array.isArray(payload.events) ? payload.events : []) {
      applyAgentObservationEvent(event);
    }
    agentObservationCursor = Math.max(agentObservationCursor, Number(payload.cursor) || 0);
  } catch (error) {
    console.error("Agent observation polling failed", error);
  } finally {
    agentObservationPolling = false;
    window.setTimeout(pollAgentObservations, 500);
  }
}

function handleSolverCandidateMaterialized(message) {
  if (!solverObservationPreview
    || message.materializationId !== solverObservationPreview.pendingMaterializationId
    || !message.candidate?.candidateId) {
    return;
  }
  solverObservationPreview.pendingMaterializationId = "";
  solverObservationPreview.pendingCandidateId = "";
  solverObservationPreview.pendingCandidateStateHash = "";
  solverObservationPreview.materializedCandidates.set(
    message.candidate.candidateId,
    cloneJson(message.candidate),
  );
  renderSolverBoard();
}

function handleSolverCandidateMaterializationError(message) {
  if (!solverObservationPreview
    || message.materializationId !== solverObservationPreview.pendingMaterializationId) {
    return;
  }
  solverObservationPreview.pendingMaterializationId = "";
  solverObservationPreview.pendingCandidateId = "";
  solverObservationPreview.pendingCandidateStateHash = "";
  setLevelSolveStatus(`Candidate preview unavailable: ${userFacingWorkerError(message.error)}`, "is-error");
}

function solverCandidateMoves(candidate) {
  const names = Array.isArray(candidate?.moves)
    ? candidate.moves.map((move) => String(move?.name || "")).filter(Boolean)
    : [];
  return names.length ? names.join(" ") : "Initial state";
}

function renderSolverObservationPanel() {
  if (!solverObservationPanel || !solverCandidateList) {
    return;
  }
  const progress = solverObservationPreview?.progress;
  const candidates = solverObservationPreview?.candidates;
  const visible = progress && Array.isArray(candidates) && candidates.length > 0;
  solverObservationPanel.hidden = !visible;
  if (!visible) {
    solverCandidateList.replaceChildren();
    return;
  }
  const storedLimit = Number(solverObservationPreview.maxStoredNodes);
  solverVisitedValue.textContent = Number.isFinite(storedLimit) && storedLimit > 0
    ? `${progress.visited} / ${storedLimit}`
    : String(progress.visited);
  solverExpandedValue.textContent = `${progress.expanded} / ${progress.frontier}`;
  solverDepthValue.textContent = `${progress.depth} / ${progress.maxDepthReached}`;
  solverCutoffValue.textContent = String(progress.advisoryCutoffs);
  const selected = selectedSolverObservationCandidate();
  const buttons = candidates.map((candidate, index) => {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "solver-candidate-button";
    button.setAttribute("role", "option");
    const isSelected = candidate.candidateId === selected?.candidateId;
    button.classList.toggle("is-selected", isSelected);
    button.setAttribute("aria-selected", isSelected ? "true" : "false");
    const summary = document.createElement("span");
    summary.className = "solver-candidate-summary";
    summary.textContent = `#${index + 1}  score ${candidate.score}  depth ${candidate.depth}`;
    const moves = document.createElement("span");
    moves.className = "solver-candidate-moves";
    moves.textContent = solverCandidateMoves(candidate);
    button.append(summary, moves);
    button.addEventListener("click", () => {
      selectSolverObservationCandidate(candidate.candidateId);
    });
    return button;
  });
  solverCandidateList.replaceChildren(...buttons);
}

function setLevelSolveStatus(text, className = "") {
  solverPaneStatuses[solverPaneMode] = { text, className };
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
  return message;
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
  levelSolutionPreview = {
    steps,
    moves: solutionMoves(solution),
    index: 0,
  };
  if (solverObservationPreview) {
    solverObservationPreview.displayCandidate = false;
  }
  renderSolverObservationPanel();
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

function sceneCellsToSlots(scene) {
  return (scene?.cells || []).map((cell) => cellSlotsFromLayers(cell.layers || []));
}

function stateDataToLevelCells(stateData, exportData = currentPreviewExportData()) {
  const scene = sceneFromStateData(stateData, {
    regions: levelRegions(),
    exportData,
  });
  const cells = sceneCellsToSlots(scene);
  return cells.length === level.cells.length ? cells : null;
}

function setSolutionStep(index) {
  if (!levelSolutionPreview) {
    return;
  }
  const nextIndex = Math.max(0, Math.min(levelSolutionPreview.steps.length - 1, index));
  levelSolutionPreview.index = nextIndex;
  if (solverObservationPreview) {
    solverObservationPreview.displayCandidate = false;
  }
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
  stopSolverObservationPlayback();
  levelSolutionPreview = null;
  solverObservationPreview = null;
  if (solverPaneMode === "custom-goal") {
    customGoalSolverObservationPreview = null;
    customGoalSolutionPreview = null;
    customGoalSolveSummaryText = "";
  } else {
    levelGoalSolverObservationPreview = null;
    levelGoalSolutionPreview = null;
    levelGoalSolveSummaryText = "";
  }
  renderSolverObservationPanel();
  if (options.preserveSolverTask !== true) {
    activeSolverTask = null;
    if (solverPaneMode === "custom-goal") {
      customGoalSolverTask = null;
    } else {
      levelGoalSolverTask = null;
    }
    syncSolverTaskReadout();
  }
  setSolveLevelButtonState(Boolean(activeLevelSolveRequest));
  levelSolveSummaryText = "";
  levelSolveStatus.title = "";
  updateSolutionControls();
  if (currentPreviewMode === "level3d" && typeof renderLevel3dBuilder === "function") {
    renderLevel3dBuilder();
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
  return typeof dispatchEditorCommandEvent === "function"
    ? dispatchEditorCommandEvent(event, { group: "solver" })
    : false;
}

function formatNumber(value) {
  return new Intl.NumberFormat("en-US").format(Number(value) || 0);
}

function formatSeconds(milliseconds) {
  return `${((Number(milliseconds) || 0) / 1000).toFixed(1)}s`;
}

function previewAcceptsKeyboardInput(controller = previewEditorRuntimeController()) {
  return Boolean(
    currentPreviewMode === "play"
    && controller?.ready
    && controller.surface?.isConnected
    && controller.frame?.contentWindow
  );
}

function focusPreviewInputTarget(controller = previewEditorRuntimeController()) {
  if (!previewAcceptsKeyboardInput(controller)) {
    previewKeyboardFocusOwned = false;
    return false;
  }
  controller.frame.focus({ preventScroll: true });
  controller.frame.contentWindow.focus();
  previewKeyboardFocusOwned = document.activeElement === controller.frame;
  return previewKeyboardFocusOwned;
}

async function copyLevelToClipboard() {
  const levelName = sanitizeLevelName(levelNameInput.value);
  try {
    const exportData = currentLevelExportData();
    const text = await exportData.session.formatLevelSource(currentEditableLevelIndex(exportData), levelName);
    await copyTextToClipboard(text);
    setStatus(levelName ? `Copied level ${levelName}` : "Copied unnamed level", "is-ok");
  } catch (error) {
    setStatus(levelShared.sourceActionErrorMessage(error, "Could not copy level"), "is-error");
  }
}

async function addLevelToSource() {
  const previewDocument = activePreviewDocument();
  if (!previewDocument) {
    setStatus("No game entry for level", "is-error");
    return;
  }
  const levelName = sanitizeLevelName(levelNameInput.value);
  const levelNamespace = sanitizeLevelNamespace(levelNamespaceInput.value);
  const source = activePreviewSource();
  const exportData = currentLevelExportData();
  let transition;
  try {
    transition = await exportData.session.insertLevelSource(currentEditableLevelIndex(exportData), {
      expectedSource: source,
      name: levelName,
      namespace: levelNamespace,
      cursor: sourceEditor.selection().from,
      createContainer: false,
    });
  } catch (error) {
    setStatus(levelShared.sourceActionErrorMessage(error, "Could not add level"), "is-error");
    return;
  }
  if (!transition?.sourceUpdate
    || !applyPuzzleSourceMutation(previewDocument, source, transition.sourceUpdate.source)) {
    setStatus("Level source changed while the Rust edit was being applied.", "is-error");
    return;
  }
  applyLevelSessionSnapshot(transition.snapshot, exportData);
  setLevelEditSource({
    start: transition.sourceUpdate.start,
    end: transition.sourceUpdate.end,
    name: levelName,
  }, previewDocument);
  levelNameInput.value = nextLevelName(levelName);
  syncLevelNameOptions();
}

function setLevelEditSource(entry, document = activeDocument()) {
  level.editDocumentId = document && isTextDocument(document) && isPuzzleDocument(document)
    ? document.id
    : null;
  level.editSourceStart = Number.isInteger(entry?.start) ? entry.start : null;
  level.editSourceEnd = Number.isInteger(entry?.end) ? entry.end : null;
  level.editSourceBodyStart = Number.isInteger(entry?.bodyStart) ? entry.bodyStart : null;
  level.editSourceBodyEnd = Number.isInteger(entry?.bodyEnd) ? entry.bodyEnd : null;
  level.editSourceName = entry?.name || "";
}

function clearLevelEditSource() {
  level.editSourceStart = null;
  level.editSourceEnd = null;
  level.editSourceBodyStart = null;
  level.editSourceBodyEnd = null;
  level.editSourceName = "";
}

function invalidateLevelEditSourceForDocument(document = activeDocument()) {
  if (!document || !level.editDocumentId || document.id !== level.editDocumentId) {
    return false;
  }
  clearLevelEditSource();
  return true;
}

function activeLevelEditDocument() {
  return documents.find((candidate) => candidate.id === level.editDocumentId) || null;
}

function activeLevelEditSource() {
  const document = activeLevelEditDocument();
  if (!document || !isTextDocument(document)) {
    return "";
  }
  return document.id === activeDocument()?.id
    ? sourceEditorDocumentValue()
    : document.source || "";
}

function currentLevelEditSourceRange(source) {
  const start = level.editSourceStart;
  const end = level.editSourceEnd;
  if (
    !Number.isInteger(start)
    || !Number.isInteger(end)
    || start < 0
    || end < start
    || end > String(source || "").length
  ) {
    return null;
  }
  const entry = sourceEditableEntryFromTarget(source, {
    start,
    end,
    bodyStart: level.editSourceBodyStart,
    bodyEnd: level.editSourceBodyEnd,
    name: level.editSourceName,
  }, { defaultName: "" });
  return entry && Number.isInteger(entry.start) && Number.isInteger(entry.end) ? entry : null;
}

async function updateLevelInSource() {
  const editDocument = activeLevelEditDocument();
  if (!editDocument || !isPuzzleDocument(editDocument) || !isTextDocument(editDocument)) {
    setStatus("No editable level source loaded", "is-error");
    return;
  }
  const levelName = sanitizeLevelName(levelNameInput.value);
  const source = activeLevelEditSource();
  const entry = currentLevelEditSourceRange(source);
  if (!entry) {
    setStatus("No typed level source target is selected", "is-error");
    return;
  }
  const transition = await dispatchLevelSessionCommand({
    type: "updateSource",
    name: levelName,
    expectedSource: source,
  });
  if (!transition?.sourceUpdate) {
    return;
  }
  setStatus(levelName ? `Updated level ${levelName}` : "Updated unnamed level", "is-ok");
}

function sanitizeLevelName(value) {
  return sourcePuzzleLevelName(editableLevelName(value));
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
  return generatedUnnamedLevelName(fallbackName) ? "" : fallbackName;
}

function generatedUnnamedLevelName(value) {
  return /^unnamed level \d+$/i.test(String(value || "").trim());
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

function currentLevelAuthoringSource(exportData = currentLevelExportData()) {
  return activeLevelEditSource() || activePreviewSource() || levelReferenceSource(exportData);
}

async function addLevelPaletteObjectToLegend(object) {
  const exportData = currentLevelExportData();
  const objectName = String(object?.name || "").trim();
  const objectEntry = engineObjects(exportData).find((candidate) => candidate.name === objectName);
  if (!objectEntry) {
    setStatus("No compiled object metadata for tile legend", "is-error");
    return false;
  }
  const editDocument = activeLevelEditDocument() || activePreviewDocument();
  if (!editDocument || !isPuzzleDocument(editDocument) || !isTextDocument(editDocument)) {
    setStatus("No editable puzzle source for tile legend", "is-error");
    return false;
  }
  const source = editDocument.id === activeDocument()?.id
    ? sourceEditorDocumentValue()
    : editDocument.source || "";
  if (sourcePlaceableObjectNames(source, exportData).has(objectName)) {
    level.addPaletteOpen = false;
    const paletteSource = source;
    level.palette = levelPaletteFromExport(paletteSource, exportData);
    level.selectedObjectId = objectEntry.id;
    setLevelActiveLayerForObject(objectEntry.id);
    renderLevelPalette();
    return true;
  }
  let result;
  try {
    result = await levelSourceRequest(source, {
      operation: "insertLegendAuto",
      selectors: [objectName],
    });
  } catch (error) {
    setStatus(`Could not add tile legend: ${error?.message || error}`, "is-error");
    return false;
  }
  const nextSource = result.source;
  if (!applyPuzzleSourceMutation(editDocument, source, nextSource)) {
    setStatus("Level source changed while the edit was being prepared; retry the edit.", "is-error");
    return false;
  }
  level.addPaletteOpen = false;
  level.palette = levelPaletteFromExport(nextSource, exportData);
  level.selectedObjectId = objectEntry.id;
  setLevelActiveLayerForObject(objectEntry.id);
  renderLevelPalette();
  renderLevelBoard();
  setStatus(`Added ${objectName} to the level legend`, "is-ok");
  return true;
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

function sourceCharEntries(_source, exportData = currentLevelExportData()) {
  if (!exportData?.manifest) {
    throw new Error("Compiled level editor source contract is unavailable.");
  }
  const integratedLegend = levelEditorLevels(exportData)[currentEditableLevelIndex(exportData)]?.legend;
  const objectNames = new Map(engineObjects(exportData).map((object) => [object.id, object.name]));
  const entries = (integratedLegend || exportData.manifest.legend || []).map((entry) => ({
    char: String(entry?.symbol || ""),
    objects: Array.isArray(entry?.objectIds)
      ? entry.objectIds.map((id) => objectNames.get(Number(id))).filter(Boolean)
      : [],
  }));
  if (!entries.some((entry) => entry.objects.length === 0)) {
    entries.unshift({ char: ".", objects: [] });
  }
  return entries
    .filter((entry) => entry.char.length === 1)
    .sort((left, right) => right.objects.length - left.objects.length);
}

function levelSourceLegendDrafts(localLegends) {
  return (Array.isArray(localLegends) ? localLegends : []).map((entry) => ({
    symbol: String(entry?.char || ""),
    selectors: Array.isArray(entry?.objects) ? entry.objects.map(String) : [],
  }));
}

function sourceTitleMatches(existing, title, namespace = "") {
  const existingTitle = String(existing || "").trim();
  const requested = String(title || "").trim();
  const requestedNamespace = sanitizeLevelNamespace(editableLevelNamespace(title) || namespace);
  const existingNamespace = sanitizeLevelNamespace(editableLevelNamespace(existingTitle) || namespace);
  const editableExisting = editableLevelName(existingTitle);
  const editableRequested = editableLevelName(requested);
  return existingTitle === requested
    || (editableRequested && existingTitle.endsWith(`.${editableRequested}`))
    || (
      editableExisting === editableRequested
      && (!requestedNamespace || !existingNamespace || requestedNamespace === existingNamespace)
    );
}

function lineIndent(line) {
  return String(line || "").match(/^[\t ]*/)?.[0] || "";
}

runButton.addEventListener("click", () => {
  runPreviewFromSourcePane();
});
previewRefreshButton?.addEventListener("click", () => {
  refreshPreviewFromPreviewPane();
});
clearPreviewLogButton?.addEventListener("click", clearPreviewLog);
previewDebugToggleButton?.addEventListener("click", () => setPreviewDebugEnabled(!previewDebugEnabled));
previewDebugPrevButton?.addEventListener("click", () => setPreviewDebugCursor(previewDebugCursor - 1));
previewDebugNextButton?.addEventListener("click", () => setPreviewDebugCursor(previewDebugCursor + 1));
previewDebugLatestButton?.addEventListener("click", () => {
  const executions = previewDebugTrace?.executions || [];
  setPreviewDebugCursor(executions.length - 1);
});
previewLogOutput?.addEventListener("click", activatePreviewLogLocationFromEvent);
previewLogOutput?.addEventListener("keydown", (event) => {
  if (event.key !== "Enter" && event.key !== " ") {
    return;
  }
  activatePreviewLogLocationFromEvent(event);
});
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
loadExamplesButton?.addEventListener("click", () => {
  setFileActionsMenuOpen(false);
  openEditorExamplePicker().catch((error) => {
    console.error(error);
    setEditorStatus(error.message || "Examples unavailable", "is-error");
  });
});
examplePickerCancelButton?.addEventListener("click", () => {
  examplePickerDialog?.close();
});
examplePickerList?.addEventListener("click", (event) => {
  const button = event.target.closest("[data-download-example]");
  if (!button || !examplePickerList.contains(button)) {
    return;
  }
  button.disabled = true;
  loadEditorExample(button.dataset.downloadExample || "").then(() => {
    examplePickerDialog?.close();
  }).catch((error) => {
    button.disabled = false;
    console.error(error);
    setEditorStatus(error.message || "Example unavailable", "is-error");
  });
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
downloadButton.addEventListener("click", downloadWebBundle);
colorSchemeToggleButton?.addEventListener("click", toggleEditorColorScheme);
importFileInput.addEventListener("change", () => {
  importFiles(importFileInput.files).catch((error) => {
    console.error(error);
    setEditorStatus(`Open failed: ${importErrorMessage(error)}`, "is-error");
  });
  importFileInput.value = "";
});
importFolderInput.addEventListener("change", () => {
  importFiles(importFolderInput.files).catch((error) => {
    console.error(error);
    setEditorStatus(`Open failed: ${importErrorMessage(error)}`, "is-error");
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
let treePointerDrag = null;
let suppressNextTreeClick = false;

function createTreeDragPreview(drag) {
  const preview = document.createElement("div");
  preview.className = "tree-drag-preview";
  preview.setAttribute("aria-hidden", "true");
  const icon = drag.row.querySelector(".tree-icon")?.cloneNode(true);
  if (icon) {
    preview.append(icon);
  }
  const label = document.createElement("span");
  label.textContent = drag.row.querySelector(".tree-label")?.textContent?.trim()
    || findNode(fileTree, drag.nodeId)?.name
    || "Item";
  preview.append(label);
  document.body.append(preview);
  drag.preview = preview;
}

function updateTreeDragFeedback(drag, clientX, clientY) {
  drag.preview.style.transform = `translate3d(${clientX + 12}px, ${clientY + 12}px, 0)`;
  const targetFolderId = dropFolderIdForPoint(clientX, clientY);
  const allowed = canDropNodeOnFolder(drag.nodeId, targetFolderId);
  drag.preview.classList.toggle("is-invalid", !allowed);
  if (allowed) {
    markDropTarget(resolvedDropFolderIdForNode(drag.nodeId, targetFolderId));
  } else {
    clearDropTargets();
  }
}

function clearTreeDragFeedback(drag) {
  drag?.row?.classList.remove("is-dragging");
  drag?.preview?.remove();
  clearDropTargets();
}

function finishTreeMove(nodeId, targetFolderId) {
  moveNodeToFolder(nodeId, targetFolderId).then((moved) => {
    if (moved) {
      setEditorStatus("Moved", "is-ok");
    }
  }).catch((error) => {
    console.error(error);
    setEditorStatus(workspaceMutationErrorMessage("Move failed", error), "is-error");
  });
}

documentList.addEventListener("click", (event) => {
  if (suppressNextTreeClick) {
    suppressNextTreeClick = false;
    event.preventDefault();
    event.stopPropagation();
    return;
  }
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
      folder.expanded = folder.expanded === false;
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
documentList.addEventListener("pointerdown", (event) => {
  if (event.button !== 0 || event.target.closest("input, button, [data-tree-action], .tree-actions")) {
    return;
  }
  const row = event.target.closest(".tree-row");
  if (!row?.dataset.dragId || row.classList.contains("draft-row")) {
    return;
  }
  // The tree owns this pointer gesture. Leaving the browser's native text
  // selection active competes with the file move once the drag threshold is
  // crossed and can leave a filename range selected instead of moving it.
  event.preventDefault();
  row.setPointerCapture?.(event.pointerId);
  treePointerDrag = {
    nodeId: row.dataset.dragId,
    pointerId: event.pointerId,
    row,
    startX: event.clientX,
    startY: event.clientY,
    active: false,
  };
  resetTreeDragDecisionCache();
});
document.addEventListener("pointermove", (event) => {
  if (!treePointerDrag || event.pointerId !== treePointerDrag.pointerId) {
    return;
  }
  const dx = event.clientX - treePointerDrag.startX;
  const dy = event.clientY - treePointerDrag.startY;
  if (!treePointerDrag.active && Math.hypot(dx, dy) < 6) {
    return;
  }
  if (!treePointerDrag.active) {
    treePointerDrag.active = true;
    draggedNodeId = treePointerDrag.nodeId;
    treePointerDrag.row.classList.add("is-dragging");
    createTreeDragPreview(treePointerDrag);
  }
  updateTreeDragFeedback(treePointerDrag, event.clientX, event.clientY);
});
document.addEventListener("pointerup", (event) => {
  if (!treePointerDrag || event.pointerId !== treePointerDrag.pointerId) {
    return;
  }
  const drag = treePointerDrag;
  treePointerDrag = null;
  clearTreeDragFeedback(drag);
  resetTreeDragDecisionCache();
  draggedNodeId = "";
  if (!drag.active) {
    return;
  }
  event.preventDefault();
  event.stopPropagation();
  suppressNextTreeClick = true;
  const targetFolderId = dropFolderIdForPoint(event.clientX, event.clientY);
  if (canDropNodeOnFolder(drag.nodeId, targetFolderId)) {
    finishTreeMove(drag.nodeId, targetFolderId);
  }
});
document.addEventListener("pointercancel", (event) => {
  if (!treePointerDrag || event.pointerId !== treePointerDrag.pointerId) {
    return;
  }
  clearTreeDragFeedback(treePointerDrag);
  treePointerDrag = null;
  draggedNodeId = "";
  resetTreeDragDecisionCache();
});
function dataTransferHasFiles(dataTransfer) {
  return Array.from(dataTransfer?.types || []).includes("Files");
}

documentList.addEventListener("dragover", (event) => {
  const hasExternalFiles = dataTransferHasFiles(event.dataTransfer);
  if (!hasExternalFiles) {
    return;
  }
  const targetFolderId = dropFolderIdForEvent(event);
  if (isDesktopHost()) {
    return;
  }
  event.preventDefault();
  event.dataTransfer.dropEffect = "copy";
  markDropTarget(targetFolderId);
});
documentList.addEventListener("dragleave", (event) => {
  if (!documentList.contains(event.relatedTarget)) {
    clearDropTargets();
  }
});
documentList.addEventListener("drop", (event) => {
  const files = event.dataTransfer?.files;
  if (!files?.length) {
    return;
  }
  event.preventDefault();
  const targetFolderId = dropFolderIdForEvent(event);
  clearDropTargets();
  if (isDesktopHost()) {
    setEditorStatus("Use Open file or Open folder in the desktop app", "is-error");
    return;
  }
  const targetFolder = targetFolderId ? findNode(fileTree, targetFolderId) : fileTree;
  if (targetFolder?.kind === "folder") {
    importFilesIntoFolder(files, targetFolder).catch((error) => {
      console.error(error);
      setEditorStatus(`Import failed: ${importErrorMessage(error)}`, "is-error");
    });
  }
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
window.addEventListener("message", async (event) => {
  const controller = editorRuntimeControllerByWindow.get(event.source);
  if (
    !controller
    || event.origin !== window.location.origin
  ) {
    return;
  }
  const fromSolverRuntime = controller.consumer === "solver";
  const consumerHandler = editorRuntimeConsumerHandler(controller);
  if (event.data?.type === "PuzzleStudioPreviewLoaded") {
    markEditorRuntimeHostLoaded(controller);
    if (consumerHandler) {
      consumerHandler.loaded?.(controller, event.data);
      return;
    }
    if (fromSolverRuntime) return;
    const session = ensurePreviewSession();
    if (!session) return;
    session.runtimeStatus = {
      title: event.data.title || "",
      href: event.data.href || "",
    };
    if (isPuzzle3dExport(currentPreviewExportData())) {
      setStatus("Starting 3D runtime", "");
    }
    return;
  }
  if (event.data?.type === "PuzzleStudioPreviewRuntimeReady") {
    if (!acceptEditorRuntimeReady(controller, event.source, event.data)) return;
    controller.displayKey = "";
    controller.visualDraftKey = "";
    controller.pending = null;
    delete controller.surface.dataset.pendingCommandId;
    if (consumerHandler) {
      consumerHandler.ready?.(controller, event.data);
      return;
    }
    if (controller.consumer === "preview" && controller === previewEditorRuntimeController()) {
      markPreviewPresentationReady(controller);
    }
    return;
  }
  if (event.data?.type === "PuzzleStudioPreviewRuntimeError") {
    const label = String(event.data.label || "runtime failed");
    const message = String(event.data.message || "unknown error");
    if (rejectPendingEditorRuntimeGeneration(controller, event.source, event.data)) {
      if (consumerHandler) {
        consumerHandler.error?.(controller, { label, message, command: null });
      } else if (fromSolverRuntime) {
        setLevelSolveStatus(`Solver ${label}: ${message}`, "is-error");
      } else if (controller.consumer === "authoring") {
        setPaneStatus("level", `Level ${label}: ${message}`, "is-error");
      } else {
        setStatus(`Preview ${label}: ${message}`, "is-error");
      }
      return;
    }
  }
  if (event.data?.type === "PuzzleStudioEditorPointer") {
    if (controller.consumer === "preview") {
      if (String(event.data.gesture || "") === "press") {
        previewKeyboardFocusOwned = true;
      }
      return;
    }
    if (consumerHandler?.pointer) {
      await consumerHandler.pointer(controller, event.data);
      return;
    }
    await dispatchEditorAuthoringPointer(controller, event.data);
    return;
  }
  const messageGenerationId = canonicalU64Identity(
    String(event.data?.runtimeGenerationId || ""),
  );
  if (
    messageGenerationId === null
    || (
      messageGenerationId !== controller.pendingGeneration?.generationId
      && messageGenerationId !== controller.activeGenerationId
    )
  ) {
    return;
  }
  if (event.data?.type === "PuzzleStudioPreviewRuntimeDiagnostic") {
    const message = String(event.data.message || "runtime diagnostic");
    appendPreviewLog("warn", message, { source: "runtime" });
    return;
  }
  if (event.data?.type === "PuzzleStudioEditorSaveShortcut") {
    invokeEditorCommand(
      "workspace.save",
      editorCommandContext(null, previewFrame, "button")
    );
    return;
  }
  if (event.data?.type === "PuzzleStudioPreviewRuntimeError") {
    const label = String(event.data.label || "runtime failed");
    const message = String(event.data.message || "unknown error");
    const commandId = u32CommandIdentity(event.data.commandId);
    const context = commandId === null ? null : editorRuntimeCommands.get(commandId);
    if (context) {
      editorRuntimeCommands.delete(commandId);
      const commandController = context.controller || controller;
      if (context.kind === "editorPointer") {
        commandController.pointerCommandPending = false;
        commandController.queuedAuthoringPointer = null;
      }
      editorRuntimeConsumerHandler(commandController)?.commandError?.(
        commandController,
        context,
      );
      if (commandController.pending?.commandId === commandId) {
        commandController.pending = null;
        commandController.displayKey = "";
        delete commandController.surface.dataset.pendingCommandId;
      }
      const commandConsumerHandler = editorRuntimeConsumerHandler(commandController);
      if (commandConsumerHandler) {
        commandConsumerHandler.error?.(commandController, { label, message, command: context });
      } else if (context.consumer === "solver") {
        setLevelSolveStatus(`Solver ${label}: ${message}`, "is-error");
      } else if (context.consumer === "authoring") {
        setPaneStatus("level", `Level ${label}: ${message}`, "is-error");
      } else {
        setStatus(`Preview ${label}: ${message}`, "is-error");
      }
      return;
    }
    controller.ready = false;
    controller.pending = null;
    controller.displayKey = "";
    delete controller.surface.dataset.pendingCommandId;
    controller.surface.dataset.runtimeReady = "false";
    controller.resolveReady?.(false);
    controller.resolveReady = null;
    if (controller.consumer === "preview" && controller === previewEditorRuntimeController()) {
      previewRuntimeReady = false;
    }
    if (consumerHandler) {
      consumerHandler.error?.(controller, { label, message, command: null });
    } else {
      setStatus(`Preview ${label}: ${message}`, "is-error");
    }
    return;
  }
  if (event.data?.type === "PuzzleStudioEditorAuthoringFrame") {
    const surfaceId = String(event.data.surfaceId || "");
    const frameRevision = canonicalU64Identity(event.data.frameRevision);
    if (
      !surfaceId
      || surfaceId !== controller.surfaceId
      || frameRevision === null
    ) {
      return;
    }
    editorRuntimeCommittedFrames.set(surfaceId, frameRevision);
    const frameController = editorRuntimeControllers.get(surfaceId);
    if (frameController) {
      frameController.surface.dataset.frameRevision = String(frameRevision);
    }
    return;
  }
  if (event.data?.type === "PuzzleStudioEditorAuthoringAccepted") {
    const commandId = u32CommandIdentity(event.data.commandId);
    const surfaceId = String(event.data.surfaceId || "");
    if (
      commandId !== null
      && surfaceId === controller.surfaceId
      && editorRuntimeCommands.get(commandId)?.controller === controller
    ) {
      controller.surface.dataset.acceptedCommandId = String(commandId);
    }
    return;
  }
  if (event.data?.type === "PuzzleStudioEditorAuthoringSubmitted") {
    const commandId = u32CommandIdentity(event.data.commandId);
    const surfaceId = String(event.data.surfaceId || "");
    if (
      commandId !== null
      && surfaceId === controller.surfaceId
      && editorRuntimeCommands.get(commandId)?.controller === controller
    ) {
      controller.surface.dataset.submittedCommandId = String(commandId);
    }
    return;
  }
  if (event.data?.type === "PuzzleStudioEditorAuthoringRendered") {
    const commandId = u32CommandIdentity(event.data.commandId);
    const surfaceId = String(event.data.surfaceId || "");
    if (
      commandId !== null
      && surfaceId === controller.surfaceId
      && editorRuntimeCommands.get(commandId)?.controller === controller
    ) {
      controller.surface.dataset.renderedCommandId = String(commandId);
    }
    return;
  }
  if (event.data?.type === "PuzzleStudioEditorAuthoringApplied") {
    const commandId = u32CommandIdentity(event.data.commandId);
    const context = commandId === null ? null : editorRuntimeCommands.get(commandId);
    const surfaceId = String(event.data.surfaceId || "");
    if (
      !context
      || context.consumer !== controller.consumer
      || context.controller !== controller
      || context.surfaceId !== surfaceId
      || commandId !== controller.pending?.commandId
    ) {
      return;
    }
    editorRuntimeCommands.delete(commandId);
    controller.displayKey = controller.pending.key;
    if (typeof context.visualDraftKey === "string") {
      controller.visualDraftKey = context.visualDraftKey;
    }
    controller.pending = null;
    controller.surface.dataset.commandId = String(commandId);
    if (controller.surface.dataset.pendingCommandId === String(commandId)) {
      delete controller.surface.dataset.pendingCommandId;
    }
    if (consumerHandler?.displayApplied) {
      consumerHandler.displayApplied(controller, context);
    } else {
      flushQueuedEditorAuthoringPointer(controller);
    }
    return;
  }
  if (event.data?.type === "PuzzleStudioEditorAuthoringHit") {
    if (consumerHandler?.hit) {
      await consumerHandler.hit(controller, event.data);
      return;
    }
    const commandId = u32CommandIdentity(event.data.commandId);
    const context = commandId === null ? null : editorRuntimeCommands.get(commandId);
    const surfaceId = String(event.data.surfaceId || "");
    const frameRevision = canonicalU64Identity(event.data.frameRevision);
    if (
      !context
      || frameRevision === null
      || context.kind !== "editorPointer"
      || context.surfaceId !== surfaceId
      || context.frameRevision !== frameRevision
      || context.controller !== controller
    ) {
      return;
    }
    editorRuntimeCommands.delete(commandId);
    if (event.data.hit && context.mutate) {
      await applyEditorAuthoringHit(event.data.hit, surfaceId, {
        erase: context.erase,
      });
    } else if (event.data.hit && context) {
      controller.surface.dataset.hoverTarget = JSON.stringify(event.data.hit);
    } else {
      delete controller.surface.dataset.hoverTarget;
    }
    if (
      (context.gesture === "release" || context.gesture === "leave")
      && controller.authoringHistory
    ) {
      const commit = { type: "commitEdit" };
      if (controller.authoringHistory.kind === "level") {
        await dispatchLevelSessionCommand(commit, { render: false });
      } else {
        await dispatchLevel3dSessionCommand(commit, { render: false });
      }
      controller.authoringHistory = null;
    }
    completeEditorAuthoringPointer(controller);
    return;
  }
  if (event.data?.type === "PuzzleStudioPreviewCommit") {
    const sessionRevision = Number(event.data.sessionRevision);
    if (Number.isInteger(sessionRevision)) {
      controller.surface.dataset.sessionRevision = String(sessionRevision);
    }
    const stateCommit = Number(event.data.stateCommit);
    if (Number.isInteger(stateCommit)) {
      controller.surface.dataset.stateCommit = String(stateCommit);
    }
    const commandId = u32CommandIdentity(event.data.commandId);
    const context = commandId === null ? null : editorRuntimeCommands.get(commandId);
    if (controller.consumer !== "preview" || (context && context.controller !== controller)) {
      return;
    }
    applyPreviewTheme(event.data.theme);
    setPreviewViewportAspect(event.data.aspectRatio);
    const previousState = previewSessionState();
    const levelIndex = Number.isInteger(Number(event.data.levelIndex))
      ? Math.trunc(Number(event.data.levelIndex))
      : previousState?.levelIndex ?? 0;
    setPreviewSessionState({
      ...previousState,
      levelIndex,
      activeModel: typeof event.data.activeModel === "string"
        ? event.data.activeModel
        : previousState?.activeModel ?? "",
      screen: event.data.screen || "",
      screenHasPuzzle: event.data.screenHasPuzzle !== false,
      levelCount: Math.max(0, Math.trunc(Number(event.data.levelCount) || 0)),
    });
    return;
  }
  if (event.data?.type === "PuzzleStudioPreviewState") {
    const sessionRevision = Number(event.data.sessionRevision);
    if (Number.isInteger(sessionRevision)) {
      controller.surface.dataset.sessionRevision = String(sessionRevision);
    }
    const stateCommit = Number(event.data.stateCommit);
    if (Number.isInteger(stateCommit)) {
      controller.surface.dataset.stateCommit = String(stateCommit);
    }
    const commandId = u32CommandIdentity(event.data.commandId);
    const context = commandId === null ? null : editorRuntimeCommands.get(commandId);
    if (context?.consumer === "solver" || context?.consumer === "authoring") {
      const commandController = context.controller;
      if (
        !commandController
        || commandController !== controller
        || commandId !== commandController.pending?.commandId
      ) {
        return;
      }
      editorRuntimeCommands.delete(commandId);
      commandController.displayKey = commandController.pending.key;
      commandController.pending = null;
      commandController.surface.dataset.commandId = String(commandId);
      if (commandController.surface.dataset.pendingCommandId === String(commandId)) {
        delete commandController.surface.dataset.pendingCommandId;
      }
      if (context.consumer === "solver") {
        const aspectWidth = Number(event.data.aspectRatio?.width);
        const aspectHeight = Number(event.data.aspectRatio?.height);
        if (aspectWidth > 0 && aspectHeight > 0) {
          solverBoard.style.aspectRatio = `${aspectWidth} / ${aspectHeight}`;
        }
      }
      return;
    }
    if (controller.consumer !== "preview" || (context && context.controller !== controller)) {
      return;
    }
    applyPreviewTheme(event.data.theme);
    setPreviewViewportAspect(event.data.aspectRatio);
    const inLevelMode = !levelBuilder.hidden || !solverPanel.hidden;
    const screenHasPuzzle = event.data.screenHasPuzzle !== false;
    const previousState = previewSessionState();
    const levelIndex = Number.isInteger(Number(event.data.levelIndex))
      ? Math.trunc(Number(event.data.levelIndex))
      : previousState?.levelIndex ?? 0;
    setPreviewSessionState({
      levelIndex,
      activeModel: typeof event.data.activeModel === "string" ? event.data.activeModel : "",
      rawScene: event.data.rawScene,
      scene: event.data.scene,
      inputs: event.data.inputs || [],
      screen: event.data.screen || "",
      screenHasPuzzle,
      levelCount: Math.max(0, Math.trunc(Number(event.data.levelCount) || 0)),
    });
    if (inLevelMode) {
      if (screenHasPuzzle && (levelPlaytestActive || !solverPanel.hidden)) {
        const displayCells = cloneJson(event.data.levelCells);
        levelDisplayCells = Array.isArray(displayCells) && displayCells.length === level.cells.length
          ? displayCells
          : null;
        renderLevelBoard();
      }
      if (levelSolutionPreview) {
        updateSolutionControls();
      }
    }
    return;
  }
  if (event.data?.type === "PuzzleStudioPreviewDebugTrace") {
    if (fromSolverRuntime) {
      return;
    }
    handlePreviewDebugTrace(event.data.debug || null, event.data.snapshot || null);
  }
});
window.addEventListener("resize", syncPreviewViewportGeometry);
window.addEventListener("resize", syncLevelBoardScale);
window.addEventListener("resize", syncSolverBoardScale);
if (window.ResizeObserver && previewFrameWrap) {
  const previewWrapObserver = new ResizeObserver(() => schedulePreviewViewportGeometrySync(2));
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
window.addEventListener("focus", () => {
  if (!previewKeyboardFocusOwned) {
    return;
  }
  requestAnimationFrame(() => {
    if (previewKeyboardFocusOwned) {
      focusPreviewInputTarget();
    }
  });
});
document.addEventListener("focusin", (event) => {
  const controller = previewEditorRuntimeController();
  if (
    previewKeyboardFocusOwned
    && event.target !== controller?.surface
    && event.target !== controller?.frame
  ) {
    previewKeyboardFocusOwned = false;
  }
});
document.addEventListener("pointerdown", (event) => {
  const surface = previewEditorRuntimeController().surface;
  if (previewKeyboardFocusOwned && surface && !surface.contains(event.target)) {
    previewKeyboardFocusOwned = false;
  }
}, true);
previewSolveButton?.addEventListener("click", () => {
  solvePreviewPaneCurrentLevel().catch((error) => {
    setLevelSolveStatus(`Solve failed: ${userFacingRuntimeError(error)}`, "is-error");
  });
});
previewEditButton?.addEventListener("click", () => {
  void openLevelPaneForCurrentPreviewLevel().catch((error) => {
    setStatus(`Level selection failed: ${userFacingRuntimeError(error)}`, "is-error");
  });
});
solverLevelSelect?.addEventListener("change", () => {
  if (solverLevelSelect.value === "") {
    syncSolverLevelSelector();
    return;
  }
  selectSolverLevel(Number(solverLevelSelect.value));
});
playModeButton.addEventListener("click", () => {
  openPreviewModePane("play");
});
editModeButton.addEventListener("click", () => {
  void openLevelPaneForCurrentDimension().catch((error) => {
    setStatus(`Level selection failed: ${userFacingRuntimeError(error)}`, "is-error");
  });
});
solverModeButton.addEventListener("click", () => {
  if (solverPaneMode === "custom-goal") {
    openPreviewModePane("solver");
    return;
  }
  openSolverPaneForCurrentLevel().catch((error) => {
    setStatus(`Source target sync failed: ${userFacingRuntimeError(error)}`, "is-error");
  });
});
solverCustomGoalToggleButton?.addEventListener("click", () => {
  setSolverPaneMode(solverPaneMode === "custom-goal" ? "level-goal" : "custom-goal");
});
solverCustomGoalPredicate?.addEventListener("change", renderCustomGoalEditor);
solverCustomGoalObject?.addEventListener("change", renderCustomGoalEditor);
solverCustomGoalAddButton?.addEventListener("click", addCustomGoalConstraint);
for (const button of editorDimensionButtons) {
  button.addEventListener("click", () => {
    const context = focusedPuzzleSourceContext();
    const previousMode = currentPreviewMode;
    const activeKind = previousMode === "visual" || previousMode === "visual3d" ? "visual" : "level";
    const first = ["edit", "level3d", "visual", "visual3d"].includes(previousMode)
      ? firstFocusedPuzzleEntry(activeKind, context)
      : firstFocusedPuzzleEntry("level", context);
    if (first) {
      openPreviewModePane(activeKind === "visual"
        ? visualModeForEditorDimension(first.dimension)
        : levelModeForEditorDimension(first.dimension));
      loadFocusedPuzzleEntry(activeKind, first, { silent: true, recordHistory: false });
      return;
    }
    setEditorDimensionMode(button.dataset.editorDimension);
  });
}
for (const button of levelPaneModeButtons) {
  button.addEventListener("click", () => {
    if (!["edit", "level3d"].includes(button.dataset.levelPaneMode)) {
      return;
    }
    void openLevelPaneForCurrentDimension({ mode: button.dataset.levelPaneMode }).catch((error) => {
      setStatus(`Level selection failed: ${userFacingRuntimeError(error)}`, "is-error");
    });
  });
}
visualModeButton.addEventListener("click", () => {
  if (typeof setVisualAnimationMode === "function") {
    setVisualAnimationMode(false, { render: false });
  }
  void openVisualPaneForCurrentDimension().then(() => {
    if (currentVisualPaneMode === "visual" && typeof renderVisualBuilder === "function") {
      renderVisualBuilder();
    }
  }).catch((error) => {
    setStatus(`Visual selection failed: ${userFacingRuntimeError(error)}`, "is-error");
  });
});
visualAnimateModeButton?.addEventListener("click", () => {
  if (currentVisualPaneMode === "visual3d" && typeof setVisual3dAnimationMode === "function") {
    setVisual3dAnimationMode(!visual3d.animationMode);
  } else if (typeof setVisualAnimationMode === "function") {
    setVisualAnimationMode(!visual.animationMode);
  }
});
for (const button of visualDimensionButtons) {
  button.addEventListener("click", () => {
    const dimension = normalizeEditorDimension(button.dataset.visualDimension);
    if (dimension === currentEditorDimension) {
      return;
    }
    setEditorDimensionMode(dimension);
  });
}
visual3dModeButton?.addEventListener("click", () => {
  if (typeof setVisualAnimationMode === "function") {
    setVisualAnimationMode(false, { render: false });
  }
  void openVisualPaneForCurrentDimension().catch((error) => {
    setStatus(`Visual selection failed: ${userFacingRuntimeError(error)}`, "is-error");
  });
});
for (const button of visualPaneModeButtons) {
  button.addEventListener("click", () => {
    if (!["visual", "visual3d"].includes(button.dataset.visualPaneMode)) {
      return;
    }
    void openVisualPaneForCurrentDimension().catch((error) => {
      setStatus(`Visual selection failed: ${userFacingRuntimeError(error)}`, "is-error");
    });
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
  const api = window.PuzzleStudioImportExport;
  if (typeof api?.schedulePuzzleScriptImportConversion === "function") {
    api.schedulePuzzleScriptImportConversion();
  } else {
    setEditorStatus("PuzzleScript import is unavailable", "is-error");
  }
});
let editorDocsLoadPromise = null;

function editorDocsPageButtons() {
  return Array.from(document.querySelectorAll("[data-docs-page]"));
}

function editorDocsArticles() {
  return Array.from(document.querySelectorAll("[data-docs-article]"));
}

function editorDocsAreLoaded() {
  return Boolean(docsPanel?.querySelector("[data-docs-article]"));
}

async function ensureEditorDocsLoaded() {
  if (!docsPanel || editorDocsAreLoaded()) {
    return;
  }
  if (!editorDocsLoadPromise) {
    docsPanel.textContent = "Loading documents...";
    editorDocsLoadPromise = window.PuzzleStudioHost.editorDocsHtml().then((html) => {
      if (!String(html || "").includes("data-docs-article")) {
        throw new Error("Editor documents payload is empty.");
      }
      docsPanel.innerHTML = html;
      window.hydrateEditorIcons?.(docsPanel);
      const active = docsPanel.querySelector(".docs-nav-button.is-active")?.dataset.docsPage
        || docsPanel.querySelector("[data-docs-page]")?.dataset.docsPage
        || "";
      if (active) {
        activateEditorDocsPage(active);
      }
    }).catch((error) => {
      docsPanel.textContent = error.message || "Documents unavailable";
      throw error;
    });
  }
  await editorDocsLoadPromise;
}

function activateEditorDocsPage(pageId) {
  for (const item of editorDocsPageButtons()) {
    const active = item.dataset.docsPage === pageId;
    item.classList.toggle("is-active", active);
    item.setAttribute("aria-selected", String(active));
  }
  for (const article of editorDocsArticles()) {
    article.hidden = article.dataset.docsArticle !== pageId;
  }
  if (docsPanel) {
    docsPanel.scrollTop = 0;
  }
}

docsTopbarButton?.addEventListener("click", () => {
  openPreviewModePane("docs");
  ensureEditorDocsLoaded().then(() => {
    docsSearchInput?.focus();
  }).catch((error) => {
    console.error(error);
    setEditorStatus(error.message || "Documents unavailable", "is-error");
  });
});
docsPanel?.addEventListener("click", (event) => {
  const exampleButton = event.target.closest("[data-load-example]");
  if (exampleButton && docsPanel.contains(exampleButton)) {
    loadEditorExample(exampleButton.dataset.loadExample || "").catch((error) => {
      console.error(error);
      setEditorStatus(error.message || "Example unavailable", "is-error");
    });
    return;
  }
  const button = event.target.closest("[data-docs-page]");
  if (!button || !docsPanel.contains(button)) {
    return;
  }
  activateEditorDocsPage(button.dataset.docsPage || "");
});
psImportSourceInput?.addEventListener("input", () => {
  const api = window.PuzzleStudioImportExport;
  if (typeof api?.schedulePuzzleScriptImportConversion === "function") {
    api.schedulePuzzleScriptImportConversion();
  } else {
    setEditorStatus("PuzzleScript import is unavailable", "is-error");
  }
});
levelBoard.addEventListener("pointerdown", startLevelPaint);
levelBoard.addEventListener("pointermove", continueLevelPaint);
levelBoard.addEventListener("pointerup", stopLevelPaint);
levelBoard.addEventListener("pointercancel", stopLevelPaint);
levelBoard.addEventListener("contextmenu", (event) => event.preventDefault());
levelBoard.addEventListener("keydown", async (event) => {
  if (handleSolutionKey(event)) {
    return;
  }
  if (!levelPlaytestActive && (event.key === "Enter" || event.key === " ") && previewSession?.state?.screenHasPuzzle !== false) {
    const mutate = levelBucketActive ? bucketFillLevelFromElement : paintLevelCellFromElement;
    if (await mutate(event.target)) {
      event.preventDefault();
      event.stopPropagation();
      return;
    }
  }
  if (!levelPlaytestActive) {
    return;
  }
  sendLevelPlaytestKey(event);
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
  sendLevelPlaytestKey(event);
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
document.addEventListener("pointerdown", (event) => {
  if (!level.addPaletteOpen || event.target?.closest?.(".level-palette-add-wrap")) {
    return;
  }
  level.addPaletteOpen = false;
  renderLevelPalette();
});
levelRotateLeftButton?.addEventListener("click", rotateLevelLeft);
levelRotateRightButton?.addEventListener("click", rotateLevelRight);
levelFlipHorizontalButton?.addEventListener("click", flipLevelHorizontal);
levelFlipVerticalButton?.addEventListener("click", flipLevelVertical);
syncLevelResizeControls();
levelNamespaceInput.addEventListener("input", () => {
  syncLevelNameOptions();
  if (document.activeElement === levelNameInput) {
    showLevelNameOptions();
  }
});
levelNamespaceInput.addEventListener("focus", syncLevelNameOptions);
levelNameInput.addEventListener("input", () => {
  syncLevelNameOptions();
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
  loadSelectedLevelNameFromInput();
});
solveLevelButton.addEventListener("click", () => {
  solveLevel().catch((error) => {
    setLevelSolveStatus(`Solve failed: ${userFacingRuntimeError(error)}`, "is-error");
  });
});
solutionSpeedSelect.addEventListener("change", changeSolutionPlaybackSpeed);
solutionSeekInput.addEventListener("input", seekSolutionStep);
solutionSeekInput.addEventListener("change", seekSolutionStep);

installEditorHoverTooltips();
bindSourceEditorEvents();
bindSourceEditorPopoverEvents();
sourceEditor.on("change", () => {
  invalidateLevelEditSourceForDocument(activeDocument());
});
registerSourceEditableTarget?.("level", {
  load: loadLevelFromSourcePosition,
});

applyPaneVisibility();
syncSolverPaneModeControls();
pollAgentObservations();

loadSource().then(() => {
  setWorkspaceFileActionsReady();
}).catch((error) => {
  setPreviewDocumentLoaded(false);
  stopEditorRuntimeController(previewEditorRuntimeController());
  resetPreviewLog("Load failed");
  appendPreviewLog("error", error?.message || String(error), { source: "workspace" });
  setEditorStatus("Load error", "is-error");
});
