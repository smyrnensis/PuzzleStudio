const themeStoreKey = "PuzzleStudioEditorTheme:v1";
const previewDefaultLogicalWidth = 4;
const previewDefaultLogicalHeight = 3;
const previewMinimumHeight = 720;
const previewMinimumLogHeight = 72;
const solverFeedbackTickMs = 250;
const solverObservationLiveIntervalMs = 500;
const solverObservationPlaybackMaxMs = 1600;
const solverObservationPlaybackMinStepMs = 20;
const solverObservationPlaybackMaxStepMs = 80;
const solutionPlaybackBaseIntervalMs = 350;
const WASM_SECTION_BLOCK_NAMES = Object.freeze({
  marks: "marks",
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
  visual: "visuals",
  visuals: "visuals",
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
});
const WASM_SECTION_BOUNDARY_BLOCKS = new Set([
  "map",
  "on_level_start",
  "on_level_clear",
  "marks",
  "group",
  "layers",
  "collision_layers",
  "legend",
  "visuals",
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
const VISUAL_COLOR_PRESETS = [
  "#000000", "#1d2b53", "#7e2553", "#008751",
  "#ab5236", "#5f574f", "#c2c3c7", "#fff1e8",
  "#ff004d", "#ffa300", "#ffec27", "#00e436",
  "#29adff", "#83769c", "#ff77a8", "#ffccaa",
];
const VISUAL_COLOR_TOKENS = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
const PREVIEW_THEME_PRESETS = {
  clean: {
    colorScheme: "light",
    bg: "#f5f3ef",
    ink: "#1f2428",
    muted: "#66727c",
    line: "#d7dde2",
    accent: "var(--preview-game-ink)",
    panelBg: "rgba(255, 255, 255, 0.94)",
    background: "var(--preview-game-bg)",
  },
  terminal: {
    colorScheme: "dark",
    bg: "#000000",
    ink: "#ffffff",
    muted: "#ffffff",
    line: "#ffffff",
    accent: "#ffffff",
    panelBg: "#000000",
    background: "var(--preview-game-bg)",
  },
  paper: {
    colorScheme: "light",
    bg: "#f4ecd9",
    ink: "#2b2419",
    muted: "#756852",
    line: "#cdbd9a",
    accent: "#8d5d2a",
    panelBg: "rgba(255, 250, 240, 0.96)",
    background: "linear-gradient(rgba(255, 255, 255, 0.26), rgba(255, 255, 255, 0.26)), repeating-linear-gradient(0deg, transparent 0 23px, rgba(141, 93, 42, 0.08) 23px 24px), var(--preview-game-bg)",
  },
  pixel: {
    colorScheme: "dark",
    bg: "#08080c",
    ink: "#f8f8f8",
    muted: "#d8d8d8",
    line: "#f8f8f8",
    accent: "#f8f8f8",
    panelBg: "#08080c",
    background: "var(--preview-game-bg)",
  },
  puzzlescript: {
    colorScheme: "dark",
    bg: "#000000",
    ink: "#ffffff",
    muted: "#ffffff",
    line: "#ffffff",
    accent: "#ffffff",
    panelBg: "#000000",
    background: "var(--preview-game-bg)",
  },
  candy: {
    colorScheme: "light",
    bg: "#fff7fb",
    ink: "#33404a",
    muted: "#7a8790",
    line: "#efbfd3",
    accent: "#d76f97",
    panelBg: "rgba(255, 255, 255, 0.96)",
    background: "repeating-linear-gradient(135deg, rgba(215, 111, 151, 0.045) 0 14px, transparent 14px 28px), var(--preview-game-bg)",
  },
  blueprint: {
    colorScheme: "dark",
    bg: "#0d334e",
    ink: "#e9f8ff",
    muted: "#aad0e0",
    line: "#78c7e8",
    accent: "#ffd166",
    panelBg: "rgba(11, 42, 64, 0.94)",
    background: "repeating-linear-gradient(0deg, rgba(120, 199, 232, 0.11) 0 1px, transparent 1px 24px), repeating-linear-gradient(90deg, rgba(120, 199, 232, 0.11) 0 1px, transparent 1px 24px), var(--preview-game-bg)",
  },
  noir: {
    colorScheme: "dark",
    bg: "#101010",
    ink: "#f4f1e8",
    muted: "#a9a097",
    line: "#59544e",
    accent: "#f2c14e",
    panelBg: "rgba(24, 24, 24, 0.96)",
    background: "linear-gradient(90deg, rgba(242, 193, 78, 0.055), transparent 38%, transparent 62%, rgba(242, 193, 78, 0.035)), var(--preview-game-bg)",
  },
};

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
const solverPreparedByBuildId = new Map();
let previewFrameHasCurrentCompiledPreview = false;
let previewTimer = 0;
let previewFrameObjectUrl = "";
let previewFrameLoadId = 0;
const previewRuntimeAssetWindows = new WeakSet();
const editorPreviewProgressSaves = new Map();
const editorPreviewProgressSaveStoreKey = "PuzzleStudioPreviewProgressSaves:v1";
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
let surfaceEntriesCache = null;
let surfaceEntriesRequest = null;
let previewLogEntries = [];
let previewDebugEnabled = false;
let previewDebugTrace = null;
let previewDebugCursor = -1;
let previewDebugSnapshot = null;
let previewPaneSourceKey = "";
let activeLevelIndex = 0;
let activeSolverTask = null;
let solverSelectedLevelIndex = null;
let activeLevelSolveRequest = null;
let editorSolverWorker = null;
const editorSolverWorkerRequests = new Map();
let activeSolverDisplaySceneRequestKey = "";
let completedSolverTaskKey = "";
let levelSolutionPreview = null;
let solverObservationPreview = null;
let levelSolveSummaryText = "";

function loadEditorPreviewProgressSaves() {
  try {
    const parsed = JSON.parse(window.localStorage.getItem(editorPreviewProgressSaveStoreKey) || "{}");
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
      return;
    }
    for (const [key, value] of Object.entries(parsed)) {
      if (typeof key === "string" && typeof value === "string") {
        editorPreviewProgressSaves.set(key, value);
      }
    }
  } catch (error) {
    console.warn("Preview progress could not be loaded from editor storage.", error);
  }
}

function saveEditorPreviewProgressSaves() {
  try {
    window.localStorage.setItem(
      editorPreviewProgressSaveStoreKey,
      JSON.stringify(Object.fromEntries(editorPreviewProgressSaves)),
    );
  } catch (error) {
    console.warn("Preview progress could not be saved to editor storage.", error);
  }
}

loadEditorPreviewProgressSaves();
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
  visual: { undo: [], redo: [] },
  visual3d: { undo: [], redo: [] },
};

function cloneVisualEditValue(value) {
  return JSON.parse(JSON.stringify(value));
}

function visualEditDocumentForKind(kind) {
  if (kind === "level3d" && typeof level3dSourceDocument === "function") {
    return level3dSourceDocument();
  }
  if (kind === "visual" && typeof activeVisualEditDocument === "function") {
    return activeVisualEditDocument();
  }
  if (kind === "visual3d" && typeof activeVisual3dEditDocument === "function") {
    return activeVisual3dEditDocument();
  }
  if (kind === "level" && typeof activePreviewDocument === "function") {
    return activePreviewDocument();
  }
  return null;
}

function visualEditSnapshot(kind) {
  const editDocument = visualEditDocumentForKind(kind);
  const tracksSource = kind === "level3d" || kind === "visual" || kind === "visual3d";
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
        layers: cloneVisualEditValue(level.layers || []),
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
  if (kind === "visual") {
    return {
      ...base,
      state: {
        width: visual.width,
        height: visual.height,
        palette: cloneVisualEditValue(visual.palette || []),
        cells: cloneVisualEditValue(visual.cells || []),
        paletteBind: cloneVisualEditValue(visual.paletteBind || null),
        shapeBind: cloneVisualEditValue(visual.shapeBind || null),
        solidSource: Boolean(visual.solidSource),
        sourcePreludeRows: cloneVisualEditValue(visual.sourcePreludeRows || []),
        sourceVisualContract: cloneVisualEditValue(visual.sourceVisualContract || null),
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
        width: visual3d.width,
        height: visual3d.height,
        depth: visual3d.depth,
        editDocumentId: visual3d.editDocumentId,
        editSourceStart: visual3d.editSourceStart,
        editSourceEnd: visual3d.editSourceEnd,
        editSourceBodyStart: visual3d.editSourceBodyStart,
        editSourceBodyEnd: visual3d.editSourceBodyEnd,
        editSourceName: visual3d.editSourceName,
        sourceVisualContract: cloneVisualEditValue(visual3d.sourceVisualContract || null),
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
        sourcePreludeRows: cloneVisualEditValue(visual3d.sourcePreludeRows || []),
        sourceSpatialOps: cloneVisualEditValue(visual3d.sourceSpatialOps || []),
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
  if (snapshot.kind === "level") {
    level.width = Math.max(1, Math.trunc(Number(state.width) || 1));
    level.height = Math.max(1, Math.trunc(Number(state.height) || 1));
    level.activeLayer = Math.max(0, Math.trunc(Number(state.activeLayer) || 0));
    level.regions = cloneVisualEditValue(state.regions || []);
    level.layers = cloneVisualEditValue(state.layers || []);
    level.cells = cloneVisualEditValue(state.cells || []);
    ensureLevelLayerMaps();
    clearSolutionPreview();
    levelDisplayCells = null;
    renderLevelBoard();
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
  } else if (snapshot.kind === "visual") {
    visual.width = clampVisualSize(state.width);
    visual.height = clampVisualSize(state.height);
    visual.palette = cloneVisualEditValue(state.palette || [{ color: "#ff004d" }]);
    visual.cells = cloneVisualEditValue(state.cells || []);
    visual.paletteBind = cloneVisualEditValue(state.paletteBind || null);
    visual.shapeBind = cloneVisualEditValue(state.shapeBind || null);
    visual.solidSource = Boolean(state.solidSource);
    visual.sourcePreludeRows = cloneVisualEditValue(state.sourcePreludeRows || []);
    visual.sourceVisualContract = cloneVisualEditValue(state.sourceVisualContract || null);
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
    visual3d.width = clampVisual3dSize(state.width);
    visual3d.height = clampVisual3dSize(state.height);
    visual3d.depth = clampVisual3dSize(state.depth);
    visual3d.editDocumentId = state.editDocumentId || null;
    visual3d.editSourceStart = Number.isInteger(state.editSourceStart) ? state.editSourceStart : null;
    visual3d.editSourceEnd = Number.isInteger(state.editSourceEnd) ? state.editSourceEnd : null;
    visual3d.editSourceBodyStart = Number.isInteger(state.editSourceBodyStart) ? state.editSourceBodyStart : null;
    visual3d.editSourceBodyEnd = Number.isInteger(state.editSourceBodyEnd) ? state.editSourceBodyEnd : null;
    visual3d.editSourceName = state.editSourceName || "";
    visual3d.sourceVisualContract = cloneVisualEditValue(state.sourceVisualContract || null);
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
    visual3d.sourcePreludeRows = cloneVisualEditValue(state.sourcePreludeRows || []);
    visual3d.sourceSpatialOps = cloneVisualEditValue(state.sourceSpatialOps || []);
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

function undoVisualEdit(kind = currentVisualEditKind()) {
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
  if (target?.closest?.(".visual-code-glyph")) {
    return false;
  }
  if (typeof isVisualEditUndoTarget === "function" && isVisualEditUndoTarget(target)) {
    return false;
  }
  return target?.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(tagName);
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
  if (typeof renderVisual3dPreview === "function") {
    window.requestAnimationFrame(renderVisual3dPreview);
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
  syncSourcePreviewRunButton();
}

function previewRuntimeIsRunning() {
  return Boolean(activePreviewRequest || previewDocumentLoaded || previewFrameHasEditorLevelState);
}

function syncSourcePreviewRunButton() {
  if (!runButton) {
    return;
  }
  const running = previewRuntimeIsRunning();
  const label = running ? "Refresh preview" : "Play preview";
  runButton.classList.toggle("is-running", running);
  runButton.setAttribute("aria-pressed", String(running));
  runButton.setAttribute("aria-label", label);
  runButton.title = label;
}

function stopPreviewRuntime() {
  if (activePreviewRequest) {
    activePreviewRequest.abort();
    activePreviewRequest = null;
  }
  previewFrameHasCurrentCompiledPreview = false;
  previewFrameHasEditorLevelState = false;
  previewSession = null;
  setPreviewDocumentLoaded(false);
  setPreviewFrameHtml(emptyPreviewDocument());
  syncPreviewLevelActionButtons();
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
  root.style.setProperty("--preview-game-accent", theme.accent || theme.ink);
  root.style.setProperty("--preview-game-panel-bg", theme.panelBg);
  root.style.setProperty("--preview-game-background", theme.background);
  root.style.colorScheme = theme.colorScheme;
}

function editorPreviewTheme() {
  const light = normalizeTheme(document.documentElement.dataset.theme) === "light";
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
    if (name === "bg" || name === "background") {
      resolved.bg = value;
      resolved.background = "var(--preview-game-bg)";
    } else if (name === "ink" || name === "text") {
      resolved.ink = value;
      if (resolved.accent === "var(--preview-game-ink)") {
        resolved.accent = value;
      }
    } else if (name === "accent") {
      resolved.accent = value;
      resolved.line = value;
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
  if (!window.PuzzleVisualRegistry) {
    window.PuzzleVisualRegistry = {
      create(config = {}) {
        return {
          aliases: { ...(config.aliases || {}) },
          entries: { ...(config.entries || {}) },
          order: {
            direction_priority: [...(config.order?.direction_priority || [])],
            priorities: [...(config.order?.priorities || [])],
          },
          animations: { ...(config.animations || {}) },
          triggers: { ...(config.triggers || {}) },
          animationDefaults: { ...(config.animationDefaults || {}) },
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
      window.GameVisuals = window.PuzzleVisualRegistry.create();
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
  window.GameVisuals = window.PuzzleVisualRegistry.create();
  if (!script) {
    return;
  }
  Function(script)();
}

function compiledPreviewGameVisualsJs(html) {
  const source = String(html || "");
  const scriptPattern = /<script>\n([\s\S]*?)\n<\/script>/g;
  let match = null;
  while ((match = scriptPattern.exec(source))) {
    const script = match[1] || "";
    if (script.includes("window.GameVisuals")) {
      return script;
    }
  }
  throw new Error("Compiled preview is missing GameVisuals script.");
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
    path: document.puzzlePath || document.name,
    source: currentSourceForDocument(document),
  }));
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
    puzzlePath: entryPath,
    workspaceRoot: document.workspaceRoot || workspaceRoot || "",
    source: entry.source,
    documents: documentsSnapshot,
    presentationManifest,
    gameCss: effectiveGameCss(document, presentationManifest),
    gameVisualsJs: effectiveGameVisualsJs(document, presentationManifest),
  };
}

function previewSessionState() {
  return previewSession?.buildId === previewBuild?.id ? previewSession.state : null;
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

function previewHasCurrentLevel() {
  return currentPreviewRuntimeLevelIndex(previewBuild?.exportData) !== null;
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
    runButton.disabled = true;
    syncSourcePreviewRunButton();
    return;
  }
  let buildInput = null;
  let requestSource = "";
  updateSourceMeta();
  resetPreviewLog(`Compiling ${document.puzzlePath || "preview"}`);
  setStatus("Compiling", "");
  runButton.disabled = true;

  if (activePreviewRequest) {
    activePreviewRequest.abort();
  }

  const controller = new AbortController();
  activePreviewRequest = controller;
  runButton.disabled = false;
  syncSourcePreviewRunButton();

  try {
    const presentationManifest = await ensurePreviewDocumentsLoaded(document);
    buildInput = capturePreviewBuildInput(document, presentationManifest);
    requestSource = buildInput.source;
    const html = await window.PuzzleStudioHost.preview({
      source: requestSource,
      workspaceDocuments: compilerDocumentsForSnapshot(buildInput.documents),
      puzzlePath: buildInput.puzzlePath,
      workspaceRoot: buildInput.workspaceRoot,
      gameCss: buildInput.gameCss,
      gameVisualsJs: buildInput.gameVisualsJs,
    }, { signal: controller.signal });
    applyCompiledPreviewHtml(html, document, buildInput);
  } catch (error) {
    if (error.name === "AbortError") {
      return;
    }
    appendCompileDiagnostics(error, { source: "compiler", document, sourceText: requestSource });
    setStatus("Compile error", "is-error");
  } finally {
    if (activePreviewRequest === controller) {
      activePreviewRequest = null;
    }
    runButton.disabled = !isPuzzleDocument(activePreviewDocument());
    syncSourcePreviewRunButton();
  }
}

async function ensurePreviewExportForLevelAction(options = {}) {
  ensurePreviewTargetsActiveDocument();
  let exportData = previewBuild?.exportData;
  const freshEnough = !options.requireFresh || !previewBuildIsStale;
  if (exportData && freshEnough && (!options.requirePreviewFrame || previewFrameHasCurrentCompiledPreview)) {
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
  exportData = previewBuild?.exportData;
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

function ensureLevel3dRuntimePreviewForOpenPane() {
  if (
    currentPreviewMode !== "level3d"
    || !level3dBuilder
    || level3dBuilder.hidden
    || typeof level3dRuntimeSnapshot !== "function"
    || !level3dRuntimeSnapshot()
  ) {
    return;
  }
  ensurePreviewExportForLevelAction({
    status: setLevel3dActionStatus,
    compilingMessage: "Compiling 3D preview",
    failureMessage: "3D preview compile failed",
    requireFresh: true,
  }).then((exportData) => {
    if (
      exportData
      && currentPreviewMode === "level3d"
      && level3dBuilder
      && !level3dBuilder.hidden
      && typeof renderLevel3dBuilder === "function"
    ) {
      renderLevel3dBuilder();
    }
  }).catch((error) => {
    setLevel3dActionStatus(`3D preview compile failed: ${userFacingRuntimeError(error)}`, "is-error");
  });
}

async function runPreviewFromSourcePane() {
  ensurePreviewTargetsActiveDocument();
  openPreviewModePane("play", { focus: false });
  runButton.disabled = true;
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
      runButton.disabled = !isPuzzleDocument(activePreviewDocument());
    }
  }
  if (!saved) {
    setStatus("Save failed", "is-error");
    return;
  }
  await renderPreview();
}

function applyCompiledPreviewHtml(html, document, buildInput) {
  const previousLevelIndex = previewBuild?.documentId === buildInput.documentId
    ? currentPreviewRuntimeLevelIndex(previewBuild?.exportData)
    : null;
  const exportData = extractPreviewExport(html);
  previewBuild = {
    ...buildInput,
    html,
    exportData,
  };
  solverPreparedByBuildId.clear();
  previewBuildIsStale = false;
  previewSession = {
    buildId: previewBuild.id,
    state: null,
    runtimeStatus: null,
  };
  previewFrameHasCurrentCompiledPreview = true;
  syncPreviewViewportAspect();
  applyPreviewTheme(exportData?.theme || null);
  setActiveLevelIndex(previousLevelIndex ?? exportData?.initialLevelIndex ?? 0, exportData);
  clearSolverTask();
  previewFrameHasEditorLevelState = false;
  setPreviewFrameHtml(editorPreviewDocument(html), { markDocumentLoaded: true });
  applyGameCss(buildInput.gameCss);
  applyGameVisuals(compiledPreviewGameVisualsJs(html));
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
  refreshVisiblePreviewSolverTask(previewBuild?.exportData);
  syncSolverLevelSelector(previewBuild?.exportData);
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
  setPreviewFrameHtml(emptyPreviewDocument());
  applyGameCss("");
  applyGameVisuals("");
  downloadButton.disabled = true;
  syncPreviewLevelActionButtons();
}

function workspaceCompilerDocuments(entryDocument) {
  return compilerDocumentsForSnapshot(workspaceSourceSnapshot(entryDocument));
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
  if (["marks", "group", "layers", "collision_layers", "win_conditions", "lose_conditions", "transitions", "levels", "visuals", "assets"].includes(block)) {
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
  if (typeof resolveWorkspaceImportPath === "function") {
    return resolveWorkspaceImportPath(baseDir, importPath, activeDocument()?.workspaceRoot || workspaceRoot || "");
  }
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

function prepareEditorSolverArtifact({ documents, puzzlePath, documentId }) {
  const worker = createWasmSolveWorker();
  const requestId = createDocumentId();
  return new Promise((resolve, reject) => {
    registerEditorSolverWorkerRequest(requestId, (message) => {
      if (message.type === "prepared") {
        resolve({
          artifactId: message.artifactId,
          modelKind: message.modelKind,
          levelCount: message.levelCount,
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
        displayed: true,
      });
    } catch (error) {
      editorSolverWorkerRequests.delete(requestId);
      reject(error);
    }
  });
}

function materializeEditorSolverState(task) {
  const worker = createWasmSolveWorker();
  const requestId = createDocumentId();
  return new Promise((resolve, reject) => {
    registerEditorSolverWorkerRequest(requestId, (message) => {
      if (message.type === "materialized") {
        resolve(message.state);
        return true;
      }
      if (message.type === "error") {
        reject(new Error(message.error || "Solver materialization failed"));
        return true;
      }
      return false;
    }, (error) => {
      reject(new Error(userFacingWorkerError(error)));
    });
    try {
      worker.postMessage({
        type: "materialize",
        requestId,
        wasm: wasmSolverWorkerConfig(),
        artifactId: task.rules.compileId,
        modelKind: task.rules.modelKind,
        levelIndex: task.level.index,
        state: task.state.data,
        materializeLevelStart: task.state.lifecycle === "playable-start",
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
    appendPlainCompileError(error, options);
    return;
  }
  for (const diagnostic of diagnostics) {
    const location = diagnosticSourceLocation(diagnostic, options);
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
  schedulePreviewViewportSync(2);
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
  previewDebugEnabled = enabled === true;
  if (!previewDebugEnabled) {
    previewDebugTrace = null;
    previewDebugCursor = -1;
    previewDebugSnapshot = null;
    clearPreviewDebugHighlight();
  }
  syncPreviewDebugControls();
  syncPreviewDebugModeToFrame();
  appendPreviewLog("system", previewDebugEnabled ? "debug mode enabled" : "debug mode disabled", {
    source: "editor",
  });
}

function syncPreviewDebugModeToFrame() {
  previewFrame?.contentWindow?.postMessage({
    type: "PuzzleStudioSetPreviewDebugMode",
    enabled: previewDebugEnabled,
  }, "*");
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
    clearPreviewDebugHighlight();
    renderPreviewLog();
    return;
  }
  if (!executions.length) {
    previewDebugStatus.textContent = "No rule execution yet";
    clearPreviewDebugHighlight();
    renderPreviewLog();
    return;
  }
  previewDebugStatus.textContent = `Rule ${previewDebugCursor + 1}/${executions.length}`;
  syncPreviewDebugHighlight();
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
  const object = engineObjectById(Number(id), previewBuild?.exportData);
  if (object?.name) {
    return object.name;
  }
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

function previewDebugIsInternalMarkOp(op) {
  const kind = String(op?.kind || "");
  if (kind !== "set_mark" && kind !== "remove_mark") {
    return false;
  }
  const mark = String(op?.markName || "").trim();
  return !mark || mark.startsWith("__") || /^mark#\d+$/.test(mark);
}

function previewDebugVisiblePatchOps(patch) {
  return (Array.isArray(patch) ? patch : []).filter((op) => !previewDebugIsInternalMarkOp(op));
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
  syncPreviewDebugHighlight();
  syncPreviewDebugState();
  renderPreviewLog();
}

function setPreviewDebugCursor(index) {
  const executions = previewDebugTrace?.executions || [];
  if (!executions.length) {
    previewDebugCursor = -1;
    syncPreviewDebugControls();
    clearPreviewDebugHighlight();
    return;
  }
  previewDebugCursor = Math.max(0, Math.min(executions.length - 1, index));
  syncPreviewDebugControls();
  syncPreviewDebugHighlight();
  syncPreviewDebugState();
}

function previewDebugPatchCells(patch) {
  const cells = new Map();
  const add = (position, kind) => {
    const x = Number(position?.x);
    const y = Number(position?.y);
    if (!Number.isFinite(x) || !Number.isFinite(y) || x < 0 || y < 0) {
      return;
    }
    const key = `${Math.trunc(x)}:${Math.trunc(y)}`;
    const existing = cells.get(key);
    if (existing) {
      existing.kinds.add(kind);
    } else {
      cells.set(key, { x: Math.trunc(x), y: Math.trunc(y), kinds: new Set([kind]) });
    }
  };
  for (const op of Array.isArray(patch) ? patch : []) {
    const kind = String(op?.kind || "change");
    if (op?.position) {
      add(op.position, kind);
    }
    if (op?.from) {
      add(op.from, kind === "move" ? "move_from" : kind);
    }
    if (op?.to) {
      add(op.to, kind === "move" ? "move_to" : kind);
    }
  }
  return Array.from(cells.values()).map((cell) => ({
    x: cell.x,
    y: cell.y,
    kind: cell.kinds.has("move_from") && cell.kinds.has("move_to")
      ? "move"
      : Array.from(cell.kinds)[cell.kinds.size - 1] || "change",
  }));
}

function previewDebugBoardSize(cells) {
  const scene = previewSession?.state?.scene;
  if (scene?.width && scene?.height) {
    return { width: Number(scene.width), height: Number(scene.height) };
  }
  const levelIndex = Number.isInteger(Number(previewSession?.state?.levelIndex))
    ? Math.trunc(Number(previewSession?.state.levelIndex))
    : currentEditableLevelIndex(previewBuild?.exportData);
  const state = previewBuild?.exportData?.levels?.[levelIndex]?.initialState;
  if (state?.width && state?.height) {
    return { width: Number(state.width), height: Number(state.height) };
  }
  const maxX = cells.reduce((max, cell) => Math.max(max, cell.x), 0);
  const maxY = cells.reduce((max, cell) => Math.max(max, cell.y), 0);
  return { width: maxX + 1, height: maxY + 1 };
}

function syncPreviewDebugHighlight() {
  if (!previewDebugEnabled || !previewFrame?.contentWindow) {
    clearPreviewDebugHighlight();
    return;
  }
  const executions = previewDebugTrace?.executions || [];
  const execution = executions[previewDebugCursor];
  if (!execution) {
    clearPreviewDebugHighlight();
    return;
  }
  const cells = previewDebugPatchCells(execution.patch);
  const board = previewDebugBoardSize(cells);
  previewFrame.contentWindow.postMessage({
    type: "PuzzleStudioPreviewDebugHighlight",
    label: `Rule ${previewDebugCursor + 1}/${executions.length}`,
    summary: previewDebugPatchSummary(execution.patch),
    board,
    cells,
  }, "*");
}

function clearPreviewDebugHighlight() {
  previewFrame?.contentWindow?.postMessage({
    type: "PuzzleStudioPreviewDebugHighlight",
    cells: [],
  }, "*");
}

function syncPreviewDebugState() {
  if (!previewDebugEnabled || !previewFrame?.contentWindow) {
    return;
  }
  const state = previewDebugStateDataForCursor();
  if (!state) {
    return;
  }
  const scene = previewDebugSnapshotScene();
  previewFrame.contentWindow.postMessage({
    type: "PuzzleStudioSetState",
    state,
    levelIndex: Number.isInteger(Number(previewDebugSnapshot?.levelIndex))
      ? Math.trunc(Number(previewDebugSnapshot.levelIndex))
      : previewSession?.state?.levelIndex ?? currentEditableLevelIndex(previewBuild?.exportData),
    regions: scene?.regions || [],
    acceptModelInput: false,
    materializeLevelStart: false,
    materializeDisplay: false,
    materializeTurnStart: false,
  }, "*");
}

function previewDebugStateDataForCursor() {
  const base = previewDebugSnapshotStateData();
  const executions = previewDebugTrace?.executions || [];
  if (!base || !executions.length || previewDebugCursor < 0) {
    return base;
  }
  const state = cloneJson(base);
  for (let index = executions.length - 1; index > previewDebugCursor; index -= 1) {
    reversePreviewDebugPatch(state, executions[index]?.patch);
  }
  return state;
}

function previewDebugSnapshotStateData() {
  const scene = previewDebugSnapshotScene();
  if (!scene?.width || !scene?.height || !Array.isArray(scene.cells)) {
    return null;
  }
  const levelIndex = Number.isInteger(Number(previewDebugSnapshot?.levelIndex))
    ? Math.trunc(Number(previewDebugSnapshot.levelIndex))
    : previewSession?.state?.levelIndex ?? currentEditableLevelIndex(previewBuild?.exportData);
  return stateDataFromSceneCells(scene, levelIndex);
}

function previewDebugSnapshotScene() {
  if (previewDebugSnapshot?.scene?.cells) {
    return previewDebugSnapshot.scene;
  }
  const scenePuzzleState = previewDebugSnapshot?.scenePuzzleState || {};
  for (const value of Object.values(scenePuzzleState)) {
    if (value?.cells) {
      return value;
    }
  }
  return previewSession?.state?.scene || null;
}

function stateDataFromSceneCells(scene, levelIndex = currentEditableLevelIndex(previewBuild?.exportData)) {
  const width = Math.max(1, Math.trunc(Number(scene.width) || 1));
  const height = Math.max(1, Math.trunc(Number(scene.height) || 1));
  const sceneLayerIndexes = scene.cells
    .flatMap((cell) => (cell.layers || []).map((layer) => Math.trunc(Number(layer.layer))))
    .filter((layer) => Number.isFinite(layer) && layer >= 0);
  const layerCount = Math.max(
    1,
    Math.trunc(Number(scene.layerCount) || 0),
    ...sceneLayerIndexes.map((layer) => layer + 1),
  );
  const slots = Array.from({ length: width * height * layerCount }, () => 0);
  for (const cell of scene.cells) {
    const x = Math.trunc(Number(cell?.x));
    const y = Math.trunc(Number(cell?.y));
    if (!Number.isFinite(x) || !Number.isFinite(y) || x < 0 || y < 0 || x >= width || y >= height) {
      continue;
    }
    for (const layer of cell.layers || []) {
      const layerIndex = Math.trunc(Number(layer.layer));
      const objectId = Math.trunc(Number(layer.objectId));
      if (Number.isFinite(layerIndex) && layerIndex >= 0 && layerIndex < layerCount && Number.isFinite(objectId)) {
        slots[((y * width + x) * layerCount) + layerIndex] = objectId;
      }
    }
  }
  const variablesLength = previewDebugVariablesLength(levelIndex);
  return {
    width,
    height,
    layerCount,
    slots,
    variables: Array.isArray(previewDebugSnapshot?.gameState?.variables)
      ? cloneJson(previewDebugSnapshot.gameState.variables)
      : Array.from({ length: variablesLength }, () => 0),
    levelFiredRules: [],
  };
}

function previewDebugVariablesLength(levelIndex) {
  const levels = previewBuild?.exportData?.levels || [];
  return levels?.[levelIndex]?.initialState?.variables?.length
    || levels?.[0]?.initialState?.variables?.length
    || 0;
}

function reversePreviewDebugPatch(state, patch) {
  const ops = Array.isArray(patch) ? patch : [];
  for (let index = ops.length - 1; index >= 0; index -= 1) {
    const op = ops[index] || {};
    if (op.kind === "add") {
      previewDebugRemoveObject(state, op.position, op.objectId);
    } else if (op.kind === "remove") {
      previewDebugAddObject(state, op.position, op.objectId);
    } else if (op.kind === "move") {
      previewDebugRemoveObject(state, op.to, op.objectId);
      previewDebugAddObject(state, op.from, op.objectId);
    } else if (op.kind === "replace") {
      previewDebugRemoveObject(state, op.position, op.add);
      previewDebugAddObject(state, op.position, op.remove);
    }
  }
}

function previewDebugObjectLayer(objectId) {
  const id = Math.trunc(Number(objectId));
  const object = (previewBuild?.exportData?.engine?.objects || []).find((candidate) => Number(candidate.id) === id);
  return Math.trunc(Number(object?.layer ?? object?.layerId ?? 0));
}

function previewDebugSlotIndex(state, position, objectId) {
  const x = Math.trunc(Number(position?.x));
  const y = Math.trunc(Number(position?.y));
  if (!Number.isFinite(x) || !Number.isFinite(y) || x < 0 || y < 0 || x >= state.width || y >= state.height) {
    return -1;
  }
  const layer = previewDebugObjectLayer(objectId);
  if (!Number.isFinite(layer) || layer < 0 || layer >= state.layerCount) {
    return -1;
  }
  return ((y * state.width + x) * state.layerCount) + layer;
}

function previewDebugAddObject(state, position, objectId) {
  const index = previewDebugSlotIndex(state, position, objectId);
  if (index >= 0) {
    state.slots[index] = Math.trunc(Number(objectId)) || 0;
  }
}

function previewDebugRemoveObject(state, position, objectId) {
  const index = previewDebugSlotIndex(state, position, objectId);
  if (index >= 0 && (!objectId || state.slots[index] === Math.trunc(Number(objectId)))) {
    state.slots[index] = 0;
  }
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

async function previewRuntimeAssetValue(kind) {
  if (!window.PuzzleStudioRuntime?.gameRuntimeAssets) {
    throw new Error("PuzzleStudio game runtime assets are unavailable.");
  }
  const assets = await window.PuzzleStudioRuntime.gameRuntimeAssets();
  if (kind === "puzzle_wasm_game.js") {
    return assets.moduleSource;
  }
  if (kind === "puzzle_wasm_game_bg.wasm.base64") {
    return assets.wasmBase64;
  }
  throw new Error(`Unknown editor preview runtime asset: ${kind}`);
}

function previewRuntimeAssetSourceAllowed(source) {
  return source && previewRuntimeAssetWindows.has(source);
}

function respondToPreviewRuntimeAssetRequest(event, payload) {
  const target = event.source;
  if (!target || !previewRuntimeAssetSourceAllowed(target)) {
    return;
  }
  const requestId = String(payload.requestId || "");
  const kind = String(payload.kind || "");
  previewRuntimeAssetValue(kind)
    .then((value) => {
      target.postMessage({
        type: "PuzzleStudioRuntimeAssetResponse",
        requestId,
        ok: true,
        value,
      }, "*");
    })
    .catch((error) => {
      target.postMessage({
        type: "PuzzleStudioRuntimeAssetResponse",
        requestId,
        ok: false,
        error: String(error?.message || error),
      }, "*");
    });
}

window.addEventListener("message", (event) => {
  const payload = event.data || {};
  if (payload.type === "PuzzleStudioRuntimeAssetRequest") {
    respondToPreviewRuntimeAssetRequest(event, payload);
  }
});

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

function setPreviewFrameHtml(html, options = {}) {
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
  nextFrame.setAttribute("allow", "autoplay");
  nextFrame.setAttribute("scrolling", "no");
  nextFrame.setAttribute("aria-hidden", "true");
  nextFrame.style.visibility = "hidden";
  previewViewport.append(nextFrame);
  if (nextFrame.contentWindow) {
    previewRuntimeAssetWindows.add(nextFrame.contentWindow);
  }

  nextFrame.addEventListener("load", () => {
    if (loadId !== previewFrameLoadId) {
      if (nextFrame.contentWindow) {
        previewRuntimeAssetWindows.delete(nextFrame.contentWindow);
      }
      nextFrame.remove();
      if (nextObjectUrl) {
        URL.revokeObjectURL(nextObjectUrl);
      }
      return;
    }
    previousFrame.removeAttribute("id");
    if (previousFrame.contentWindow) {
      previewRuntimeAssetWindows.delete(previousFrame.contentWindow);
    }
    nextFrame.id = "previewFrame";
    nextFrame.removeAttribute("aria-hidden");
    nextFrame.style.visibility = "";
    previousFrame.remove();
    previewFrame = nextFrame;
    previewFrameObjectUrl = nextObjectUrl;
    if (options.markDocumentLoaded) {
      setPreviewDocumentLoaded(true);
    }
    schedulePreviewViewportSync(6);
    syncPreviewDebugModeToFrame();
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
  const progressSaveData = JSON.stringify(Object.fromEntries(editorPreviewProgressSaves)).replace(/<\//g, "<\\/");
  const progressSaveScript = `<script id="puzzle-studio-editor-preview-progress-save-script">
window.PuzzleStudioEditorPreviewProgressSaves = ${progressSaveData};
<\/script>`;
  const consoleScript = `<script id="puzzle-studio-editor-preview-log-script">
(() => {
  const isEditorSaveShortcut = (event) => {
    if (!event || event.altKey) {
      return false;
    }
    const modifier = (event.metaKey && !event.ctrlKey) || (event.ctrlKey && !event.metaKey);
    const key = event.key && event.key.length === 1 ? event.key.toLowerCase() : event.key;
    return modifier && key === "s";
  };
  document.addEventListener("keydown", (event) => {
    if (!isEditorSaveShortcut(event)) {
      return;
    }
    event.preventDefault();
    event.stopImmediatePropagation();
    try {
      window.parent.postMessage({ type: "PuzzleStudioEditorSaveShortcut" }, "*");
    } catch (_error) {
      // Editor shortcuts must not affect the preview runtime.
    }
  }, true);
  const formatArg = (value, depth = 0) => {
    if (typeof value === "string") {
      return value;
    }
    if (value instanceof Error) {
      const headline = [value.name || "Error", value.message || ""]
        .filter(Boolean)
        .join(": ");
      const stack = String(value.stack || "");
      if (!stack) {
        return headline || String(value);
      }
      return value.message && stack.includes(value.message)
        ? stack
        : [headline, stack].filter(Boolean).join("\\n");
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
    if (window.PuzzleStudioPreviewRuntimeFailure && event.message === "Script error.") {
      return;
    }
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
    const failure = window.PuzzleStudioPreviewRuntimeFailure;
    const reasonMessage = String(event.reason?.message || event.reason || "");
    if (failure?.message && failure.message === reasonMessage) {
      return;
    }
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
  const ensureDebugOverlay = () => {
    let overlay = document.getElementById("puzzle-studio-debug-overlay");
    if (!overlay) {
      overlay = document.createElement("div");
      overlay.id = "puzzle-studio-debug-overlay";
      overlay.setAttribute("aria-hidden", "true");
      overlay.style.position = "absolute";
      overlay.style.inset = "0";
      overlay.style.pointerEvents = "none";
      overlay.style.zIndex = "2147483647";
      overlay.style.display = "none";
      document.body.append(overlay);
    }
    return overlay;
  };
  const debugBoardTarget = () => (
    document.querySelector("img[alt^='Board']")
    || document.querySelector("canvas")
    || document.querySelector("[aria-label^='Puzzle board'] img")
    || document.querySelector("[aria-label^='Puzzle board'] canvas")
    || document.querySelector("[aria-label^='Puzzle board']")
    || document.querySelector(".board")
    || document.body
  );
  const clearDebugOverlay = () => {
    const overlay = document.getElementById("puzzle-studio-debug-overlay");
    if (!overlay) {
      return;
    }
    overlay.replaceChildren();
    overlay.style.display = "none";
  };
  const renderDebugOverlay = (payload) => {
    const cells = Array.isArray(payload?.cells) ? payload.cells : [];
    if (!cells.length) {
      clearDebugOverlay();
      return;
    }
    const board = payload?.board || {};
    const width = Math.max(1, Math.trunc(Number(board.width) || 1));
    const height = Math.max(1, Math.trunc(Number(board.height) || 1));
    const target = debugBoardTarget();
    const rect = target.getBoundingClientRect();
    const overlay = ensureDebugOverlay();
    const bodyRect = document.body.getBoundingClientRect();
    overlay.replaceChildren();
    overlay.style.display = "block";
    overlay.style.left = "0";
    overlay.style.top = "0";
    overlay.style.width = "100%";
    overlay.style.height = "100%";
    const layer = document.createElement("div");
    layer.style.position = "absolute";
    layer.style.left = String(rect.left - bodyRect.left) + "px";
    layer.style.top = String(rect.top - bodyRect.top) + "px";
    layer.style.width = String(rect.width) + "px";
    layer.style.height = String(rect.height) + "px";
    layer.style.boxSizing = "border-box";
    overlay.append(layer);
    for (const cell of cells) {
      const x = Math.trunc(Number(cell?.x));
      const y = Math.trunc(Number(cell?.y));
      if (!Number.isFinite(x) || !Number.isFinite(y) || x < 0 || y < 0 || x >= width || y >= height) {
        continue;
      }
      const marker = document.createElement("div");
      const kind = String(cell.kind || "change");
      const color = kind === "remove" || kind === "move_from"
        ? "rgba(255, 0, 77, 0.72)"
        : kind === "add" || kind === "move_to"
          ? "rgba(0, 228, 54, 0.72)"
          : "rgba(41, 173, 255, 0.68)";
      marker.style.position = "absolute";
      marker.style.left = String((x / width) * 100) + "%";
      marker.style.top = String((y / height) * 100) + "%";
      marker.style.width = String(100 / width) + "%";
      marker.style.height = String(100 / height) + "%";
      marker.style.boxSizing = "border-box";
      marker.style.border = "2px solid " + color;
      marker.style.background = color.replace(/0\\.[0-9]+/, "0.14");
      marker.style.boxShadow = "0 0 0 1px rgba(0, 0, 0, 0.28) inset";
      layer.append(marker);
    }
  };
  window.addEventListener("message", (event) => {
    if (event.data?.type === "PuzzleStudioPreviewDebugHighlight") {
      renderDebugOverlay(event.data);
    }
  });
})();
<\/script>`;
  let next = html;
  if (!next.includes("puzzle-studio-editor-preview-progress-save-script")) {
    if (next.includes("</head>")) {
      next = next.replace("</head>", `${progressSaveScript}\n  </head>`);
    } else if (next.includes("<body")) {
      next = next.replace("<body", `${progressSaveScript}\n<body`);
    } else {
      next = `${progressSaveScript}\n${next}`;
    }
  }
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

function syncPreviewViewportAspect(sceneName = previewSession?.state?.screen || "") {
  setPreviewViewportAspect(previewAspectForScene(previewBuild?.exportData, sceneName));
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

function previewAspectForScene(exportData = previewBuild?.exportData, sceneName = "") {
  const scenes = exportData?.scenes || [];
  let scene = sceneName
    ? scenes.find((candidate) => candidate?.name === sceneName)
    : null;
  if (!scene) {
    const initialName = exportData?.surface?.focus || scenes[0]?.name || "";
    scene = initialName
      ? scenes.find((candidate) => candidate?.name === initialName)
      : null;
  }
  const width = Number(scene?.layout?.aspectRatio?.width);
  const height = Number(scene?.layout?.aspectRatio?.height);
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
      loadFocusedPuzzleEntry("level", firstLevel, { silent: true, recordHistory: false });
    } else if (currentPreviewMode === "visual" || currentPreviewMode === "visual3d") {
      loadFocusedPuzzleEntry("visual", firstVisual, { silent: true, recordHistory: false });
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
      const activeSourceDocument = activeDocument();
      if (
        isPuzzleDocument(activeSourceDocument)
        && currentSourceForDocument(activeSourceDocument) === text
      ) {
        const previousPreviewDocument = activePreviewDocument();
        const previousPreviewTarget = previousPreviewDocument
          ? documentIdentityKey(previousPreviewDocument)
          : "";
        activeSourceDocument.declaresGameEntry = entryInfo?.declaresGameEntry === true;
        ensurePreviewTargetsActiveDocument();
        const nextPreviewDocument = activePreviewDocument();
        const nextPreviewTarget = nextPreviewDocument
          ? documentIdentityKey(nextPreviewDocument)
          : "";
        recordLoadedPreviewTarget(nextPreviewDocument);
        if (nextPreviewTarget !== previousPreviewTarget) {
          schedulePreview();
        }
      }
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
  if (typeof window.PuzzleStudioRuntime?.workspaceSourceEntries !== "function") {
    const message = "Workspace source entries are unavailable.";
    if (options.reportUnavailable !== false) {
      setStatus(message, "is-error");
    }
    throw new Error(message);
  }
  let rawEntries;
  try {
    rawEntries = window.PuzzleStudioRuntime.workspaceSourceEntries(text);
  } catch (error) {
    const message = `Source entries unavailable: ${userFacingRuntimeError(error)}`;
    if (options.reportUnavailable !== false) {
      setStatus(message, "is-error");
    }
    throw new Error(message);
  }
  const entries = normalizeResolvedSourceTargets(text, rawEntries);
  surfaceEntriesCache = { source: text, entries };
  return entries;
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
  const target = currentLevelSourceLocation({ sourceScope: "workspace" });
  if (!target || target.document?.id !== context.document.id) {
    return null;
  }
  const levelIndex = Number.isInteger(target.levelIndex) ? target.levelIndex : currentEditableLevelIndex();
  return {
    dimension: "2d",
    target: {
      ...target,
      document: context.document,
      name: previewBuild?.exportData?.levels?.[levelIndex]?.name || target.name || "",
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
    return finishFocusedPuzzleEntryLoad(loadVisualSourceTarget(target, { silent: options.silent !== false, recordHistory: Boolean(options.recordHistory), switchMode: true }));
  }
  if (mode === "visual3d" && dimension === "3d" && typeof loadVisual3dSourceTarget === "function") {
    currentVisualPaneMode = "visual3d";
    return finishFocusedPuzzleEntryLoad(loadVisual3dSourceTarget(target, { silent: options.silent !== false, recordHistory: Boolean(options.recordHistory), switchMode: true }));
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
  return document?.id === activeDocument()?.id
    ? sourceEditorDocumentValue()
    : document?.source || "";
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
  level.layers = [cloneVisualEditValue(level.cells)];
  level.activeLayer = 0;
  level.showCompositeLayers = false;
  renderLevelBoard();
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
  return `${prefix}levels ${normalizedBundle} {\n  legend {\n    ${LEVEL3D_EMPTY_CHAR} = empty\n  }\n\n${levelSource}\n}\n`;
}

function addEmptyLevel3dToFocusedSource() {
  const document = focusedPuzzleTextDocument();
  if (!document) {
    setPaneStatus("level", "No puzzle source for 3D level", "is-error");
    return false;
  }
  const name = "level 1";
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

function openLevelPaneForCurrentPreviewLevel() {
  const build = previewBuild;
  const exportData = currentPreviewExportData();
  if (!build || previewSession?.buildId !== build.id || !Array.isArray(exportData?.levels) || !exportData.levels.length) {
    setPaneStatus("level", "No compiled preview level", "is-error");
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
  const loaded = loadResolvedSourceTarget({
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
  const context = await focusedPuzzleSourceContextWithEntries();
  if (!context) {
    return false;
  }
  ensurePreviewTargetsActiveDocument();
  const mode = ["edit", "level3d"].includes(options.mode)
    ? options.mode
    : levelModeForEditorDimension(currentEditorDimension);
  const loaded = loadAvailableLevelPaneEntry(context, {
    mode,
    silent: true,
    recordHistory: false,
  });
  if (!loaded) {
    openPreviewModePane(mode);
    currentLevelPaneMode = mode;
    if (mode === "edit") {
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

function openVisualPaneForCurrentDimension() {
  const context = focusedPuzzleSourceContext();
  const first = firstFocusedPuzzleEntry("visual", context);
  const mode = first
    ? visualModeForEditorDimension(first.dimension)
    : visualModeForEditorDimension(currentEditorDimension);
  openPreviewModePane(mode);
  if (first) {
    loadFocusedPuzzleEntry("visual", first, { silent: true, recordHistory: false });
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
  const wasLevelMode = isPaneVisible("level") || isPaneVisible("solver");
  const wasVisualMode = currentPreviewMode === "visual";
  const previewMode = normalizePreviewMode(mode);
  if (currentPreviewMode === "solver" && previewMode !== "solver") {
    setEditorSolverDisplayedArtifact("");
  }
  hideEditorHoverTooltip();
  if (previewMode !== "edit" && levelPlaytestActive) {
    stopLevelPlaytest({ syncPreview: false });
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
  const enteringLevelMode = (editMode || solverMode) && !wasLevelMode;
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
  ensureLevel3dPaneFrameWidth();
  syncPreviewViewportScale();
  scheduleBoardScaleSync(3);
  if (!isPaneVisible("sounds")) {
    stopSoundPlayback();
  }
  if (editMode) {
    resetLevelBuilderFromSource(false);
    const loadedSourceLevel = loadLevelPaneEntryForMode("edit", focusedPuzzleSourceContext(), {
      silent: true,
      recordHistory: false,
      openPane: false,
    });
    if (!loadedSourceLevel && (enteringLevelMode || !level.cells.length)) {
      loadLevelFromPreviewState();
    } else if (levelSolutionPreview) {
      clearSolutionPreview();
      renderLevelBoard();
    }
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
    ensureLevel3dRuntimePreviewForOpenPane();
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

function currentPreviewRuntimeLevelIndex(exportData = currentPreviewExportData()) {
  if (!Array.isArray(exportData?.levels) || !exportData.levels.length) {
    return null;
  }
  const state = previewSessionState();
  if (!state || state.screenHasPuzzle === false || !Number.isInteger(Number(state.levelIndex))) {
    return null;
  }
  return normalizedLevelIndex(Math.trunc(Number(state.levelIndex)), exportData);
}

function restoreCompiledGamePreview() {
  if (!previewBuild?.html || !previewFrame) {
    return;
  }
  if (!previewFrameHasEditorLevelState && previewFrameHasCurrentCompiledPreview) {
    return;
  }
  previewFrameHasEditorLevelState = false;
  previewFrameHasCurrentCompiledPreview = true;
  const session = ensurePreviewSession();
  session.state = null;
  session.runtimeStatus = null;
  setPreviewDocumentLoaded(false);
  setPreviewFrameHtml(editorPreviewDocument(previewBuild?.html), { markDocumentLoaded: true });
  syncPreviewLevelActionButtons();
}

function activePreviewModeAcceptsLevelState() {
  return currentPreviewMode === "edit" && levelPlaytestActive;
}

function resetLevelBuilderFromSource(resetCells = true) {
  clearLevelEditSource();
  levelDisplayCells = null;
  const exportData = currentPreviewExportData();
  level.exportData = exportData;
  level.palette = levelPaletteFromExport(levelReferenceSource(exportData), exportData);
  level.activeLayer = normalizedLevelActiveLayer(level.activeLayer, exportData);
  const size = initialLevelSize(exportData);
  if (resetCells) {
    level.width = size.width || level.width;
    level.height = size.height || level.height;
    level.regions = defaultLevelRegions(level.width, level.height);
    level.cells = makeEmptyCells(level.width, level.height, exportData);
    level.layers = [cloneVisualEditValue(level.cells)];
    level.activeLayer = 0;
    level.showCompositeLayers = false;
  }
  ensureLevelLayerMaps(exportData);
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
  return previewBuild?.exportData || null;
}

function currentLevelExportData(exportData = null) {
  return exportData || level.exportData || currentPreviewExportData() || null;
}

function sourcePlaceableObjectNames(source, exportData = currentLevelExportData()) {
  return new Set(sourceCharEntries(source, exportData)
    .filter((entry) => entry.objects.length === 1)
    .map((entry) => entry.objects[0]));
}

function engineObjects(exportData = currentLevelExportData()) {
  return [...(exportData?.engine?.objects || [])]
    .sort((left, right) => left.layer - right.layer || left.name.localeCompare(right.name));
}

function engineObjectById(objectId, exportData = currentLevelExportData()) {
  return (exportData?.engine?.objects || []).find((object) => object.id === objectId) || null;
}

function layerCount(exportData = currentLevelExportData()) {
  return exportData?.engine?.layerCount
    || exportData?.levels?.[0]?.initialState?.layerCount
    || 1;
}

function initialLevelSize(exportData = currentPreviewExportData()) {
  const state = exportData?.levels?.[currentEditableLevelIndex(exportData)]?.initialState;
  if (state?.width && state?.height) {
    return { width: state.width, height: state.height };
  }
  return { width: 9, height: 5 };
}

function currentEditableLevelIndex(exportData = previewBuild?.exportData) {
  return setActiveLevelIndex(activeLevelIndex, exportData);
}

function setActiveLevelIndex(index, exportData = previewBuild?.exportData) {
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

function normalizedLevelIndex(index, exportData = previewBuild?.exportData) {
  const levels = exportData?.levels || [];
  if (!levels.length) {
    return 0;
  }
  const fallback = exportData.initialLevelIndex ?? 0;
  const rawIndex = index ?? fallback;
  return Math.max(0, Math.min(levels.length - 1, Math.trunc(Number(rawIndex) || 0)));
}

function cloneJson(value) {
  return value == null ? null : JSON.parse(JSON.stringify(value));
}

function solverLevelDescriptor(exportData, levelIndex) {
  const levelEntry = exportData?.levels?.[levelIndex];
  if (!levelEntry) {
    return null;
  }
  const descriptor = {
    index: levelIndex,
    levelName: levelEntry.name || "",
  };
  if (!isPuzzle3dExport(exportData)) {
    descriptor.levelPuzzle = levelEntry.puzzle || "";
    descriptor.levelPack = levelEntry.pack ?? null;
  }
  return descriptor;
}

function solverCompileId(solverBuild) {
  return solverBuild?.solverPrepared?.artifactId || "";
}

function clearSolverTask() {
  activeSolverTask = null;
  activeSolverDisplaySceneRequestKey = "";
  syncSolverLevelSelector();
  syncSolverTaskReadout();
  setSolveLevelButtonState(Boolean(activeLevelSolveRequest));
}

function setActiveSolverTask(task) {
  activeSolverTask = task ? cloneJson(task) : null;
  syncSolverLevelSelector();
  syncSolverTaskReadout();
  setSolveLevelButtonState(Boolean(activeLevelSolveRequest));
  return activeSolverTask;
}

function solverTaskRunKey(task = activeSolverTask) {
  if (!task) {
    return "";
  }
  return JSON.stringify({
    producer: task.producer || "",
    rules: {
      compileId: task.rules?.compileId || "",
      documentId: task.rules?.documentId || "",
      modelKind: task.rules?.modelKind || "",
    },
    level: task.level || null,
    state: task.state || null,
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

function previewStateMatchesSolverTask(task = activeSolverTask, previewState = previewSession?.state) {
  return Boolean(
    task?.producer === "preview-level"
    && previewState?.screenHasPuzzle !== false
    && Number.isInteger(Number(previewState?.levelIndex))
    && normalizedLevelIndex(Math.trunc(Number(previewState.levelIndex)), previewBuild?.exportData) === solverTaskLevelIndex(task)
  );
}

function applyPreviewSceneToActiveSolverTask(previewState = previewSession?.state) {
  if (
    !activeSolverTask
    || levelSolutionPreview
    || solverObservationPreview
    || !previewStateMatchesSolverTask(activeSolverTask, previewState)
    || !previewState?.scene
    || previewBuildIsStale
  ) {
    return false;
  }
  activeSolverTask.scene = cloneJson(previewState.scene);
  return true;
}

async function refreshActiveSolverTaskDisplayScene(exportData = previewBuild?.exportData) {
  if (!activeSolverTask || levelSolutionPreview || solverObservationPreview) {
    return false;
  }
  if (activeSolverTask.rules?.modelKind !== "2d") {
    return false;
  }
  if (applyPreviewSceneToActiveSolverTask()) {
    activeSolverDisplaySceneRequestKey = "";
    renderSolverBoard();
    return true;
  }

  const requestKey = solverTaskRunKey(activeSolverTask);
  activeSolverDisplaySceneRequestKey = requestKey;
  try {
    const state = await materializeEditorSolverState(activeSolverTask);
    if (!activeSolverTask || solverTaskRunKey(activeSolverTask) !== requestKey) {
      return false;
    }
    const levelIndex = solverTaskLevelIndex(activeSolverTask);
    const regions = exportData?.levels?.[levelIndex]?.regions || [];
    const scene = sceneFromStateData(state, { regions, exportData });
    if (!scene) {
      throw new Error("Solver display state did not contain a 2D board.");
    }
    activeSolverTask.scene = scene;
    activeSolverDisplaySceneRequestKey = "";
    renderSolverBoard();
    return true;
  } catch (error) {
    if (activeSolverTask && solverTaskRunKey(activeSolverTask) === requestKey) {
      activeSolverDisplaySceneRequestKey = "";
      setLevelSolveStatus(`Solver display failed: ${userFacingRuntimeError(error)}`, "is-error");
      renderSolverBoard();
    }
    return false;
  }
}

function scheduleActiveSolverTaskDisplaySceneRefresh(exportData = previewBuild?.exportData) {
  if (!activeSolverTask || activeSolverTask.rules?.modelKind !== "2d") {
    return;
  }
  refreshActiveSolverTaskDisplayScene(exportData).catch((error) => {
    if (activeSolverTask) {
      setLevelSolveStatus(`Solver display failed: ${userFacingRuntimeError(error)}`, "is-error");
      renderSolverBoard();
    }
  });
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

function syncSolverLevelSelector(exportData = previewBuild?.exportData) {
  if (!solverLevelSelect) {
    return;
  }
  const levels = Array.isArray(exportData?.levels) ? exportData.levels : [];
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

function selectSolverLevel(index, exportData = previewBuild?.exportData) {
  if (activeLevelSolveRequest) {
    syncSolverLevelSelector(exportData);
    return false;
  }
  const levels = Array.isArray(exportData?.levels) ? exportData.levels : [];
  if (!levels.length) {
    clearSolverTask();
    renderSolverBoard();
    return false;
  }
  const levelIndex = normalizedLevelIndex(index, exportData);
  solverSelectedLevelIndex = levelIndex;
  const task = createPreviewSolverTask(previewBuild, levelIndex);
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
  scheduleActiveSolverTaskDisplaySceneRefresh(exportData);
  return true;
}

function createSolverTask({ producer, solverBuild, exportData, levelIndex, stateKind, lifecycle, stateData, scene = null, puzzle3dSnapshot = null } = {}) {
  const targetIndex = normalizedLevelIndex(levelIndex, exportData);
  const levelInfo = solverLevelDescriptor(exportData, targetIndex);
  const modelKind = isPuzzle3dExport(exportData) ? "3d" : "2d";
  const prepared = solverBuild?.solverPrepared;
  if (!levelInfo || !stateData || !prepared?.artifactId || prepared.modelKind !== modelKind) {
    return null;
  }
  return {
    producer,
    rules: {
      compileId: solverCompileId(solverBuild),
      documentId: solverBuild.documentId || "",
      modelKind,
    },
    level: levelInfo,
    state: {
      kind: stateKind,
      lifecycle,
      data: cloneJson(stateData),
    },
    scene: cloneJson(scene),
    puzzle3dSnapshot: cloneJson(puzzle3dSnapshot),
  };
}

function createPreviewSolverTask(build, levelIndex) {
  const exportData = build?.exportData;
  const targetIndex = normalizedLevelIndex(levelIndex, exportData);
  if (isPuzzle3dExport(exportData)) {
    const snapshot = puzzle3dSnapshotForLevel(exportData, targetIndex);
    const stateData = puzzle3dStateDataFromSnapshot(snapshot, exportData, { levelIndex: targetIndex });
    return createSolverTask({
      producer: "preview-level",
      solverBuild: build,
      exportData,
      levelIndex: targetIndex,
      stateKind: "compiled-start",
      lifecycle: "playable-start",
      stateData,
      puzzle3dSnapshot: snapshot,
    });
  }
  const stateData = compiledLevelStateData(exportData, targetIndex);
  return createSolverTask({
    producer: "preview-level",
    solverBuild: build,
    exportData,
    levelIndex: targetIndex,
    stateKind: "compiled-start",
    lifecycle: "playable-start",
    stateData,
  });
}

function createEditorSolverTask({ solverBuild, exportData, levelIndex, stateData, scene = null, puzzle3dSnapshot = null } = {}) {
  return createSolverTask({
    producer: "level-editor",
    solverBuild,
    exportData,
    levelIndex,
    stateKind: "editor-staged",
    lifecycle: "playable-start",
    stateData,
    scene,
    puzzle3dSnapshot,
  });
}

function previewSolverTaskLevelIndex(exportData = previewBuild?.exportData) {
  const state = previewSessionState();
  if (
    state
    && state.screenHasPuzzle !== false
    && Number.isInteger(Number(state.levelIndex))
  ) {
    return normalizedLevelIndex(Math.trunc(Number(state.levelIndex)), exportData);
  }
  if (Number.isInteger(solverSelectedLevelIndex)) {
    return normalizedLevelIndex(solverSelectedLevelIndex, exportData);
  }
  return null;
}

function setPreviewSolverTaskFromActiveLevel(exportData = previewBuild?.exportData) {
  const levelIndex = previewSolverTaskLevelIndex(exportData);
  if (levelIndex === null) {
    clearSolverTask();
    return false;
  }
  const task = createPreviewSolverTask(previewBuild, levelIndex);
  if (!task) {
    clearSolverTask();
    return false;
  }
  setActiveSolverTask(task);
  clearSolutionPreview({ preserveSolverTask: true });
  setLevelSolveStatus("");
  if (!applyPreviewSceneToActiveSolverTask()) {
    scheduleActiveSolverTaskDisplaySceneRefresh(exportData);
  }
  return true;
}

function refreshVisiblePreviewSolverTask(exportData = previewBuild?.exportData) {
  if (currentPreviewMode !== "solver") {
    return false;
  }
  if (setPreviewSolverTaskFromActiveLevel(exportData)) {
    return true;
  }
  if (!levelSolveStatus?.textContent?.trim()) {
    setLevelSolveStatus("No level to solve", "is-error");
  }
  return false;
}

async function prepareSolverBuild(build, status = setLevelSolveStatus) {
  if (!build?.id || !Array.isArray(build.documents) || !build.documents.length) {
    status("No solver source snapshot", "is-error");
    return null;
  }
  const cached = solverPreparedByBuildId.get(build.id);
  if (cached?.artifactId) {
    return { ...build, solverPrepared: cached };
  }
  status("Preparing solver", "");
  try {
    const prepared = await prepareEditorSolverArtifact({
      documents: compilerDocumentsForSnapshot(build.documents),
      puzzlePath: build.puzzlePath,
      documentId: build.documentId,
    });
    solverPreparedByBuildId.set(build.id, prepared);
    status("Solver ready", "");
    return { ...build, solverPrepared: prepared };
  } catch (error) {
    status(`Solver prepare failed: ${userFacingRuntimeError(error)}`, "is-error");
    return null;
  }
}

async function ensurePreviewSolverBuild() {
  const build = previewBuild;
  if (!build || previewSession?.buildId !== build.id) {
    setLevelSolveStatus("No active preview", "is-error");
    return null;
  }
  const preparedBuild = await prepareSolverBuild(build);
  if (!preparedBuild || previewBuild?.id !== build.id) {
    setLevelSolveStatus("Preview changed while preparing solver", "is-error");
    return null;
  }
  return preparedBuild;
}

async function prepareCurrentDraftSolverBuild() {
  ensurePreviewTargetsActiveDocument();
  const document = activePreviewDocument();
  if (!isPuzzleDocument(document)) {
    setLevelSolveStatus("No rule model for edited level", "is-error");
    return null;
  }
  const presentationManifest = await ensurePreviewDocumentsLoaded(document);
  const build = capturePreviewBuildInput(document, presentationManifest);
  return prepareSolverBuild(build);
}

async function openSolverPaneForCurrentLevel() {
  openPreviewModePane("solver");
  solverSelectedLevelIndex = null;
  const build = await ensurePreviewSolverBuild();
  if (!build) {
    clearSolverTask();
    renderSolverBoard();
    return false;
  }
  const exportData = build.exportData;
  refreshVisiblePreviewSolverTask(exportData);
  requestFocusedPreviewState();
  await refreshActiveSolverTaskDisplayScene(exportData);
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

function loadLevelSourceTarget(target, options = {}) {
  if (!isPuzzleDocument(activeDocument()) || !isTextDocument(activeDocument())) {
    return null;
  }
  ensurePreviewTargetsActiveDocument();
  const document = activeDocument();
  const source = sourceEditorDocumentValue();
  const sourceEntry = sourceEditableEntryFromTarget(source, target, { defaultName: "" });
  return loadLevelSourceEntry(source, sourceEntry, { ...options, document });
}

function loadLevelSourceEntry(source, entry, options = {}) {
  ensurePreviewTargetsActiveDocument();
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
  let exportData = null;
  try {
    exportData = levelEditorSourceExportData(source);
  } catch (error) {
    reportLevelSourceLoadFailure(
      `Could not load level editor source contract: ${userFacingRuntimeError(error)}`,
      options,
    );
    return null;
  }
  return loadLevelSourceEntryWithExportData(source, entry, exportData, loadOptions);
}

function levelEditorSourceExportData(source) {
  if (typeof window.PuzzleStudioRuntime?.levelEditorSourceSession !== "function") {
    throw new Error("Editor WASM function is missing: levelEditorSourceSession");
  }
  const session = window.PuzzleStudioRuntime.levelEditorSourceSession(source);
  const contract = session.manifest();
  if (contract?.version !== 2 || contract?.kind !== "puzzle2d-level-editor") {
    throw new Error(`Unsupported level editor source contract version: ${contract?.version ?? "missing"}`);
  }
  if (!Array.isArray(contract.objects)) {
    throw new Error("Level editor source contract has invalid objects");
  }
  const objects = contract.objects.map((object) => ({
    id: Number(object.id),
    name: String(object.name || ""),
    layer: Math.max(0, Math.trunc(Number(object.layer) || 0)),
    visual: String(object.name || ""),
  }));
  if (objects.some((object) => !object.name || !Number.isSafeInteger(object.id) || object.id <= 0)
    || new Set(objects.map((object) => object.id)).size !== objects.length) {
    throw new Error("Level editor source contract contains invalid object identities");
  }
  applyLevelEditorContractVisuals(session, objects);
  return {
    __kind: "puzzle2d",
    source,
    editorSourceContract: contract,
    editorSourceSession: session,
    engine: {
      objects,
      layerCount: Math.max(1, ...objects.map((object) => object.layer + 1)),
    },
    levels: Array.isArray(contract.levels)
      ? contract.levels.map((level) => ({
        name: String(level?.name || ""),
        sourceLevelIndex: Math.max(0, Math.trunc(Number(level?.sourceLevelIndex) || 0)),
        regions: Array.isArray(level?.regions) ? level.regions : [],
        width: Math.max(1, Math.trunc(Number(level?.width) || 0)),
        height: Math.max(1, Math.trunc(Number(level?.height) || 0)),
        layerCount: Math.max(1, Math.trunc(Number(level?.layerCount) || 0)),
        authoredLayerCount: Math.max(0, Math.trunc(Number(level?.authoredLayerCount) || 0)),
        editorLegend: Array.isArray(level?.legend) ? level.legend : [],
      }))
      : [],
    initialLevelIndex: 0,
  };
}

function levelEditorContractState(level, slots) {
  if (!level || !(slots instanceof Uint32Array)) {
    throw new Error("Level editor source contract contains an invalid level slot buffer");
  }
  return {
    width: level.width,
    height: level.height,
    layerCount: level.layerCount,
    slots: Array.from(slots),
  };
}

function applyLevelEditorContractVisuals(session, objects) {
  ensureGameVisualsRuntime();
  const aliases = {};
  const entries = {};
  for (const object of objects) {
    const payload = session.visual(object.id);
    if (!payload) {
      continue;
    }
    const visualName = `object:${object.id}`;
    aliases[object.name] = visualName;
    entries[visualName] = payload;
  }
  window.PuzzleStudio.disposeAssetScripts();
  window.GameVisuals = window.PuzzleVisualRegistry.create({ aliases, entries });
}

function loadLevelSourceEntryWithExportData(source, entry, exportData, options = {}) {
  const levels = exportData?.levels || [];
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
  const integrationDiagnostics = Array.isArray(exportData.editorSourceContract?.diagnostics)
    ? exportData.editorSourceContract.diagnostics.filter((diagnostic) => typeof diagnostic === "string" && diagnostic)
    : [];
  if (integrationDiagnostics.length) {
    const message = `Level editor loaded with source diagnostics: ${integrationDiagnostics[0]}`;
    setPaneStatus("level", message, "is-error");
    if (!options.silent) {
      setStatus(message, "is-error");
    }
  } else if (!options.silent) {
    setStatus(`Loaded level ${levelName}`, "is-ok");
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
  level.width = state.width;
  level.height = state.height;
  level.regions = state.regions;
  level.layers = state.layers;
  level.cells = state.cells;
  level.exportData = exportData;
  level.palette = levelPaletteFromExport(referenceSource, exportData);
  level.activeLayer = normalizedLevelActiveLayer(level.activeLayer, exportData);
  if (!level.palette.some((entry) => entry.id === level.selectedObjectId)) {
    level.selectedObjectId = level.palette[0]?.id ?? 0;
  }
  renderLevelPalette();
  renderLevelBoard();
  if (levelPlaytestActive && !previewBuildIsStale && exportData === currentPreviewExportData()) {
    sendLevelStateToPreview(options.levelIndex ?? currentEditableLevelIndex(exportData), levelStateData(exportData), {
      materializeLevelStart: false,
      materializeDisplay: false,
      silent: true,
    });
  }
  return true;
}

function sourceLevelStateFromEntry(source, entry, exportData = currentLevelExportData(), options = {}) {
  if (!entry || !exportData?.engine?.objects?.length) {
    return null;
  }
  if (exportData.editorSourceContract) {
    const levelIndex = previewLevelIndexForSourceEntry(entry, exportData);
    const integrated = exportData.levels?.[levelIndex];
    const session = exportData.editorSourceSession;
    if (!integrated || !session) {
      return null;
    }
    if (!integrated.initialState) {
      integrated.initialState = levelEditorContractState(integrated, session.levelSlots(levelIndex));
    }
    if (!integrated.authoringLayers) {
      integrated.authoringLayers = Array.from({ length: integrated.authoredLayerCount }, (_, authoredLayer) => (
        levelEditorContractState(integrated, session.levelSlots(levelIndex, authoredLayer))
      ));
    }
    const layers = integrated.authoringLayers.map((state) => (
      stateDataToEditorCells(state, exportData)
    ));
    return {
      width: integrated.initialState.width,
      height: integrated.initialState.height,
      regions: normalizedLevelRegions(integrated.regions || [], integrated.initialState.width, integrated.initialState.height),
      layers,
      cells: stateDataToEditorCells(integrated.initialState, exportData),
    };
  }
  const parsed = sourceLevelRowsAndLocalLegends(source, entry);
  if (!parsed.rows.length) {
    return null;
  }
  const referenceSource = options.referenceSource || levelReferenceSource(exportData);
  const charEntries = [
    ...sourceCharEntries(referenceSource, exportData),
    ...parsed.localLegends.map((row) => legendEntryFromRow(row, new Set(engineObjects(exportData).map((object) => object.name)))).filter(Boolean),
  ];
  const charMap = new Map(charEntries.map((charEntry) => [charEntry.char, charEntry.objects]));
  const regionGroups = sourceLevelRegionGroups(parsed.rows);
  if (!regionGroups?.length) {
    return null;
  }
  const regions = [];
  let width = 0;
  let height = 0;
  let asciiLayerCount = 1;
  for (const regionLayers of regionGroups) {
    asciiLayerCount = Math.max(asciiLayerCount, regionLayers.length);
    const baseLayer = regionLayers[0] || [];
    const regionWidth = Math.max(1, ...baseLayer.map((row) => [...row].length));
    const regionHeight = Math.max(1, baseLayer.length);
    if (regionLayers.some((layerRows) => layerRows.length !== regionHeight || layerRows.some((row) => [...row].length !== regionWidth))) {
      return null;
    }
    regions.push({ index: regions.length, x: width, y: 0, width: regionWidth, height: regionHeight });
    width += regionWidth;
    height = Math.max(height, regionHeight);
  }
  const previousWidth = level.width;
  const previousHeight = level.height;
  level.width = width;
  level.height = height;
  const layers = Array.from({ length: asciiLayerCount }, () => makeEmptyCells(width, height, exportData));
  const objectIdsByName = new Map(engineObjects(exportData).map((object) => [object.name, object.id]));
  for (const [regionIndex, regionLayers] of regionGroups.entries()) {
    const region = regions[regionIndex];
    for (const [asciiLayerIndex, group] of regionLayers.entries()) {
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
          layers[asciiLayerIndex][((region.y + y) * width) + region.x + x] = slots;
        }
      }
    }
  }
  const previousLayers = level.layers;
  level.layers = layers;
  const cells = levelCompositeCells({ includeHidden: true, exportData });
  level.layers = previousLayers;
  level.width = previousWidth;
  level.height = previousHeight;
  return {
    width,
    height,
    regions: normalizedLevelRegions(regions, width, height),
    layers,
    cells,
  };
}

function stateDataToEditorCells(state, exportData) {
  const width = Math.max(1, Math.trunc(Number(state?.width) || 0));
  const height = Math.max(1, Math.trunc(Number(state?.height) || 0));
  const layerCount = Math.max(1, Math.trunc(Number(state?.layerCount) || 0));
  if (!Array.isArray(state?.slots) || state.slots.length !== width * height * layerCount) {
    throw new Error("Level editor source contract state slot count does not match its dimensions");
  }
  return Array.from({ length: width * height }, (_, cellIndex) => {
    const slots = makeEmptyCell(exportData);
    for (let layer = 0; layer < Math.min(layerCount, slots.length); layer += 1) {
      slots[layer] = Math.max(0, Math.trunc(Number(state.slots[(cellIndex * layerCount) + layer]) || 0));
    }
    return slots;
  });
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

function sourceLevelRegionGroups(rows) {
  const regions = [];
  let currentLayers = [];
  let currentRows = [];
  let expectsLayerRows = false;
  const flushLayer = () => {
    if (!currentRows.length) {
      return !expectsLayerRows;
    }
    currentLayers.push(currentRows);
    currentRows = [];
    expectsLayerRows = false;
    return true;
  };
  const flushRegion = () => {
    if (!flushLayer()) {
      return false;
    }
    if (currentLayers.length) {
      regions.push(currentLayers);
      currentLayers = [];
    }
    return true;
  };
  for (const row of rows) {
    const text = String(row || "");
    if (!text.trim()) {
      if (!flushRegion()) {
        return null;
      }
      continue;
    }
    if (text === "+") {
      if (!flushLayer()) {
        return null;
      }
      expectsLayerRows = true;
      continue;
    }
    currentRows.push(text);
  }
  if (!flushRegion()) {
    return null;
  }
  return regions;
}

function previewLevelIndexForSourceEntry(entry, exportData = previewBuild?.exportData) {
  const levels = exportData?.levels || [];
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
      options.position ?? (
        sourceViewOffsetToDocumentOffset(sourceEditor.selectionStart, "start")
      ),
    ) || 0)),
  );
  const resolvedMode = previewModeForSourceTarget(sourceCursorResolveRegion);
  // The source structure (which block the caret sits in) only changes when the
  // text changes. While the text is unchanged and the caret is still inside the
  // last resolved target's range, the target is identical and the preview is
  // already in sync, so skip the full-source scan entirely. This covers cursor
  // navigation (arrows / click / selectionchange) without re-parsing.
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
  // input + selectionchange + arrow keyup each fire this for the same edit, and
  // resolve_source_target is a synchronous full-source WASM parse. Skip the
  // parse when the source and caret are unchanged from the last resolve so a
  // single keystroke costs one parse instead of two or three.
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
    .then((target) => {
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
      const key = target ? loadResolvedSourceTarget(target, loadOptions) || "" : "";
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
  const start = sourceDocumentOffsetToViewOffset(sourceStart, "start");
  sourceEditor.setSelectionRange(start, start);
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
  const levelName = exportData?.levels?.[levelIndex]?.name || "";
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
  const levelName = exportData?.levels?.[targetIndex]?.name || "";
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
  const name = sourcePuzzleLevelHeaderName(code);
  if (name === null) {
    return null;
  }
  let levelIndex = 0;
  for (const previous of lines.slice(0, lineIndex)) {
    if (sourcePuzzleLevelHeaderName(levelScannerCode(previous.raw)) !== null) {
      levelIndex += 1;
    }
  }
  return {
    name,
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
      sourceName = sourcePuzzleLevelHeaderName(code);
      if (sourceName === null) {
        break;
      }
      const name = levelDefinitionName(levelsRange, sourceName, ordinal);
      entry = code.endsWith("{")
        ? bracedLevelEntry(source, lines, index, name, levelsRange.bodyEnd)
        : unbracedLevelEntry(lines, index, index + 1, name, levelsRange.bodyEnd);
    } else if (tokens.length === 1 && tokens[0] === "{") {
      entry = bracedLevelEntry(source, lines, index, levelDefinitionName(levelsRange, "", ordinal), levelsRange.bodyEnd);
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
    const name = sourcePuzzleLevelHeaderName(code);
    if (name === null) {
      continue;
    }
    const entry = code.endsWith("{")
      ? bracedLevelEntry(source, lines, index, name, source.length)
      : endDelimitedStandaloneLevelEntry(lines, index, name);
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
  return (tokens[0] === "level" && sourcePuzzleLevelHeaderName(line) !== null && String(line || "").trim().endsWith("{"))
    || (tokens.length === 1 && tokens[0] === "{")
    || (tokens[0] !== "level" && startsInlineBlockForWasm(tokens, line));
}

function isLevelsSectionBoundary(tokens) {
  return startsPuzzleSectionForWasm(tokens) && !["level"].includes(tokens[0] || "");
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
  const levels = previewBuild?.exportData?.levels || [];
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
  return `level ${Math.max(0, index) + 1}`;
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
  const sourceLayers = Array.isArray(level.layers) && level.layers.length
    ? level.layers
    : [Array.isArray(level.cells) && level.cells.length ? level.cells : makeEmptyLevelLayer(level.width, level.height, exportData)];
  level.layers = sourceLayers.map((layerCells) => normalizeLevelLayerMap(layerCells, exportData));
  if (!level.layers.length) {
    level.layers = [makeEmptyLevelLayer(level.width, level.height, exportData)];
  }
  level.activeLayer = normalizedLevelActiveLayer(level.activeLayer);
  level.cells = levelCompositeCells({ includeHidden: true, exportData });
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

function setLevelLayer(layerIndex) {
  ensureLevelLayerMaps();
  level.activeLayer = normalizedLevelActiveLayer(layerIndex);
  level.showCompositeLayers = false;
  levelLayerInsertMode = false;
  levelLayerRemoveMode = false;
  renderLevelPalette();
  renderLevelBoard();
  setStatus(`Editing layer ${level.activeLayer + 1}`, "is-ok");
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

function insertLevelLayerAt(index) {
  if (levelPlaytestActive) {
    return false;
  }
  const before = visualEditSnapshot("level");
  ensureLevelLayerMaps();
  const insertAt = Math.max(0, Math.min(levelLayerCount2d(), Math.trunc(Number(index) || 0)));
  level.layers.splice(insertAt, 0, makeEmptyLevelLayer());
  level.activeLayer = insertAt;
  level.showCompositeLayers = false;
  levelLayerInsertMode = false;
  level.cells = levelCompositeCells({ includeHidden: true });
  renderLevelPalette();
  renderLevelBoard();
  pushVisualEditUndoSnapshot("level", before);
  setStatus(`Added layer ${level.activeLayer + 1}`, "is-ok");
  return true;
}

function removeLevelLayerAt(index) {
  if (levelPlaytestActive || levelLayerCount2d() <= 1) {
    return false;
  }
  const before = visualEditSnapshot("level");
  ensureLevelLayerMaps();
  const removeAt = Math.max(0, Math.min(levelLayerCount2d() - 1, Math.trunc(Number(index) || 0)));
  level.layers.splice(removeAt, 1);
  level.activeLayer = normalizedLevelActiveLayer(Math.min(removeAt, level.layers.length - 1));
  level.showCompositeLayers = false;
  levelLayerRemoveMode = false;
  level.cells = levelCompositeCells({ includeHidden: true });
  renderLevelPalette();
  renderLevelBoard();
  pushVisualEditUndoSnapshot("level", before);
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
    if (window.PuzzleRenderer) {
      new window.PuzzleRenderer(view, { renderMode: "dom", themeRoot: view }).render(levelScene(levelLayerCells(index, exportData), exportData));
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

function selectLevelPaletteIndex(index) {
  const objects = level.palette.filter((object) => object.id !== 0);
  const object = objects[index];
  return object ? selectLevelPaletteObject(object.id) : false;
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
  if (!exportData?.engine?.objects?.length) {
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
  const cells = displayedLevelCells();
  const exportData = currentLevelExportData();
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
  if (!solverBoard) {
    return;
  }
  const exportData = previewBuild?.exportData;
  if (isPuzzle3dExport(exportData) && typeof renderSolverRuntimePreview === "function") {
    renderSolverRuntimePreview();
    return;
  }
  if (typeof clearSolverRuntimePreview === "function") {
    clearSolverRuntimePreview();
  }
  const scene = displayedSolverScene(exportData);
  if (!scene) {
    solverBoard.replaceChildren();
    syncSolverBoardScale();
    scheduleBoardScaleSync();
    return;
  }
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
  const scene = previewSceneForLevel(levelIndex, exportData);
  if (!scene?.width || !scene?.height || !Array.isArray(scene.cells)) {
    return false;
  }
  clearSolutionPreview();
  stopLevelPlaytest({ syncPreview: false });
  levelDisplayCells = null;
  level.width = scene.width;
  level.height = scene.height;
  level.regions = normalizedLevelRegions(scene.regions, level.width, level.height);
  level.cells = scene.cells.map((cell) => cellSlotsFromLayers(cell.layers || [], exportData));
  level.layers = [cloneVisualEditValue(level.cells)];
  level.activeLayer = 0;
  level.showCompositeLayers = false;
  level.exportData = exportData;
  level.palette = levelPaletteFromExport(levelReferenceSource(exportData), exportData);
  const levelName = exportData?.levels?.[levelIndex]?.name;
  if (levelName) {
    setLevelNameInputs(levelName);
  }
  renderLevelPalette();
  renderLevelBoard();
  if (requestRender) {
    sendLevelStateToPreview(levelIndex, levelStateData(exportData));
  }
  return true;
}

function applyPreviewSceneToLevel(scene) {
  if (!scene?.width || !scene?.height || !Array.isArray(scene.cells)) {
    return false;
  }
  const exportData = currentPreviewExportData();
  clearSolutionPreview();
  stopLevelPlaytest({ syncPreview: false });
  levelDisplayCells = null;
  level.width = scene.width;
  level.height = scene.height;
  level.regions = normalizedLevelRegions(scene.regions, level.width, level.height);
  level.cells = scene.cells.map((cell) => cellSlotsFromLayers(cell.layers || [], exportData));
  level.layers = [cloneVisualEditValue(level.cells)];
  level.activeLayer = 0;
  level.showCompositeLayers = false;
  level.exportData = exportData;
  level.palette = levelPaletteFromExport(levelReferenceSource(exportData), exportData);
  renderLevelPalette();
  renderLevelBoard();
  scheduleBoardScaleSync(2);
  return true;
}

function initialPreviewScene() {
  return previewSceneForLevel(previewBuild?.exportData?.initialLevelIndex || 0);
}

function previewSceneForLevel(levelIndex, exportData = previewBuild?.exportData) {
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
  const exportData = options.exportData || previewBuild?.exportData;
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
            visual: object.visual,
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

function cellSlotsFromLayers(layers, exportData = previewBuild?.exportData) {
  const slots = makeEmptyCell(exportData);
  for (const layer of layers) {
    if (Number.isInteger(layer.layer) && layer.layer >= 0 && layer.layer < slots.length) {
      slots[layer.layer] = objectIdForLayer(layer, exportData);
    }
  }
  return slots;
}

function objectIdForLayer(layer, exportData = previewBuild?.exportData) {
  const explicit = Number(layer?.objectId) || 0;
  if (explicit) {
    return explicit;
  }
  const name = layer?.object || "";
  const visual = layer?.visual || "";
  const object = (exportData?.engine?.objects || []).find((entry) =>
    (name && entry.name === name) || (visual && entry.visual === visual)
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

function displayedSolverScene(exportData = previewBuild?.exportData) {
  if (levelSolutionPreview?.kind !== "puzzle3d" && Array.isArray(levelSolutionPreview?.cells)) {
    const fallback = activeSolverTask?.scene;
    const state = activeSolverTask?.state?.data;
    return sceneFromCellSlots(levelSolutionPreview.cells, {
      width: fallback?.width || state?.width || level.width,
      height: fallback?.height || state?.height || level.height,
      regions: fallback?.regions || [],
      exportData,
    });
  }
  if (solverObservationPreview?.cells) {
    const fallback = activeSolverTask?.scene;
    const state = activeSolverTask?.state?.data;
    return sceneFromCellSlots(solverObservationPreview.cells, {
      width: fallback?.width || state?.width || level.width,
      height: fallback?.height || state?.height || level.height,
      regions: fallback?.regions || [],
      exportData,
    });
  }
  if (activeSolverTask?.scene) {
    return cloneJson(activeSolverTask.scene);
  }
  return null;
}

function displayedSolverCells() {
  if (levelSolutionPreview?.kind !== "puzzle3d" && Array.isArray(levelSolutionPreview?.cells)) {
    return levelSolutionPreview.cells;
  }
  if (solverObservationPreview) {
    return solverObservationPreview.cells;
  }
  if (activeSolverTask?.scene) {
    return sceneCellsToSlots(activeSolverTask.scene, []);
  }
  const scene = displayedSolverScene();
  return sceneCellsToSlots(scene, []);
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
    visual: object.visual,
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

  ensureLevelLayerMaps();
  const previousLayers = level.layers;
  const resizeLayerCells = (sourceCells) => {
    const targetOffsetX = normalizedMode === "expand" && edge === "left" ? 1 : 0;
    const targetOffsetY = normalizedMode === "expand" && edge === "top" ? 1 : 0;
    const sourceOffsetX = normalizedMode === "shrink" && edge === "left" ? 1 : 0;
    const sourceOffsetY = normalizedMode === "shrink" && edge === "top" ? 1 : 0;
    const resized = makeEmptyCells(nextWidth, nextHeight);
    for (let y = 0; y < nextHeight; y += 1) {
      for (let x = 0; x < nextWidth; x += 1) {
        const sourceX = normalizedMode === "expand" ? x - targetOffsetX : x + sourceOffsetX;
        const sourceY = normalizedMode === "expand" ? y - targetOffsetY : y + sourceOffsetY;
        if (sourceX >= 0 && sourceX < level.width && sourceY >= 0 && sourceY < level.height) {
          resized[y * nextWidth + x] = cloneCellSlots(sourceCells[sourceY * level.width + sourceX]);
        }
      }
    }
    return resized;
  };
  level.layers = previousLayers.map(resizeLayerCells);

  level.width = nextWidth;
  level.height = nextHeight;
  level.regions = resizeLevelRegions(levelRegions(), edge, nextWidth, nextHeight, delta);
  level.cells = levelCompositeCells({ includeHidden: true });
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

function deactivateLevelBucketModeAfterUse() {
  if (!levelBucketActive) {
    return;
  }
  levelBucketActive = false;
  syncLevelBucketButton();
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
  ensureLevelLayerMaps();
  const previousLayers = level.layers;
  const previousRegions = levelRegions();
  const transformLayerCells = (sourceCells) => {
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
          nextCells[y * nextWidth + x] = cloneCellSlots(sourceCells[source.y * previousWidth + source.x]);
        }
      }
    }
    return nextCells;
  };
  level.width = nextWidth;
  level.height = nextHeight;
  level.regions = normalizedLevelRegions(previousRegions.map((region) => mapRegion(region, previousWidth, previousHeight)), nextWidth, nextHeight);
  level.layers = previousLayers.map(transformLayerCells);
  level.cells = levelCompositeCells({ includeHidden: true });
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

function bucketFillLevelFromIndex(index) {
  const activeCells = levelLayerCells();
  if (levelPlaytestActive || !Number.isInteger(index) || index < 0 || index >= activeCells.length) {
    return false;
  }
  clearSolutionPreview();
  levelDisplayCells = null;
  const replacement = paintCellSlots(activeCells[index], level.selectedObjectId);
  const target = cloneCellSlots(activeCells[index]);
  if (sameCellSlots(target, replacement)) {
    setStatus("Connected area already has that tile", "is-ok");
    deactivateLevelBucketModeAfterUse();
    return true;
  }

  const visited = new Uint8Array(activeCells.length);
  const stack = [index];
  let changed = 0;
  while (stack.length) {
    const current = stack.pop();
    if (visited[current] || !sameCellSlots(activeCells[current], target)) {
      continue;
    }
    visited[current] = 1;
    activeCells[current] = cloneCellSlots(replacement);
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
    deactivateLevelBucketModeAfterUse();
    return true;
  }
  setLevelSolveStatus("");
  level.cells = levelCompositeCells({ includeHidden: true });
  renderLevelBoard();
  deactivateLevelBucketModeAfterUse();
  setStatus(level.selectedObjectId ? "Filled connected area" : "Erased connected area", "is-ok");
  return true;
}

function paintLevelCellAtIndex(index, objectId, options = {}) {
  void options;
  if (levelPlaytestActive) {
    return false;
  }
  clearSolutionPreview();
  levelDisplayCells = null;
  const activeCells = levelLayerCells();
  if (!Number.isInteger(index) || index < 0 || index >= activeCells.length) {
    return false;
  }
  const next = paintCellSlots(activeCells[index], objectId);
  if (sameCellSlots(activeCells[index], next)) {
    return false;
  }
  activeCells[index] = next;
  level.cells = levelCompositeCells({ includeHidden: true });
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
    requirePreviewFrame: true,
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
  levelPlaytestActive = true;
  levelDisplayCells = stateDataToLevelCells(stateData, exportData);
  updateLevelPlaytestControls();
  renderLevelBoard();
  sendLevelStateToPreview(levelIndex, stateData, {
    acceptModelInput: true,
    materializeLevelStart: true,
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
  levelDisplayCells = null;
  if (levelPaintDrag && levelBoard.hasPointerCapture?.(levelPaintDrag.pointerId)) {
    levelBoard.releasePointerCapture(levelPaintDrag.pointerId);
  }
  levelPaintDrag = null;
  updateLevelPlaytestControls();
  renderLevelBoard();
  if (options.syncPreview !== false) {
    restoreCompiledGamePreview();
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

function sameCellSlots(left, right) {
  if (!Array.isArray(left) || !Array.isArray(right) || left.length !== right.length) {
    return false;
  }
  return left.every((value, index) => value === right[index]);
}

function paintCellSlots(slots, objectId, exportData = currentLevelExportData()) {
  const next = cloneCellSlots(slots, exportData);
  if (!objectId) {
    return makeEmptyCell(exportData);
  }
  const object = engineObjectById(objectId, exportData);
  if (!object) {
    return cloneCellSlots(slots, exportData);
  }
  if (Number.isInteger(object.layer) && object.layer >= 0 && object.layer < next.length) {
    next[object.layer] = object.id;
  }
  return next;
}

function syncPreviewStateFromLevel() {
  const exportData = previewBuild?.exportData;
  if (!exportData) {
    return;
  }
  if (isPuzzle3dExport(exportData) && typeof sendLevel3dSnapshotToRuntime === "function") {
    if (currentPreviewMode !== "level3d") {
      return;
    }
    setPreviewSessionState({
      ...(previewSessionState() || {}),
      levelIndex: currentEditableLevelIndex(exportData),
      scene: null,
    });
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
  setPreviewSessionState({
    ...(previewSessionState() || {}),
    levelIndex,
    scene: null,
  });

  sendLevelStateToPreview(levelIndex, stateData);
}

function sendLevelStateToPreview(levelIndex = currentEditableLevelIndex(), stateData = null, options = {}) {
  if (!activePreviewModeAcceptsLevelState()) {
    return;
  }
  const exportData = previewBuild?.exportData;
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
  renderLevelBoard();
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
  activeLevelSolveRequest = { id: requestId, backend: "wasm-worker", worker, progressCount: 0 };
  registerEditorSolverWorkerRequest(requestId, (message) => {
    if (message.type === "progress") {
      handleLevelSolveProgress({ requestId, observation: message.observation });
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
  return {
    levelIndex: task.level.index,
    state: task.state.data,
    materializeLevelStart: task.state.lifecycle === "playable-start",
    maxDepth: 512,
    maxNodes: 5_000_000,
  };
}

async function solveEditedLevelFromEditor() {
  ensurePreviewTargetsActiveDocument();
  const exportData = currentLevelExportData();
  if (!exportData) {
    setLevelSolveStatus("No rule model for edited level", "is-error");
    return;
  }
  const solverBuild = await prepareCurrentDraftSolverBuild();
  if (!solverBuild) return;
  const levelIndex = currentEditableLevelIndex(exportData);
  if (isPuzzle3dExport(exportData)) {
    const snapshot = typeof level3dRuntimeSnapshot === "function" ? level3dRuntimeSnapshot() : null;
    const stateData = puzzle3dStateDataFromSnapshot(snapshot, exportData, { levelIndex });
    if (!stateData || !snapshot) {
      setLevelSolveStatus("No 3D level state", "is-error");
      return;
    }
    setActiveSolverTask(createEditorSolverTask({
      solverBuild,
      exportData,
      levelIndex,
      stateData,
      puzzle3dSnapshot: snapshot,
    }), exportData);
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
  if (activeLevelSolveRequest.backend === "wasm-worker") {
    activeLevelSolveRequest.worker.postMessage({
      type: "cancel",
      requestId: activeLevelSolveRequest.id,
      wasm: wasmSolverWorkerConfig(),
    });
    setLevelSolveStatus("Cancelling", "");
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
  const taskComplete = !isSolving && isSolverTaskComplete();
  const label = isSolving ? "Cancel" : "Solve";
  const visibleLabel = label;
  const title = taskComplete ? "This level has already been solved" : visibleLabel;
  syncSolverLevelSelector();
  for (const button of [solveLevelButton, previewSolveButton, levelSolveShortcutButton, level3dSolveShortcutButton]) {
    if (!button) {
      continue;
    }
    button.classList.toggle("is-solving", Boolean(isSolving));
    const previewHasNoLevel = button === previewSolveButton && !isSolving && !previewHasCurrentLevel();
    button.disabled = taskComplete || previewHasNoLevel;
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
  showSolverObservation(message.observation);
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
  const frames = observations.filter((observation) => observation?.state);
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
  if (!observation?.state) {
    return;
  }
  if (observation.state.kind === "puzzle3d") {
    const snapshot = typeof puzzle3dSolutionStepSnapshot === "function"
      ? puzzle3dSolutionStepSnapshot({ state: observation.state })
      : null;
    if (snapshot) {
      solverObservationPreview = {
        kind: "puzzle3d",
        snapshot,
        progress: cloneJson(observation.progress || null),
      };
      levelSolutionPreview = null;
      renderSolverRuntimePreview?.();
    }
    return;
  }
  const exportData = previewBuild?.exportData;
  const scene = sceneFromStateData(observation.state, {
    regions: activeSolverTask?.scene?.regions || levelRegions(),
    exportData,
  });
  const cells = sceneCellsToSlots(scene, displayedSolverCells());
  if (!cells.length) {
    return;
  }
  solverObservationPreview = {
    cells,
    progress: cloneJson(observation.progress || null),
  };
  levelSolutionPreview = null;
  renderSolverBoard();
  updateSolutionControls();
  const progress = observation.progress || {};
  const visited = Number.isFinite(progress.visited) ? progress.visited : null;
  const depth = Number.isFinite(progress.depth) ? progress.depth : null;
  const parts = [];
  if (visited !== null) {
    parts.push(`${visited} states`);
  }
  if (depth !== null) {
    parts.push(`depth ${depth}`);
  }
  setLevelSolveStatus(parts.length ? `Searching: ${parts.join(", ")}` : "Searching", "");
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
  if (solution.model === "3d" || steps[0]?.scene?.kind === "puzzle3d" || steps[0]?.state?.kind === "puzzle3d") {
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
    cells: sceneCellsToSlots(solutionStepScene(steps[0]), displayedSolverCells()),
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

function stateDataToLevelCells(stateData, exportData = previewBuild?.exportData) {
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
    solutionStepScene(levelSolutionPreview.steps[nextIndex]),
    levelSolutionPreview.cells.length ? levelSolutionPreview.cells : displayedSolverCells(),
  );
  updateSolutionControls();
  renderSolverBoard();
}

function solutionStepScene(step) {
  if (step?.scene) {
    return step.scene;
  }
  if (step?.state) {
    return sceneFromStateData(step.state, {
      regions: activeSolverTask?.scene?.regions || levelRegions(),
      exportData: previewBuild?.exportData,
    });
  }
  return null;
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
  if (options.preserveSolverTask !== true) {
    activeSolverTask = null;
    syncSolverTaskReadout();
  }
  setSolveLevelButtonState(Boolean(activeLevelSolveRequest));
  levelSolveSummaryText = "";
  levelSolveStatus.title = "";
  updateSolutionControls();
  if (currentPreviewMode === "level3d" && typeof renderLevel3dBuilder === "function") {
    renderLevel3dBuilder();
  }
  if (typeof clearSolverRuntimePreview === "function") {
    clearSolverRuntimePreview();
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

function sendPreviewKey(event) {
  previewFrame.contentWindow?.postMessage({
    type: "PuzzleStudioKey",
    key: event.key,
    code: event.code,
    repeat: event.repeat,
    altKey: event.altKey,
    ctrlKey: event.ctrlKey,
    metaKey: event.metaKey,
    shiftKey: event.shiftKey,
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

function levelSourceText() {
  const levelName = sanitizeLevelName(levelNameInput.value);
  return levelDefinitionSource(levelName, levelSourceData(currentLevelAuthoringSource()), "", { leadingBlank: false, bodyIndent: "" });
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
  const source = activePreviewSource();
  let sourceData = null;
  try {
    sourceData = levelSourceData(source);
  } catch (error) {
    setStatus(`Could not create level source: ${error?.message || error}`, "is-error");
    return;
  }
  const nextSource = insertLevel(source, levelName, sourceData, levelNamespace);
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
    start: entry.start,
    end: entry.start + replacement.length,
  };
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

function updateLevelInSource() {
  const editDocument = activeLevelEditDocument();
  if (!editDocument || !isPuzzleDocument(editDocument) || !isTextDocument(editDocument)) {
    setStatus("No editable level source loaded", "is-error");
    return;
  }
  const levelName = sanitizeLevelName(levelNameInput.value);
  const source = activeLevelEditSource();
  let sourceData = null;
  try {
    sourceData = levelSourceData(source);
  } catch (error) {
    setStatus(`Could not create level source: ${error?.message || error}`, "is-error");
    return;
  }
  const entry = currentLevelEditSourceRange(source);
  const result = replaceLevelSourceEntry(source, entry, levelName, sourceData);
  if (!result) {
    setStatus("No editable level source loaded", "is-error");
    return;
  }
  editDocument.source = result.source;
  if (editDocument.id === activeDocument()?.id) {
    setSourceEditorValue(result.source, { resetUndo: false });
  }
  setLevelEditSource({
    ...entry,
    start: result.start,
    end: result.end,
    name: levelName,
  }, editDocument);
  scheduleLocalSave();
  schedulePreview();
  setStatus(levelName ? `Updated level ${levelName}` : "Updated unnamed level", "is-ok");
}

function levelStateData(exportData) {
  ensureLevelLayerMaps(exportData);
  const width = level.width;
  const height = level.height;
  const layerCount = exportData?.engine?.layerCount;
  if (!width || !height || !layerCount) {
    return null;
  }

  const slots = Array.from({ length: width * height * layerCount }, () => 0);
  const compositeCells = levelCompositeCells({ includeHidden: true, exportData });
  compositeCells.forEach((cellSlots, cellIndex) => {
    const sourceSlots = cloneCellSlots(cellSlots, exportData);
    for (let layer = 0; layer < layerCount; layer += 1) {
      slots[(cellIndex * layerCount) + layer] = sourceSlots[layer] || 0;
    }
  });

  const levelIndex = currentEditableLevelIndex(exportData);
  const variablesLength = exportData.levels?.[levelIndex]?.initialState?.variables?.length
    || exportData.levels?.[0]?.initialState?.variables?.length
    || 0;

  return {
    width,
    height,
    layerCount,
    levelIndex,
    slots,
    variables: Array.from({ length: variablesLength }, () => 0),
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

  const variablesLength = exportData.levels?.[levelIndex]?.initialState?.variables?.length
    || exportData.levels?.[0]?.initialState?.variables?.length
    || 0;

  return {
    width,
    height,
    layerCount,
    levelIndex,
    slots,
    variables: Array.from({ length: variablesLength }, () => 0),
  };
}

function compiledLevelStateData(exportData, levelIndex) {
  const state = exportData?.levels?.[levelIndex]?.initialState;
  if (!state) {
    return null;
  }
  return {
    ...JSON.parse(JSON.stringify(state)),
    levelIndex,
  };
}

function puzzle3dSnapshotForActiveSolverTask() {
  return activeSolverTask?.puzzle3dSnapshot
    ? cloneJson(activeSolverTask.puzzle3dSnapshot)
    : null;
}

function puzzle3dSnapshotForLevel(exportData, levelIndex) {
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

function puzzle3dStateDataFromSnapshot(snapshot, exportData = previewBuild?.exportData, options = {}) {
  if (!snapshot) {
    return null;
  }
  const levelIndex = options.levelIndex ?? (Number.isInteger(Number(snapshot.levelIndex))
    ? Math.trunc(Number(snapshot.levelIndex))
    : normalizedLevelIndex(null, exportData));
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
    variables: Array.isArray(levelEntry?.initialState?.variables)
      ? cloneJson(levelEntry.initialState.variables)
      : [],
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

function previewExportDiagnosticsSummary(diagnostics) {
  const parts = [];
  parts.push(`htmlLength=${diagnostics.htmlLength}`);
  parts.push(`markers=${diagnostics.markers.join("|") || "none"}`);
  if (diagnostics.kind) {
    parts.push(`kind=${diagnostics.kind}`);
  }
  if (diagnostics.levelCount !== null) {
    parts.push(`levels=${diagnostics.levelCount}`);
  }
  parts.push(`engineObjects=${diagnostics.hasEngineObjects ? "yes" : "no"}`);
  if (diagnostics.parseError) {
    parts.push(`parseError=${diagnostics.parseError}`);
  }
  return parts.join(", ");
}

function previewMetadataErrorMessage(inspection) {
  if (!inspection.exportData) {
    return `Compiled preview is missing PuzzleExport metadata (${previewExportDiagnosticsSummary(inspection.diagnostics)}).`;
  }
  return `Compiled preview metadata is incomplete (${previewExportDiagnosticsSummary(inspection.diagnostics)}).`;
}

function inspectPreviewExport(html) {
  const source = String(html || "");
  const diagnostics = {
    htmlLength: source.length,
    markers: [],
    kind: "",
    levelCount: null,
    hasEngineObjects: false,
    parseError: "",
  };
  if (!source) {
    return { exportData: null, diagnostics };
  }
  if (source.includes("window.Puzzle3DFrameFixture")) {
    diagnostics.markers.push("Puzzle3DFrameFixture");
  }
  if (source.includes("window.Puzzle3DFixture")) {
    diagnostics.markers.push("Puzzle3DFixture");
  }
  if (source.includes("window.PuzzleExport")) {
    diagnostics.markers.push("PuzzleExport");
  }
  if (source.includes("window.PuzzleEditorPreviewExportJson")) {
    diagnostics.markers.push("PuzzleEditorPreviewExportJson");
  }
  const editorPreviewExportLiteral = extractAssignedStringLiteral(source, "PuzzleEditorPreviewExportJson");
  if (editorPreviewExportLiteral) {
    try {
      const parsed = JSON.parse(JSON.parse(editorPreviewExportLiteral));
      if (parsed && typeof parsed === "object" && !parsed.__kind) {
        parsed.__kind = parsed?.kind === "puzzle3d" ? "puzzle3d" : "puzzle2d";
      }
      diagnostics.kind = parsed?.__kind || "";
      diagnostics.levelCount = Array.isArray(parsed?.levels) ? parsed.levels.length : null;
      diagnostics.hasEngineObjects = Array.isArray(parsed?.engine?.objects);
      return { exportData: parsed, diagnostics };
    } catch (error) {
      diagnostics.parseError = error instanceof Error ? error.message : String(error);
      console.error(error);
      return { exportData: null, diagnostics };
    }
  }
  for (const candidate of [
    { kind: "puzzle3d", windowName: "Puzzle3DFrameFixture" },
    { kind: "puzzle3d", windowName: "Puzzle3DFixture" },
    { kind: "puzzle2d", windowName: "PuzzleExport" },
  ]) {
    const literal = extractJsonParseStringLiteral(source, candidate.windowName);
    if (!literal) {
      continue;
    }
    try {
      const parsed = JSON.parse(JSON.parse(literal));
      if (parsed && typeof parsed === "object" && !parsed.__kind) {
        parsed.__kind = candidate.kind;
      }
      diagnostics.kind = candidate.kind;
      diagnostics.levelCount = Array.isArray(parsed?.levels) ? parsed.levels.length : null;
      diagnostics.hasEngineObjects = Array.isArray(parsed?.engine?.objects);
      return { exportData: parsed, diagnostics };
    } catch (error) {
      diagnostics.parseError = error instanceof Error ? error.message : String(error);
      console.error(error);
      return { exportData: null, diagnostics };
    }
  }
  if (diagnostics.markers.length) {
    diagnostics.parseError = "JSON.parse string literal not found for preview export marker";
  }
  return { exportData: null, diagnostics };
}

function extractAssignedStringLiteral(source, windowName) {
  const assignmentPattern = new RegExp(`window\\.${windowName}\\s*=\\s*`, "g");
  const match = assignmentPattern.exec(source);
  if (!match) {
    return null;
  }
  let index = match.index + match[0].length;
  while (/\s/.test(source[index] || "")) {
    index += 1;
  }
  return source[index] === "\"" ? extractStringLiteralAt(source, index) : null;
}

function extractJsonParseStringLiteral(source, windowName) {
  const assignmentPattern = new RegExp(`window\\.${windowName}\\s*=\\s*JSON\\.parse\\s*\\(`, "g");
  const match = assignmentPattern.exec(source);
  if (!match) {
    return null;
  }
  let index = match.index + match[0].length;
  while (/\s/.test(source[index] || "")) {
    index += 1;
  }
  if (source[index] !== "\"") {
    return null;
  }
  return extractStringLiteralAt(source, index);
}

function extractStringLiteralAt(source, start) {
  let index = start + 1;
  let escaped = false;
  while (index < source.length) {
    const char = source[index];
    if (escaped) {
      escaped = false;
    } else if (char === "\\") {
      escaped = true;
    } else if (char === "\"") {
      return source.slice(start, index + 1);
    }
    index += 1;
  }
  return null;
}

function extractPreviewExport(html) {
  return inspectPreviewExport(html).exportData;
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

function levelRows() {
  return levelSourceData().rows;
}

function currentLevelAuthoringSource(exportData = currentLevelExportData()) {
  return activeLevelEditSource() || activePreviewSource() || levelReferenceSource(exportData);
}

function levelSourceData(source = currentLevelAuthoringSource(), exportData = currentLevelExportData()) {
  ensureLevelLayerMaps(exportData);
  const charEntries = sourceCharEntries(source, exportData);
  const allocator = createLevelLegendAllocator(charEntries, sourceReservedLegendChars(source));
  const rows = [];
  const regions = levelRegions();
  for (const [regionIndex, region] of regions.entries()) {
    if (regionIndex > 0) {
      rows.push("");
    }
    for (let asciiLayerIndex = 0; asciiLayerIndex < levelLayerCount2d(); asciiLayerIndex += 1) {
      if (asciiLayerIndex > 0) {
        rows.push("+");
      }
      const layerCells = level.layers[asciiLayerIndex] || makeEmptyLevelLayer(level.width, level.height, exportData);
      for (let y = region.y; y < region.y + region.height; y += 1) {
        const row = [];
        for (let x = region.x; x < region.x + region.width; x += 1) {
          row.push(charForSourceCell(layerCells[y * level.width + x], charEntries, allocator, exportData));
        }
        rows.push(row.join(""));
      }
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
  const ascii = [..."xyzabcdefghijklmnopqrstuvwABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789@$%&?!~^:;,_-*/<>|()[]"];
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

function charForSourceCell(slots, entries, allocator, exportData = currentLevelExportData()) {
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

function objectNamesForSlots(slots, exportData = currentLevelExportData()) {
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

function addLevelPaletteObjectToLegend(object) {
  ensurePreviewTargetsActiveDocument();
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
  let legendEntry = null;
  try {
    const allocator = createLevelLegendAllocator(sourceCharEntries(source, exportData), sourceReservedLegendChars(source));
    const char = allocator.charForObjects([objectName]);
    legendEntry = allocator.localLegends.find((entry) => entry.char === char) || { char, objects: [objectName] };
  } catch (error) {
    setStatus(`Could not add tile legend: ${error?.message || error}`, "is-error");
    return false;
  }
  const nextSource = insertCommonLegendEntry(source, legendEntry);
  if (!nextSource) {
    setStatus("Could not find a source location for tile legend", "is-error");
    return false;
  }
  editDocument.source = nextSource;
  if (editDocument.id === activeDocument()?.id) {
    setSourceEditorValue(nextSource, { resetUndo: false });
  }
  level.addPaletteOpen = false;
  level.palette = levelPaletteFromExport(nextSource, exportData);
  level.selectedObjectId = objectEntry.id;
  setLevelActiveLayerForObject(objectEntry.id);
  renderLevelPalette();
  renderLevelBoard();
  scheduleLocalSave();
  schedulePreview();
  setStatus(`Added legend ${legendEntry.char} = ${objectName}`, "is-ok");
  return true;
}

function insertCommonLegendEntry(source, entry) {
  const sourceText = String(source || "");
  const objectName = String(entry?.objects?.[0] || "").trim();
  const char = String(entry?.char || "");
  if (char.length !== 1 || !objectName) {
    return "";
  }
  const lines = sourceLinesWithOffsets(sourceText);
  const levelRanges = sourceLevelLocalRanges(sourceText);
  for (let index = 0; index < lines.length; index += 1) {
    if (isOffsetInRanges(lines[index].start, levelRanges)) {
      continue;
    }
    const code = levelScannerCode(lines[index].raw);
    if (!/^legend(?:\s*\{)?\s*$/.test(code)) {
      continue;
    }
    const result = collectLegendBlockRows(lines, index + 1, levelRanges);
    const closeLine = lines[result.endIndex];
    if (!closeLine || levelScannerCode(closeLine.raw) !== "}") {
      continue;
    }
    const bodyIndent = legendBlockInsertionIndent(lines, index + 1, result.endIndex, levelRanges, `${lineIndent(lines[index].raw)}  `);
    const insertion = `${bodyIndent}${char} = ${objectName}\n`;
    return `${sourceText.slice(0, closeLine.start)}${insertion}${sourceText.slice(closeLine.start)}`;
  }
  const block = `legend {\n  ${char} = ${objectName}\n}\n\n`;
  const firstLevels = findLevelsRanges(sourceText)[0] || null;
  const offset = Number.isInteger(firstLevels?.start) ? firstLevels.start : sourceText.length;
  const before = sourceText.slice(0, offset).trimEnd();
  const after = sourceText.slice(offset).trimStart();
  return `${before}${before ? "\n\n" : ""}${block}${after}`;
}

function legendBlockInsertionIndent(lines, startIndex, endIndex, levelRanges, emptyBlockIndent) {
  for (let index = endIndex - 1; index >= startIndex; index -= 1) {
    if (isOffsetInRanges(lines[index].start, levelRanges)) {
      continue;
    }
    const code = levelScannerCode(lines[index].raw);
    if (!code) {
      continue;
    }
    const tokens = splitLevelTokens(code);
    if (isLegendRowForWasm(tokens)) {
      return lineIndent(lines[index].raw);
    }
  }
  return emptyBlockIndent;
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

function sourceCharEntries(source, exportData = currentLevelExportData()) {
  if (exportData?.editorSourceContract) {
    const integratedLegend = exportData.levels?.[currentEditableLevelIndex(exportData)]?.editorLegend;
    const objectNames = new Map(engineObjects(exportData).map((object) => [object.id, object.name]));
    const entries = (integratedLegend || exportData.editorSourceContract.legend || []).map((entry) => ({
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
        const name = sourcePuzzleLevelHeaderName(code);
        if (name === null) {
          break;
        }
        entry = code.endsWith("{")
          ? bracedLevelEntry(source, lines, index, name, levelsRange.bodyEnd)
          : unbracedLevelEntry(lines, index, index + 1, name, levelsRange.bodyEnd);
      } else if (tokens.length === 1 && tokens[0] === "{") {
        entry = bracedLevelEntry(source, lines, index, "", levelsRange.bodyEnd);
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
  const levelName = sourcePuzzleLevelName(name);
  const lifecycle = options.lifecycle || {};
  const startLifecycleLines = Array.isArray(lifecycle.start) ? lifecycle.start : [];
  const clearLifecycleLines = Array.isArray(lifecycle.clear) ? lifecycle.clear : [];
  const rowIndent = Object.prototype.hasOwnProperty.call(options, "bodyIndent") ? options.bodyIndent : levelIndent;
  const hasRegionBreak = rows.some((row) => row.trim() === "");
  const hasLocalLegends = localLegends.length > 0;
  const hasLifecycle = startLifecycleLines.length > 0 || clearLifecycleLines.length > 0;
  const lines = hasRegionBreak || hasLocalLegends || hasLifecycle
    ? [
      levelName ? sourcePuzzleLevelHeaderSource(levelName, levelIndent, { openBlock: true }) : `${levelIndent}{`,
      ...levelBodyBlockSourceLines(startLifecycleLines, rowIndent),
      ...levelLegendSourceLines(localLegends, rowIndent),
      ...rows.map((row) => levelMapRowSourceLine(row, rowIndent)),
      ...levelBodyBlockSourceLines(clearLifecycleLines, rowIndent),
      `${levelIndent}}`,
    ]
    : levelName
      ? [
        sourcePuzzleLevelHeaderSource(levelName, levelIndent),
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

runButton.addEventListener("click", () => {
  runPreviewFromSourcePane();
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
window.addEventListener("message", (event) => {
  if (event.data?.type === "PuzzleStudioEditorSaveShortcut") {
    if (event.source && previewFrame?.contentWindow && event.source !== previewFrame.contentWindow) {
      return;
    }
    invokeEditorCommand(
      "workspace.save",
      editorCommandContext(null, previewFrame, "button")
    );
    return;
  }
  if (event.data?.type === "PuzzleStudioPreviewLayout") {
    return;
  }
  if (event.data?.type === "PuzzleStudioPreviewLoaded") {
    const session = ensurePreviewSession();
    if (!session) {
      return;
    }
    session.runtimeStatus = {
      title: event.data.title || "",
      href: event.data.href || "",
    };
    if (isPuzzle3dExport(previewBuild?.exportData)) {
      setStatus("Starting 3D runtime", "");
    } else {
      appendPreviewLog("system", "Preview ready", { source: "runtime" });
      setStatus("Preview ready", "is-ok");
    }
    return;
  }
  if (event.data?.type === "PuzzleStudioPreviewRuntimeReady") {
    if (event.source && previewFrame?.contentWindow && event.source !== previewFrame.contentWindow) {
      return;
    }
    appendPreviewLog("system", "Preview ready", { source: "runtime" });
    setStatus("Preview ready", "is-ok");
    return;
  }
  if (event.data?.type === "PuzzleStudioPreviewRuntimeError") {
    if (event.source && previewFrame?.contentWindow && event.source !== previewFrame.contentWindow) {
      return;
    }
    const label = String(event.data.label || "runtime failed");
    const message = String(event.data.message || "unknown error");
    setStatus(`Preview ${label}: ${message}`, "is-error");
    return;
  }
  if (event.data?.type === "PuzzleStudioPreviewState") {
    applyPreviewTheme(event.data.theme || previewBuild?.exportData?.theme || null);
    syncPreviewViewportAspect(event.data.screen || "");
    const inLevelMode = !levelBuilder.hidden || !solverPanel.hidden;
    const screenHasPuzzle = event.data.screenHasPuzzle !== false;
    const previousState = previewSessionState();
    const levelIndex = Number.isInteger(Number(event.data.levelIndex))
      ? Math.trunc(Number(event.data.levelIndex))
      : previousState?.levelIndex ?? 0;
    const nextState = setPreviewSessionState({
      levelIndex,
      rawScene: event.data.rawScene,
      scene: event.data.scene,
      puzzle3Snapshot: event.data.puzzle3Snapshot || null,
      inputs: event.data.inputs || [],
      screen: event.data.screen || "",
      screenHasPuzzle,
    });
    if (inLevelMode) {
      if (screenHasPuzzle && event.data.scene && (levelPlaytestActive || !solverPanel.hidden)) {
        const displayCells = sceneCellsToSlots(event.data.scene, []);
        levelDisplayCells = displayCells.length === level.cells.length ? displayCells : null;
        if (applyPreviewSceneToActiveSolverTask(nextState)) {
          activeSolverDisplaySceneRequestKey = "";
          renderSolverBoard();
        }
        renderLevelBoard();
      }
      if (levelSolutionPreview) {
        updateSolutionControls();
      }
    }
    return;
  }
  if (event.data?.type === "PuzzleStudioPreviewProgressSave") {
    if (event.source && previewFrame?.contentWindow && event.source !== previewFrame.contentWindow) {
      return;
    }
    const key = String(event.data.storageKey || "");
    const saveJson = String(event.data.saveJson || "");
    if (key && saveJson) {
      editorPreviewProgressSaves.set(key, saveJson);
      saveEditorPreviewProgressSaves();
      scheduleLocalSave();
    }
    return;
  }
  if (event.data?.type === "PuzzleStudioPreviewProgressSaveClear") {
    if (event.source && previewFrame?.contentWindow && event.source !== previewFrame.contentWindow) {
      return;
    }
    const key = String(event.data.storageKey || "");
    if (key) {
      editorPreviewProgressSaves.delete(key);
      saveEditorPreviewProgressSaves();
      scheduleLocalSave();
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
  if (event.data?.type === "PuzzleStudioPreviewDebugTrace") {
    if (event.source && previewFrame?.contentWindow && event.source !== previewFrame.contentWindow) {
      return;
    }
    handlePreviewDebugTrace(event.data.debug || null, event.data.snapshot || null);
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
previewSolveButton?.addEventListener("click", () => {
  solvePreviewPaneCurrentLevel().catch((error) => {
    setLevelSolveStatus(`Solve failed: ${userFacingRuntimeError(error)}`, "is-error");
  });
});
previewEditButton?.addEventListener("click", () => {
  openLevelPaneForCurrentPreviewLevel();
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
  openSolverPaneForCurrentLevel().catch((error) => {
    setStatus(`Source target sync failed: ${userFacingRuntimeError(error)}`, "is-error");
  });
});
for (const button of editorDimensionButtons) {
  button.addEventListener("click", () => {
    const context = focusedPuzzleSourceContext();
    const previousMode = currentPreviewMode;
    const activeKind = previousMode === "visual" || previousMode === "visual3d" ? "visual" : "level";
    const first = ["edit", "level3d", "visual", "visual3d"].includes(previousMode)
      ? firstFocusedPuzzleEntry(activeKind, context)
      : firstFocusedPuzzleEntry("level", context);
    if (first) {
      ensurePreviewTargetsActiveDocument();
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
  openVisualPaneForCurrentDimension();
  if (currentVisualPaneMode === "visual" && typeof renderVisualBuilder === "function") {
    renderVisualBuilder();
  }
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
  openVisualPaneForCurrentDimension();
});
for (const button of visualPaneModeButtons) {
  button.addEventListener("click", () => {
    const context = focusedPuzzleSourceContext();
    if (!["visual", "visual3d"].includes(button.dataset.visualPaneMode)) {
      return;
    }
    const first = firstFocusedPuzzleEntry("visual", context);
    if (!first) {
      return;
    }
    openPreviewModePane(visualModeForEditorDimension(first.dimension));
    loadFocusedPuzzleEntry("visual", first, { silent: true, recordHistory: false });
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
levelBoard.addEventListener("keydown", (event) => {
  if (handleSolutionKey(event)) {
    return;
  }
  if (!levelPlaytestActive && (event.key === "Enter" || event.key === " ") && previewSession?.state?.screenHasPuzzle !== false) {
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
sourceEditor.addEventListener("input", () => {
  invalidateLevelEditSourceForDocument(activeDocument());
});
registerSourceEditableTarget?.("level", {
  load: loadLevelFromSourcePosition,
});

applyPaneVisibility();

loadSource().then(() => {
  setWorkspaceFileActionsReady();
}).catch((error) => {
  setPreviewDocumentLoaded(false);
  setPreviewFrameHtml(emptyPreviewDocument());
  resetPreviewLog("Load failed");
  appendPreviewLog("error", error?.message || String(error), { source: "workspace" });
  setEditorStatus("Load error", "is-error");
});
