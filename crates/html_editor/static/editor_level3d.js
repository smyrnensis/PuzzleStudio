// 3D level authoring controls. Valid state rendering and hit testing are Rust-owned.
let level3dLayerHover = null;
let level3dPreviewScrubDrag = null;
let level3dSliceScrubDrag = null;
let level3dPlaytestActive = false;
let level3dPreviewCameraState = null;
let level3dPreviewOrigin = { x: 0, y: 0, z: 0 };
let level3dSurfaceResizeFrame = 0;
let level3dViewMode = "stage";
const LEVEL3D_EDITOR_MAX_SIZE = 256;
const LEVEL3D_PALETTE_PREVIEW_SIZE = 42;
const LEVEL3D_FRAME_VIRTUAL_WIDTH = 960;
const LEVEL3D_FRAME_VIRTUAL_HEIGHT = 720;
const LEVEL3D_FRAME_MIN_WIDTH = 320;
const LEVEL3D_FRAME_MAX_WIDTH = 960;
const LEVEL3D_FRAME_GAP = 12;
const LEVEL3D_SLICE_SCRUB_STEP_PX = 18;
const LEVEL3D_CAMERA_MIN_PITCH_DEGREES = -90;
const LEVEL3D_CAMERA_MAX_PITCH_DEGREES = 90;
const LEVEL3D_EMPTY_CHAR = ".";
const LEVEL3D_LEGEND_CHAR_CANDIDATES = "@$%&?!~^:;_+-*/XYZUVWQRSTABCDEFGHIJKLMNOPabcdefghijklmnopqrstuvw0123456789";
let level3d = {
  width: 0,
  depth: 0,
  height: 0,
  slice: 0,
  selectedChar: LEVEL3D_EMPTY_CHAR,
  editMode: "replace",
  layerFillActive: false,
  layerGridVisible: true,
  hiddenLayers: [],
  stageResizeMode: null,
  stageExpandMode: false,
  previewFrames: false,
  palette: [],
  slices: [],
  sourceLocalLegends: [],
  sourceTargetStart: null,
  sourceKey: "",
  sourceDocumentId: "",
};
let level3dAutoSelectionKey = "";
let level3dSourcePreviewGeneration = 0;

function level3dEditorThemeColor(name, alpha) {
  const value = window.getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  const match = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(value);
  if (!match) {
    throw new Error(`Editor theme color ${name} must be a six-digit hex color.`);
  }
  const [, red, green, blue] = match;
  return `rgba(${Number.parseInt(red, 16)}, ${Number.parseInt(green, 16)}, ${Number.parseInt(blue, 16)}, ${alpha})`;
}

function renderLevel3dBuilder() {
  if (!level3dBuilder) {
    return;
  }
  syncLevel3dFrameLayout();
  syncLevel3dViewMode();
  syncLevel3dControlsFromPreview();
  renderLevel3dPalette();
  renderLevel3dLayerPalette();
  renderLevel3dPreviewControls();
  renderLevel3dLayerControls();
  if (!previewBuild?.html) {
    renderLevel3dLayerBoard();
  }
  renderLevel3dSourcePreview();
  renderLevel3dRuntime();
  updateLevel3dPlaytestControls();
}

function syncLevel3dFrameLayout() {
  if (!level3dBuilder) {
    return;
  }
  const container = level3dBuilder.querySelector(".tool-pane-scroll") || level3dBuilder;
  const availableWidth = Math.max(1, Math.floor(level3dContentInlineSize(container)));
  const maxFrameWidth = Math.max(
    LEVEL3D_FRAME_MIN_WIDTH,
    Math.min(LEVEL3D_FRAME_MAX_WIDTH, availableWidth),
  );
  const frameWidth = Math.max(LEVEL3D_FRAME_MIN_WIDTH, maxFrameWidth);
  const scale = frameWidth / LEVEL3D_FRAME_VIRTUAL_WIDTH;
  const frameHeight = Math.max(1, Math.round(LEVEL3D_FRAME_VIRTUAL_HEIGHT * scale));
  setLevel3dBuilderStyleProperty("--level3d-frame-virtual-width", `${LEVEL3D_FRAME_VIRTUAL_WIDTH}px`);
  setLevel3dBuilderStyleProperty("--level3d-frame-virtual-height", `${LEVEL3D_FRAME_VIRTUAL_HEIGHT}px`);
  setLevel3dBuilderStyleProperty("--level3d-frame-width", `${frameWidth}px`);
  setLevel3dBuilderStyleProperty("--level3d-frame-height", `${frameHeight}px`);
  setLevel3dBuilderStyleProperty("--level3d-frame-scale", String(scale));
  setLevel3dBuilderStyleProperty("--level3d-frame-inverse-scale", String(1 / scale));
  setLevel3dBuilderStyleProperty("--level3d-frame-gap", `${LEVEL3D_FRAME_GAP}px`);
}

function syncLevel3dViewMode() {
  const mode = level3dViewMode === "layer" ? "layer" : "stage";
  const workspace = level3dBuilder?.querySelector(".level3d-workspace");
  workspace?.classList.toggle("is-view-stage", mode === "stage");
  workspace?.classList.toggle("is-view-layer", mode === "layer");
  level3dStageViewButton?.classList.toggle("is-active", mode === "stage");
  level3dLayerViewButton?.classList.toggle("is-active", mode === "layer");
  level3dStageViewButton?.setAttribute("aria-pressed", String(mode === "stage"));
  level3dLayerViewButton?.setAttribute("aria-pressed", String(mode === "layer"));
}

function setLevel3dViewMode(mode) {
  const nextMode = mode === "layer" ? "layer" : "stage";
  if (level3dPlaytestActive && nextMode !== "stage") {
    return;
  }
  if (level3dViewMode === nextMode) {
    syncLevel3dViewMode();
    return;
  }
  level3dViewMode = nextMode;
  level3dLayerHover = null;
  syncLevel3dViewMode();
  scheduleLevel3dSurfaceResize();
}

function setLevel3dBuilderStyleProperty(name, value) {
  if (level3dBuilder.style.getPropertyValue(name) !== value) {
    level3dBuilder.style.setProperty(name, value);
  }
}

function level3dContentInlineSize(element) {
  if (!element) {
    return 0;
  }
  const style = window.getComputedStyle(element);
  const padding = (parseFloat(style.paddingLeft) || 0) + (parseFloat(style.paddingRight) || 0);
  const clientWidth = element.clientWidth;
  if (Number.isFinite(clientWidth) && clientWidth > 0) {
    return Math.max(0, clientWidth - padding);
  }
  const rectWidth = element.getBoundingClientRect?.().width;
  const measuredWidth = Number.isFinite(rectWidth) && rectWidth > 0 ? rectWidth : 0;
  return Math.max(0, measuredWidth - padding);
}

function level3dScaledSurfaceMetrics(element) {
  const rect = element?.getBoundingClientRect?.() || { left: 0, top: 0, width: 0, height: 0 };
  const width = Math.max(1, Math.round(element?.offsetWidth || LEVEL3D_FRAME_VIRTUAL_WIDTH));
  const height = Math.max(1, Math.round(element?.offsetHeight || LEVEL3D_FRAME_VIRTUAL_HEIGHT));
  return {
    rect,
    width,
    height,
    scaleX: Math.max(0.0001, (Number(rect.width) || width) / width),
    scaleY: Math.max(0.0001, (Number(rect.height) || height) / height),
  };
}

function level3dEventPointInScaledSurface(event, element) {
  const metrics = level3dScaledSurfaceMetrics(element);
  return {
    x: (event.clientX - metrics.rect.left) / metrics.scaleX,
    y: (event.clientY - metrics.rect.top) / metrics.scaleY,
    width: metrics.width,
    height: metrics.height,
  };
}

function syncLevel3dControlsFromPreview() {
  const exportData = previewBuild?.exportData;
  applyDefaultLevel3dSelectionForActiveDocument(exportData);
  const sourceDocument = level3dSourceDocument();
  const source = level3dEditorSource(sourceDocument);
  const sourceDefinition = currentLevel3dSourceDefinition(source);
  const levelEntry = sourceDefinition
    ? level3dExportEntryForSourceDefinition(sourceDefinition, exportData)
    : null;
  if (!sourceDefinition) {
    syncLevel3dPaletteWithoutLoadedLevel(source, sourceDocument);
    if (level3dNameInput && !level3dNameInput.dataset.userEdited) {
      level3dNameInput.value = "level 1";
    }
    if (level3dBundleInput && !level3dBundleInput.dataset.userEdited) {
      level3dBundleInput.value = currentLevel3dBundleName(exportData);
    }
  }
  const sourceName = levelEntry?.name || sourceDefinition?.name || "";
  if (sourceName && level3dNameInput) {
    level3dNameInput.value = level3dNameInput.value || sourceName;
    if (!level3dNameInput.dataset.userEdited) {
      level3dNameInput.value = sourceName;
    }
  }
  if (level3dBundleInput && !level3dBundleInput.dataset.userEdited) {
    level3dBundleInput.value = levelEntry ? currentLevel3dBundleName(exportData) : (sourceDefinition?.bundle || currentLevel3dBundleName(exportData));
  }
  const sourceKey = levelEntry
    ? currentLevel3dEditorSourceKey(levelEntry, sourceDocument, source)
    : currentLevel3dEditorSourceKey(sourceDefinition, sourceDocument, source);
  if (levelEntry && sourceKey !== level3d.sourceKey) {
    loadLevel3dFromEntry(levelEntry, source, exportData, sourceKey, sourceDocument);
  } else if (sourceDefinition && sourceKey !== level3d.sourceKey) {
    loadLevel3dFromSourceDefinition(sourceDefinition, source, sourceKey, sourceDocument);
  }
  const size = levelEntry?.size || sourceDefinition?.size || exportData?.size || {};
  syncLevel3dSizeControls(size);
}

function syncLevel3dSizeControls(size = {}) {
  const hasLevelSize = Boolean(level3d.slices.length || size.width || size.depth || size.height);
  const width = hasLevelSize ? Math.max(1, Math.trunc(Number(level3d.width || size.width) || 1)) : 0;
  const depth = hasLevelSize ? Math.max(1, Math.trunc(Number(level3d.depth || size.depth) || 1)) : 0;
  const height = hasLevelSize ? Math.max(1, Math.trunc(Number(level3d.height || size.height) || 1)) : 0;
  if (level3dWidthInput && document.activeElement !== level3dWidthInput) {
    level3dWidthInput.value = String(width || 1);
  }
  if (level3dDepthInput && document.activeElement !== level3dDepthInput) {
    level3dDepthInput.value = String(depth || 1);
  }
  if (level3dHeightInput && document.activeElement !== level3dHeightInput) {
    level3dHeightInput.value = String(height || 1);
  }
  if (level3dSizeLabel) {
    level3dSizeLabel.textContent = `${width} × ${depth} × ${height}`;
  }
  if (level3dLayerSizeLabel) {
    level3dLayerSizeLabel.textContent = `${width} × ${depth} × ${height}`;
  }
  renderLevel3dPreviewControls();
  renderLevel3dLayerControls();
}

function renderLevel3dPreviewControls() {
  const camera = level3dPreviewCamera();
  renderLevel3dPreviewScrub(level3dCameraYawScrub, Math.round(camera.yawDegrees));
  renderLevel3dPreviewScrub(level3dCameraPitchScrub, Math.round(camera.pitchDegrees));
  renderLevel3dPreviewScrub(level3dCameraRollScrub, Math.round(camera.rollDegrees));
  renderLevel3dPreviewScrub(level3dCameraZoomScrub, Number(camera.zoom.toFixed(2)));
  const origin = level3dPreviewOriginState();
  renderLevel3dPreviewScrub(level3dOriginXScrub, level3dFormatPreviewDecimal(origin.x));
  renderLevel3dPreviewScrub(level3dOriginYScrub, level3dFormatPreviewDecimal(origin.y));
  renderLevel3dPreviewScrub(level3dOriginZScrub, level3dFormatPreviewDecimal(origin.z));
}

function renderLevel3dPreviewScrub(button, value) {
  if (!button) {
    return;
  }
  button.textContent = String(value);
}

function level3dFormatPreviewDecimal(value) {
  return String(Number((Number(value) || 0).toFixed(1)));
}

function currentLevel3dEditorSourceKey(
  levelSource = currentLevel3dEntry() || currentLevel3dSourceDefinition(level3dEditorSource()),
  document = level3dSourceDocument(),
  source = level3dEditorSource(document),
) {
  if (!levelSource) {
    return "";
  }
  const documentId = document?.id || "";
  const index = currentEditableLevelIndex(previewBuild?.exportData);
  const sourcePosition = Number.isInteger(levelSource.start) ? levelSource.start : index;
  return `${documentId}:${sourcePosition}:${levelSource.name || ""}:${String(source || "").length}`;
}

function syncLevel3dPaletteWithoutLoadedLevel(source, document = level3dSourceDocument()) {
  if (level3d.slices.length || level3d.sourceKey) {
    clearLoadedLevel3dState(source, document);
  } else {
    const palette = sourceLevel3dPaletteEntries(source);
    if (!sameLevel3dPalette(level3d.palette, palette)) {
      level3d.palette = palette;
      level3d.selectedChar = level3dSelectedCharForPalette(palette, level3d.selectedChar);
    }
    level3d.sourceDocumentId = document?.id || "";
  }
}

function clearLoadedLevel3dState(source, document = level3dSourceDocument()) {
  const palette = sourceLevel3dPaletteEntries(source);
  level3d = {
    width: 0,
    depth: 0,
    height: 0,
    slice: 0,
    selectedChar: level3dSelectedCharForPalette(palette, level3d.selectedChar),
    editMode: level3dEditMode(),
    layerFillActive: Boolean(level3d.layerFillActive),
    layerGridVisible: level3d.layerGridVisible !== false,
    hiddenLayers: Array.isArray(level3d.hiddenLayers) ? [...level3d.hiddenLayers] : [],
    stageResizeMode: null,
    stageExpandMode: false,
    previewFrames: level3d.previewFrames,
    palette,
    slices: [],
    sourceLocalLegends: [],
    sourceTargetStart: null,
    sourceKey: "",
    sourceDocumentId: document?.id || "",
  };
}

function sourceLevel3dPaletteEntries(source) {
  const entries = normalizedLevel3dLegendEntries(
    level3dSourceEntries(source).flatMap((entry) => entry.legend),
  );
  return entries.some((entry) => entry.objects.length > 0) ? entries : [];
}

function level3dSelectedCharForPalette(palette, current) {
  const visiblePalette = level3dVisiblePaletteEntries(palette);
  if (visiblePalette.some((entry) => entry.char === current)) {
    return current;
  }
  return visiblePalette.find((entry) => entry.objects.length > 0)?.char
    || visiblePalette[0]?.char
    || LEVEL3D_EMPTY_CHAR;
}

function level3dVisiblePaletteEntries(palette = level3d.palette) {
  return (palette || []).filter((entry) => {
    if (entry.temporary === true) {
      return false;
    }
    const objects = entry.objects || [];
    return !objects.length || objects.some((name) => level3dLayerIsVisible(level3dObjectLayer(name)));
  });
}

function sameLevel3dPalette(left = [], right = []) {
  if (left.length !== right.length) {
    return false;
  }
  return left.every((entry, index) => (
    entry.char === right[index]?.char
    && level3dObjectSetKey(entry.objects || []) === level3dObjectSetKey(right[index]?.objects || [])
  ));
}

function loadLevel3dFromEntry(entry, source, exportData = previewBuild?.exportData, sourceKey = "", document = level3dSourceDocument()) {
  const definitions = level3dSourceEntries(source);
  const definition = definitions.find((candidate) => candidate.name === entry?.name)
    || definitions.find((candidate) => candidate.levelIndex === currentEditableLevelIndex(exportData))
    || null;
  const legendEntries = normalizedLevel3dLegendEntries(definition?.legend || []);
  const levelData = level3dRowsForEntry(entry, legendEntries);
  const slices = level3dSlicesFromRows(levelData.rows, level3dEmptyChar(legendEntries));
  level3d.palette = legendEntries;
  level3d.slices = slices;
  level3d.height = slices.length;
  level3d.depth = slices[0]?.length || Math.max(1, Math.trunc(Number(entry?.size?.depth) || 1));
  level3d.width = slices[0]?.[0]?.length || Math.max(1, Math.trunc(Number(entry?.size?.width) || 1));
  level3d.slice = Math.max(0, Math.min(level3d.height - 1, Math.trunc(Number(level3d.slice) || 0)));
  if (!level3dVisiblePaletteEntries(level3d.palette).some((entry) => entry.char === level3d.selectedChar)) {
    level3d.selectedChar = level3dSelectedCharForPalette(level3d.palette, level3d.selectedChar);
  }
  level3d.sourceDocumentId = document?.id || level3d.sourceDocumentId || "";
  level3d.sourceLocalLegends = definition?.localLegends || [];
  level3d.sourceTargetStart = Number.isInteger(definition?.start) ? definition.start : null;
  level3d.sourceKey = sourceKey || currentLevel3dEditorSourceKey(entry, document, source);
  resetLevel3dPreviewState(exportData);
}

function loadLevel3dFromSourceDefinition(definition, source, sourceKey = "", document = level3dSourceDocument()) {
  const legendEntries = normalizedLevel3dLegendEntries(definition?.legend || []);
  const slices = level3dSlicesFromRows(definition.rows, level3dEmptyChar(legendEntries));
  level3d.palette = legendEntries;
  level3d.slices = slices;
  level3d.height = slices.length;
  level3d.depth = slices[0]?.length || 1;
  level3d.width = slices[0]?.[0]?.length || 1;
  level3d.slice = Math.max(0, Math.min(level3d.height - 1, Math.trunc(Number(level3d.slice) || 0)));
  if (!level3dVisiblePaletteEntries(level3d.palette).some((entry) => entry.char === level3d.selectedChar)) {
    level3d.selectedChar = level3dSelectedCharForPalette(level3d.palette, level3d.selectedChar);
  }
  level3d.sourceDocumentId = document?.id || level3d.sourceDocumentId || "";
  level3d.sourceLocalLegends = definition?.localLegends || [];
  level3d.sourceTargetStart = Number.isInteger(definition?.start) ? definition.start : null;
  level3d.sourceKey = sourceKey || currentLevel3dEditorSourceKey(definition, document, source);
  resetLevel3dPreviewState(previewBuild?.exportData);
}

function normalizedLevel3dLegendEntries(entries) {
  const unique = [];
  const chars = new Set();
  for (const entry of entries || []) {
    const ch = String(entry.char || "").charAt(0);
    if (!ch || chars.has(ch)) {
      continue;
    }
    unique.push({
      char: ch,
      objects: Array.isArray(entry.objects) ? entry.objects.filter(Boolean) : [],
      objectIds: Array.isArray(entry.objectIds)
        ? entry.objectIds.filter((id) => Number.isInteger(id) && id > 0)
        : [],
      temporary: entry.temporary === true,
    });
    chars.add(ch);
  }
  if (!unique.some((entry) => entry.objects.length === 0)) {
    unique.unshift({ char: LEVEL3D_EMPTY_CHAR, objects: [], objectIds: [] });
  }
  return unique.length
    ? unique
    : [{ char: LEVEL3D_EMPTY_CHAR, objects: [], objectIds: [] }];
}

function level3dEmptyChar(entries = level3d.palette) {
  return (entries || []).find((entry) => entry.objects.length === 0)?.char || LEVEL3D_EMPTY_CHAR;
}

function level3dSlicesFromRows(rows, emptyChar = LEVEL3D_EMPTY_CHAR) {
  const slices = [];
  let current = [];
  for (const raw of Array.isArray(rows) ? rows : []) {
    const row = String(raw ?? "");
    if (row === "") {
      slices.push(current);
      current = [];
      continue;
    }
    current.push(row);
  }
  slices.push(current);

  const nonEmptySlices = slices.length ? slices : [[]];
  const depth = Math.max(1, ...nonEmptySlices.map((slice) => slice.length));
  const width = Math.max(1, ...nonEmptySlices.flatMap((slice) => slice.map((row) => row.length)));
  return nonEmptySlices.map((slice) => Array.from({ length: depth }, (_, row) => (
    String(slice[row] || "").padEnd(width, emptyChar).slice(0, width)
  )));
}

function level3dRowsFromState() {
  const rows = [];
  for (let slice = 0; slice < level3d.slices.length; slice += 1) {
    if (slice > 0) {
      rows.push("");
    }
    rows.push(...level3d.slices[slice]);
  }
  return rows;
}

function resizeLevel3dWidth(nextWidth, options = {}) {
  if (level3dPlaytestActive) {
    syncLevel3dSizeControls();
    return false;
  }
  const before = visualEditSnapshot("level3d");
  const width = normalizedLevel3dWidth(nextWidth);
  const currentWidth = Math.max(1, Math.trunc(Number(level3d.width) || 1));
  if (width === currentWidth || !level3d.slices.length) {
    syncLevel3dSizeControls();
    return false;
  }
  const empty = level3dEmptyChar();
  const growAtLeft = options.edge === "left";
  const delta = Math.abs(width - currentWidth);
  for (let sliceIndex = 0; sliceIndex < level3d.slices.length; sliceIndex += 1) {
    const rows = level3d.slices[sliceIndex] || [];
    for (let row = 0; row < Math.max(1, level3d.depth || rows.length || 1); row += 1) {
      const current = String(rows[row] || "").padEnd(currentWidth, empty).slice(0, currentWidth);
      if (width > currentWidth) {
        rows[row] = growAtLeft
          ? `${empty.repeat(delta)}${current}`
          : `${current}${empty.repeat(delta)}`;
      } else {
        rows[row] = growAtLeft ? current.slice(delta) : current.slice(0, width);
      }
    }
    level3d.slices[sliceIndex] = rows;
  }
  level3d.width = width;
  syncLevel3dSizeControls();
  renderLevel3dLayerBoard();
  renderLevel3dSourcePreview();
  renderLevel3dRuntime();
  pushVisualEditUndoSnapshot("level3d", before);
  if (options.status !== false) {
    setLevel3dActionStatus(`Width ${width}`, "is-ok");
  }
  return true;
}

function normalizedLevel3dWidth(value) {
  return Math.max(1, Math.min(LEVEL3D_EDITOR_MAX_SIZE, Math.trunc(Number(value) || 1)));
}

function resizeLevel3dDepth(nextDepth, options = {}) {
  if (level3dPlaytestActive) {
    syncLevel3dSizeControls();
    return false;
  }
  const before = visualEditSnapshot("level3d");
  const depth = normalizedLevel3dDepth(nextDepth);
  const currentDepth = Math.max(1, Math.trunc(Number(level3d.depth) || 1));
  if (depth === currentDepth || !level3d.slices.length) {
    syncLevel3dSizeControls();
    return false;
  }
  const empty = level3dEmptyChar();
  const width = Math.max(1, Math.trunc(Number(level3d.width) || 1));
  const growAtFront = options.edge === "front";
  const delta = Math.abs(depth - currentDepth);
  for (let sliceIndex = 0; sliceIndex < level3d.slices.length; sliceIndex += 1) {
    const rows = Array.isArray(level3d.slices[sliceIndex]) ? level3d.slices[sliceIndex] : [];
    const normalizedRows = Array.from({ length: currentDepth }, (_, row) => (
      String(rows[row] || "").padEnd(width, empty).slice(0, width)
    ));
    if (depth > currentDepth) {
      const addedRows = Array.from({ length: delta }, () => empty.repeat(width));
      level3d.slices[sliceIndex] = growAtFront
        ? normalizedRows.concat(addedRows)
        : addedRows.concat(normalizedRows);
    } else {
      level3d.slices[sliceIndex] = growAtFront
        ? normalizedRows.slice(0, depth)
        : normalizedRows.slice(delta);
    }
  }
  level3d.depth = depth;
  syncLevel3dSizeControls();
  renderLevel3dLayerBoard();
  renderLevel3dSourcePreview();
  renderLevel3dRuntime();
  pushVisualEditUndoSnapshot("level3d", before);
  if (options.status !== false) {
    setLevel3dActionStatus(`Depth ${depth}`, "is-ok");
  }
  return true;
}

function normalizedLevel3dDepth(value) {
  return Math.max(1, Math.min(LEVEL3D_EDITOR_MAX_SIZE, Math.trunc(Number(value) || 1)));
}

function resizeLevel3dHeight(nextHeight, options = {}) {
  if (level3dPlaytestActive) {
    syncLevel3dSizeControls();
    return false;
  }
  const before = visualEditSnapshot("level3d");
  const height = normalizedLevel3dHeight(nextHeight);
  const currentHeight = Math.max(1, Math.trunc(Number(level3d.height) || 1));
  if (height === currentHeight || !level3d.slices.length) {
    syncLevel3dSizeControls();
    return false;
  }
  const empty = level3dEmptyChar();
  const nextSlices = level3d.slices.slice();
  const growAtBottom = options.edge === "bottom";
  if (height > currentHeight) {
    const added = height - currentHeight;
    for (let index = 0; index < added; index += 1) {
      if (growAtBottom) {
        nextSlices.push(emptyLevel3dSlice(empty));
      } else {
        nextSlices.unshift(emptyLevel3dSlice(empty));
      }
    }
  } else {
    if (growAtBottom) {
      nextSlices.splice(height, currentHeight - height);
    } else {
      nextSlices.splice(0, currentHeight - height);
    }
  }
  level3d.slices = nextSlices;
  level3d.height = height;
  level3d.slice = Math.max(0, Math.min(height - 1, Math.trunc(Number(level3d.slice) || 0)));
  syncLevel3dSizeControls();
  renderLevel3dLayerBoard();
  renderLevel3dSourcePreview();
  renderLevel3dRuntime();
  pushVisualEditUndoSnapshot("level3d", before);
  if (options.status !== false) {
    setLevel3dActionStatus(`Height ${height}`, "is-ok");
  }
  return true;
}

function normalizedLevel3dHeight(value) {
  return Math.max(1, Math.min(LEVEL3D_EDITOR_MAX_SIZE, Math.trunc(Number(value) || 1)));
}

function emptyLevel3dSlice(empty = level3dEmptyChar()) {
  const width = Math.max(1, Math.trunc(Number(level3d.width) || 1));
  const depth = Math.max(1, Math.trunc(Number(level3d.depth) || 1));
  return Array.from({ length: depth }, () => empty.repeat(width));
}

function level3dStateLevelData() {
  return { rows: level3dRowsFromState(), unknownCells: 0 };
}

function currentLevel3dEntry(exportData = previewBuild?.exportData) {
  if (!isPuzzle3dExport(exportData)) {
    return null;
  }
  const levels = Array.isArray(exportData?.levels) ? exportData.levels : [];
  if (!levels.length) {
    return null;
  }
  const index = Math.max(0, Math.min(levels.length - 1, currentEditableLevelIndex(exportData)));
  return levels[index] || null;
}

function level3dSourceDocument() {
  const current = documents.find((document) => document.id === level3d.sourceDocumentId);
  return current || activePreviewDocument() || activeDocument();
}

function level3dEditorSource(document = level3dSourceDocument()) {
  return document ? sourceForDocument(document) : activePreviewSource();
}

function level3dExportEntryForSourceDefinition(definition, exportData = previewBuild?.exportData) {
  if (!isPuzzle3dExport(exportData) || !definition) {
    return null;
  }
  const levels = Array.isArray(exportData?.levels) ? exportData.levels : [];
  const byName = levels.find((level) => level?.name === definition.name);
  if (byName) {
    return byName;
  }
  return Number.isInteger(definition.levelIndex) ? levels[definition.levelIndex] || null : null;
}

function applyDefaultLevel3dSelectionForActiveDocument(exportData = previewBuild?.exportData) {
  const document = activeDocument();
  const documentId = document?.id || "";
  if (!documentId || (level3d.sourceKey && level3d.sourceDocumentId === documentId)) {
    return false;
  }
  const source = isPuzzleDocument(document) && isTextDocument(document) ? sourceForDocument(document) : "";
  const selectionKey = [
    documentId,
    document?.puzzlePath || "",
    String(source || "").length,
    focusedPuzzleLevel3dEntries(source).length,
    Array.isArray(exportData?.levels) ? exportData.levels.length : 0,
  ].join(":");
  if (level3dAutoSelectionKey === selectionKey) {
    return false;
  }
  level3dAutoSelectionKey = selectionKey;
  const target = firstLevel3dTargetInDocument(document);
  if (!target) {
    level3d.sourceDocumentId = "";
    level3d.sourceTargetStart = null;
    level3d.sourceKey = "";
    return false;
  }
  loadLevel3dSourceDefinition(target.entry, target.source, {
    document: target.document,
    exportData,
    render: false,
    silent: true,
    switchMode: false,
  });
  return true;
}

function firstLevel3dTargetInDocument(document) {
  if (!isPuzzleDocument(document) || !isTextDocument(document)) {
    return null;
  }
  const source = sourceForDocument(document);
  const entry = firstLevel3dSourceDefinition(source);
  return entry ? { document, source, entry } : null;
}

function firstLevel3dSourceDefinition(source) {
  return level3dSourceEntries(source)[0] || null;
}

function currentLevel3dBundleName(exportData = previewBuild?.exportData) {
  const sourceEntry = level3dSourceEntries(level3dEditorSource())[0];
  if (sourceEntry?.bundle) return sourceEntry.bundle;
  const bundles = Object.keys(exportData?.levelBundles || {}).filter((name) => !["default", "levels"].includes(name));
  return bundles[0] || "levels";
}

function level3dNameControlConfig(source = level3dEditorSource()) {
  return focusedLevelNameControlConfig(source, {
    nameInput: level3dNameInput,
    datalist: level3dNameOptions,
  });
}

function syncLevel3dNameOptions() {
  if (typeof syncSourceLevelNameDatalist !== "function") {
    return [];
  }
  return syncSourceLevelNameDatalist(level3dNameControlConfig());
}

function level3dNamePickerConfig(sourceDocument = level3dSourceDocument()) {
  const source = level3dEditorSource(sourceDocument);
  return {
    ...level3dNameControlConfig(source),
    load: (match) => {
      if (match?.dimension && typeof loadLevelNameEntry === "function") {
        return loadLevelNameEntry(match);
      }
      return loadLevel3dNameEntry({ entry: match?.entry, range: match?.range, source, sourceDocument });
    },
  };
}

function loadLevel3dNameEntry({ entry, range, source = level3dEditorSource(), sourceDocument = level3dSourceDocument() }) {
  return loadLevel3dSourceDefinition({
    ...entry,
    bundle: range?.bundle || "",
    model: range?.model || "",
    rows: entry?.rows || [],
  }, source, {
    document: sourceDocument,
    recordHistory: true,
    silent: false,
  });
}

function showLevel3dNameOptions() {
  if (typeof showSourceLevelNameMenu !== "function") {
    return syncLevel3dNameOptions();
  }
  syncLevel3dNameOptions();
  return showSourceLevelNameMenu(level3dNamePickerConfig());
}

function hideLevel3dNameOptions() {
  if (typeof hideSourceLevelNameMenu === "function") {
    hideSourceLevelNameMenu(level3dNameInput);
  }
}

function loadSelectedLevel3dNameFromInput() {
  if (typeof loadSourceLevelNameSelection !== "function") {
    return false;
  }
  const sourceDocument = level3dSourceDocument();
  return loadSourceLevelNameSelection(level3dNamePickerConfig(sourceDocument));
}

function isPuzzle3dExport(exportData) {
  return exportData?.kind === "puzzle3d";
}

function renderLevel3dSourcePreview() {
  if (!level3dSourcePreview) {
    return;
  }
  let levelName;
  let sourceData;
  try {
    syncLevel3dNameOptions();
    levelName = sanitizeLevel3dName(level3dNameInput?.value || currentLevel3dSourceDefinition(level3dEditorSource())?.name || "level 1");
    sourceData = level3dSourceData();
  } catch (error) {
    level3dSourcePreview.textContent = `Could not inspect 3D level: ${userFacingRuntimeError(error)}`;
    return;
  }
  const generation = ++level3dSourcePreviewGeneration;
  levelSourceRequest(level3dEditorSource(), {
    operation: "format",
    name: levelName,
    rows: sourceData.rows,
    localLegends: level3dLocalLegendDrafts(sourceData),
  }).then((result) => {
    if (generation === level3dSourcePreviewGeneration) {
      level3dSourcePreview.textContent = result.text;
    }
  }).catch((error) => {
    if (generation === level3dSourcePreviewGeneration) {
      level3dSourcePreview.textContent = `Could not format 3D level: ${userFacingRuntimeError(error)}`;
    }
  });
}

function level3dSourceData(source = level3dEditorSource(), exportData = previewBuild?.exportData) {
  if (level3d.slices.length) {
    return level3dStateLevelData();
  }
  const sourceDefinition = currentLevel3dSourceDefinition(source);
  if (!sourceDefinition) {
    return { rows: [], unknownCells: 0 };
  }
  const entry = level3dExportEntryForSourceDefinition(sourceDefinition, exportData);
  if (!entry) {
    return { rows: sourceDefinition.rows || [], unknownCells: 0 };
  }
  const legendEntries = sourceDefinition.legend;
  const rows = level3dRowsForEntry(entry, legendEntries);
  return rows;
}

function level3dRowsForEntry(entry, legendEntries) {
  const size = entry.size || {};
  const width = Math.max(0, Math.trunc(Number(size.width) || 0));
  const depth = Math.max(0, Math.trunc(Number(size.depth) || 0));
  const height = Math.max(0, Math.trunc(Number(size.height) || 0));
  const exactByObjects = new Map();
  for (const legend of legendEntries) {
    exactByObjects.set(level3dObjectSetKey(legend.objects), legend.char);
  }
  const cellMap = new Map();
  for (const cell of entry.cells || []) {
    const position = cell.position || {};
    const objects = (cell.objects || []).map((object) => object.name || object.visual || "").filter(Boolean);
    cellMap.set(`${position.x},${position.y},${position.z}`, objects);
  }
  const rows = [];
  let unknownCells = 0;
  for (let slice = 0; slice < height; slice += 1) {
    if (slice > 0) {
      rows.push("");
    }
    const z = slice;
    for (let row = 0; row < depth; row += 1) {
      const y = row;
      let text = "";
      for (let x = 0; x < width; x += 1) {
        const objects = cellMap.get(`${x},${y},${z}`) || [];
        const ch = exactByObjects.get(level3dObjectSetKey(objects));
        if (ch) {
          text += ch;
        } else if (objects.length === 0) {
          text += exactByObjects.get("") || LEVEL3D_EMPTY_CHAR;
        } else {
          text += "?";
          unknownCells += 1;
        }
      }
      rows.push(text);
    }
  }
  return { rows, unknownCells };
}

function normalizeLevel3dSourceData(levelData) {
  if (Array.isArray(levelData)) {
    return { rows: levelData, unknownCells: 0 };
  }
  return {
    rows: Array.isArray(levelData?.rows) ? levelData.rows : [],
    unknownCells: Number(levelData?.unknownCells) || 0,
  };
}

function level3dSourceEntries(source) {
  return surfaceEntriesForSource(source)
    .filter((entry) => sourceTargetMatches(entry, "level", "3d"))
    .map((entry) => {
      const contract = entry?.sourceLevel;
      if (
        !contract
        || !Array.isArray(contract.rows)
        || !Array.isArray(contract.legend)
        || contract.legend.some((legend) => !Array.isArray(legend.objects))
      ) {
        throw new Error("3D level source target is missing its typed sourceLevel contract.");
      }
      return {
        ...entry,
        bundle: entry.params?.bundle || "",
        model: entry.params?.model || "",
        rows: contract.rows,
        legend: contract.legend.map((legend) => ({
          char: legend.symbol,
          objects: legend.objects || [],
          objectIds: legend.objectIds || [],
        })),
        localLegends: (contract.localLegends || []).map((legend) => ({
          symbol: legend.symbol,
          selectors: legend.selectors || [],
          objectIds: legend.objectIds || [],
        })),
      };
    });
}

function currentLevel3dSourceDefinition(source) {
  const entries = level3dSourceEntries(source);
  const loaded = Number.isInteger(level3d.sourceTargetStart)
    ? entries.find((entry) => entry.start === level3d.sourceTargetStart)
    : null;
  if (loaded) {
    return loaded;
  }
  const requestedBundle = String(level3dBundleInput?.value || "").trim();
  const requestedName = String(level3dNameInput?.value || "").trim();
  return entries.find((entry) => (
    (!requestedBundle || entry.bundle === requestedBundle)
    && (!requestedName || entry.name === requestedName)
  )) || entries.find((entry) => entry.legend.some((legend) => legend.objects.length > 0))
    || entries[0]
    || null;
}

function level3dObjectSetKey(objects) {
  return [...objects].sort().join("\u0000");
}

function selectLevel3dPaletteEntry(entry) {
  if (!entry || level3dPlaytestActive) return false;
  setLevel3dStageResizeMode(null);
  level3d.selectedChar = entry.char;
  renderLevel3dPalette();
  renderLevel3dLayerPalette();
  renderLevel3dLayerBoard();
  renderLevel3dRuntime();
  return true;
}

function selectLevel3dPaletteIndex(index) {
  const entry = level3dVisiblePaletteEntries()[index];
  return entry ? selectLevel3dPaletteEntry(entry) : false;
}

function renderLevel3dPalette() {
  if (!level3dPalette) {
    return;
  }
  level3dPalette.replaceChildren();
  level3dPalette.classList.add("is-visual-only");
  const exportData = previewBuild?.exportData;
  const paintSelectionActive = level3dPaintSelectionActive();
  level3dPalette.append(level3dFrameToggleButton());
  level3dPalette.append(level3dExpandModeButton());
  level3dPalette.append(level3dShrinkModeButton());
  level3dPalette.append(level3dEditModeButton("add"));
  level3dPalette.append(level3dEditModeButton("replace"));
  for (const entry of level3dVisiblePaletteEntries()) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "level-token level3d-token";
    button.classList.toggle("is-selected", paintSelectionActive && entry.char === level3d.selectedChar);
    button.dataset.label = level3dPaletteEntryLabel(entry);
    button.title = level3dPaletteEntryLabel(entry);
    button.setAttribute("aria-label", `Paint ${level3dPaletteEntryLabel(entry)}`);
    button.disabled = level3dPlaytestActive;

    const visual = document.createElement("canvas");
    visual.className = "level3d-token-preview";
    visual.setAttribute("aria-hidden", "true");
    button.append(visual);

    const label = document.createElement("span");
    label.className = "tile-label level3d-token-label";
    label.textContent = entry.objects.length ? entry.objects.join(" ") : "empty";
    button.append(label);

    button.addEventListener("click", () => selectLevel3dPaletteEntry(entry));
    level3dPalette.append(button);
    drawLevel3dPalettePreview(visual, entry, exportData);
  }
}

function renderLevel3dLayerPalette() {
  if (!level3dLayerPalette) {
    return;
  }
  level3dLayerPalette.replaceChildren();
  level3dLayerPalette.classList.add("is-visual-only");
  const exportData = previewBuild?.exportData;
  const transformRow = document.createElement("div");
  transformRow.className = "level3d-layer-palette-row level3d-layer-transform-row";
  transformRow.append(
    level3dLayerGridButton(),
    level3dLayerResizeModeButton("expand"),
    level3dLayerResizeModeButton("shrink"),
    level3dLayerVisibilityControl(),
    level3dLayerTransformButton("rotate-left"),
    level3dLayerTransformButton("rotate-right"),
    level3dLayerTransformButton("flip-horizontal"),
    level3dLayerTransformButton("flip-vertical"),
  );
  level3dLayerPalette.append(transformRow);
  const editRow = document.createElement("div");
  editRow.className = "level3d-layer-palette-row level3d-layer-edit-row";
  editRow.append(
    level3dLayerFillButton(),
    level3dLayerEraserButton(),
  );
  const group = document.createElement("div");
  group.className = "level-palette-group";
  for (const entry of level3dVisiblePaletteEntries()) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "level-token level3d-layer-token";
    button.classList.toggle("is-selected", entry.char === level3d.selectedChar);
    button.dataset.label = level3dPaletteEntryLabel(entry);
    button.title = level3dPaletteEntryLabel(entry);
    button.setAttribute("aria-label", `Paint top-down ${level3dPaletteEntryLabel(entry)}`);
    button.disabled = level3dPlaytestActive;

    const visual = document.createElement("canvas");
    visual.className = "level3d-layer-token-preview";
    visual.setAttribute("aria-hidden", "true");
    button.append(visual);

    const label = document.createElement("span");
    label.className = "tile-label level3d-token-label";
    label.textContent = entry.objects.length ? entry.objects.join(" ") : "empty";
    button.append(label);

    button.addEventListener("click", () => selectLevel3dPaletteEntry(entry));
    group.append(button);
    drawLevel3dTopDownTilePreview(visual, entry, exportData);
  }
  editRow.append(group);
  level3dLayerPalette.append(editRow);
}

function level3dLayerGridButton() {
  const active = level3d.layerGridVisible !== false;
  const button = document.createElement("button");
  button.type = "button";
  button.className = "icon-button visual-icon-button level-grid-button";
  button.classList.toggle("is-selected", active);
  button.setAttribute("aria-label", "Toggle top-down grid");
  button.setAttribute("aria-pressed", active ? "true" : "false");
  button.title = "Toggle grid";
  button.dataset.tooltip = "Toggle grid";
  button.disabled = level3dPlaytestActive;
  button.innerHTML = `
    ${editorIconSvg("grid-2x2", { className: "level-grid-token-icon" })}
  `;
  button.addEventListener("click", toggleLevel3dLayerGrid);
  return button;
}

function toggleLevel3dLayerGrid() {
  if (level3dPlaytestActive) return false;
  level3d.layerGridVisible = level3d.layerGridVisible === false;
  renderLevel3dLayerPalette();
  renderLevel3dLayerBoard();
  return true;
}

function level3dLayerVisibilityControl() {
  const wrap = document.createElement("span");
  wrap.className = "level-layer-visibility-wrap";
  const button = document.createElement("button");
  button.type = "button";
  button.className = "icon-button visual-icon-button level-layer-visibility-button";
  const hasHiddenLayers = normalizedLevel3dHiddenLayers().size > 0;
  button.classList.toggle("has-hidden-layers", hasHiddenLayers);
  button.setAttribute("aria-label", "Layer visibility");
  button.setAttribute("aria-expanded", "false");
  button.title = "Layer visibility";
  button.dataset.tooltip = "Layer visibility";
  button.disabled = level3dPlaytestActive;
  button.innerHTML = `
    ${editorIconSvg("list-filter", { className: "level-layer-visibility-icon" })}
  `;
  const menu = document.createElement("div");
  menu.className = "level-layer-visibility-menu";
  menu.setAttribute("role", "menu");
  menu.setAttribute("aria-label", "Layer visibility");
  menu.hidden = true;
  renderLevel3dLayerVisibilityMenu(menu);
  button.addEventListener("click", (event) => {
    event.stopPropagation();
    if (level3dPlaytestActive) {
      return;
    }
    menu.hidden = !menu.hidden;
    button.classList.toggle("is-open", !menu.hidden);
    button.setAttribute("aria-expanded", String(!menu.hidden));
  });
  menu.addEventListener("click", (event) => {
    event.stopPropagation();
  });
  menu.addEventListener("change", (event) => {
    const checkbox = event.target?.closest?.("[data-level3d-visibility-layer]");
    if (!checkbox) {
      return;
    }
    setLevel3dLayerVisible(Number(checkbox.dataset.level3dVisibilityLayer), checkbox.checked);
  });
  wrap.append(button, menu);
  return wrap;
}

function renderLevel3dLayerVisibilityMenu(menu) {
  if (!menu) {
    return;
  }
  menu.replaceChildren(...level3dLayerVisibilityEntries().map((entry) => {
    const label = document.createElement("label");
    label.className = "option-button level-layer-visibility-option";
    label.setAttribute("role", "menuitemcheckbox");
    label.setAttribute("aria-checked", String(level3dLayerIsVisible(entry.layer)));
    const checkbox = document.createElement("input");
    checkbox.type = "checkbox";
    checkbox.checked = level3dLayerIsVisible(entry.layer);
    checkbox.dataset.level3dVisibilityLayer = String(entry.layer);
    checkbox.disabled = level3dPlaytestActive;
    const text = document.createElement("span");
    text.className = "level-layer-visibility-label";
    text.textContent = entry.label;
    label.append(checkbox, text);
    return label;
  }));
}

function level3dLayerVisibilityEntries(exportData = previewBuild?.exportData) {
  const count = level3dLayerCount(exportData);
  const layerNames = typeof sourceLayerNameEntries === "function"
    ? sourceLayerNameEntries(level3dEditorSource(), exportData)
    : new Map();
  return Array.from({ length: count }, (_, layerIndex) => ({
    layer: layerIndex,
    label: layerNames.get(layerIndex) || "",
  }));
}

function level3dLayerCount(exportData = previewBuild?.exportData) {
  const explicit = Math.trunc(Number(
    exportData?.layerCount
      ?? exportData?.engine?.layerCount
      ?? exportData?.levels?.[0]?.layerCount,
  ) || 0);
  if (explicit > 0) {
    return explicit;
  }
  const layers = [];
  for (const entry of level3d.palette || []) {
    for (const name of entry.objects || []) {
      const layer = Math.trunc(Number(level3dObjectDescriptor(name, exportData)?.layer));
      if (Number.isInteger(layer) && layer >= 0) {
        layers.push(layer);
      }
    }
  }
  return layers.length ? Math.max(...layers) + 1 : 1;
}

function normalizedLevel3dHiddenLayers(exportData = previewBuild?.exportData) {
  const count = Math.max(1, level3dLayerCount(exportData));
  const hidden = new Set();
  for (const layerIndex of level3d.hiddenLayers || []) {
    const normalized = Math.trunc(Number(layerIndex));
    if (Number.isInteger(normalized) && normalized >= 0 && normalized < count) {
      hidden.add(normalized);
    }
  }
  level3d.hiddenLayers = [...hidden].sort((left, right) => left - right);
  return hidden;
}

function level3dLayerIsVisible(layerIndex, exportData = previewBuild?.exportData) {
  const normalized = Math.trunc(Number(layerIndex));
  if (!Number.isInteger(normalized) || normalized < 0) {
    return true;
  }
  return !normalizedLevel3dHiddenLayers(exportData).has(normalized);
}

function setLevel3dLayerVisible(layerIndex, visible) {
  const count = Math.max(1, level3dLayerCount());
  const normalized = Math.max(0, Math.min(count - 1, Math.trunc(Number(layerIndex) || 0)));
  const hidden = normalizedLevel3dHiddenLayers();
  if (visible) {
    hidden.delete(normalized);
  } else {
    hidden.add(normalized);
  }
  level3d.hiddenLayers = [...hidden].sort((left, right) => left - right);
  const keepOpen = Boolean(document.querySelector(".level3d-layer-palette .level-layer-visibility-menu:not([hidden])"));
  renderLevel3dLayerPalette();
  if (keepOpen) {
    const menu = document.querySelector(".level3d-layer-palette .level-layer-visibility-menu");
    const button = menu?.closest(".level-layer-visibility-wrap")?.querySelector(".level-layer-visibility-button");
    if (menu && button) {
      menu.hidden = false;
      button.classList.add("is-open");
      button.setAttribute("aria-expanded", "true");
    }
  }
  renderLevel3dLayerBoard();
  setLevel3dActionStatus(level3d.hiddenLayers.length ? "Layer visibility filtered" : "All layers visible", "is-ok");
}

function level3dLayerResizeModeButton(mode) {
  const active = level3dStageResizeMode() === mode;
  const button = document.createElement("button");
  button.type = "button";
  button.className = `icon-button visual-icon-button ${mode === "expand" ? "level-expand-button" : "level-shrink-button"}`;
  button.classList.toggle("is-selected", active);
  button.setAttribute("aria-label", mode === "expand" ? "Toggle top-down expansion" : "Toggle top-down shrinking");
  button.setAttribute("aria-pressed", active ? "true" : "false");
  button.title = mode === "expand" ? "Expand" : "Shrink";
  button.dataset.tooltip = button.title;
  button.disabled = level3dPlaytestActive;
  button.innerHTML = mode === "expand"
    ? `
      ${editorIconSvg("expand")}
    `
    : `
      ${editorIconSvg("shrink")}
    `;
  button.addEventListener("click", () => {
    setLevel3dStageResizeMode(level3dStageResizeMode() === mode ? null : mode);
    renderLevel3dPalette();
    renderLevel3dLayerPalette();
    syncLevel3dLayerResizeControls();
    renderLevel3dRuntime();
  });
  return button;
}

function level3dLayerTransformButton(kind) {
  const config = {
    "rotate-left": {
      label: "Rotate top-down left",
      title: "Rotate left",
      icon: `
        ${editorIconSvg("rotate-ccw")}
      `,
      action: rotateLevel3dLayerLeft,
    },
    "rotate-right": {
      label: "Rotate top-down right",
      title: "Rotate right",
      icon: `
        ${editorIconSvg("rotate-cw")}
      `,
      action: rotateLevel3dLayerRight,
    },
    "flip-horizontal": {
      label: "Flip top-down horizontal",
      title: "Flip horizontal",
      icon: `
        ${editorIconSvg("flip-horizontal")}
      `,
      action: flipLevel3dLayerHorizontal,
    },
    "flip-vertical": {
      label: "Flip top-down vertical",
      title: "Flip vertical",
      icon: `
        ${editorIconSvg("flip-vertical")}
      `,
      action: flipLevel3dLayerVertical,
    },
  }[kind];
  const button = document.createElement("button");
  button.type = "button";
  button.className = "icon-button visual-icon-button level3d-layer-transform-button";
  button.setAttribute("aria-label", config.label);
  button.title = config.title;
  button.dataset.tooltip = config.title;
  button.disabled = level3dPlaytestActive;
  button.innerHTML = config.icon;
  button.addEventListener("click", () => {
    setLevel3dStageResizeMode(null);
    config.action();
  });
  return button;
}

function level3dLayerFillButton() {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "icon-button level-palette-tool-button visual-fill-button visual-icon-button";
  button.classList.toggle("is-active", Boolean(level3d.layerFillActive));
  button.setAttribute("aria-label", "Fill top-down 3D level area");
  button.setAttribute("aria-pressed", level3d.layerFillActive ? "true" : "false");
  button.title = "Fill";
  button.dataset.tooltip = "Fill";
  button.disabled = level3dPlaytestActive;
  button.innerHTML = `
    ${editorIconSvg("paint-bucket")}
  `;
  button.addEventListener("click", toggleLevel3dLayerFill);
  return button;
}

function toggleLevel3dLayerFill() {
  if (level3dPlaytestActive) return false;
  level3d.layerFillActive = !level3d.layerFillActive;
  renderLevel3dLayerPalette();
  return true;
}

function deactivateLevel3dLayerFillModeAfterUse() {
  if (!level3d.layerFillActive) {
    return;
  }
  level3d.layerFillActive = false;
  renderLevel3dLayerPalette();
}

function level3dLayerEraserButton() {
  const entry = level3d.palette.find((candidate) => !candidate.objects?.length)
    || { char: level3dEmptyChar(), objects: [] };
  const button = document.createElement("button");
  button.type = "button";
  button.className = "icon-button level-palette-tool-button visual-icon-button level-eraser-button";
  button.classList.toggle("is-active", level3d.selectedChar === entry.char);
  button.setAttribute("aria-label", "Paint top-down Eraser");
  button.setAttribute("aria-pressed", level3d.selectedChar === entry.char ? "true" : "false");
  button.title = "Eraser";
  button.dataset.tooltip = "Eraser";
  button.disabled = level3dPlaytestActive;
  button.append(renderLevelEraserIcon());
  button.addEventListener("click", selectLevel3dEraser);
  return button;
}

function selectLevel3dEraser() {
  const entry = level3d.palette.find((candidate) => !candidate.objects?.length)
    || { char: level3dEmptyChar(), objects: [] };
  return selectLevel3dPaletteEntry(entry);
}

function level3dPaintSelectionActive() {
  return !level3dStageResizeMode();
}

function level3dEditMode() {
  return level3d.editMode === "add" ? "add" : "replace";
}

function setLevel3dEditMode(mode) {
  level3d.editMode = mode === "add" ? "add" : "replace";
}

function level3dEditModeButton(mode) {
  const active = level3dEditMode() === mode;
  const button = document.createElement("button");
  button.type = "button";
  button.className = `icon-button source-action-button level3d-edit-mode-button level3d-${mode}-button`;
  button.classList.toggle("is-selected", active);
  button.dataset.label = mode === "add" ? "Add tile" : "Replace tile";
  button.title = mode === "add" ? "Add tile" : "Replace tile";
  button.setAttribute("aria-label", mode === "add" ? "Use add tile mode" : "Use replace tile mode");
  button.setAttribute("aria-pressed", active ? "true" : "false");
  button.disabled = level3dPlaytestActive;
  button.innerHTML = mode === "add"
    ? `
      ${editorIconSvg("square-plus")}
    `
    : `
      ${editorIconSvg("square-pen")}
    `;
  button.addEventListener("click", () => {
    setLevel3dStageResizeMode(null);
    setLevel3dEditMode(mode);
    renderLevel3dPalette();
    renderLevel3dLayerPalette();
    renderLevel3dRuntime();
  });
  return button;
}

function level3dStageResizeMode() {
  const mode = level3d.stageResizeMode || (level3d.stageExpandMode ? "expand" : null);
  return mode === "expand" || mode === "shrink" ? mode : null;
}

function setLevel3dStageResizeMode(mode) {
  const normalized = mode === "expand" || mode === "shrink" ? mode : null;
  level3d.stageResizeMode = normalized;
  level3d.stageExpandMode = normalized === "expand";
}

function level3dFrameToggleButton() {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "icon-button source-action-button level3d-frame-toggle-button";
  button.classList.toggle("is-selected", Boolean(level3d.previewFrames));
  button.dataset.label = "Cell and stage frames";
  button.title = "Cell and stage frames";
  button.setAttribute("aria-label", "Toggle occupied cell and stage frames in the 3D preview");
  button.setAttribute("aria-pressed", level3d.previewFrames ? "true" : "false");
  button.disabled = level3dPlaytestActive;
  button.innerHTML = `
    ${editorIconSvg("grid-2x2", { className: "level3d-frame-token-icon" })}
  `;
  button.addEventListener("click", toggleLevel3dFrameVisibility);
  return button;
}

function toggleLevel3dFrameVisibility() {
  if (level3dPlaytestActive) return false;
  level3d.previewFrames = !level3d.previewFrames;
  renderLevel3dPalette();
  renderLevel3dLayerPalette();
  renderLevel3dRuntime();
  return true;
}

function level3dExpandModeButton() {
  return level3dStageResizeModeButton({
    mode: "expand",
    className: "level3d-expand-button",
    label: "Expand stage",
    ariaLabel: "Toggle 3D stage expansion",
    icon: `
      ${editorIconSvg("expand", { className: "level3d-expand-token-icon" })}
    `,
  });
}

function level3dShrinkModeButton() {
  return level3dStageResizeModeButton({
    mode: "shrink",
    className: "level3d-shrink-button",
    label: "Shrink stage",
    ariaLabel: "Toggle 3D stage shrinking",
    icon: `
      ${editorIconSvg("shrink", { className: "level3d-shrink-token-icon" })}
    `,
  });
}

function level3dStageResizeModeButton({ mode, className, label, ariaLabel, icon }) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = `icon-button source-action-button ${className}`;
  button.classList.toggle("is-selected", level3dStageResizeMode() === mode);
  button.dataset.label = label;
  button.title = label;
  button.setAttribute("aria-label", ariaLabel);
  button.setAttribute("aria-pressed", level3dStageResizeMode() === mode ? "true" : "false");
  button.disabled = level3dPlaytestActive;
  button.innerHTML = icon;
  button.addEventListener("click", () => {
    setLevel3dStageResizeMode(level3dStageResizeMode() === mode ? null : mode);
    renderLevel3dPalette();
    renderLevel3dLayerPalette();
    renderLevel3dRuntime();
  });
  return button;
}

function level3dPaletteEntryLabel(entry) {
  return entry.objects.length ? `${entry.char} = ${entry.objects.join(" ")}` : `${entry.char} = empty`;
}

function drawLevel3dPalettePreview(canvas, entry, exportData = previewBuild?.exportData) {
  if (!(canvas instanceof HTMLCanvasElement)) {
    return;
  }
  const { width, height, scale } = resizeLevel3dPreviewCanvas(
    canvas,
    LEVEL3D_PALETTE_PREVIEW_SIZE,
    LEVEL3D_PALETTE_PREVIEW_SIZE,
  );
  const ctx = canvas.getContext("2d");
  if (!ctx) {
    return;
  }
  ctx.setTransform(scale, 0, 0, scale, 0, 0);
  ctx.imageSmoothingEnabled = true;
  ctx.imageSmoothingQuality = "high";
  ctx.clearRect(0, 0, width, height);
  const visuals = level3dPreviewVisuals(exportData);
  const objects = (entry.objects || [])
    .map((name) => level3dPaletteObjectDescriptor(name, exportData, visuals))
    .filter(Boolean);
  const snapshot = {
    size: { width: 1, depth: 1, height: 1 },
    visuals,
    render: {
      ...(exportData?.render || {}),
      camera: level3dPalettePreviewCamera(exportData),
    },
  };
  if (!objects.length) {
    if ((entry.objects || []).length) {
      drawLevel3dUnavailableTilePreview(ctx, width, height, entry.char);
    } else {
      drawLevel3dEmptyTilePreview(ctx, width, height, snapshot, level3dPalettePreviewOptions(snapshot.render.camera));
    }
    return;
  }
  drawLevel3dCellsPreview(ctx, width, height, snapshot, [{
    position: { x: 0, y: 0, z: 0 },
    objects,
  }], level3dPalettePreviewOptions(snapshot.render.camera));
}

function level3dPalettePreviewCamera(source) {
  return {
    ...level3dPreviewCamera(source),
    zoom: 1,
  };
}

function level3dPalettePreviewOptions(camera) {
  return {
    camera,
    origin: { x: 0, y: 0, z: 0 },
    padding: 0.72,
  };
}

function resizeLevel3dPreviewCanvas(canvas, fallbackWidth, fallbackHeight) {
  const rect = canvas.getBoundingClientRect();
  const width = Math.max(1, Math.round(rect.width || fallbackWidth));
  const height = Math.max(1, Math.round(rect.height || fallbackHeight));
  const scale = Math.max(1, window.devicePixelRatio || 1);
  const backingWidth = Math.max(1, Math.round(width * scale));
  const backingHeight = Math.max(1, Math.round(height * scale));
  if (canvas.width !== backingWidth || canvas.height !== backingHeight) {
    canvas.width = backingWidth;
    canvas.height = backingHeight;
  }
  return { width, height, scale };
}

function drawLevel3dEmptyTilePreview(ctx, width, height, snapshot, options = {}) {
  const view = level3dPreviewView(snapshot, width, height, { ...options, padding: 0.96 });
  const face = level3dPlacementFace("zPos", { x: 0, y: 0, z: -1 }, view, { kind: "empty" });
  ctx.save();
  ctx.fillStyle = "rgba(255, 255, 255, 0.24)";
  ctx.strokeStyle = "rgba(157, 163, 170, 0.72)";
  ctx.lineWidth = 1.25;
  drawLevel3dPolygonPath(ctx, face.polygon);
  ctx.fill();
  ctx.stroke();
  ctx.restore();
}

function drawLevel3dUnavailableTilePreview(ctx, width, height, label = "?") {
  ctx.save();
  const text = String(label || "?").slice(0, 2);
  ctx.fillStyle = "rgba(157, 163, 170, 0.78)";
  ctx.font = `800 ${Math.max(12, Math.floor(Math.min(width, height) * 0.42))}px ui-monospace, SFMono-Regular, Menlo, Consolas, monospace`;
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.fillText(text, width / 2, height / 2);
  ctx.restore();
}

function level3dCellLabel(ch) {
  const entry = level3d.palette.find((candidate) => candidate.char === ch);
  return entry ? level3dPaletteEntryLabel(entry) : ch;
}

function renderLevel3dLayerControls() {
  const height = Math.max(1, Math.trunc(Number(level3d.height) || 1));
  level3d.slice = Math.max(0, Math.min(height - 1, Math.trunc(Number(level3d.slice) || 0)));
  if (level3dLayerInput instanceof HTMLInputElement && document.activeElement !== level3dLayerInput) {
    level3dLayerInput.min = "1";
    level3dLayerInput.max = String(height);
    level3dLayerInput.value = String(level3d.slice + 1);
  }
  if (level3dLayerTotal) {
    level3dLayerTotal.textContent = String(height);
  }
  if (level3dAddSliceAboveButton) {
    level3dAddSliceAboveButton.disabled = level3dPlaytestActive || !level3d.slices.length;
  }
  if (level3dAddSliceBelowButton) {
    level3dAddSliceBelowButton.disabled = level3dPlaytestActive || !level3d.slices.length;
  }
  if (level3dPreviousLayerButton) {
    level3dPreviousLayerButton.disabled = level3d.slice <= 0;
  }
  if (level3dNextLayerButton) {
    level3dNextLayerButton.disabled = level3d.slice >= height - 1;
  }
}

function renderLevel3dLayerBoard() {
  if (!level3dLayerBoard) {
    return;
  }
  const width = Math.max(1, Math.trunc(Number(level3d.width) || 1));
  const depth = Math.max(1, Math.trunc(Number(level3d.depth) || 1));
  level3dLayerBoard.style.setProperty("--level3d-layer-width", width);
  level3dLayerBoard.style.setProperty("--level3d-layer-depth", depth);
  level3dLayerBoard.style.setProperty("--level3d-layer-cell-size", `${level3dLayerCellSize(width, depth)}px`);
  level3dLayerBoard.parentElement?.style.setProperty("--level3d-layer-width", width);
  level3dLayerBoard.parentElement?.style.setProperty("--level3d-layer-depth", depth);
  level3dLayerBoard.closest?.(".level3d-layer-frame-viewport")?.style.setProperty("--level3d-layer-width", width);
  level3dLayerBoard.closest?.(".level3d-layer-frame-viewport")?.style.setProperty("--level3d-layer-depth", depth);
  level3dLayerBoard.classList.add("is-grid-board");
  level3dLayerBoard.classList.toggle("has-grid", level3d.layerGridVisible !== false);
  syncLevel3dLayerResizeControls();
  if (!level3d.slices.length) {
    level3dLayerBoard.classList.add("is-empty");
    level3dLayerBoard.replaceChildren();
    return;
  }
  level3dLayerBoard.classList.remove("is-empty");
  renderLevel3dLayerGrid();
}

function level3dLayerCellSize(width, depth) {
  const frameWidth = level3dBuilderCssPixels("--level3d-frame-width", LEVEL3D_FRAME_MIN_WIDTH);
  const frameHeight = level3dBuilderCssPixels(
    "--level3d-frame-height",
    Math.round(LEVEL3D_FRAME_VIRTUAL_HEIGHT * (frameWidth / LEVEL3D_FRAME_VIRTUAL_WIDTH)),
  );
  const viewport = level3dLayerBoard?.closest?.(".level3d-layer-frame-viewport");
  const viewportStyle = viewport ? window.getComputedStyle(viewport) : null;
  const paddingX = viewportStyle
    ? (parseFloat(viewportStyle.paddingLeft) || 0) + (parseFloat(viewportStyle.paddingRight) || 0)
    : 0;
  const paddingY = viewportStyle
    ? (parseFloat(viewportStyle.paddingTop) || 0) + (parseFloat(viewportStyle.paddingBottom) || 0)
    : 0;
  const edgeColumnWidth = 24 * 2;
  const edgeRowHeight = 24 * 2;
  const gridGap = 6 * 2;
  const usableWidth = Math.max(1, frameWidth - paddingX - edgeColumnWidth - gridGap);
  const usableHeight = Math.max(1, frameHeight - paddingY - edgeRowHeight - gridGap);
  const widthFit = Math.floor(usableWidth / Math.max(1, width));
  const depthFit = Math.floor(usableHeight / Math.max(1, depth));
  const fitted = Math.max(1, Math.min(widthFit, depthFit));
  const quantum = level3dLayerVisualPixelQuantum();
  return quantum > 1 ? Math.max(quantum, Math.floor(fitted / quantum) * quantum) : fitted;
}

function level3dBuilderCssPixels(name, fallback) {
  if (!level3dBuilder) {
    return fallback;
  }
  const value = window.getComputedStyle(level3dBuilder).getPropertyValue(name);
  const parsed = parseFloat(value);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback;
}

function level3dLayerVisualPixelQuantum(exportData = previewBuild?.exportData) {
  const visuals = level3dPreviewVisuals(exportData);
  const dimensions = [];
  for (const entry of level3dVisiblePaletteEntries()) {
    for (const name of entry.objects || []) {
      const object = level3dPaletteObjectDescriptor(name, exportData, visuals);
      const visual = object ? (visuals?.[object.visual] || visuals?.[object.name]) : null;
      const size = level3dTopDownVisualSize(visual);
      if (size) {
        dimensions.push(size.width, size.depth);
      }
    }
  }
  return dimensions.reduce((quantum, value) => level3dLcm(quantum, value), 1);
}

function level3dTopDownVisualSize(visual) {
  if (!visual) {
    return null;
  }
  const blocks = level3dVisualLayers(visual);
  return {
    depth: Math.max(1, ...blocks.map((rows) => rows.length)),
    width: Math.max(1, ...blocks.flatMap((rows) => rows.map((row) => row.length))),
  };
}

function level3dLcm(left, right) {
  const a = Math.max(1, Math.trunc(Number(left) || 1));
  const b = Math.max(1, Math.trunc(Number(right) || 1));
  return Math.max(1, Math.trunc((a * b) / level3dGcd(a, b)));
}

function level3dGcd(left, right) {
  let a = Math.max(1, Math.trunc(Number(left) || 1));
  let b = Math.max(1, Math.trunc(Number(right) || 1));
  while (b) {
    const next = a % b;
    a = b;
    b = next;
  }
  return a;
}

function syncLevel3dLayerResizeControls() {
  const container = level3dLayerBoard?.parentElement;
  const mode = level3dStageResizeMode();
  container?.classList.toggle("is-resize-mode", Boolean(mode));
  container?.classList.toggle("is-resize-expand", mode === "expand");
  container?.classList.toggle("is-resize-shrink", mode === "shrink");
  container?.querySelectorAll?.("[data-level3d-layer-edge]").forEach((button) => {
    const edge = button.dataset.level3dLayerEdge || "";
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
    button.setAttribute("aria-label", `${action} top-down ${axis} ${side}`.trim());
    button.title = `${action} ${axis}`.trim();
    button.disabled = level3dPlaytestActive
      || !mode
      || (mode === "shrink" && (
        ((edge === "left" || edge === "right") && level3d.width <= 1)
        || ((edge === "top" || edge === "bottom") && level3d.depth <= 1)
      ));
  });
}

function renderLevel3dLayerGrid() {
  if (!level3dLayerBoard) {
    return;
  }
  const width = Math.max(1, Math.trunc(Number(level3d.width) || 1));
  const depth = Math.max(1, Math.trunc(Number(level3d.depth) || 1));
  const cells = [];
  for (let row = 0; row < depth; row += 1) {
    for (let x = 0; x < width; x += 1) {
      const y = row;
      const position = { x, y, z: currentLevel3dLayerZ() };
      const ch = level3dCharAtPosition(position);
      const cell = document.createElement("button");
      cell.type = "button";
      cell.className = "cell level3d-layer-cell";
      cell.dataset.index = String(row * width + x);
      cell.dataset.x = String(x);
      cell.dataset.y = String(y);
      cell.dataset.z = String(position.z);
      cell.setAttribute("role", "button");
      cell.setAttribute("aria-label", level3dCellLabel(ch));
      cell.tabIndex = 0;
      const entry = level3d.palette.find((candidate) => candidate.char === ch);
      cell.classList.toggle("is-empty", !entry || !entry.objects?.length);
      cell.classList.toggle("is-hover", level3dLayerHover
        && level3dLayerHover.x === x
        && level3dLayerHover.y === y
        && level3dLayerHover.z === position.z);
      cell.append(level3dLayerCellVisual(ch));
      cells.push(cell);
    }
  }
  level3dLayerBoard.replaceChildren(...cells);
}

function level3dLayerCellVisual(ch) {
  const root = document.createElement("span");
  root.className = "level3d-layer-cell-visual";
  const entry = level3d.palette.find((candidate) => candidate.char === ch);
  if (!entry || !entry.objects?.length) {
    root.classList.add("is-empty");
    return root;
  }
  const preview = document.createElement("canvas");
  preview.className = "level3d-layer-cell-preview";
  const cellSize = level3dCurrentLayerCellSize();
  preview.width = cellSize;
  preview.height = cellSize;
  drawLevel3dTopDownTilePreview(preview, entry, previewBuild?.exportData, {
    fallbackWidth: cellSize,
    fallbackHeight: cellSize,
    crop: false,
  });
  root.append(preview);
  return root;
}

function level3dCurrentLayerCellSize() {
  const value = level3dLayerBoard?.style.getPropertyValue("--level3d-layer-cell-size")
    || (level3dLayerBoard ? window.getComputedStyle(level3dLayerBoard).getPropertyValue("--level3d-layer-cell-size") : "");
  const parsed = parseFloat(value);
  return Number.isFinite(parsed) && parsed > 0 ? Math.round(parsed) : 56;
}

function drawLevel3dTopDownTilePreview(canvas, entry, exportData = previewBuild?.exportData, options = {}) {
  if (!(canvas instanceof HTMLCanvasElement)) {
    return;
  }
  const fallbackWidth = Math.max(1, Math.round(Number(options.fallbackWidth) || 56));
  const fallbackHeight = Math.max(1, Math.round(Number(options.fallbackHeight) || fallbackWidth));
  const { width, height, scale } = resizeLevel3dPreviewCanvas(canvas, fallbackWidth, fallbackHeight);
  const ctx = canvas.getContext("2d");
  if (!ctx) {
    return;
  }
  ctx.setTransform(scale, 0, 0, scale, 0, 0);
  ctx.imageSmoothingEnabled = false;
  ctx.clearRect(0, 0, width, height);
  const visuals = level3dPreviewVisuals(exportData);
  const projections = (entry.objects || [])
    .map((name) => level3dPaletteObjectDescriptor(name, exportData, visuals))
    .filter(Boolean)
    .filter((object) => level3dLayerIsVisible(object.layer, exportData))
    .map((object) => level3dTopDownVisualProjection(visuals?.[object.visual] || visuals?.[object.name], { crop: options.crop === true }))
    .filter(Boolean);
  if (!projections.length) {
    drawLevel3dUnavailableTilePreview(ctx, width, height, entry?.char);
    return;
  }
  const tileWidth = Math.max(1, ...projections.map((projection) => projection.width));
  const tileDepth = Math.max(1, ...projections.map((projection) => projection.depth));
  const cellSize = Math.max(1, Math.floor(Math.min(width / tileWidth, height / tileDepth)));
  const offsetX = Math.floor((width - tileWidth * cellSize) / 2);
  const offsetY = Math.floor((height - tileDepth * cellSize) / 2);
  for (const projection of projections) {
    const projectionOffsetX = offsetX + Math.floor((tileWidth - projection.width) * cellSize / 2);
    const projectionOffsetY = offsetY + Math.floor((tileDepth - projection.depth) * cellSize / 2);
    for (let row = 0; row < projection.depth; row += 1) {
      for (let column = 0; column < projection.width; column += 1) {
        const fill = projection.pixels[row]?.[column];
        if (!fill) {
          continue;
        }
        ctx.fillStyle = fill;
        ctx.fillRect(
          projectionOffsetX + column * cellSize,
          projectionOffsetY + row * cellSize,
          cellSize,
          cellSize,
        );
      }
    }
  }
}

function level3dTopDownVisualProjection(visual, options = {}) {
  if (!visual) {
    return null;
  }
  const blocks = level3dVisualLayers(visual);
  const depth = Math.max(1, ...blocks.map((rows) => rows.length));
  const width = Math.max(1, ...blocks.flatMap((rows) => rows.map((row) => row.length)));
  const pixels = Array.from({ length: depth }, () => Array.from({ length: width }, () => ""));
  for (let row = 0; row < depth; row += 1) {
    for (let column = 0; column < width; column += 1) {
      for (let z = 0; z < blocks.length; z += 1) {
        const token = blocks[z]?.[row]?.[column];
        const fill = visual.palette?.[token];
        if (fill && level3dParseColor(fill)?.a > 0) {
          pixels[row][column] = fill;
          break;
        }
      }
    }
  }
  const projection = { width, depth, pixels };
  return options.crop === true ? level3dCropTopDownProjection(projection) : projection;
}

function level3dCropTopDownProjection(projection) {
  let minX = projection.width;
  let maxX = -1;
  let minY = projection.depth;
  let maxY = -1;
  for (let row = 0; row < projection.depth; row += 1) {
    for (let column = 0; column < projection.width; column += 1) {
      if (!projection.pixels[row]?.[column]) {
        continue;
      }
      minX = Math.min(minX, column);
      maxX = Math.max(maxX, column);
      minY = Math.min(minY, row);
      maxY = Math.max(maxY, row);
    }
  }
  if (maxX < minX || maxY < minY) {
    return projection;
  }
  const width = maxX - minX + 1;
  const depth = maxY - minY + 1;
  const pixels = [];
  for (let row = minY; row <= maxY; row += 1) {
    pixels.push(projection.pixels[row].slice(minX, maxX + 1));
  }
  return { width, depth, pixels };
}

function currentLevel3dLayerZ() {
  const height = Math.max(1, Math.trunc(Number(level3d.height) || 1));
  const slice = Math.max(0, Math.min(height - 1, Math.trunc(Number(level3d.slice) || 0)));
  return slice;
}

function level3dSliceArrayIndexForZ(z = currentLevel3dLayerZ()) {
  const height = Math.max(1, Math.trunc(Number(level3d.height) || 1));
  return Math.max(0, Math.min(height - 1, Math.trunc(Number(z) || 0)));
}

function level3dCharAtPosition(position) {
  const x = Math.trunc(Number(position?.x) || 0);
  const y = Math.trunc(Number(position?.y) || 0);
  const z = Math.trunc(Number(position?.z) || 0);
  const slice = level3d.slices[level3dSliceArrayIndexForZ(z)] || [];
  const row = Math.max(0, Math.min(Math.max(1, level3d.depth || 1) - 1, y));
  const text = String(slice[row] || "").padEnd(Math.max(1, level3d.width || 1), level3dEmptyChar());
  return text[x] || level3dEmptyChar();
}

function setLevel3dLayer(value) {
  const height = Math.max(1, Math.trunc(Number(level3d.height) || 1));
  level3d.slice = Math.max(0, Math.min(height - 1, Math.trunc(Number(value) || 0)));
  level3dLayerHover = null;
  renderLevel3dLayerControls();
  renderLevel3dLayerBoard();
  renderLevel3dRuntime();
}

function moveLevel3dLayer(delta) {
  setLevel3dLayer(level3d.slice + delta);
}

function insertLevel3dSlice(relative) {
  if (level3dPlaytestActive || !level3d.slices.length) {
    syncLevel3dSizeControls();
    return false;
  }
  const before = visualEditSnapshot("level3d");
  const height = Math.max(1, Math.trunc(Number(level3d.height) || level3d.slices.length || 1));
  const current = Math.max(0, Math.min(height - 1, Math.trunc(Number(level3d.slice) || 0)));
  const insertIndex = relative === "below" ? current + 1 : current;
  level3d.slices.splice(insertIndex, 0, emptyLevel3dSlice(level3dEmptyChar()));
  level3d.height = height + 1;
  level3d.slice = insertIndex;
  syncLevel3dSizeControls();
  renderLevel3dLayerControls();
  renderLevel3dLayerBoard();
  renderLevel3dSourcePreview();
  renderLevel3dRuntime();
  pushVisualEditUndoSnapshot("level3d", before);
  setLevel3dActionStatus(`Added slice ${insertIndex + 1}`, "is-ok");
  return true;
}

function applyLevel3dLayerInput() {
  if (!(level3dLayerInput instanceof HTMLInputElement)) {
    return;
  }
  setLevel3dLayer(Math.trunc(Number(level3dLayerInput.value) || 1) - 1);
}

function level3dSliceScrubTarget(event) {
  return event.target?.closest?.("[data-level3d-slice-scrub]") || null;
}

function startLevel3dSliceScrub(event) {
  const target = level3dSliceScrubTarget(event);
  if (!target || event.button !== 0) {
    return;
  }
  event.preventDefault();
  event.stopPropagation();
  level3dSliceScrubDrag = {
    pointerId: event.pointerId,
    target,
    inputTarget: event.target === level3dLayerInput,
    startX: event.clientX,
    moved: false,
    slice: level3d.slice,
  };
  target.setPointerCapture?.(event.pointerId);
  target.classList.add("is-dragging");
  document.documentElement.classList.add("is-visual3d-slice-scrubbing");
}

function continueLevel3dSliceScrub(event) {
  if (!level3dSliceScrubDrag || level3dSliceScrubDrag.pointerId !== event.pointerId) {
    return;
  }
  event.preventDefault();
  event.stopPropagation();
  const deltaX = event.clientX - level3dSliceScrubDrag.startX;
  if (Math.abs(deltaX) > 2) {
    level3dSliceScrubDrag.moved = true;
  }
  setLevel3dLayer(level3dSliceScrubDrag.slice + Math.round(deltaX / LEVEL3D_SLICE_SCRUB_STEP_PX));
}

function stopLevel3dSliceScrub(event) {
  if (!level3dSliceScrubDrag || level3dSliceScrubDrag.pointerId !== event.pointerId) {
    return;
  }
  event.preventDefault();
  event.stopPropagation();
  finishLevel3dSliceScrub(event.pointerId);
}

function finishLevel3dSliceScrub(pointerId = null) {
  if (!level3dSliceScrubDrag) {
    return;
  }
  const { target, inputTarget, moved } = level3dSliceScrubDrag;
  if (pointerId !== null && target.hasPointerCapture?.(pointerId)) {
    target.releasePointerCapture(pointerId);
  }
  target.classList.remove("is-dragging");
  document.documentElement.classList.remove("is-visual3d-slice-scrubbing");
  level3dSliceScrubDrag = null;
  if (!moved && inputTarget && level3dLayerInput instanceof HTMLInputElement) {
    level3dLayerInput.focus();
    level3dLayerInput.select();
  }
}

function bucketFillLevel3dLayerFromPosition(position) {
  if (level3dPlaytestActive || !level3dPositionInBounds(position)) {
    return false;
  }
  const targetKey = level3dVisibleObjectKeyForChar(level3dCharAtPosition(position));
  const visited = new Set();
  const stack = [{ ...position }];
  let changed = false;
  while (stack.length) {
    const current = stack.pop();
    const key = `${current.x},${current.y},${current.z}`;
    if (
      visited.has(key)
      || !level3dPositionInBounds(current)
      || level3dVisibleObjectKeyForChar(level3dCharAtPosition(current)) !== targetKey
    ) {
      continue;
    }
    visited.add(key);
    if (paintLevel3dCellAtPosition(current)) {
      changed = true;
    }
    if (current.x > 0) {
      stack.push({ x: current.x - 1, y: current.y, z: current.z });
    }
    if (current.x < level3d.width - 1) {
      stack.push({ x: current.x + 1, y: current.y, z: current.z });
    }
    if (current.y > 0) {
      stack.push({ x: current.x, y: current.y - 1, z: current.z });
    }
    if (current.y < level3d.depth - 1) {
      stack.push({ x: current.x, y: current.y + 1, z: current.z });
    }
  }
  deactivateLevel3dLayerFillModeAfterUse();
  if (changed) {
    setLevel3dActionStatus(level3d.selectedChar ? "Filled connected slice area" : "Erased connected slice area", "is-ok");
  }
  return true;
}

function resizeLevel3dLayerEdge(edge, mode = level3dStageResizeMode() || "expand") {
  const normalizedMode = mode === "shrink" ? "shrink" : "expand";
  const delta = normalizedMode === "shrink" ? -1 : 1;
  if (edge === "left" || edge === "right") {
    return resizeLevel3dWidth((level3d.width || 1) + delta, { edge });
  }
  if (edge === "top" || edge === "bottom") {
    return resizeLevel3dDepth((level3d.depth || 1) + delta, { edge: edge === "top" ? "back" : "front" });
  }
  return false;
}

function transformLevel3dLayerCells({ nextWidth, nextDepth, mapCell, message }) {
  if (level3dPlaytestActive || !level3d.slices.length) {
    return false;
  }
  const previousWidth = Math.max(1, Math.trunc(Number(level3d.width) || 1));
  const previousDepth = Math.max(1, Math.trunc(Number(level3d.depth) || 1));
  const width = Math.max(1, Math.trunc(Number(nextWidth) || previousWidth));
  const depth = Math.max(1, Math.trunc(Number(nextDepth) || previousDepth));
  const before = visualEditSnapshot("level3d");
  const targets = level3d.slices.map((_slice, index) => index);
  for (const sliceIndex of targets) {
    level3d.slices[sliceIndex] = transformLevel3dRowsWithMap(
      level3d.slices[sliceIndex],
      previousWidth,
      previousDepth,
      width,
      depth,
      mapCell,
    );
  }
  level3d.width = width;
  level3d.depth = depth;
  syncLevel3dSizeControls();
  renderLevel3dLayerBoard();
  renderLevel3dSourcePreview();
  renderLevel3dRuntime();
  pushVisualEditUndoSnapshot("level3d", before);
  setLevel3dActionStatus(`${message} all slices`, "is-ok");
  return true;
}

function transformLevel3dRowsWithMap(rows, previousWidth, previousDepth, nextWidth, nextDepth, mapCell) {
  const empty = level3dEmptyChar();
  const sourceRows = Array.from({ length: previousDepth }, (_unused, row) => (
    String(rows?.[row] || "").padEnd(previousWidth, empty).slice(0, previousWidth)
  ));
  const nextRows = [];
  for (let y = 0; y < nextDepth; y += 1) {
    const chars = [];
    for (let x = 0; x < nextWidth; x += 1) {
      const source = mapCell(x, y, previousWidth, previousDepth);
      const ch = source
        && source.x >= 0
        && source.x < previousWidth
        && source.y >= 0
        && source.y < previousDepth
        ? sourceRows[source.y][source.x]
        : empty;
      chars.push(ch || empty);
    }
    nextRows.push(chars.join(""));
  }
  return nextRows;
}

function rotateLevel3dLayerLeft() {
  return transformLevel3dLayerCells({
    nextWidth: level3d.depth,
    nextDepth: level3d.width,
    mapCell: (x, y, width) => ({ x: width - 1 - y, y: x }),
    message: "Rotated left",
  });
}

function rotateLevel3dLayerRight() {
  return transformLevel3dLayerCells({
    nextWidth: level3d.depth,
    nextDepth: level3d.width,
    mapCell: (x, y, _width, height) => ({ x: y, y: height - 1 - x }),
    message: "Rotated right",
  });
}

function flipLevel3dLayerHorizontal() {
  return transformLevel3dLayerCells({
    nextWidth: level3d.width,
    nextDepth: level3d.depth,
    mapCell: (x, y, width) => ({ x: width - 1 - x, y }),
    message: "Flipped horizontal",
  });
}

function flipLevel3dLayerVertical() {
  return transformLevel3dLayerCells({
    nextWidth: level3d.width,
    nextDepth: level3d.depth,
    mapCell: (x, y, _width, height) => ({ x, y: height - 1 - y }),
    message: "Flipped vertical",
  });
}

function paintLevel3dCellAtPosition(position, ch = level3d.selectedChar) {
  if (level3dPlaytestActive) {
    return false;
  }
  const exportData = previewBuild?.exportData;
  const x = Math.trunc(Number(position?.x));
  const y = Math.trunc(Number(position?.y));
  const z = Math.trunc(Number(position?.z));
  if (!level3dPositionInBounds({ x, y, z })) {
    return false;
  }
  const sliceIndex = z;
  const row = y;
  const slice = level3d.slices[sliceIndex];
  if (!slice) {
    return false;
  }
  const paintChar = String(ch || level3dEmptyChar()).charAt(0);
  const current = String(slice[row] || "").padEnd(level3d.width, level3dEmptyChar()).slice(0, level3d.width);
  const nextChar = level3dLayerMergedChar(current[x], paintChar, exportData);
  if (!nextChar) {
    setLevel3dActionStatus("No available legend char for merged layer tile", "is-error");
    return false;
  }
  if (current[x] === nextChar) {
    return false;
  }
  slice[row] = `${current.slice(0, x)}${nextChar}${current.slice(x + 1)}`;
  renderLevel3dSourcePreview();
  renderLevel3dLayerBoard();
  renderLevel3dRuntime();
  return true;
}

function level3dLayerMergedChar(currentChar, paintChar, exportData = previewBuild?.exportData) {
  const nextChar = String(paintChar || level3dEmptyChar()).charAt(0);
  const emptyChar = level3dEmptyChar();
  const paintEntry = level3d.palette.find((entry) => entry.char === nextChar);
  const currentEntry = level3d.palette.find((entry) => entry.char === currentChar)
    || level3d.palette.find((entry) => entry.char === emptyChar)
    || { objects: [] };
  const mergedObjects = [
    ...(currentEntry.objects || []).filter((name) => !level3dObjectNameIsVisible(name, exportData)),
    ...((paintEntry?.objects || []).filter((name) => level3dObjectNameIsVisible(name, exportData))),
  ];
  return level3dEnsureCharForObjectNames(mergedObjects);
}

function level3dVisibleObjectKeyForChar(ch, exportData = previewBuild?.exportData) {
  const entry = level3d.palette.find((candidate) => candidate.char === ch)
    || level3d.palette.find((candidate) => candidate.char === level3dEmptyChar())
    || { objects: [] };
  return level3dObjectSetKey((entry.objects || [])
    .filter((name) => level3dObjectNameIsVisible(name, exportData)));
}

function level3dObjectNameIsVisible(name, exportData = previewBuild?.exportData) {
  return level3dLayerIsVisible(level3dObjectLayer(name, exportData), exportData);
}

function level3dObjectLayer(name, exportData = previewBuild?.exportData) {
  const layer = Number(level3dObjectDescriptor(name, exportData)?.layer);
  return Number.isFinite(layer) ? layer : null;
}

function level3dCharForObjectNames(objects) {
  const key = level3dObjectSetKey((objects || []).filter(Boolean));
  return level3d.palette.find((entry) => level3dObjectSetKey(entry.objects || []) === key)?.char || null;
}

function level3dEnsureCharForObjectNames(objects) {
  const cleanObjects = (objects || []).filter(Boolean);
  const existing = level3dCharForObjectNames(cleanObjects);
  if (existing) {
    return existing;
  }
  const ch = nextTemporaryLevel3dLegendChar();
  if (!ch) {
    return null;
  }
  level3d.palette.push({ char: ch, objects: [...cleanObjects], temporary: true });
  renderLevel3dPalette();
  renderLevel3dLayerPalette();
  return ch;
}

function nextTemporaryLevel3dLegendChar(extraUsed = []) {
  const used = new Set([
    LEVEL3D_EMPTY_CHAR,
    ...level3d.palette.map((entry) => entry.char),
    ...extraUsed,
  ]);
  return [...LEVEL3D_LEGEND_CHAR_CANDIDATES].find((candidate) => !used.has(candidate)) || "";
}

function level3dPositionInBounds(position) {
  return Number.isInteger(position.x)
    && Number.isInteger(position.y)
    && Number.isInteger(position.z)
    && position.x >= 0
    && position.y >= 0
    && position.z >= 0
    && position.x < Math.max(1, level3d.width || 1)
    && position.y < Math.max(1, level3d.depth || 1)
    && position.z < Math.max(1, level3d.height || 1);
}

function level3dTemporaryLegendEntriesForLevelData(levelData) {
  const { rows } = normalizeLevel3dSourceData(levelData);
  const used = level3dUsedCharsInRows(rows);
  const seenChars = new Set();
  const entries = [];
  for (const entry of level3d.palette || []) {
    const ch = String(entry.char || "").charAt(0);
    if (!entry.temporary || !ch || !used.has(ch) || seenChars.has(ch)) {
      continue;
    }
    entries.push({ char: ch, objects: Array.isArray(entry.objects) ? entry.objects.filter(Boolean) : [] });
    seenChars.add(ch);
  }
  return entries;
}

function level3dUsedCharsInRows(rows) {
  const used = new Set();
  for (const row of rows || []) {
    for (const ch of String(row || "")) {
      used.add(ch);
    }
  }
  return used;
}

function sanitizeLevel3dName(value) {
  return sourcePuzzleLevelName(value, "level 1");
}

function sanitizeLevel3dBundle(value) {
  const cleaned = String(value || "").trim().replace(/[^\w:.]/g, "_").replace(/^_+/, "");
  return cleaned || currentLevel3dBundleName();
}

function currentLevel3dSourceLocation() {
  const document = level3dSourceDocument();
  const source = level3dEditorSource(document);
  const entry = currentLevel3dSourceDefinition(source);
  return entry
    ? { document, start: entry.start, key: `${entry.bundle}:${entry.name}` }
    : null;
}

function loadLevel3dFromSourcePosition(position, options = {}) {
  if (!isPuzzleDocument(activeDocument()) || !isTextDocument(activeDocument())) {
    return null;
  }
  const source = sourceEditorDocumentValue();
  const safePosition = Math.max(0, Math.trunc(Number(position) || 0));
  const entry = level3dSourceEntries(source).find((candidate) => (
    safePosition >= candidate.start && safePosition <= candidate.end
  ));
  if (!entry) {
    return null;
  }
  return loadLevel3dSourceDefinition(entry, source, options);
}

function loadLevel3dSourceTarget(target, options = {}) {
  if (!isPuzzleDocument(activeDocument()) || !isTextDocument(activeDocument())) {
    return null;
  }
  const source = sourceEditorDocumentValue();
  const entry = Number.isInteger(target?.start)
    ? level3dSourceEntries(source).find((candidate) => candidate.start === target.start)
    : null;
  if (!entry) {
    return null;
  }
  return loadLevel3dSourceDefinition(entry, source, options);
}

function loadLevel3dSourceDefinition(entry, source, options = {}) {
  if (level3dPlaytestActive) {
    stopLevel3dPlaytest({ syncPreview: false });
  }
  const sourceDocument = options.document || activeDocument();
  const exportData = options.exportData || previewBuild?.exportData;
  const levels = Array.isArray(exportData?.levels) ? exportData.levels : [];
  let levelIndex = Number.isInteger(entry.levelIndex) ? entry.levelIndex : -1;
  const byName = levels.findIndex((levelEntry) => levelEntry?.name === entry.name);
  if (byName >= 0) {
    levelIndex = byName;
  }
  if (options.recordHistory && typeof pushSourceNavigationHistory === "function") {
    pushSourceNavigationHistory();
  }
  if (Number.isInteger(levelIndex) && levelIndex >= 0) {
    setActiveLevelIndex(levelIndex, exportData);
  }
  if (level3dNameInput) {
    level3dNameInput.value = entry.name || "level 1";
    delete level3dNameInput.dataset.userEdited;
  }
  if (level3dBundleInput) {
    level3dBundleInput.value = entry.bundle || currentLevel3dBundleName(exportData);
    delete level3dBundleInput.dataset.userEdited;
  }
  syncLevel3dNameOptions();
  const sourceKey = currentLevel3dEditorSourceKey(entry, sourceDocument, source);
  if (entry.rows?.length) {
    loadLevel3dFromSourceDefinition(entry, source, sourceKey, sourceDocument);
  } else if (levels[levelIndex]) {
    loadLevel3dFromEntry(levels[levelIndex], source, exportData, sourceKey, sourceDocument);
  } else {
    loadLevel3dFromSourceDefinition(entry, source, sourceKey, sourceDocument);
  }
  if (options.switchMode !== false && currentPreviewMode !== "level3d") {
    setPreviewMode("level3d");
  } else if (options.render !== false) {
    renderLevel3dBuilder();
  }
  if (!options.silent) {
    setLevel3dActionStatus(`Loaded 3D level ${entry.name}`, "is-ok");
    setStatus(`Loaded 3D level ${entry.name}`, "is-ok");
  }
  return `level3d:${entry.bundle || ""}:${entry.name}:${entry.start}`;
}

function renderLevel3dRuntime() {
  const exportData = previewBuild?.exportData;
  if (!previewBuild?.html || !exportData || !level3d.slices.length) {
    setLevel3dActionStatus(previewBuild?.html ? "Load a 3D level first" : "Run Preview first", "");
    return;
  }
  const layer = level3dViewMode === "layer";
  const surfaceId = layer ? "level-authoring-layer" : "level-authoring-stage";
  const host = layer
    ? level3dLayerBoard
    : document.querySelector("#level3dStageRuntimeMount");
  const selected = level3dSelectedEntry();
  const resize = level3dStageResizeMode();
  const interaction = level3dPlaytestActive
    ? { kind: "play" }
    : resize
      ? { kind: "resize", mode: resize }
      : {
        kind: "paint",
        operation: selected?.objects?.length
          ? (level3dEditMode() === "add" ? "add" : "replace")
          : "erase",
        };
  const camera = layer
    ? level3dLayerCamera()
    : level3dPreviewCamera(level3dRuntimeSnapshot());
  const origin = level3dPreviewOriginState();
  const cells = [];
  for (let z = 0; z < level3d.slices.length; z += 1) {
    for (let y = 0; y < level3d.depth; y += 1) {
      const row = String(level3d.slices[z]?.[y] || "")
        .padEnd(level3d.width, level3dEmptyChar())
        .slice(0, level3d.width);
      for (let x = 0; x < level3d.width; x += 1) {
        const entry = level3d.palette.find((candidate) => candidate.char === row[x]);
        if (!entry || !Array.isArray(entry.objectIds)) {
          setLevel3dActionStatus(
            `3D level cell (${x}, ${y}, ${z}) has no typed legend identity for ${JSON.stringify(row[x])}.`,
            "is-error",
          );
          return;
        }
        const objectIds = [...entry.objectIds];
        cells.push({ position: { x, y, z }, objectIds });
      }
    }
  }
  const payload = {
    model: editorModelName(exportData),
    levelIndex: currentEditableLevelIndex(exportData),
    draft: {
      kind: "grid3d",
      level: {
        size: { width: level3d.width, depth: level3d.depth, height: level3d.height },
        cells,
      },
    },
    presentation: {
      surface: { surfaceId, interaction },
      renderer: level3dEditorRendererStrategy(exportData, {
        camera,
        origin,
        sliceZ: layer ? currentLevel3dLayerZ() : null,
      }),
    },
  };
  queueEditorRuntimeDisplay({
    host,
    role: "authoring",
    surfaceId,
    kind: "hydrateDraft",
    payload,
    key: `draft:${surfaceId}:${JSON.stringify(payload)}`,
    onError: (error) => setLevel3dActionStatus(
      `3D level display failed: ${userFacingRuntimeError(error)}`,
      "is-error",
    ),
  });
}

function sendLevel3dSnapshotToRuntime() {
  renderLevel3dRuntime();
}

async function startLevel3dPlaytest() {
  if (level3dPlaytestActive) {
    return;
  }
  const exportData = await ensurePreviewExportForLevelAction({
    status: setLevel3dActionStatus,
    noDocumentMessage: "No 3D level to play",
    compilingMessage: "Compiling preview for play",
    failureMessage: "Preview compile failed",
  });
  if (!exportData) {
    return;
  }
  if (!previewBuild?.html || !level3d.slices.length) {
    setLevel3dActionStatus("Load a 3D level first", "is-error");
    return;
  }
  if (typeof clearSolutionPreview === "function") {
    clearSolutionPreview();
  }
  level3dPlaytestActive = true;
  setLevel3dViewMode("stage");
  updateLevel3dPlaytestControls();
  renderLevel3dRuntime();
  focusLevel3dPlaytestTarget();
  requestAnimationFrame(focusLevel3dPlaytestTarget);
  setLevel3dActionStatus("Playing 3D level", "is-ok");
}

function stopLevel3dPlaytest(options = {}) {
  if (!level3dPlaytestActive) {
    updateLevel3dPlaytestControls();
    return;
  }
  level3dPlaytestActive = false;
  updateLevel3dPlaytestControls();
  if (options.syncPreview !== false) {
    renderLevel3dRuntime();
  }
  setLevel3dActionStatus("Stopped 3D level play", "");
}

function toggleLevel3dPlaytest() {
  if (level3dPlaytestActive) {
    stopLevel3dPlaytest();
  } else {
    startLevel3dPlaytest().catch((error) => {
      setLevel3dActionStatus(`Play failed: ${userFacingRuntimeError(error)}`, "is-error");
    });
  }
}

function updateLevel3dPlaytestControls() {
  if (!level3dBuilder) {
    return;
  }
  level3dBuilder.classList.toggle("is-playtesting", level3dPlaytestActive);
  if (level3dPlaytestButton) {
    const label = level3dPlaytestActive ? "Stop" : "Play";
    const title = level3dPlaytestActive ? "Stop" : "Play";
    level3dPlaytestButton.classList.toggle("is-playing", level3dPlaytestActive);
    level3dPlaytestButton.setAttribute("aria-label", label);
    level3dPlaytestButton.title = title;
  }
  for (const element of [
    level3dBundleInput,
    level3dNameInput,
    level3dWidthInput,
    level3dDepthInput,
    level3dHeightInput,
    copyLevel3dButton,
    addLevel3dButton,
    updateLevel3dButton,
    level3dCameraYawScrub,
    level3dCameraPitchScrub,
    level3dCameraRollScrub,
    level3dCameraZoomScrub,
    level3dOriginXScrub,
    level3dOriginYScrub,
    level3dOriginZScrub,
    level3dResetPreviewButton,
    level3dLayerViewButton,
  ]) {
    if (element) {
      element.disabled = level3dPlaytestActive;
    }
  }
  level3dPalette?.querySelectorAll("button").forEach((button) => {
    button.disabled = level3dPlaytestActive;
  });
  level3dLayerPalette?.querySelectorAll("button").forEach((button) => {
    button.disabled = level3dPlaytestActive;
  });
}

function focusLevel3dPlaytestTarget() {
  editorRuntimeControllers.get("level-authoring-stage")?.surface.focus?.({ preventScroll: true });
}

function level3dPlaytestFrameWindow() {
  const controller = editorRuntimeControllers.get("level-authoring-stage");
  return controller?.ready ? controller.frame?.contentWindow || null : null;
}

function sendLevel3dPlaytestKey(event) {
  if (!level3dPlaytestActive) {
    return false;
  }
  const controller = editorRuntimeControllers.get("level-authoring-stage");
  if (!controller?.ready || !controller.frame?.contentWindow) {
    setLevel3dActionStatus("3D play runtime is not ready", "is-error");
    return false;
  }
  postEditorPreviewCommand("syntheticKey", {
    key: event.key,
    code: event.code,
    repeat: event.repeat,
    altKey: event.altKey,
    ctrlKey: event.ctrlKey,
    metaKey: event.metaKey,
    shiftKey: event.shiftKey,
    trace: false,
  }, controller.frame);
  event.preventDefault();
  event.stopPropagation();
  return true;
}

function level3dRuntimePreviewResources(exportData = previewBuild?.exportData) {
  const authoring = exportData?.puzzle3AuthoringResources
    || exportData?.resources
    || (exportData?.objects || exportData?.visuals ? exportData : null)
    || previewBuild?.exportData
    || {};
  return {
    layerCount: authoring.layerCount,
    objects: authoring.objects || {},
    visuals: level3dPreviewVisuals(authoring),
  };
}

function level3dPreviewVisuals(exportData = previewBuild?.exportData) {
  return exportData?.visuals || {};
}

function level3dLayerRuntimeSnapshot() {
  const snapshot = level3dRuntimeSnapshot();
  if (!snapshot) {
    return null;
  }
  const z = currentLevel3dLayerZ();
  const size = {
    width: Math.max(1, Math.trunc(Number(level3d.width || snapshot.size?.width) || 1)),
    depth: Math.max(1, Math.trunc(Number(level3d.depth || snapshot.size?.depth) || 1)),
    height: 1,
  };
  const cells = (snapshot.cells || [])
    .filter((cell) => Math.trunc(Number(cell.position?.z) || 0) === z)
    .map((cell) => ({
      ...JSON.parse(JSON.stringify(cell)),
      position: {
        x: Math.trunc(Number(cell.position?.x) || 0),
        y: Math.trunc(Number(cell.position?.y) || 0),
        z: 0,
      },
      objects: (cell.objects || []).filter((object) => level3dObjectIsVisible(object, snapshot)),
    }));
  const next = JSON.parse(JSON.stringify(snapshot));
  next.render = level3dLayerSettings(snapshot.render || {});
  next.render.camera = level3dLayerCamera();
  next.size = size;
  next.cells = cells;
  const levelIndex = Math.max(0, Math.min((next.levels || []).length - 1, Math.trunc(Number(next.levelIndex) || 0)));
  next.levelIndex = levelIndex;
  if (Array.isArray(next.levels) && next.levels[levelIndex]) {
    next.levels[levelIndex].size = { ...size };
    next.levels[levelIndex].cells = cells;
  } else {
    next.levels = [{
      name: level3dNameInput?.value || "layer",
      label: level3dNameInput?.value || "Layer",
      size: { ...size },
      cells,
    }];
    next.levelIndex = 0;
  }
  return next;
}

function level3dObjectIsVisible(object, exportData = previewBuild?.exportData) {
  const layer = Number.isInteger(Number(object?.layer))
    ? Math.trunc(Number(object.layer))
    : level3dObjectLayer(object?.name || object?.visual || "", exportData);
  return level3dLayerIsVisible(layer, exportData);
}

function level3dLayerCamera() {
  return { yawDegrees: 0, pitchDegrees: 90, rollDegrees: 0, zoom: 1, projection: "orthographic" };
}

function level3dLayerSettings(settings = {}) {
  return {
    ...settings,
    interactiveLook: false,
    interactiveZoom: false,
    grid: {
      visibility: 0.42,
      occupiedCells: false,
      stageFrame: true,
      xyPlane: true,
      color: "rgba(31, 36, 40, 0.36)",
      frameColor: "rgba(29, 37, 44, 0.72)",
    },
    fitContent: { enabled: true, mode: "xy", margin: 18 },
  };
}

function scheduleLevel3dSurfaceResize() {
  if (level3dSurfaceResizeFrame) {
    return;
  }
  level3dSurfaceResizeFrame = requestAnimationFrame(() => {
    level3dSurfaceResizeFrame = 0;
    syncLevel3dFrameLayout();
    if (!level3dBuilder?.hidden) {
      renderLevel3dRuntime();
    }
  });
}

function level3dSelectedEntry() {
  return level3dVisiblePaletteEntries().find((entry) => entry.char === level3d.selectedChar)
    || level3dVisiblePaletteEntries().find((entry) => entry.objects.length > 0)
    || level3dVisiblePaletteEntries()[0]
    || { char: level3dEmptyChar(), objects: [] };
}

function level3dRuntimeSnapshot() {
  const exportData = previewBuild?.exportData;
  if (!isPuzzle3dExport(exportData)) {
    return null;
  }
  const levelIndex = currentEditableLevelIndex(exportData);
  const levelEntry = exportData.levels?.[levelIndex];
  if (!levelEntry) {
    return JSON.parse(JSON.stringify(exportData));
  }
  const snapshot = JSON.parse(JSON.stringify(exportData));
  const edited = level3dEditedSnapshotAppliesToLevel(exportData, levelIndex)
    ? level3dSnapshotLevelData(snapshot)
    : null;
  snapshot.levelIndex = levelIndex;
  if (edited) {
    snapshot.levels[levelIndex].size = edited.size;
    snapshot.levels[levelIndex].cells = edited.cells;
    snapshot.size = { ...edited.size };
    snapshot.cells = edited.cells;
  } else {
    snapshot.size = { ...(levelEntry.size || snapshot.size || {}) };
    snapshot.cells = Array.isArray(levelEntry.cells) ? JSON.parse(JSON.stringify(levelEntry.cells)) : [];
  }
  return level3dSnapshotWithPreviewGrid(snapshot);
}

function level3dEditedSnapshotAppliesToLevel(exportData = previewBuild?.exportData, levelIndex = currentEditableLevelIndex(exportData)) {
  if (!level3d.slices.length || !exportData) {
    return false;
  }
  const levelEntry = Array.isArray(exportData?.levels) ? exportData.levels[levelIndex] : null;
  if (!levelEntry) {
    return false;
  }
  const document = level3dSourceDocument();
  if (level3d.sourceDocumentId && document?.id && level3d.sourceDocumentId !== document.id) {
    return false;
  }
  const source = level3dEditorSource(document);
  const expectedKey = currentLevel3dEditorSourceKey(levelEntry, document, source);
  if (level3d.sourceKey && expectedKey && level3d.sourceKey === expectedKey) {
    return true;
  }
  return Boolean(levelEntry.name && level3dNameInput?.value && levelEntry.name === level3dNameInput.value);
}

function level3dSnapshotWithPreviewGrid(snapshot) {
  if (!snapshot) {
    return snapshot;
  }
  const next = JSON.parse(JSON.stringify(snapshot));
  next.render = level3dPreviewSettings(next.render || {});
  return next;
}

function level3dPreviewSettings(settings = {}) {
  const next = {
    ...settings,
    fitContent: { enabled: true, mode: "stage", margin: 18 },
  };
  if (level3d.previewFrames) {
    next.grid = {
      ...(typeof settings.grid === "object" && settings.grid ? settings.grid : {}),
      visibility: 1,
      occupiedCells: true,
      stageFrame: true,
    };
  } else if (settings.grid && typeof settings.grid === "object") {
    next.grid = {
      ...settings.grid,
      visibility: 0,
      occupiedCells: false,
      stageFrame: false,
    };
  } else {
    next.grid = { visibility: 0 };
  }
  return next;
}

function level3dPreviewGridSettings(snapshot) {
  const raw = snapshot?.render?.grid;
  if (!raw || raw === false || raw === true) {
    return { visibility: 0 };
  }
  return {
    visibility: level3dGridVisibility(raw),
    color: raw.color,
    frameColor: raw.frameColor || raw.frame_color,
    occupiedCells: raw.occupied_cells !== false && raw.occupiedCells !== false,
    stageFrame: Boolean(raw.stageFrame ?? raw.stage_frame ?? raw.frame),
  };
}

function level3dGridVisibility(raw) {
  return level3dClampNumber(raw.visibility, 0, 1);
}

function level3dSnapshotLevelData(exportData = previewBuild?.exportData) {
  if (!level3d.slices.length || !exportData) {
    return null;
  }
  const cells = [];
  for (let slice = 0; slice < level3d.slices.length; slice += 1) {
    const rows = level3d.slices[slice] || [];
    for (let row = 0; row < level3d.depth; row += 1) {
      const text = String(rows[row] || "").padEnd(level3d.width, level3dEmptyChar()).slice(0, level3d.width);
      for (let x = 0; x < level3d.width; x += 1) {
        const objects = level3dObjectsForChar(text[x], exportData);
        if (!objects.length) {
          continue;
        }
        cells.push({
          position: {
            x,
            y: row,
            z: slice,
          },
          objects,
        });
      }
    }
  }
  return {
    size: { width: level3d.width, depth: level3d.depth, height: level3d.height },
    cells,
  };
}

function level3dObjectsForChar(ch, exportData = previewBuild?.exportData) {
  const entry = level3d.palette.find((candidate) => candidate.char === ch);
  if (!entry?.objects?.length) {
    return [];
  }
  return entry.objects.map((name) => level3dObjectDescriptor(name, exportData)).filter(Boolean);
}

function level3dPaletteObjectDescriptor(
  name,
  exportData = previewBuild?.exportData,
  visuals = level3dRuntimePreviewResources(exportData).visuals,
) {
  const object = level3dObjectDescriptor(name, exportData);
  if (!object) {
    return null;
  }
  return level3dObjectHasPreviewVisual(object, exportData, visuals) ? object : null;
}

function level3dObjectHasPreviewVisual(
  object,
  exportData = previewBuild?.exportData,
  visuals = level3dRuntimePreviewResources(exportData).visuals,
) {
  return Boolean(object && (
    visuals?.[object.visual]
    || visuals?.[object.name]
  ));
}

function level3dObjectDescriptor(name, exportData = previewBuild?.exportData) {
  const resources = level3dRuntimePreviewResources(exportData);
  const fromObjects = resources.objects?.[name];
  if (fromObjects) {
    return { ...fromObjects };
  }
  for (const level of exportData?.levels || []) {
    for (const cell of level.cells || []) {
      const object = (cell.objects || []).find((candidate) => candidate.name === name || candidate.visual === name);
      if (object) {
        return { ...object };
      }
    }
  }
  return { name, visual: name };
}

function level3dPlacementFace(side, position, view, metadata, zOffset = 0) {
  const corners = level3dCellFaceCorners(side, {
    x: Number(position.x),
    y: Number(position.y),
    z: Number(position.z) + zOffset,
  });
  const projected = corners.map((corner) => level3dProjectPoint(corner, view));
  return {
    ...metadata,
    polygon: projected.map(({ x, y }) => ({ x, y })),
    center: projected.reduce(
      (total, point) => ({ x: total.x + point.x / projected.length, y: total.y + point.y / projected.length }),
      { x: 0, y: 0 },
    ),
    depth: projected.reduce((total, point) => total + point.depth, 0) / projected.length,
    order: level3dGridOrder(level3dFaceCenter(corners), view.camera),
  };
}

function level3dCellFaceCorners(side, position) {
  const x0 = position.x - 0.5;
  const x1 = position.x + 0.5;
  const y0 = position.y - 0.5;
  const y1 = position.y + 0.5;
  const z0 = position.z - 0.5;
  const z1 = position.z + 0.5;
  if (side === "xNeg") {
    return [{ x: x0, y: y0, z: z0 }, { x: x0, y: y0, z: z1 }, { x: x0, y: y1, z: z1 }, { x: x0, y: y1, z: z0 }];
  }
  if (side === "xPos") {
    return [{ x: x1, y: y0, z: z1 }, { x: x1, y: y0, z: z0 }, { x: x1, y: y1, z: z0 }, { x: x1, y: y1, z: z1 }];
  }
  if (side === "yNeg") {
    return [{ x: x0, y: y0, z: z0 }, { x: x1, y: y0, z: z0 }, { x: x1, y: y0, z: z1 }, { x: x0, y: y0, z: z1 }];
  }
  if (side === "yPos") {
    return [{ x: x0, y: y1, z: z1 }, { x: x1, y: y1, z: z1 }, { x: x1, y: y1, z: z0 }, { x: x0, y: y1, z: z0 }];
  }
  if (side === "zPos") {
    return [{ x: x0, y: y0, z: z1 }, { x: x1, y: y0, z: z1 }, { x: x1, y: y1, z: z1 }, { x: x0, y: y1, z: z1 }];
  }
  return [{ x: x1, y: y0, z: z0 }, { x: x0, y: y0, z: z0 }, { x: x0, y: y1, z: z0 }, { x: x1, y: y1, z: z0 }];
}

function drawLevel3dCellsPreview(ctx, width, height, snapshot, cells, options = {}) {
  const view = level3dPreviewView(snapshot, width, height, options);
  const primitives = [];
  for (const cell of cells || []) {
    for (const object of cell.objects || []) {
      primitives.push(...level3dObjectPreviewFaces(cell.position, object, snapshot, view));
    }
  }
  if (options.fitContent) {
    fitLevel3dPreviewPrimitives(primitives, width, height, options);
  }
  primitives.sort((left, right) => level3dPrimitiveOrder(left) - level3dPrimitiveOrder(right));
  for (const primitive of primitives) {
    ctx.fillStyle = primitive.fill;
    drawLevel3dPolygonPath(ctx, primitive.points);
    ctx.fill();
  }
}

function fitLevel3dPreviewPrimitives(primitives, width, height, options = {}) {
  const points = primitives.flatMap((primitive) => primitive.points || []);
  if (!points.length) {
    return;
  }
  const minX = Math.min(...points.map((point) => point.x));
  const maxX = Math.max(...points.map((point) => point.x));
  const minY = Math.min(...points.map((point) => point.y));
  const maxY = Math.max(...points.map((point) => point.y));
  const contentWidth = Math.max(0.001, maxX - minX);
  const contentHeight = Math.max(0.001, maxY - minY);
  const margin = Math.max(0, Number(options.fitMargin) || 0);
  const availableWidth = Math.max(1, width - margin * 2);
  const availableHeight = Math.max(1, height - margin * 2);
  const scale = Math.min(availableWidth / contentWidth, availableHeight / contentHeight);
  const centerX = (minX + maxX) / 2;
  const centerY = (minY + maxY) / 2;
  const targetX = width / 2;
  const targetY = height / 2;
  for (const primitive of primitives) {
    primitive.points = (primitive.points || []).map((point) => ({
      x: targetX + (point.x - centerX) * scale,
      y: targetY + (point.y - centerY) * scale,
    }));
  }
}

function drawLevel3dPreviewGrid(ctx, snapshot, view) {
  const grid = level3dPreviewGridSettings(snapshot);
  if (!grid.visibility) {
    return;
  }
  const lines = [];
  if (grid.occupiedCells) {
    lines.push(...level3dOccupiedCellFrameLines(snapshot, view, grid));
  }
  if (grid.stageFrame) {
    lines.push(...level3dStageFrameLines(snapshot, view, grid));
  }
  lines.sort((left, right) => level3dPrimitiveOrder(left) - level3dPrimitiveOrder(right));
  ctx.save();
  ctx.lineCap = "round";
  ctx.lineJoin = "round";
  for (const line of lines) {
    ctx.lineWidth = line.width;
    ctx.strokeStyle = line.stroke;
    ctx.globalAlpha = line.alpha;
    ctx.beginPath();
    ctx.moveTo(line.from.x, line.from.y);
    ctx.lineTo(line.to.x, line.to.y);
    ctx.stroke();
  }
  ctx.restore();
}

function level3dStageFrameLines(snapshot, view, grid) {
  if (!window.Puzzle3VisualCore?.stageFrameEdges) {
    return [];
  }
  return Puzzle3VisualCore.stageFrameEdges(snapshot?.size || {}).map((edge) => {
    const line = level3dProjectedGridLine(edge.from, edge.to, "stageFrame", view, grid);
    line.renderPriority = 2;
    return line;
  });
}

function level3dOccupiedCellFrameLines(snapshot, view, grid) {
  const occupied = new Set((snapshot.cells || [])
    .filter((cell) => cell.objects?.length)
    .map((cell) => level3dVoxelKey(cell.position.x, cell.position.y, cell.position.z)));
  const edgeLines = new Map();
  for (const cell of snapshot.cells || []) {
    if (!cell.objects?.length) {
      continue;
    }
    for (const face of level3dOccupiedCellFaces(cell.position)) {
      if (level3dDirectionDepth(face.normal, view.camera) <= 0) {
        continue;
      }
      if (occupied.has(level3dVoxelKey(face.neighbor.x, face.neighbor.y, face.neighbor.z))) {
        continue;
      }
      const order = level3dGridOrder(level3dFaceCenter(face.corners), view.camera);
      for (let index = 0; index < face.corners.length; index += 1) {
        const from = face.corners[index];
        const to = face.corners[(index + 1) % face.corners.length];
        const key = level3dFrameEdgeKey(from, to);
        const line = level3dProjectedGridLine(from, to, "occupied", view, grid, order);
        const existing = edgeLines.get(key);
        if (existing && level3dPrimitiveOrder(existing) >= level3dPrimitiveOrder(line)) {
          continue;
        }
        edgeLines.set(key, line);
      }
    }
  }
  return [...edgeLines.values()];
}

function level3dOccupiedCellFaces(position) {
  const x = Number(position?.x) || 0;
  const y = Number(position?.y) || 0;
  const z = Number(position?.z) || 0;
  const x0 = x - 0.5;
  const x1 = x + 0.5;
  const y0 = y - 0.5;
  const y1 = y + 0.5;
  const z0 = z - 0.5;
  const z1 = z + 0.5;
  return [
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
}

function level3dProjectedGridLine(from, to, kind, view, grid, order = null) {
  const projectedFrom = level3dProjectPoint(from, view);
  const projectedTo = level3dProjectPoint(to, view);
  return {
    from: projectedFrom,
    to: projectedTo,
    order: order || level3dGridOrder(level3dMidpoint3(from, to), view.camera),
    depth: (projectedFrom.depth + projectedTo.depth) / 2,
    stroke: level3dGridStroke(kind, grid),
    alpha: grid.visibility,
    width: kind === "stageFrame" ? 1.6 : 1.5,
  };
}

function level3dGridStroke(kind, grid) {
  if (kind === "stageFrame") {
    return grid.frameColor || "rgba(29, 37, 44, 0.82)";
  }
  return grid.color || "rgba(31, 36, 40, 0.62)";
}

function level3dFrameEdgeKey(a, b) {
  const first = `${a.x},${a.y},${a.z}`;
  const second = `${b.x},${b.y},${b.z}`;
  return first < second ? `${first}|${second}` : `${second}|${first}`;
}

function level3dObjectPreviewFaces(position, object, snapshot, view) {
  const voxels = level3dObjectPreviewVoxels(position, object, snapshot);
  const occupied = new Set(voxels.map((voxel) => level3dVoxelGeometryKey(voxel.position, voxel.scale)));
  const faces = [];
  for (const voxel of voxels) {
    for (const face of level3dVoxelFaces(voxel)) {
      if (occupied.has(level3dVoxelGeometryKey({
        x: voxel.position.x + face.offset.x * voxel.scale,
        y: voxel.position.y + face.offset.y * voxel.scale,
        z: voxel.position.z + face.offset.z * voxel.scale,
      }, voxel.scale))) {
        continue;
      }
      const projected = face.corners.map((corner) => level3dProjectPoint(corner, view));
      faces.push({
        points: projected.map(({ x, y }) => ({ x, y })),
        depth: projected.reduce((total, point) => total + point.depth, 0) / projected.length,
        order: level3dGridOrder(level3dFaceCenter(face.corners), view.camera),
        fill: level3dShadeFill(voxel.fill, face.light),
      });
    }
  }
  return faces;
}

function level3dObjectPreviewVoxels(position, object, snapshot) {
  const visuals = level3dRuntimePreviewResources(snapshot).visuals;
  const visual = visuals?.[object.visual] || visuals?.[object.name];
  if (!visual) {
    return [];
  }
  const blocks = level3dVisualLayers(visual);
  const visualHeight = Math.max(1, blocks.length);
  const visualDepth = Math.max(1, ...blocks.map((rows) => rows.length));
  const visualWidth = Math.max(1, ...blocks.flatMap((rows) => rows.map((row) => row.length)));
  const scale = 1 / Math.max(visualWidth, visualDepth, visualHeight);
  const voxels = [];
  for (let z = 0; z < blocks.length; z += 1) {
    const rows = blocks[z] || [];
    for (let row = 0; row < rows.length; row += 1) {
      for (let column = 0; column < rows[row].length; column += 1) {
        const fill = visual.palette?.[rows[row][column]];
        if (!fill || level3dParseColor(fill)?.a <= 0) {
          continue;
        }
        const grid = {
          x: column,
          y: Math.max(0, visualDepth - 1 - row),
          z: Math.max(0, visualHeight - 1 - z),
        };
        const voxelPosition = {
          x: Number(position.x) + (grid.x + 0.5 - visualWidth / 2) * scale,
          y: Number(position.y) + (grid.y + 0.5 - visualDepth / 2) * scale,
          z: Number(position.z) + (grid.z + 0.5 - visualHeight / 2) * scale,
        };
        voxels.push({
          fill,
          scale,
          position: voxelPosition,
          bounds: level3dVoxelBounds(voxelPosition, scale),
        });
      }
    }
  }
  return voxels;
}

function level3dVoxelFaces(voxel) {
  const { x0, x1, y0, y1, z0, z1 } = voxel.bounds;
  return [
    { offset: { x: 0, y: 0, z: -1 }, light: -0.22, corners: [{ x: x1, y: y0, z: z0 }, { x: x0, y: y0, z: z0 }, { x: x0, y: y1, z: z0 }, { x: x1, y: y1, z: z0 }] },
    { offset: { x: 0, y: 0, z: 1 }, light: 0.10, corners: [{ x: x0, y: y0, z: z1 }, { x: x1, y: y0, z: z1 }, { x: x1, y: y1, z: z1 }, { x: x0, y: y1, z: z1 }] },
    { offset: { x: -1, y: 0, z: 0 }, light: -0.08, corners: [{ x: x0, y: y0, z: z0 }, { x: x0, y: y0, z: z1 }, { x: x0, y: y1, z: z1 }, { x: x0, y: y1, z: z0 }] },
    { offset: { x: 1, y: 0, z: 0 }, light: 0.02, corners: [{ x: x1, y: y0, z: z1 }, { x: x1, y: y0, z: z0 }, { x: x1, y: y1, z: z0 }, { x: x1, y: y1, z: z1 }] },
    { offset: { x: 0, y: 1, z: 0 }, light: -0.04, corners: [{ x: x0, y: y1, z: z1 }, { x: x1, y: y1, z: z1 }, { x: x1, y: y1, z: z0 }, { x: x0, y: y1, z: z0 }] },
    { offset: { x: 0, y: -1, z: 0 }, light: -0.16, corners: [{ x: x0, y: y0, z: z0 }, { x: x1, y: y0, z: z0 }, { x: x1, y: y0, z: z1 }, { x: x0, y: y0, z: z1 }] },
  ];
}

function level3dPreviewView(snapshot, width, height, options = {}) {
  const camera = options.camera || level3dPreviewCamera(snapshot);
  const size = snapshot?.size || { width: 1, depth: 1, height: 1 };
  const bounds = level3dProjectedBoundsUnit(size, camera);
  const boundsWidth = Math.max(0.001, bounds.maxX - bounds.minX);
  const boundsHeight = Math.max(0.001, bounds.maxY - bounds.minY);
  const padding = Number(options.padding) || 0.72;
  const scale = Math.min(width / boundsWidth, height / boundsHeight) * padding * camera.zoom;
  const previewOrigin = options.origin || level3dPreviewOriginState();
  return {
    camera,
    center: {
      x: Number(previewOrigin.x) || 0,
      y: Number(previewOrigin.y) || 0,
      z: Number(previewOrigin.z) || 0,
    },
    origin: {
      x: width / 2,
      y: height / 2,
    },
    scale,
  };
}

function level3dPreviewCamera(source) {
  if (!level3dPreviewCameraState) {
    level3dPreviewCameraState = level3dBasePreviewCamera(source);
  }
  level3dPreviewCameraState.yawDegrees = level3dNormalizeDegrees(level3dPreviewCameraState.yawDegrees);
  level3dPreviewCameraState.pitchDegrees = level3dClampNumber(
    level3dPreviewCameraState.pitchDegrees,
    LEVEL3D_CAMERA_MIN_PITCH_DEGREES,
    LEVEL3D_CAMERA_MAX_PITCH_DEGREES,
  );
  level3dPreviewCameraState.rollDegrees = level3dNormalizeDegrees(level3dPreviewCameraState.rollDegrees ?? 0);
  level3dPreviewCameraState.zoom = level3dClampNumber(level3dPreviewCameraState.zoom, 0.25, 4);
  level3dPreviewCameraState.projection = level3dCameraProjection(level3dPreviewCameraState.projection);
  return level3dPreviewCameraState;
}

function level3dCameraProjection(value) {
  const projection = String(value || "").toLowerCase();
  if (projection !== "perspective" && projection !== "orthographic") {
    throw new Error("3D camera projection must be perspective or orthographic.");
  }
  return projection;
}

function level3dBasePreviewCamera(source) {
  const camera = source?.render?.camera || previewBuild?.exportData?.render?.camera || {};
  return {
    projection: level3dCameraProjection(camera.projection),
    yawDegrees: Number(camera.yawDegrees ?? 15),
    pitchDegrees: Number(camera.pitchDegrees ?? 55),
    rollDegrees: Number(camera.rollDegrees ?? 0),
    zoom: Number(camera.zoom ?? 1),
  };
}

function level3dPreviewOriginState() {
  level3dPreviewOrigin = {
    x: level3dClampNumber(level3dPreviewOrigin?.x, -128, 128),
    y: level3dClampNumber(level3dPreviewOrigin?.y, -128, 128),
    z: level3dClampNumber(level3dPreviewOrigin?.z, -128, 128),
  };
  return level3dPreviewOrigin;
}

function level3dEditorRendererStrategy(exportData = previewBuild?.exportData, options = {}) {
  const camera = options.camera || level3dPreviewCamera(exportData);
  const origin = options.origin || level3dPreviewOriginState();
  return {
    kind: "grid3d",
    sliceZ: Number.isInteger(options.sliceZ) ? options.sliceZ : null,
    hiddenLayers: [],
    camera: {
      projection: camera.projection,
      yawDegrees: camera.yawDegrees,
      pitchDegrees: camera.pitchDegrees,
      rollDegrees: camera.rollDegrees,
      zoom: camera.zoom,
    },
    view: { target: { x: origin.x, y: origin.y, z: origin.z } },
    settings: {
      gridVisible: level3d.previewFrames,
      occupiedCellFrames: level3d.previewFrames,
      stageFrame: level3d.previewFrames,
    },
  };
}

function level3dProjectedBoundsUnit(size, camera) {
  const width = Math.max(1, Number(size.width) || 1);
  const depth = Math.max(1, Number(size.depth) || 1);
  const height = Math.max(1, Number(size.height) || 1);
  const view = {
    camera,
    center: { x: (width - 1) / 2, y: (depth - 1) / 2, z: (height - 1) / 2 },
    origin: { x: 0, y: 0 },
    scale: 1,
  };
  const corners = [];
  for (const x of [-0.5, width - 0.5]) {
    for (const y of [-0.5, depth - 0.5]) {
      for (const z of [-0.55, height - 0.5]) {
        corners.push(level3dProjectPoint({ x, y, z }, view));
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

function level3dProjectPoint(position, view) {
  if (view?.threeProjection) {
    return level3dProjectThreeSurfacePoint(position, view);
  }
  if (level3dUsesRuntimeProjectionView(view)) {
    return level3dProjectRuntimeSurfacePoint(position, view);
  }
  if (!window.Puzzle3VisualCore?.projectOrthographic) {
    throw new Error("3D level projection requires Puzzle3VisualCore.projectOrthographic");
  }
  return Puzzle3VisualCore.projectOrthographic(position, view);
}

function level3dProjectThreeSurfacePoint(position, view) {
  const projected = level3dProjectThreeCanvasPoint(position, view);
  return level3dRuntimeCanvasPointToSurface(projected, view);
}

function level3dProjectThreeCanvasPoint(position, view) {
  const projection = view.threeProjection || {};
  const size = projection.size || { width: 1, depth: 1, height: 1 };
  const camera = view.camera || {};
  const target = projection.target || { x: 0, y: 0, z: 0 };
  const cameraFrame = level3dCameraRenderFrame(camera);
  const distance = Math.max(0.0001, Number(projection.distance) || 1);
  const cameraPosition = {
    x: target.x - cameraFrame.forward.x * distance,
    y: target.y - cameraFrame.forward.y * distance,
    z: target.z - cameraFrame.forward.z * distance,
  };
  const world = {
    x: Number(position.x) - (Math.max(1, Number(size.width) || 1) - 1) / 2,
    y: (Math.max(1, Number(size.height) || 1) - 1) / 2 - Number(position.z),
    z: Number(position.y) - (Math.max(1, Number(size.depth) || 1) - 1) / 2,
  };
  const relative = {
    x: world.x - cameraPosition.x,
    y: world.y - cameraPosition.y,
    z: world.z - cameraPosition.z,
  };
  const cameraX = level3dDotVector(relative, cameraFrame.right);
  const cameraY = level3dDotVector(relative, cameraFrame.up);
  const cameraDepth = Math.max(0.0001, level3dDotVector(relative, cameraFrame.forward));
  const width = Math.max(1, Number(view.width) || 1);
  const height = Math.max(1, Number(view.height) || 1);
  const aspect = Math.max(0.01, Number(projection.aspect) || width / height);
  let ndcX = 0;
  let ndcY = 0;
  if (projection.projection === "orthographic") {
    const visibleHeight = Math.max(0.0001, Number(projection.visibleHeight) || 1);
    const visibleWidth = visibleHeight * aspect;
    ndcX = cameraX / (visibleWidth / 2);
    ndcY = cameraY / (visibleHeight / 2);
  } else {
    const tanHalfFov = Math.tan(level3dDegreesToRadians(Number(projection.fovDegrees) || 34) / 2);
    ndcX = cameraX / (cameraDepth * tanHalfFov * aspect);
    ndcY = cameraY / (cameraDepth * tanHalfFov);
  }
  return {
    x: ((ndcX + 1) / 2) * width,
    y: ((1 - ndcY) / 2) * height,
    depth: cameraDepth,
  };
}

function level3dCameraRenderFrame(camera) {
  const yaw = level3dDegreesToRadians(camera.yawDegrees ?? 0);
  const pitch = level3dDegreesToRadians(camera.pitchDegrees ?? 35);
  const roll = level3dDegreesToRadians(camera.rollDegrees ?? 0);
  const horizontal = Math.cos(pitch);
  const baseRight = { x: Math.cos(yaw), y: 0, z: Math.sin(yaw) };
  const forward = {
    x: Math.sin(yaw) * horizontal,
    y: -Math.sin(pitch),
    z: -Math.cos(yaw) * horizontal,
  };
  const baseUp = level3dCrossVector(baseRight, forward);
  const cosRoll = Math.cos(roll);
  const sinRoll = Math.sin(roll);
  return {
    right: {
      x: baseRight.x * cosRoll + baseUp.x * sinRoll,
      y: baseRight.y * cosRoll + baseUp.y * sinRoll,
      z: baseRight.z * cosRoll + baseUp.z * sinRoll,
    },
    up: {
      x: -baseRight.x * sinRoll + baseUp.x * cosRoll,
      y: -baseRight.y * sinRoll + baseUp.y * cosRoll,
      z: -baseRight.z * sinRoll + baseUp.z * cosRoll,
    },
    forward,
  };
}

function level3dCrossVector(left, right) {
  return {
    x: left.y * right.z - left.z * right.y,
    y: left.z * right.x - left.x * right.z,
    z: left.x * right.y - left.y * right.x,
  };
}

function level3dDotVector(left, right) {
  return left.x * right.x + left.y * right.y + left.z * right.z;
}

function level3dUsesRuntimeProjectionView(view) {
  return view?.coordinateSpace === "canvas-css-px";
}

function level3dProjectRuntimeSurfacePoint(position, view) {
  const projected = level3dProjectRuntimeCanvasPoint(position, view);
  return level3dRuntimeCanvasPointToSurface(projected, view);
}

function level3dProjectRuntimeCanvasPoint(position, view) {
  const projectionView = {
    camera: {
      yawDegrees: Number(view.camera?.yawDegrees ?? 0),
      pitchDegrees: Number(view.camera?.pitchDegrees ?? 35),
      rollDegrees: Number(view.camera?.rollDegrees ?? 0),
      zoom: 1,
    },
    center: view.center || { x: 0, y: 0, z: 0 },
    origin: {
      x: Number(view.originX) || 0,
      y: Number(view.originY) || 0,
    },
    scale: Math.max(0.0001, Number(view.scale) || 1),
  };
  if (!window.Puzzle3VisualCore?.projectOrthographic) {
    throw new Error("3D runtime projection requires Puzzle3VisualCore.projectOrthographic");
  }
  return Puzzle3VisualCore.projectOrthographic(position, projectionView);
}

function level3dRuntimeCanvasPointToSurface(point, view) {
  const sourceWidth = Math.max(1, Number(view.width) || 1);
  const sourceHeight = Math.max(1, Number(view.height) || 1);
  const viewportWidth = Math.max(1, Number(view.viewport?.width) || Number(view.canvasRect?.width) || sourceWidth);
  const viewportHeight = Math.max(1, Number(view.viewport?.height) || Number(view.canvasRect?.height) || sourceHeight);
  const targetWidth = Math.max(1, Number(view.surfaceWidth) || viewportWidth);
  const targetHeight = Math.max(1, Number(view.surfaceHeight) || viewportHeight);
  const canvasRect = level3dRuntimeCanvasRect(view, viewportWidth, viewportHeight);
  const viewportPoint = {
    x: canvasRect.x + Number(point.x) * canvasRect.width / sourceWidth,
    y: canvasRect.y + Number(point.y) * canvasRect.height / sourceHeight,
  };
  return {
    x: viewportPoint.x * targetWidth / viewportWidth,
    y: viewportPoint.y * targetHeight / viewportHeight,
    depth: point.depth,
  };
}

function level3dRuntimeCanvasRect(view, viewportWidth, viewportHeight) {
  const rect = view?.canvasRect || {};
  const width = Math.max(1, Number(rect.width) || Number(view?.width) || viewportWidth || 1);
  const height = Math.max(1, Number(rect.height) || Number(view?.height) || viewportHeight || 1);
  return {
    x: Number.isFinite(Number(rect.x)) ? Number(rect.x) : 0,
    y: Number.isFinite(Number(rect.y)) ? Number(rect.y) : 0,
    width,
    height,
  };
}

function level3dDirectionDepth(vector, camera = level3dPreviewCamera()) {
  const yaw = level3dDegreesToRadians(camera.yawDegrees ?? 0);
  const pitch = level3dDegreesToRadians(camera.pitchDegrees ?? 35);
  const yawY = vector.x * Math.sin(yaw) + vector.y * Math.cos(yaw);
  return -yawY * Math.cos(pitch) + vector.z * Math.sin(pitch);
}

function level3dGridOrder(position, camera = level3dPreviewCamera()) {
  const yaw = level3dDegreesToRadians(camera.yawDegrees ?? 0);
  const pitch = level3dDegreesToRadians(camera.pitchDegrees ?? 35);
  return {
    x: level3dSignedAxis(-Math.sin(yaw) * Math.cos(pitch)) * position.x,
    y: level3dSignedAxis(-Math.cos(yaw) * Math.cos(pitch)) * position.y,
    z: level3dSignedAxis(Math.sin(pitch)) * position.z,
  };
}

function level3dPrimitiveOrder(primitive) {
  const order = primitive.order || { x: 0, y: 0, z: 0 };
  return order.x * 1000000 + order.y * 1000 + order.z + (primitive.depth || 0) * 0.001;
}

function level3dSignedAxis(value) {
  return Math.abs(value) < 0.000001 ? 0 : (value > 0 ? 1 : -1);
}

function level3dFaceCenter(corners) {
  return corners.reduce(
    (total, corner) => ({
      x: total.x + corner.x / corners.length,
      y: total.y + corner.y / corners.length,
      z: total.z + corner.z / corners.length,
    }),
    { x: 0, y: 0, z: 0 },
  );
}

function level3dMidpoint3(a, b) {
  return {
    x: (a.x + b.x) / 2,
    y: (a.y + b.y) / 2,
    z: (a.z + b.z) / 2,
  };
}

function pointInLevel3dPolygon(point, polygon) {
  let inside = false;
  for (let left = 0, right = polygon.length - 1; left < polygon.length; right = left, left += 1) {
    const a = polygon[left];
    const b = polygon[right];
    const crosses = ((a.y > point.y) !== (b.y > point.y))
      && point.x < ((b.x - a.x) * (point.y - a.y)) / ((b.y - a.y) || 1) + a.x;
    if (crosses) {
      inside = !inside;
    }
  }
  return inside;
}

function drawLevel3dPolygonPath(ctx, points) {
  ctx.beginPath();
  points.forEach((point, index) => {
    if (index === 0) {
      ctx.moveTo(point.x, point.y);
    } else {
      ctx.lineTo(point.x, point.y);
    }
  });
  ctx.closePath();
}

function level3dVisualLayers(visual, now = performance.now()) {
  const frames = Array.isArray(visual?.frames) ? visual.frames : [];
  if (!frames.length) {
    throw new Error("3D level editor visual frames are missing.");
  }
  const frameDuration = Number(visual.frameDurationMs)
    || (Number(visual.durationMs) > 0 ? Number(visual.durationMs) / frames.length : 0);
  const index = frames.length > 1 && frameDuration > 0
    ? Math.floor(now / frameDuration) % frames.length
    : 0;
  const layers = frames[index]?.layers;
  if (!Array.isArray(layers) || !layers.length) {
    throw new Error("3D level editor visual layers are missing.");
  }
  return layers;
}

function level3dVoxelBounds(position, scale) {
  return {
    x0: position.x - scale / 2,
    x1: position.x + scale / 2,
    y0: position.y - scale / 2,
    y1: position.y + scale / 2,
    z0: position.z - scale / 2,
    z1: position.z + scale / 2,
  };
}

function level3dVoxelGeometryKey(position, scale) {
  return [
    level3dQuantize(position.x),
    level3dQuantize(position.y),
    level3dQuantize(position.z),
    level3dQuantize(scale),
  ].join(",");
}

function level3dVoxelKey(x, y, z) {
  return `${x},${y},${z}`;
}

function level3dQuantize(value) {
  return String(Math.round(Number(value) * 1000000) / 1000000);
}

function level3dShadeFill(fill, light) {
  const color = level3dParseColor(fill);
  if (!color) {
    return fill;
  }
  return `rgba(${level3dLightenChannel(color.r, light)}, ${level3dLightenChannel(color.g, light)}, ${level3dLightenChannel(color.b, light)}, ${color.a})`;
}

function level3dLightenChannel(value, amount) {
  const next = amount >= 0
    ? value + (255 - value) * amount
    : value * (1 + amount);
  return Math.max(0, Math.min(255, Math.round(next)));
}

function level3dParseColor(fill) {
  if (typeof fill !== "string") {
    return null;
  }
  if (fill.startsWith("rgb(") || fill.startsWith("rgba(")) {
    const match = fill.match(/^rgba?\(([^)]+)\)$/);
    if (!match) {
      return null;
    }
    const channels = match[1].split(",").map((part) => Number(part.trim()));
    return channels.length >= 3 ? {
      r: channels[0],
      g: channels[1],
      b: channels[2],
      a: channels.length >= 4 ? channels[3] : 1,
    } : null;
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

function level3dDegreesToRadians(value) {
  return (Number(value) * Math.PI) / 180;
}

function level3dNormalizeDegrees(value) {
  const normalized = Number(value) % 360;
  return normalized < 0 ? normalized + 360 : normalized;
}

function level3dClampNumber(value, min, max) {
  const number = Number(value);
  if (!Number.isFinite(number)) {
    return min;
  }
  return Math.max(min, Math.min(max, number));
}

async function copyLevel3dToClipboard() {
  try {
    const levelName = sanitizeLevel3dName(level3dNameInput?.value || currentLevel3dEntry()?.name || "level 1");
    const source = level3dEditorSource();
    const levelData = level3dSourceData();
    const result = await levelSourceRequest(source, {
      operation: "format",
      name: levelName,
      rows: levelData.rows,
      localLegends: level3dLocalLegendDrafts(levelData),
    });
    await copyTextToClipboard(result.text);
    setLevel3dActionStatus("Copied 3D level", "is-ok");
  } catch (error) {
    setLevel3dActionStatus(`Could not copy 3D level: ${userFacingRuntimeError(error)}`, "is-error");
  }
}

async function addLevel3dToSource() {
  const sourceDocument = level3dSourceDocument();
  if (!sourceDocument) {
    setLevel3dActionStatus("No game entry for 3D level", "is-error");
    return;
  }
  let source;
  try {
    const levelName = sanitizeLevel3dName(level3dNameInput?.value || "level 1");
    const bundle = sanitizeLevel3dBundle(level3dBundleInput?.value || "");
    source = level3dEditorSource(sourceDocument);
    const levelData = level3dSourceData();
    const result = await levelSourceRequest(source, {
      operation: "insert",
      name: levelName,
      namespace: bundle,
      rows: levelData.rows,
      localLegends: level3dLocalLegendDrafts(levelData),
      cursor: currentLevel3dSourceDefinition(source)?.start ?? null,
      createContainer: false,
    });
    if (!applyPuzzleSourceMutation(sourceDocument, source, result.source)) {
      setLevel3dActionStatus("3D level source changed while the edit was being prepared; retry the edit.", "is-error");
      return;
    }
    recordLevel3dSourceMutation(sourceDocument);
    level3d.sourceTargetStart = null;
    level3dNameInput.value = nextLevelName(levelName);
    syncLevel3dNameOptions();
    setLevel3dActionStatus("Added 3D level", "is-ok");
  } catch (error) {
    setLevel3dActionStatus(`Could not add 3D level: ${userFacingRuntimeError(error)}`, "is-error");
  }
}

async function updateLevel3dInSource() {
  const sourceDocument = level3dSourceDocument();
  if (!sourceDocument) {
    setLevel3dActionStatus("No game entry for 3D level", "is-error");
    return;
  }
  let source;
  try {
    const levelName = sanitizeLevel3dName(level3dNameInput?.value || "level 1");
    source = level3dEditorSource(sourceDocument);
    const target = currentLevel3dSourceDefinition(source);
    if (!target) {
      setLevel3dActionStatus("No typed 3D level source target is selected", "is-error");
      return;
    }
    const levelData = level3dSourceData();
    const result = await levelSourceRequest(source, {
      operation: "update",
      targetStart: target.start,
      name: levelName,
      rows: levelData.rows,
      localLegends: level3dLocalLegendDrafts(levelData),
    });
    if (!applyPuzzleSourceMutation(sourceDocument, source, result.source)) {
      setLevel3dActionStatus("3D level source changed while the edit was being prepared; retry the edit.", "is-error");
      return;
    }
    recordLevel3dSourceMutation(sourceDocument);
    level3d.sourceTargetStart = result.start;
    setLevel3dActionStatus(`Updated 3D level ${levelName}`, "is-ok");
  } catch (error) {
    setLevel3dActionStatus(`Could not update 3D level: ${userFacingRuntimeError(error)}`, "is-error");
  }
}

function level3dLocalLegendDrafts(levelData) {
  const used = level3dUsedCharsInRows(levelData.rows);
  const bySymbol = new Map((level3d.sourceLocalLegends || []).map((entry) => [
    entry.symbol,
    { symbol: entry.symbol, selectors: [...(entry.selectors || [])] },
  ]));
  for (const entry of level3d.palette || []) {
    if (!entry.temporary || !used.has(entry.char)) continue;
    bySymbol.set(entry.char, { symbol: entry.char, selectors: [...(entry.objects || [])] });
  }
  return [...bySymbol.values()];
}

function recordLevel3dSourceMutation(sourceDocument) {
  level3d.sourceDocumentId = sourceDocument.id || level3d.sourceDocumentId || "";
  level3d.sourceKey = "";
}

function resetLevel3dPreviewView() {
  if (level3dPlaytestActive) {
    return;
  }
  resetLevel3dPreviewState(previewBuild?.exportData);
  renderLevel3dPreviewControls();
  renderLevel3dRuntime();
  setLevel3dActionStatus("Reset 3D preview view", "is-ok");
}

function resetLevel3dPreviewState(source = previewBuild?.exportData) {
  level3dPreviewCameraState = level3dBasePreviewCamera(source);
  level3dPreviewOrigin = level3dDefaultPreviewTarget(source);
}

function level3dDefaultPreviewTarget(source = previewBuild?.exportData) {
  const size = level3d.slices.length
    ? { width: level3d.width, depth: level3d.depth, height: level3d.height }
    : source?.size || level3dRuntimeSnapshot()?.size || {};
  return {
    x: (Math.max(1, Number(size.width) || 1) - 1) / 2,
    y: (Math.max(1, Number(size.depth) || 1) - 1) / 2,
    z: (Math.max(1, Number(size.height) || 1) - 1) / 2,
  };
}

function level3dPreviewScrubTarget(event) {
  return event.target?.closest?.("[data-level3d-preview]") || null;
}

function startLevel3dPreviewScrub(event) {
  if (level3dPlaytestActive) {
    return;
  }
  const target = level3dPreviewScrubTarget(event);
  if (!target || event.button !== 0) {
    return;
  }
  event.preventDefault();
  event.stopPropagation();
  const kind = target.dataset.level3dPreview;
  level3dPreviewScrubDrag = {
    pointerId: event.pointerId,
    target,
    kind,
    startY: event.clientY,
    value: level3dPreviewValue(kind),
  };
  target.setPointerCapture?.(event.pointerId);
  target.classList.add("is-dragging");
  document.documentElement.classList.add("is-visual3d-camera-scrubbing");
}

function continueLevel3dPreviewScrub(event) {
  if (!level3dPreviewScrubDrag || level3dPreviewScrubDrag.pointerId !== event.pointerId) {
    return;
  }
  event.preventDefault();
  event.stopPropagation();
  const delta = level3dPreviewScrubDrag.startY - event.clientY;
  setLevel3dPreviewValue(
    level3dPreviewScrubDrag.kind,
    level3dPreviewScrubDrag.value + delta * level3dPreviewScrubScale(level3dPreviewScrubDrag.kind),
    { status: false },
  );
}

function stopLevel3dPreviewScrub(event) {
  if (!level3dPreviewScrubDrag || level3dPreviewScrubDrag.pointerId !== event.pointerId) {
    return;
  }
  event.preventDefault();
  event.stopPropagation();
  finishLevel3dPreviewScrub(event.pointerId);
}

function finishLevel3dPreviewScrub(pointerId = null) {
  if (!level3dPreviewScrubDrag) {
    return;
  }
  const { target } = level3dPreviewScrubDrag;
  if (pointerId !== null && target.hasPointerCapture?.(pointerId)) {
    target.releasePointerCapture(pointerId);
  }
  target.classList.remove("is-dragging");
  document.documentElement.classList.remove("is-visual3d-camera-scrubbing");
  level3dPreviewScrubDrag = null;
}

function adjustLevel3dPreviewScrubWithKey(event) {
  if (level3dPlaytestActive) {
    return;
  }
  const target = level3dPreviewScrubTarget(event);
  if (!target || !["ArrowLeft", "ArrowDown", "ArrowRight", "ArrowUp"].includes(event.key)) {
    return;
  }
  event.preventDefault();
  const direction = event.key === "ArrowLeft" || event.key === "ArrowDown" ? -1 : 1;
  const kind = target.dataset.level3dPreview;
  const multiplier = event.shiftKey ? 10 : 1;
  setLevel3dPreviewValue(kind, level3dPreviewValue(kind) + direction * level3dPreviewKeyStep(kind) * multiplier);
}

function level3dPreviewValue(kind) {
  const camera = level3dPreviewCamera();
  const origin = level3dPreviewOriginState();
  if (kind === "width") {
    return Math.max(1, Math.trunc(Number(level3d.width) || 1));
  }
  if (kind === "depth") {
    return Math.max(1, Math.trunc(Number(level3d.depth) || 1));
  }
  if (kind === "height") {
    return Math.max(1, Math.trunc(Number(level3d.height) || 1));
  }
  if (kind === "yaw") {
    return camera.yawDegrees;
  }
  if (kind === "pitch") {
    return camera.pitchDegrees;
  }
  if (kind === "roll") {
    return camera.rollDegrees;
  }
  if (kind === "zoom") {
    return camera.zoom;
  }
  if (kind === "originX") {
    return origin.x;
  }
  if (kind === "originY") {
    return origin.y;
  }
  if (kind === "originZ") {
    return origin.z;
  }
  return 0;
}

function setLevel3dPreviewValue(kind, value, options = {}) {
  if (level3dPlaytestActive) {
    return;
  }
  if (kind === "width") {
    resizeLevel3dWidth(Math.round(value), { edge: "right", status: options.status });
    return;
  }
  if (kind === "depth") {
    resizeLevel3dDepth(Math.round(value), { edge: "back", status: options.status });
    return;
  }
  if (kind === "height") {
    resizeLevel3dHeight(Math.round(value), { status: options.status });
    return;
  }
  const camera = level3dPreviewCamera();
  const origin = level3dPreviewOriginState();
  if (kind === "yaw") {
    camera.yawDegrees = level3dNormalizeDegrees(value);
  } else if (kind === "pitch") {
    camera.pitchDegrees = level3dClampNumber(
      value,
      LEVEL3D_CAMERA_MIN_PITCH_DEGREES,
      LEVEL3D_CAMERA_MAX_PITCH_DEGREES,
    );
  } else if (kind === "roll") {
    camera.rollDegrees = level3dNormalizeDegrees(value);
  } else if (kind === "zoom") {
    camera.zoom = level3dClampNumber(value, 0.25, 4);
  } else if (kind === "originX") {
    origin.x = level3dClampNumber(value, -128, 128);
  } else if (kind === "originY") {
    origin.y = level3dClampNumber(value, -128, 128);
  } else if (kind === "originZ") {
    origin.z = level3dClampNumber(value, -128, 128);
  }
  renderLevel3dPreviewControls();
  renderLevel3dRuntime();
}

function level3dPreviewScrubScale(kind) {
  if (kind === "zoom") {
    return 0.01;
  }
  if (kind === "originX" || kind === "originY" || kind === "originZ") {
    return 0.05;
  }
  if (kind === "width" || kind === "depth" || kind === "height") {
    return 0.08;
  }
  return 0.5;
}

function level3dPreviewKeyStep(kind) {
  if (kind === "zoom") {
    return 0.05;
  }
  if (kind === "originX" || kind === "originY" || kind === "originZ") {
    return 0.25;
  }
  return 1;
}

function setLevel3dActionStatus(text, className = "") {
  if (level3dActionStatus) {
    level3dActionStatus.className = `visual-action-status tool-feedback-bar ${className || ""}`.trim();
    level3dActionStatus.textContent = text || "";
  }
  if (typeof setPaneStatus === "function") {
    setPaneStatus("level", text, className);
  }
}

level3dBundleInput?.addEventListener("input", () => {
  level3dBundleInput.dataset.userEdited = "true";
  syncLevel3dNameOptions();
  if (document.activeElement === level3dNameInput) {
    showLevel3dNameOptions();
  }
  renderLevel3dSourcePreview();
});
level3dBundleInput?.addEventListener("focus", syncLevel3dNameOptions);
level3dNameInput?.addEventListener("input", () => {
  level3dNameInput.dataset.userEdited = "true";
  renderLevel3dSourcePreview();
  showLevel3dNameOptions();
});
level3dNameInput?.addEventListener("focus", showLevel3dNameOptions);
level3dNameInput?.addEventListener("blur", () => window.setTimeout(hideLevel3dNameOptions, 120));
level3dNameInput?.addEventListener("keydown", (event) => {
  if (event.key === "Escape") {
    hideLevel3dNameOptions();
  }
});
level3dNameInput?.addEventListener("change", () => {
  if (!loadSelectedLevel3dNameFromInput()) {
    renderLevel3dSourcePreview();
  }
});
level3dWidthInput?.addEventListener("input", () => {
  if (level3dWidthInput.value === "") {
    return;
  }
  resizeLevel3dWidth(level3dWidthInput.value, { edge: "right", status: false });
});
level3dWidthInput?.addEventListener("change", () => {
  const changed = resizeLevel3dWidth(level3dWidthInput.value, { edge: "right" });
  if (!changed) {
    syncLevel3dSizeControls();
  }
});
level3dDepthInput?.addEventListener("input", () => {
  if (level3dDepthInput.value === "") {
    return;
  }
  resizeLevel3dDepth(level3dDepthInput.value, { edge: "back", status: false });
});
level3dDepthInput?.addEventListener("change", () => {
  const changed = resizeLevel3dDepth(level3dDepthInput.value, { edge: "back" });
  if (!changed) {
    syncLevel3dSizeControls();
  }
});
level3dHeightInput?.addEventListener("input", () => {
  if (level3dHeightInput.value === "") {
    return;
  }
  resizeLevel3dHeight(level3dHeightInput.value, { status: false });
});
level3dHeightInput?.addEventListener("change", () => {
  const changed = resizeLevel3dHeight(level3dHeightInput.value);
  if (!changed) {
    syncLevel3dSizeControls();
  }
});
for (const scrub of [
  level3dCameraYawScrub,
  level3dCameraPitchScrub,
  level3dCameraRollScrub,
  level3dCameraZoomScrub,
  level3dOriginXScrub,
  level3dOriginYScrub,
  level3dOriginZScrub,
]) {
  scrub?.addEventListener("pointerdown", startLevel3dPreviewScrub);
  scrub?.addEventListener("pointermove", continueLevel3dPreviewScrub);
  scrub?.addEventListener("pointerup", stopLevel3dPreviewScrub);
  scrub?.addEventListener("pointercancel", stopLevel3dPreviewScrub);
  scrub?.addEventListener("keydown", adjustLevel3dPreviewScrubWithKey);
}
window.addEventListener("pointerup", stopLevel3dPreviewScrub, true);
window.addEventListener("pointercancel", stopLevel3dPreviewScrub, true);
window.addEventListener("blur", () => finishLevel3dPreviewScrub());
document.addEventListener("click", (event) => {
  document.querySelectorAll(".level3d-layer-palette .level-layer-visibility-menu:not([hidden])").forEach((menu) => {
    const wrap = menu.closest(".level-layer-visibility-wrap");
    if (wrap?.contains(event.target)) {
      return;
    }
    menu.hidden = true;
    const button = wrap?.querySelector(".level-layer-visibility-button");
    button?.classList.remove("is-open");
    button?.setAttribute("aria-expanded", "false");
  });
});
level3dResetPreviewButton?.addEventListener("click", resetLevel3dPreviewView);
level3dLayerInput?.addEventListener("change", applyLevel3dLayerInput);
level3dLayerInput?.addEventListener("keydown", (event) => {
  if (event.key !== "Enter") {
    return;
  }
  event.preventDefault();
  applyLevel3dLayerInput();
});
const level3dSliceScrub = document.querySelector("[data-level3d-slice-scrub]");
level3dSliceScrub?.addEventListener("pointerdown", startLevel3dSliceScrub);
level3dSliceScrub?.addEventListener("pointermove", continueLevel3dSliceScrub);
level3dSliceScrub?.addEventListener("pointerup", stopLevel3dSliceScrub);
level3dSliceScrub?.addEventListener("pointercancel", stopLevel3dSliceScrub);
window.addEventListener("pointerup", stopLevel3dSliceScrub, true);
window.addEventListener("pointercancel", stopLevel3dSliceScrub, true);
window.addEventListener("blur", () => finishLevel3dSliceScrub());
document.querySelectorAll("[data-level3d-layer-edge]").forEach((button) => {
  button.addEventListener("click", () => {
    const mode = level3dStageResizeMode();
    if (!mode) {
      return;
    }
    resizeLevel3dLayerEdge(button.dataset.level3dLayerEdge, mode);
  });
});
window.addEventListener("resize", scheduleLevel3dSurfaceResize);
document.addEventListener("keydown", (event) => {
  const tagName = event.target?.tagName || "";
  if (level3dBuilder.hidden || !level3dPlaytestActive || ["INPUT", "TEXTAREA", "SELECT"].includes(tagName)) {
    return;
  }
  sendLevel3dPlaytestKey(event);
});
if (window.ResizeObserver) {
  const level3dSurfaceObserver = new ResizeObserver(scheduleLevel3dSurfaceResize);
  if (level3dBuilder) {
    level3dSurfaceObserver.observe(level3dBuilder);
  }
  const level3dScrollSurface = level3dBuilder?.querySelector?.(".tool-pane-scroll");
  if (level3dScrollSurface) {
    level3dSurfaceObserver.observe(level3dScrollSurface);
  }
  if (level3dStageCanvas) {
    level3dSurfaceObserver.observe(level3dStageCanvas);
  }
  if (level3dLayerBoard) {
    level3dSurfaceObserver.observe(level3dLayerBoard);
  }
}
registerSourceEditableTarget?.("level3d", {
  load: loadLevel3dFromSourcePosition,
});
