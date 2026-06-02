// 3D level editor source roundtrip and runtime bridge. Rendering stays in puzzle3_app.js.
let level3dRuntimeFrameKey = "";
let level3dRuntimeFrameLoaded = false;
let level3dLayerFrameKey = "";
let level3dLayerFrameLoaded = false;
let level3dSolverFrame = null;
let level3dSolverFrameKey = "";
let level3dSolverFrameLoaded = false;
let level3dLayerHover = null;
let level3dLayerRendererView = null;
let level3dStageOverlay = null;
let level3dStageHit = null;
let level3dStageRendererView = null;
let level3dLayerPaintDrag = null;
let level3dPreviewScrubDrag = null;
let level3dSliceScrubDrag = null;
let level3dPlaytestActive = false;
let level3dPlaytestSnapshot = null;
let level3dPreviewCameraState = null;
let level3dPreviewOrigin = { x: 0, y: 0, z: 0 };
let level3dSurfaceResizeFrame = 0;
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
const LEVEL3D_PREVIEW_SURFACE_MESSAGE = "PuzzleStudioPreviewSurfaceUpdate";
const LEVEL3D_PREVIEW_SURFACE_KIND = "puzzle3-level";
const LEVEL3D_PREVIEW_SURFACE_MODE = "isolated";
const LEVEL3D_MODEL_COMPONENT_PREVIEW_MESSAGE = "PuzzleStudioRenderPuzzle3ModelComponent";
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
  layerPaletteCollapsed: false,
  layerGridVisible: true,
  hiddenLayers: [],
  stageResizeMode: null,
  stageExpandMode: false,
  previewFrames: false,
  palette: [],
  slices: [],
  sourceKey: "",
  sourceDocumentId: "",
};
let level3dAutoSelectionKey = "";

function renderLevel3dBuilder() {
  if (!level3dBuilder) {
    return;
  }
  syncLevel3dFrameLayout();
  syncLevel3dControlsFromPreview();
  renderLevel3dPalette();
  renderLevel3dLayerPalette();
  renderLevel3dPreviewControls();
  renderLevel3dLayerControls();
  renderLevel3dLayerBoard();
  renderLevel3dSourcePreview();
  renderLevel3dRuntime();
  renderLevel3dStageOverlay();
  updateLevel3dPlaytestControls();
}

function syncLevel3dFrameLayout() {
  if (!level3dBuilder) {
    return;
  }
  const workspace = level3dBuilder.querySelector(".level3d-workspace");
  const container = level3dBuilder.querySelector(".tool-pane-scroll") || level3dBuilder;
  const availableWidth = Math.max(1, Math.floor(level3dContentInlineSize(container)));
  const canFitSideBySide = availableWidth >= LEVEL3D_FRAME_MIN_WIDTH * 2 + LEVEL3D_FRAME_GAP;
  const maxFrameWidth = Math.max(
    LEVEL3D_FRAME_MIN_WIDTH,
    Math.min(LEVEL3D_FRAME_MAX_WIDTH, canFitSideBySide
      ? Math.floor((availableWidth - LEVEL3D_FRAME_GAP) / 2)
      : availableWidth),
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
  workspace?.classList.toggle("is-stacked", !canFitSideBySide);
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
  const exportData = previewExport || extractPreviewExport(latestHtml);
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
      level3dNameInput.value = "level_1";
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
  const index = currentEditableLevelIndex(previewExport || extractPreviewExport(latestHtml));
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
    layerPaletteCollapsed: Boolean(level3d.layerPaletteCollapsed),
    layerGridVisible: level3d.layerGridVisible !== false,
    hiddenLayers: Array.isArray(level3d.hiddenLayers) ? [...level3d.hiddenLayers] : [],
    stageResizeMode: null,
    stageExpandMode: false,
    previewFrames: level3d.previewFrames,
    palette,
    slices: [],
    sourceKey: "",
    sourceDocumentId: document?.id || "",
  };
  level3dStageHit = null;
}

function sourceLevel3dPaletteEntries(source) {
  const entries = normalizedLevel3dLegendEntries(sourceLevel3dLegendEntries(source));
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
  return (palette || []).filter((entry) => entry.temporary !== true);
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

function loadLevel3dFromEntry(entry, source, exportData = previewExport, sourceKey = "", document = level3dSourceDocument()) {
  const legendEntries = normalizedLevel3dLegendEntries(sourceLevel3dLegendEntries(source));
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
  level3d.sourceKey = sourceKey || currentLevel3dEditorSourceKey(entry, document, source);
  resetLevel3dPreviewState(exportData);
}

function loadLevel3dFromSourceDefinition(definition, source, sourceKey = "", document = level3dSourceDocument()) {
  const legendEntries = normalizedLevel3dLegendEntries(sourceLevel3dLegendEntries(source));
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
  level3d.sourceKey = sourceKey || currentLevel3dEditorSourceKey(definition, document, source);
  resetLevel3dPreviewState(previewExport || extractPreviewExport(latestHtml));
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
      temporary: entry.temporary === true,
    });
    chars.add(ch);
  }
  if (!unique.some((entry) => entry.objects.length === 0)) {
    unique.unshift({ char: LEVEL3D_EMPTY_CHAR, objects: [] });
  }
  return unique.length ? unique : [{ char: LEVEL3D_EMPTY_CHAR, objects: [] }];
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
  level3dStageHit = null;
  renderLevel3dStageOverlay();
  refreshLevel3dRuntimePreviews();
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
  level3dStageHit = null;
  renderLevel3dStageOverlay();
  refreshLevel3dRuntimePreviews();
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
  level3dStageHit = null;
  renderLevel3dStageOverlay();
  refreshLevel3dRuntimePreviews();
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

function currentLevel3dEntry(exportData = previewExport) {
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

function level3dExportEntryForSourceDefinition(definition, exportData = previewExport) {
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

function applyDefaultLevel3dSelectionForActiveDocument(exportData = previewExport) {
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
    findLevels3Ranges(source).length,
    Array.isArray(exportData?.levels) ? exportData.levels.length : 0,
  ].join(":");
  if (level3dAutoSelectionKey === selectionKey) {
    return false;
  }
  level3dAutoSelectionKey = selectionKey;
  const target = firstLevel3dTargetInDocument(document);
  if (!target) {
    level3d.sourceDocumentId = "";
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

function firstLevel3dTargetNearDocument(document) {
  const direct = firstLevel3dTargetInDocument(document);
  if (direct) {
    return direct;
  }
  let dir = directoryName(document?.puzzlePath || "");
  const visited = new Set();
  while (!visited.has(dir)) {
    visited.add(dir);
    const found = puzzleTextDocuments()
      .filter((candidate) => normalizePath(directoryName(candidate.puzzlePath)) === normalizePath(dir))
      .sort((left, right) => normalizePath(left.puzzlePath).localeCompare(normalizePath(right.puzzlePath)))
      .map((candidate) => firstLevel3dTargetInDocument(candidate))
      .find(Boolean);
    if (found) {
      return found;
    }
    if (!dir) {
      break;
    }
    const parent = directoryName(dir);
    if (parent === dir) {
      break;
    }
    dir = parent;
  }
  return null;
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
  let levelIndex = 0;
  for (const range of findLevels3Ranges(source)) {
    const definition = findLevel3dDefinitions(source, range)[0];
    if (!definition) {
      continue;
    }
    return {
      ...definition,
      bundle: range.bundle,
      model: range.model,
      levelIndex,
      rows: rowsForLevel3dDefinition(source, definition),
    };
  }
  return null;
}

function currentLevel3dBundleName(exportData = previewExport) {
  const sourceRange = findLevels3InsertionRange(level3dEditorSource(), "");
  if (sourceRange?.bundle) {
    return sourceRange.bundle;
  }
  if (!findLevels3Ranges(level3dEditorSource()).length) {
    return "levels";
  }
  const bundles = Object.keys(exportData?.levelBundles || {}).filter((name) => !["default", "levels"].includes(name));
  return bundles[0] || "levels";
}

function level3dNameControlConfig(source = level3dEditorSource()) {
  return {
    source,
    scopeValue: String(level3dBundleInput?.value || "").trim(),
    nameInput: level3dNameInput,
    datalist: level3dNameOptions,
    findRanges: findLevels3Ranges,
    findDefinitions: findLevel3dDefinitions,
    rangeScope: (range) => String(range?.bundle || "").trim(),
    entryName: (entry) => entry?.name || "",
    optionValue: (entry) => entry?.name || "",
  };
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
    load: ({ entry, range }) => loadLevel3dNameEntry({ entry, range, source, sourceDocument }),
  };
}

function loadLevel3dNameEntry({ entry, range, source = level3dEditorSource(), sourceDocument = level3dSourceDocument() }) {
  return loadLevel3dSourceDefinition({
    ...entry,
    bundle: range?.bundle || "",
    model: range?.model || "",
    rows: rowsForLevel3dDefinition(source, entry),
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
  return Boolean(
    exportData?.__kind === "puzzle3d"
      || exportData?.directions?.front
      || exportData?.directions?.forward
      || exportData?.levelBundles,
  );
}

function renderLevel3dSourcePreview() {
  if (!level3dSourcePreview) {
    return;
  }
  syncLevel3dNameOptions();
  const levelName = sanitizeLevel3dName(level3dNameInput?.value || currentLevel3dSourceDefinition(level3dEditorSource())?.name || "level_1");
  const sourceData = level3dSourceData();
  level3dSourcePreview.textContent = level3dSnippetSource(levelName, sourceData, "", { bodyIndent: "" });
}

function level3dSourceData(source = level3dEditorSource(), exportData = previewExport || extractPreviewExport(latestHtml)) {
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
  const legendEntries = sourceLevel3dLegendEntries(source);
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
    const objects = (cell.objects || []).map((object) => object.name || object.sprite || "").filter(Boolean);
    cellMap.set(`${position.x},${position.y},${position.z}`, objects);
  }
  const rows = [];
  let unknownCells = 0;
  for (let slice = 0; slice < height; slice += 1) {
    if (slice > 0) {
      rows.push("");
    }
    const z = height - 1 - slice;
    for (let row = 0; row < depth; row += 1) {
      const y = depth - 1 - row;
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

function sourceLevel3dLegendEntries(source) {
  const entries = [];
  for (const range of findLevels3Ranges(source)) {
    entries.push(...sourceLevel3dLegendEntriesForRange(source, range));
  }
  if (!entries.some((entry) => entry.objects.length === 0)) {
    entries.unshift({ char: LEVEL3D_EMPTY_CHAR, objects: [] });
  }
  return entries;
}

function currentLevel3dSourceDefinition(source) {
  const ranges = findLevels3Ranges(source);
  if (!ranges.length) {
    return null;
  }
  const requestedBundle = String(level3dBundleInput?.value || "").trim();
  const requestedName = String(level3dNameInput?.value || "").trim();
  for (const range of ranges) {
    if (requestedBundle && range.bundle !== requestedBundle) {
      continue;
    }
    const definitions = findLevel3dDefinitions(source, range);
    if (!definitions.length) {
      continue;
    }
    const exact = requestedName
      ? definitions.find((definition) => definition.name === requestedName)
      : null;
    const definition = exact || defaultLevel3dSourceDefinition(source, [range]);
    if (!definition) {
      continue;
    }
    return level3dSourceDefinitionFromRange(source, range, definition);
  }
  return defaultLevel3dSourceDefinition(source, ranges);
}

function defaultLevel3dSourceDefinition(source, ranges = findLevels3Ranges(source)) {
  for (const range of ranges) {
    if (!sourceLevel3dRangeHasReadableLegend(source, range)) {
      continue;
    }
    const definition = findLevel3dDefinitions(source, range)[0];
    if (!definition) {
      continue;
    }
    return level3dSourceDefinitionFromRange(source, range, definition);
  }
  for (const range of ranges) {
    const definition = findLevel3dDefinitions(source, range)[0];
    if (!definition) {
      continue;
    }
    return level3dSourceDefinitionFromRange(source, range, definition);
  }
  return null;
}

function sourceLevel3dRangeHasReadableLegend(source, range) {
  const entries = sourceLevel3dLegendEntriesForRange(source, range);
  return entries.some((entry) => entry.objects.length > 0);
}

function sourceLevel3dLegendEntriesForRange(source, range) {
  const block = String(source || "").slice(range?.bodyStart || 0, range?.bodyEnd || 0);
  const legendMatch = block.match(/(^|\n)([\t ]*)legend\s*\{\n([\s\S]*?)\n\2\}/m);
  if (!legendMatch) {
    return [];
  }
  return sourceLevel3dLegendEntriesFromBlock(legendMatch[3]);
}

function sourceLevel3dLegendEntriesFromBlock(block) {
  const entries = [];
  for (const raw of String(block || "").split("\n")) {
    const match = raw.trim().match(/^(\S)\s*=\s*(.+?)\s*$/);
    if (!match) {
      continue;
    }
    const expression = match[2].trim();
    entries.push({
      char: match[1],
      objects: expression === "empty" ? [] : expression.split(/\s+/).filter(Boolean),
    });
  }
  return entries;
}

function level3dSourceDefinitionFromRange(source, range, definition) {
  return definition ? {
    ...definition,
    bundle: range.bundle,
    model: range.model,
    rows: rowsForLevel3dDefinition(source, definition),
  } : null;
}

function rowsForLevel3dDefinition(source, definition) {
  const rows = String(source || "")
    .slice(definition.bodyStart, definition.bodyEnd)
    .split("\n")
    .map((line) => level3dScannerCode(line))
    .map((line) => line.trim());
  while (rows.length && !rows[0]) {
    rows.shift();
  }
  while (rows.length && !rows[rows.length - 1]) {
    rows.pop();
  }
  return rows;
}

function level3dObjectSetKey(objects) {
  return [...objects].sort().join("\u0000");
}

function renderLevel3dPalette() {
  if (!level3dPalette) {
    return;
  }
  level3dPalette.replaceChildren();
  level3dPalette.classList.add("is-sprite-only");
  const exportData = previewExport || extractPreviewExport(latestHtml);
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

    button.addEventListener("click", () => {
      setLevel3dStageResizeMode(null);
      level3d.selectedChar = entry.char;
      level3dStageHit = null;
      renderLevel3dPalette();
      renderLevel3dLayerPalette();
      renderLevel3dLayerBoard();
      renderLevel3dLayerOverlay();
      renderLevel3dStageOverlay();
    });
    level3dPalette.append(button);
    drawLevel3dPalettePreview(visual, entry, exportData);
  }
}

function renderLevel3dLayerPalette() {
  if (!level3dLayerPalette) {
    return;
  }
  level3dLayerPalette.replaceChildren();
  level3dLayerPalette.classList.add("is-sprite-only");
  level3dLayerPalette.classList.toggle("is-collapsed", Boolean(level3d.layerPaletteCollapsed));
  const exportData = previewExport || extractPreviewExport(latestHtml);
  const transformRow = document.createElement("div");
  transformRow.className = "level3d-layer-palette-row level3d-layer-transform-row";
  transformRow.append(
    level3dLayerResizeModeButton("expand"),
    level3dLayerResizeModeButton("shrink"),
    level3dLayerGridButton(),
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
    level3dLayerScopeToggle(),
    level3dLayerPaletteCollapseButton(),
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

    button.addEventListener("click", () => {
      setLevel3dStageResizeMode(null);
      level3d.selectedChar = entry.char;
      level3dStageHit = null;
      renderLevel3dPalette();
      renderLevel3dLayerPalette();
      renderLevel3dLayerBoard();
      renderLevel3dStageOverlay();
    });
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
  button.className = "sprite-icon-button level-grid-button";
  button.classList.toggle("is-selected", active);
  button.setAttribute("aria-label", "Toggle top-down grid");
  button.setAttribute("aria-pressed", active ? "true" : "false");
  button.title = "Toggle grid";
  button.dataset.tooltip = "Toggle grid";
  button.disabled = level3dPlaytestActive;
  button.innerHTML = `
    <svg class="level-grid-token-icon lucide lucide-grid2x2-icon lucide-grid-2x2" xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
      <path d="M12 3v18"></path>
      <path d="M3 12h18"></path>
      <rect x="3" y="3" width="18" height="18" rx="2"></rect>
    </svg>
  `;
  button.addEventListener("click", () => {
    level3d.layerGridVisible = level3d.layerGridVisible === false;
    renderLevel3dLayerPalette();
    renderLevel3dLayerBoard();
  });
  return button;
}

function level3dLayerVisibilityControl() {
  const wrap = document.createElement("span");
  wrap.className = "level-layer-visibility-wrap";
  const button = document.createElement("button");
  button.type = "button";
  button.className = "sprite-icon-button level-layer-visibility-button";
  const hasHiddenLayers = normalizedLevel3dHiddenLayers().size > 0;
  button.classList.toggle("has-hidden-layers", hasHiddenLayers);
  button.setAttribute("aria-label", "Layer visibility");
  button.setAttribute("aria-expanded", "false");
  button.title = "Layer visibility";
  button.dataset.tooltip = "Layer visibility";
  button.disabled = level3dPlaytestActive;
  button.innerHTML = `
    <svg class="lucide lucide-list-filter level-layer-visibility-icon" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" aria-hidden="true">
      <path d="M3 6h18"></path>
      <path d="M7 12h10"></path>
      <path d="M10 18h4"></path>
    </svg>
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
    label.className = "level-layer-visibility-option";
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

function level3dLayerVisibilityEntries(exportData = previewExport) {
  const count = level3dLayerCount(exportData);
  return Array.from({ length: count }, (_, layerIndex) => ({
    layer: layerIndex,
    label: `Layer ${layerIndex + 1}`,
  }));
}

function level3dLayerCount(exportData = previewExport) {
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

function normalizedLevel3dHiddenLayers(exportData = previewExport) {
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

function level3dLayerIsVisible(layerIndex, exportData = previewExport) {
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
  button.className = `sprite-icon-button ${mode === "expand" ? "level-expand-button" : "level-shrink-button"}`;
  button.classList.toggle("is-selected", active);
  button.setAttribute("aria-label", mode === "expand" ? "Toggle top-down expansion" : "Toggle top-down shrinking");
  button.setAttribute("aria-pressed", active ? "true" : "false");
  button.title = mode === "expand" ? "Expand" : "Shrink";
  button.dataset.tooltip = button.title;
  button.disabled = level3dPlaytestActive;
  button.innerHTML = mode === "expand"
    ? `
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="m15 15 6 6"></path>
        <path d="m15 9 6-6"></path>
        <path d="M21 16v5h-5"></path>
        <path d="M21 8V3h-5"></path>
        <path d="M3 16v5h5"></path>
        <path d="m3 21 6-6"></path>
        <path d="M3 8V3h5"></path>
        <path d="M9 9 3 3"></path>
      </svg>
    `
    : `
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="m15 15 6 6m-6-6v4.8m0-4.8h4.8"></path>
        <path d="M9 19.8V15m0 0H4.2M9 15l-6 6"></path>
        <path d="M15 4.2V9m0 0h4.8M15 9l6-6"></path>
        <path d="M9 4.2V9m0 0H4.2M9 9 3 3"></path>
      </svg>
    `;
  button.addEventListener("click", () => {
    setLevel3dStageResizeMode(level3dStageResizeMode() === mode ? null : mode);
    level3dStageHit = null;
    renderLevel3dPalette();
    renderLevel3dLayerPalette();
    syncLevel3dLayerResizeControls();
    renderLevel3dStageOverlay();
  });
  return button;
}

function level3dLayerTransformButton(kind) {
  const config = {
    "rotate-left": {
      label: "Rotate top-down left",
      title: "Rotate left",
      icon: `
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M3 12a9 9 0 1 0 2.64-6.36L3 8"></path>
          <path d="M3 3v5h5"></path>
        </svg>
      `,
      action: rotateLevel3dLayerLeft,
    },
    "rotate-right": {
      label: "Rotate top-down right",
      title: "Rotate right",
      icon: `
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M21 12a9 9 0 1 1-2.64-6.36L21 8"></path>
          <path d="M21 3v5h-5"></path>
        </svg>
      `,
      action: rotateLevel3dLayerRight,
    },
    "flip-horizontal": {
      label: "Flip top-down horizontal",
      title: "Flip horizontal",
      icon: `
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M8 3H5a2 2 0 0 0-2 2v14c0 1.1.9 2 2 2h3"></path>
          <path d="M16 3h3a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2h-3"></path>
          <path d="M12 20v2"></path>
          <path d="M12 14v2"></path>
          <path d="M12 8v2"></path>
          <path d="M12 2v2"></path>
        </svg>
      `,
      action: flipLevel3dLayerHorizontal,
    },
    "flip-vertical": {
      label: "Flip top-down vertical",
      title: "Flip vertical",
      icon: `
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M21 8V5a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v3"></path>
          <path d="M21 16v3a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-3"></path>
          <path d="M4 12H2"></path>
          <path d="M10 12H8"></path>
          <path d="M16 12h-2"></path>
          <path d="M22 12h-2"></path>
        </svg>
      `,
      action: flipLevel3dLayerVertical,
    },
  }[kind];
  const button = document.createElement("button");
  button.type = "button";
  button.className = "sprite-icon-button level3d-layer-transform-button";
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

function level3dLayerScopeToggle() {
  const group = document.createElement("div");
  group.className = "level-scope-toggle sprite3d-scope-toggle level3d-layer-scope-toggle";
  group.setAttribute("role", "group");
  group.setAttribute("aria-label", "Top-down 3D level edit scope");
  const label = document.createElement("span");
  label.className = "sprite3d-scope-toggle-label";
  label.textContent = "Scope";
  group.append(label);
  group.append(level3dLayerScopeButton("add", "Mono layer", `
    <svg xmlns="http://www.w3.org/2000/svg" class="lucide lucide-diamond-icon lucide-diamond" viewBox="0 0 24 24" aria-hidden="true">
      <path d="M2.7 10.3a2.41 2.41 0 0 0 0 3.41l7.59 7.59a2.41 2.41 0 0 0 3.41 0l7.59-7.59a2.41 2.41 0 0 0 0-3.41l-7.59-7.59a2.41 2.41 0 0 0-3.41 0Z"></path>
    </svg>
  `));
  group.append(level3dLayerScopeButton("replace", "All layers", `
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M5 8 12 4l7 4-7 4-7-4Z"></path>
      <path d="M5 12l7 4 7-4"></path>
      <path d="M5 16l7 4 7-4"></path>
    </svg>
  `));
  return group;
}

function level3dLayerScopeButton(mode, label, icon) {
  const active = level3dEditMode() === mode;
  const button = document.createElement("button");
  button.type = "button";
  button.className = "level-scope-button sprite3d-scope-button sprite-icon-button";
  button.classList.toggle("is-active", active);
  button.setAttribute("aria-label", label);
  button.setAttribute("aria-pressed", active ? "true" : "false");
  button.title = label;
  button.dataset.tooltip = label;
  button.disabled = level3dPlaytestActive;
  button.innerHTML = icon;
  button.addEventListener("click", () => {
    setLevel3dStageResizeMode(null);
    setLevel3dEditMode(mode);
    level3dStageHit = null;
    renderLevel3dPalette();
    renderLevel3dLayerPalette();
    renderLevel3dStageOverlay();
  });
  return button;
}

function level3dLayerFillButton() {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "level-palette-tool-button sprite-fill-button sprite-icon-button";
  button.classList.toggle("is-active", Boolean(level3d.layerFillActive));
  button.setAttribute("aria-label", "Fill top-down 3D level area");
  button.setAttribute("aria-pressed", level3d.layerFillActive ? "true" : "false");
  button.title = "Fill";
  button.dataset.tooltip = "Fill";
  button.disabled = level3dPlaytestActive;
  button.innerHTML = `
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="m19 11-8-8-8.6 8.6a2 2 0 0 0 0 2.8l5.2 5.2a2 2 0 0 0 2.8 0L19 11Z"></path>
      <path d="m5 2 5 5"></path>
      <path d="M2 13h15"></path>
      <path d="M22 20a2 2 0 1 1-4 0c0-1.6 1.7-2.4 2-4 .3 1.6 2 2.4 2 4Z"></path>
    </svg>
  `;
  button.addEventListener("click", () => {
    level3d.layerFillActive = !level3d.layerFillActive;
    renderLevel3dLayerPalette();
  });
  return button;
}

function level3dLayerPaletteCollapseButton() {
  const collapsed = Boolean(level3d.layerPaletteCollapsed);
  const button = document.createElement("button");
  button.type = "button";
  button.className = "level-palette-toggle-button";
  button.classList.toggle("is-active", collapsed);
  button.classList.toggle("is-collapsed", collapsed);
  button.setAttribute("aria-expanded", String(!collapsed));
  button.setAttribute("aria-label", collapsed ? "Show top-down palette" : "Hide top-down palette");
  button.title = collapsed ? "Show palette" : "Hide palette";
  button.dataset.tooltip = button.title;
  button.disabled = level3dPlaytestActive;
  button.innerHTML = `
    <svg class="palette-open-icon" viewBox="0 0 24 24" aria-hidden="true">
      <rect x="4" y="4" width="6" height="6" rx="1"></rect>
      <rect x="14" y="4" width="6" height="6" rx="1"></rect>
      <rect x="4" y="14" width="6" height="6" rx="1"></rect>
      <rect x="14" y="14" width="6" height="6" rx="1"></rect>
    </svg>
    <svg class="palette-closed-icon" viewBox="0 0 24 24" aria-hidden="true">
      <rect x="4" y="4" width="6" height="6" rx="1"></rect>
      <rect x="14" y="4" width="6" height="6" rx="1"></rect>
      <rect x="4" y="14" width="6" height="6" rx="1"></rect>
      <rect x="14" y="14" width="6" height="6" rx="1"></rect>
      <path d="M3 21 21 3"></path>
    </svg>
  `;
  button.addEventListener("click", () => {
    level3d.layerPaletteCollapsed = !level3d.layerPaletteCollapsed;
    renderLevel3dLayerPalette();
  });
  return button;
}

function level3dLayerEraserButton() {
  const entry = level3d.palette.find((candidate) => !candidate.objects?.length)
    || { char: level3dEmptyChar(), objects: [] };
  const button = document.createElement("button");
  button.type = "button";
  button.className = "level-palette-tool-button sprite-icon-button level-eraser-button";
  button.classList.toggle("is-active", level3d.selectedChar === entry.char);
  button.setAttribute("aria-label", "Paint top-down Eraser");
  button.setAttribute("aria-pressed", level3d.selectedChar === entry.char ? "true" : "false");
  button.title = "Eraser";
  button.dataset.tooltip = "Eraser";
  button.disabled = level3dPlaytestActive;
  button.append(renderLevelEraserIcon());
  button.addEventListener("click", () => {
    setLevel3dStageResizeMode(null);
    level3d.selectedChar = entry.char;
    renderLevel3dPalette();
    renderLevel3dLayerPalette();
    renderLevel3dLayerBoard();
    renderLevel3dStageOverlay();
  });
  return button;
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
  button.className = `source-action-button level3d-edit-mode-button level3d-${mode}-button`;
  button.classList.toggle("is-selected", active);
  button.dataset.label = mode === "add" ? "Add tile" : "Replace tile";
  button.title = mode === "add" ? "Add tile" : "Replace tile";
  button.setAttribute("aria-label", mode === "add" ? "Use add tile mode" : "Use replace tile mode");
  button.setAttribute("aria-pressed", active ? "true" : "false");
  button.disabled = level3dPlaytestActive;
  button.innerHTML = mode === "add"
    ? `
      <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-square-plus-icon lucide-square-plus" aria-hidden="true">
        <rect width="18" height="18" x="3" y="3" rx="2"></rect>
        <path d="M8 12h8"></path>
        <path d="M12 8v8"></path>
      </svg>
    `
    : `
      <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-square-pen-icon lucide-square-pen" aria-hidden="true">
        <path d="M12 3H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"></path>
        <path d="M18.375 2.625a1 1 0 0 1 3 3l-9.013 9.014a2 2 0 0 1-.853.505l-2.873.84a.5.5 0 0 1-.62-.62l.84-2.873a2 2 0 0 1 .506-.852z"></path>
      </svg>
    `;
  button.addEventListener("click", () => {
    setLevel3dStageResizeMode(null);
    setLevel3dEditMode(mode);
    level3dStageHit = null;
    renderLevel3dPalette();
    renderLevel3dLayerPalette();
    renderLevel3dStageOverlay();
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
  button.className = "source-action-button level3d-frame-toggle-button";
  button.classList.toggle("is-selected", Boolean(level3d.previewFrames));
  button.dataset.label = "Cell and stage frames";
  button.title = "Cell and stage frames";
  button.setAttribute("aria-label", "Toggle occupied cell and stage frames in the 3D preview");
  button.setAttribute("aria-pressed", level3d.previewFrames ? "true" : "false");
  button.disabled = level3dPlaytestActive;
  button.innerHTML = `
    <svg class="level3d-frame-token-icon lucide lucide-grid2x2-icon lucide-grid-2x2" xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
      <path d="M12 3v18"></path>
      <path d="M3 12h18"></path>
      <rect x="3" y="3" width="18" height="18" rx="2"></rect>
    </svg>
  `;
  button.addEventListener("click", () => {
    level3d.previewFrames = !level3d.previewFrames;
    renderLevel3dPalette();
    renderLevel3dLayerPalette();
    renderLevel3dStageOverlay();
    refreshLevel3dRuntimePreviews();
  });
  return button;
}

function level3dExpandModeButton() {
  return level3dStageResizeModeButton({
    mode: "expand",
    className: "level3d-expand-button",
    label: "Expand stage",
    ariaLabel: "Toggle 3D stage expansion",
    icon: `
      <svg class="level3d-expand-token-icon" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="m15 15 6 6"></path>
        <path d="m15 9 6-6"></path>
        <path d="M21 16v5h-5"></path>
        <path d="M21 8V3h-5"></path>
        <path d="M3 16v5h5"></path>
        <path d="m3 21 6-6"></path>
        <path d="M3 8V3h5"></path>
        <path d="M9 9 3 3"></path>
      </svg>
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
      <svg class="level3d-shrink-token-icon lucide lucide-shrink-icon lucide-shrink" xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="m15 15 6 6m-6-6v4.8m0-4.8h4.8"></path>
        <path d="M9 19.8V15m0 0H4.2M9 15l-6 6"></path>
        <path d="M15 4.2V9m0 0h4.8M15 9l6-6"></path>
        <path d="M9 4.2V9m0 0H4.2M9 9 3 3"></path>
      </svg>
    `,
  });
}

function level3dStageResizeModeButton({ mode, className, label, ariaLabel, icon }) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = `source-action-button ${className}`;
  button.classList.toggle("is-selected", level3dStageResizeMode() === mode);
  button.dataset.label = label;
  button.title = label;
  button.setAttribute("aria-label", ariaLabel);
  button.setAttribute("aria-pressed", level3dStageResizeMode() === mode ? "true" : "false");
  button.disabled = level3dPlaytestActive;
  button.innerHTML = icon;
  button.addEventListener("click", () => {
    setLevel3dStageResizeMode(level3dStageResizeMode() === mode ? null : mode);
    level3dStageHit = null;
    renderLevel3dPalette();
    renderLevel3dLayerPalette();
    renderLevel3dStageOverlay();
  });
  return button;
}

function level3dPaletteEntryLabel(entry) {
  return entry.objects.length ? `${entry.char} = ${entry.objects.join(" ")}` : `${entry.char} = empty`;
}

function drawLevel3dPalettePreview(canvas, entry, exportData = previewExport) {
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
  const sprites = level3dPreviewSprites(exportData);
  const objects = (entry.objects || [])
    .map((name) => level3dPaletteObjectDescriptor(name, exportData, sprites))
    .filter(Boolean);
  const snapshot = {
    size: { width: 1, depth: 1, height: 1 },
    camera: level3dPalettePreviewCamera(exportData),
    sprites,
    settings: exportData?.settings || {},
  };
  if (!objects.length) {
    if ((entry.objects || []).length) {
      drawLevel3dUnavailableTilePreview(ctx, width, height, entry.char);
    } else {
      drawLevel3dEmptyTilePreview(ctx, width, height, snapshot, level3dPalettePreviewOptions(snapshot.camera));
    }
    return;
  }
  drawLevel3dCellsPreview(ctx, width, height, snapshot, [{
    position: { x: 0, y: 0, z: 0 },
    objects,
  }], level3dPalettePreviewOptions(snapshot.camera));
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
    renderLevel3dLayerOverlay();
    return;
  }
  level3dLayerBoard.classList.remove("is-empty");
  renderLevel3dLayerGrid();
  renderLevel3dLayerOverlay();
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
  const quantum = level3dLayerSpritePixelQuantum();
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

function level3dLayerSpritePixelQuantum(exportData = previewExport) {
  const sprites = level3dPreviewSprites(exportData);
  const dimensions = [];
  for (const entry of level3dVisiblePaletteEntries()) {
    for (const name of entry.objects || []) {
      const object = level3dPaletteObjectDescriptor(name, exportData, sprites);
      const sprite = object ? (sprites?.[object.sprite] || sprites?.[object.name]) : null;
      const size = level3dTopDownSpriteSize(sprite);
      if (size) {
        dimensions.push(size.width, size.depth);
      }
    }
  }
  return dimensions.reduce((quantum, value) => level3dLcm(quantum, value), 1);
}

function level3dTopDownSpriteSize(sprite) {
  if (!sprite) {
    return null;
  }
  const blocks = level3dBitmapBlocks(sprite.bitmap || []);
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
      const y = depth - 1 - row;
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
  drawLevel3dTopDownTilePreview(preview, entry, previewExport, {
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

function drawLevel3dTopDownTilePreview(canvas, entry, exportData = previewExport, options = {}) {
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
  const sprites = level3dPreviewSprites(exportData);
  const projections = (entry.objects || [])
    .map((name) => level3dPaletteObjectDescriptor(name, exportData, sprites))
    .filter(Boolean)
    .filter((object) => level3dLayerIsVisible(object.layer, exportData))
    .map((object) => level3dTopDownSpriteProjection(sprites?.[object.sprite] || sprites?.[object.name], { crop: options.crop === true }))
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

function level3dTopDownSpriteProjection(sprite, options = {}) {
  if (!sprite) {
    return null;
  }
  const blocks = level3dBitmapBlocks(sprite.bitmap || []);
  const depth = Math.max(1, ...blocks.map((rows) => rows.length));
  const width = Math.max(1, ...blocks.flatMap((rows) => rows.map((row) => row.length)));
  const pixels = Array.from({ length: depth }, () => Array.from({ length: width }, () => ""));
  for (let row = 0; row < depth; row += 1) {
    for (let column = 0; column < width; column += 1) {
      for (let z = 0; z < blocks.length; z += 1) {
        const token = blocks[z]?.[row]?.[column];
        const fill = sprite.palette?.[token];
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
  return height - 1 - slice;
}

function level3dSliceArrayIndexForZ(z = currentLevel3dLayerZ()) {
  const height = Math.max(1, Math.trunc(Number(level3d.height) || 1));
  return Math.max(0, Math.min(height - 1, height - 1 - Math.trunc(Number(z) || 0)));
}

function level3dCharAtPosition(position) {
  const x = Math.trunc(Number(position?.x) || 0);
  const y = Math.trunc(Number(position?.y) || 0);
  const z = Math.trunc(Number(position?.z) || 0);
  const slice = level3d.slices[level3dSliceArrayIndexForZ(z)] || [];
  const row = Math.max(0, Math.min(Math.max(1, level3d.depth || 1) - 1, Math.max(1, level3d.depth || 1) - 1 - y));
  const text = String(slice[row] || "").padEnd(Math.max(1, level3d.width || 1), level3dEmptyChar());
  return text[x] || level3dEmptyChar();
}

function setLevel3dLayer(value) {
  const height = Math.max(1, Math.trunc(Number(level3d.height) || 1));
  level3d.slice = Math.max(0, Math.min(height - 1, Math.trunc(Number(value) || 0)));
  level3dLayerHover = null;
  renderLevel3dLayerControls();
  renderLevel3dLayerBoard();
  renderLevel3dStageOverlay();
}

function moveLevel3dLayer(delta) {
  setLevel3dLayer(level3d.slice + delta);
}

function handleLevel3dSliceHorizontalInput(event) {
  if (
    level3dBuilder.hidden
    || level3dPlaytestActive
    || (event.key !== "ArrowLeft" && event.key !== "ArrowRight")
  ) {
    return false;
  }
  const targetElement = event.target instanceof Element ? event.target : null;
  if (targetElement && targetElement !== document.body && !level3dBuilder.contains(targetElement)) {
    return false;
  }
  const tagName = event.target?.tagName || "";
  if (["INPUT", "TEXTAREA", "SELECT"].includes(tagName) || event.target?.isContentEditable) {
    return false;
  }
  if (targetElement?.closest?.("[data-level3d-preview]")) {
    return false;
  }
  event.preventDefault();
  event.stopPropagation();
  moveLevel3dLayer(event.key === "ArrowLeft" ? -1 : 1);
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
  document.documentElement.classList.add("is-sprite3d-slice-scrubbing");
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
  document.documentElement.classList.remove("is-sprite3d-slice-scrubbing");
  level3dSliceScrubDrag = null;
  if (!moved && inputTarget && level3dLayerInput instanceof HTMLInputElement) {
    level3dLayerInput.focus();
    level3dLayerInput.select();
  }
}

function level3dLayerCellFromPointer(event) {
  const position = level3dLayerPositionFromEvent(event);
  return position ? { position } : null;
}

function paintLevel3dLayerHoverCell() {
  if (!level3dLayerHover) {
    return false;
  }
  return paintLevel3dCellAtPosition(level3dLayerHover);
}

function startLevel3dLayerPaint(event) {
  if (level3dPlaytestActive) {
    level3dLayerBoard?.focus();
    return;
  }
  if (event.button !== 0) {
    return;
  }
  const cell = level3dLayerCellFromPointer(event);
  if (!cell) {
    return;
  }
  event.preventDefault();
  level3dLayerBoard?.focus();
  if (level3d.layerFillActive) {
    withVisualEditHistory("level3d", () => bucketFillLevel3dLayerFromPosition(cell.position));
    return;
  }
  level3dLayerPaintDrag = {
    pointerId: event.pointerId,
    lastKey: "",
    beforeSnapshot: visualEditSnapshot("level3d"),
    changed: false,
  };
  level3dLayerBoard?.setPointerCapture?.(event.pointerId);
  continueLevel3dLayerPaint(event);
}

function bucketFillLevel3dLayerFromPosition(position) {
  if (level3dPlaytestActive || !level3dPositionInBounds(position)) {
    return false;
  }
  const targetChar = level3dCharAtPosition(position);
  const visited = new Set();
  const stack = [{ ...position }];
  let changed = false;
  while (stack.length) {
    const current = stack.pop();
    const key = `${current.x},${current.y},${current.z}`;
    if (visited.has(key) || !level3dPositionInBounds(current) || level3dCharAtPosition(current) !== targetChar) {
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
  if (changed) {
    setLevel3dActionStatus(level3d.selectedChar ? "Filled connected slice area" : "Erased connected slice area", "is-ok");
  }
  return changed;
}

function continueLevel3dLayerPaint(event) {
  const cell = level3dLayerCellFromPointer(event);
  if (!cell) {
    if (level3dLayerHover) {
      level3dLayerHover = null;
      renderLevel3dLayerOverlay();
    }
    return;
  }
  const hoverKey = `${cell.position.x},${cell.position.y},${cell.position.z}`;
  const previousHoverKey = level3dLayerHover
    ? `${level3dLayerHover.x},${level3dLayerHover.y},${level3dLayerHover.z}`
    : "";
  level3dLayerHover = cell.position;
  if (hoverKey !== previousHoverKey) {
    renderLevel3dLayerOverlay();
  }
  if (!level3dLayerPaintDrag || event.pointerId !== level3dLayerPaintDrag.pointerId) {
    return;
  }
  if (hoverKey === level3dLayerPaintDrag.lastKey) {
    return;
  }
  level3dLayerPaintDrag.lastKey = hoverKey;
  if (paintLevel3dCellAtPosition(cell.position)) {
    level3dLayerPaintDrag.changed = true;
  }
}

function stopLevel3dLayerPaint(event) {
  if (!level3dLayerPaintDrag || event.pointerId !== level3dLayerPaintDrag.pointerId) {
    return;
  }
  if (level3dLayerBoard?.hasPointerCapture?.(event.pointerId)) {
    level3dLayerBoard.releasePointerCapture(event.pointerId);
  }
  if (level3dLayerPaintDrag.changed) {
    pushVisualEditUndoSnapshot("level3d", level3dLayerPaintDrag.beforeSnapshot);
  }
  level3dLayerPaintDrag = null;
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
  level3dStageHit = null;
  renderLevel3dStageOverlay();
  refreshLevel3dRuntimePreviews();
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
  const exportData = previewExport || extractPreviewExport(latestHtml);
  const x = Math.trunc(Number(position?.x));
  const y = Math.trunc(Number(position?.y));
  const z = Math.trunc(Number(position?.z));
  if (!level3dPositionInBounds({ x, y, z })) {
    return false;
  }
  const sliceIndex = level3d.height - 1 - z;
  const row = level3d.depth - 1 - y;
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
  renderLevel3dStageOverlay();
  refreshLevel3dRuntimePreviews();
  return true;
}

function level3dLayerMergedChar(currentChar, paintChar, exportData = previewExport || extractPreviewExport(latestHtml)) {
  const nextChar = String(paintChar || level3dEmptyChar()).charAt(0);
  const emptyChar = level3dEmptyChar();
  const paintEntry = level3d.palette.find((entry) => entry.char === nextChar);
  if (!paintEntry?.objects?.length) {
    return emptyChar;
  }
  const currentEntry = level3d.palette.find((entry) => entry.char === currentChar)
    || level3d.palette.find((entry) => entry.char === emptyChar)
    || { objects: [] };
  const paintLayers = new Set(
    paintEntry.objects
      .map((name) => level3dObjectLayer(name, exportData))
      .filter((layer) => layer !== null),
  );
  if (!paintLayers.size) {
    return nextChar;
  }
  const mergedObjects = [
    ...(currentEntry.objects || []).filter((name) => !paintLayers.has(level3dObjectLayer(name, exportData))),
    ...paintEntry.objects,
  ];
  return level3dEnsureCharForObjectNames(mergedObjects);
}

function level3dObjectLayer(name, exportData = previewExport || extractPreviewExport(latestHtml)) {
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

function level3dSnippetSource(name, levelData, indent = "", options = {}) {
  const legendLines = level3dTemporaryLegendSourceLines(levelData, indent);
  const levelSource = levelDefinition3dSource(name, levelData, indent, options).trimEnd();
  return legendLines.length
    ? `${legendLines.join("\n")}\n\n${levelSource}\n`
    : `${levelSource}\n`;
}

function levelDefinition3dSource(name, levelData, indent = "", options = {}) {
  const { rows } = normalizeLevel3dSourceData(levelData);
  const bodyIndent = Object.prototype.hasOwnProperty.call(options, "bodyIndent") ? options.bodyIndent : `${indent}  `;
  return [
    `${indent}level ${sanitizeLevel3dName(name)} {`,
    ...rows.map((row) => String(row || "").length ? `${bodyIndent}${row}` : ""),
    `${indent}}`,
  ].join("\n") + (options.trailingNewline === false ? "" : "\n");
}

function level3dTemporaryLegendSourceLines(levelData, indent) {
  const entries = level3dTemporaryLegendEntriesForLevelData(levelData);
  if (!entries.length) {
    return [];
  }
  const bodyIndent = `${indent}  `;
  return [
    `${indent}legend {`,
    ...entries.map((entry) => `${bodyIndent}${entry.char} = ${entry.objects.join(" ")}`),
    `${indent}}`,
  ];
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
  const cleaned = String(value || "").trim().replace(/[^\w:.]/g, "_").replace(/^_+/, "");
  return cleaned || "level_1";
}

function sanitizeLevel3dBundle(value) {
  const cleaned = String(value || "").trim().replace(/[^\w:.]/g, "_").replace(/^_+/, "");
  return cleaned || currentLevel3dBundleName();
}

function findLevels3Ranges(source) {
  const text = String(source || "");
  const lines = level3dSourceLinesWithOffsets(text);
  const ranges = [];
  for (let index = 0; index < lines.length; index += 1) {
    const code = level3dScannerCode(lines[index].raw);
    const header = parseLevels3Header(code);
    if (!header) {
      continue;
    }
    const close = findLevel3dBlockClose(lines, index);
    if (!close) {
      continue;
    }
    ranges.push({
      bundle: header.bundle,
      model: header.model,
      start: lines[index].start,
      bodyStart: lines[index].end + 1,
      bodyEnd: close.start,
      end: close.end,
      indent: lineIndent(lines[index].raw) || "  ",
    });
    index = close.index;
  }
  return ranges;
}

function parseLevels3Header(code) {
  const match = String(code || "").match(/^levels3(?:\s+([A-Za-z_][\w:.]*)(?:\s+of\s+([A-Za-z_][\w:.]*))?)?\s*\{$/);
  if (!match) {
    return null;
  }
  return { bundle: match[1] || "levels", model: match[2] || "" };
}

function findLevel3dBlockClose(lines, startIndex) {
  let depth = 0;
  for (let index = startIndex; index < lines.length; index += 1) {
    const code = level3dScannerCode(lines[index].raw);
    if (!code) {
      continue;
    }
    if (code.endsWith("{")) {
      depth += 1;
    }
    if (code === "}") {
      depth -= 1;
      if (depth === 0) {
        return { ...lines[index], index };
      }
    }
  }
  return null;
}

function findLevel3dDefinitions(source, range) {
  const text = String(source || "");
  const lines = level3dSourceLinesWithOffsets(text);
  const definitions = [];
  const startIndex = lines.findIndex((line) => line.start >= range.bodyStart);
  for (let index = Math.max(0, startIndex); index < lines.length && lines[index].start < range.bodyEnd; index += 1) {
    const code = level3dScannerCode(lines[index].raw);
    const match = code.match(/^level\s+([A-Za-z_][\w:.]*)\s*\{$/);
    if (!match) {
      continue;
    }
    const close = findLevel3dBlockClose(lines, index);
    if (!close || close.end > range.end) {
      continue;
    }
    definitions.push({
      name: match[1],
      start: lines[index].start,
      bodyStart: lines[index].end + 1,
      bodyEnd: close.start,
      end: close.end,
      nextIndex: close.index + 1,
      indent: lineIndent(lines[index].raw) || range.indent || "  ",
      bodyIndent: level3dDefinitionBodyIndent(lines, index, close.index, lineIndent(lines[index].raw) || range.indent || "  "),
    });
    index = close.index;
  }
  return definitions;
}

function findLevel3dDefinitionAtPosition(source, position) {
  const safePosition = Math.max(0, Math.trunc(Number(position) || 0));
  let levelIndex = 0;
  for (const range of findLevels3Ranges(source)) {
    for (const entry of findLevel3dDefinitions(source, range)) {
      const currentIndex = levelIndex;
      levelIndex += 1;
      if (safePosition < entry.start || safePosition > entry.end) {
        continue;
      }
      return {
        ...entry,
        bundle: range.bundle,
        model: range.model,
        levelIndex: currentIndex,
        rows: rowsForLevel3dDefinition(source, entry),
      };
    }
  }
  return null;
}

function findLevels3InsertionRange(source, bundle = "") {
  const requested = String(bundle || "").trim();
  const ranges = findLevels3Ranges(source);
  return (requested ? ranges.find((range) => range.bundle === requested) : null)
    || ranges[0]
    || null;
}

function insertLevel3d(source, name, levelData, bundle = "") {
  let workingSource = String(source || "");
  let range = findLevels3InsertionRange(workingSource, bundle);
  if (!range) {
    return "";
  }
  const prepared = prepareLevel3dLegendSourceForWrite(workingSource, range, levelData);
  if (!prepared) {
    return "";
  }
  workingSource = prepared.source;
  range = findLevels3InsertionRange(workingSource, bundle);
  if (!range) {
    return "";
  }
  const definitions = findLevel3dDefinitions(workingSource, range);
  const indent = definitions[0]?.indent || `${range.indent || ""}  `;
  const bodyIndent = definitions[0]?.bodyIndent || `${indent}  `;
  const levelSource = levelDefinition3dSource(name, prepared.levelData, indent, { bodyIndent }).trimEnd();
  return `${workingSource.slice(0, range.bodyEnd).trimEnd()}\n\n${levelSource}\n${workingSource.slice(range.bodyEnd)}`;
}

function replaceLevel3dByName(source, name, levelData, bundle = "") {
  const requested = sanitizeLevel3dName(name);
  const requestedBundle = String(bundle || "").trim();
  for (const range of findLevels3Ranges(source)) {
    if (requestedBundle && range.bundle !== requestedBundle) {
      continue;
    }
    const entry = findLevel3dDefinitions(source, range).find((candidate) => candidate.name === requested);
    if (!entry) {
      continue;
    }
    const prepared = prepareLevel3dLegendSourceForWrite(source, range, levelData);
    if (!prepared) {
      return null;
    }
    const updatedRange = findLevels3Ranges(prepared.source).find((candidate) => (
      candidate.bundle === range.bundle
      && candidate.model === range.model
    ));
    const updatedEntry = updatedRange
      ? findLevel3dDefinitions(prepared.source, updatedRange).find((candidate) => candidate.name === requested)
      : null;
    if (!updatedEntry) {
      return null;
    }
    const replacement = levelDefinition3dSource(requested, prepared.levelData, updatedEntry.indent, { bodyIndent: updatedEntry.bodyIndent }).trimEnd();
    return {
      source: replaceEditorSourceRangePreservingLineBoundary(prepared.source, updatedEntry.start, updatedEntry.end, replacement),
    };
  }
  return null;
}

function prepareLevel3dLegendSourceForWrite(source, range, levelData) {
  const prepared = level3dReconciledTemporaryLegendEntries(source, range, levelData);
  if (!prepared) {
    return null;
  }
  return {
    source: insertLevel3dLegendEntries(source, range, prepared.entries),
    levelData: prepared.levelData,
  };
}

function level3dReconciledTemporaryLegendEntries(source, range, levelData) {
  let { rows } = normalizeLevel3dSourceData(levelData);
  rows = [...rows];
  const existingEntries = sourceLevel3dLegendEntriesForRange(source, range);
  const existingByChar = new Map(existingEntries.map((entry) => [entry.char, entry]));
  const existingByObjects = new Map(existingEntries.map((entry) => [level3dObjectSetKey(entry.objects || []), entry.char]));
  const usedChars = new Set([
    LEVEL3D_EMPTY_CHAR,
    ...sourceLevel3dLegendEntries(source).map((entry) => entry.char),
    ...level3dUsedCharsInRows(rows),
  ]);
  const entries = [];
  for (const entry of level3dTemporaryLegendEntriesForLevelData({ rows })) {
    const objects = Array.isArray(entry.objects) ? entry.objects.filter(Boolean) : [];
    const key = level3dObjectSetKey(objects);
    const existingCharForObjects = existingByObjects.get(key);
    if (existingCharForObjects) {
      rows = replaceLevel3dRowsChar(rows, entry.char, existingCharForObjects);
      continue;
    }
    let ch = entry.char;
    const existingForChar = existingByChar.get(ch);
    if (existingForChar && level3dObjectSetKey(existingForChar.objects || []) !== key) {
      ch = nextTemporaryLevel3dLegendChar([...usedChars]);
      if (!ch) {
        return null;
      }
      rows = replaceLevel3dRowsChar(rows, entry.char, ch);
    }
    entries.push({ char: ch, objects });
    existingByChar.set(ch, { char: ch, objects });
    existingByObjects.set(key, ch);
    usedChars.add(ch);
  }
  return { entries, levelData: { ...normalizeLevel3dSourceData(levelData), rows } };
}

function replaceLevel3dRowsChar(rows, from, to) {
  if (!from || from === to) {
    return rows;
  }
  return rows.map((row) => String(row || "").split(from).join(to));
}

function insertLevel3dLegendEntries(source, range, entries) {
  if (!entries.length) {
    return source;
  }
  const legend = findLevel3dLegendBlock(source, range);
  if (legend) {
    const lines = entries.map((entry) => `${legend.bodyIndent}${entry.char} = ${entry.objects.join(" ")}`);
    return `${source.slice(0, legend.closeStart)}${lines.join("\n")}\n${source.slice(legend.closeStart)}`;
  }
  const indent = `${range.indent || ""}  `;
  const bodyIndent = `${indent}  `;
  const lines = [
    `${indent}legend {`,
    ...entries.map((entry) => `${bodyIndent}${entry.char} = ${entry.objects.join(" ")}`),
    `${indent}}`,
    "",
  ];
  return `${source.slice(0, range.bodyStart)}${lines.join("\n")}\n${source.slice(range.bodyStart)}`;
}

function findLevel3dLegendBlock(source, range) {
  const block = String(source || "").slice(range?.bodyStart || 0, range?.bodyEnd || 0);
  const match = /(^|\n)([\t ]*)legend\s*\{\n([\s\S]*?)\n\2\}/m.exec(block);
  if (!match) {
    return null;
  }
  const closeToken = `\n${match[2]}}`;
  const closeOffset = match[0].lastIndexOf(closeToken);
  if (closeOffset < 0) {
    return null;
  }
  const closeStart = (range?.bodyStart || 0) + match.index + closeOffset + 1;
  const bodyIndent = level3dLegendBodyIndent(match[3], match[2]);
  return { closeStart, bodyIndent };
}

function level3dLegendBodyIndent(body, indent) {
  for (const line of String(body || "").split("\n")) {
    if (!level3dScannerCode(line)) {
      continue;
    }
    const rowIndent = lineIndent(line);
    if (rowIndent.length > indent.length) {
      return rowIndent;
    }
    break;
  }
  return `${indent}  `;
}

function level3dDefinitionBodyIndent(lines, headerIndex, closeIndex, indent) {
  for (let index = headerIndex + 1; index < closeIndex; index += 1) {
    const line = lines[index];
    const code = level3dScannerCode(line.raw);
    if (!code) {
      continue;
    }
    const childIndent = lineIndent(line.raw);
    if (childIndent.startsWith(indent) && childIndent.length > indent.length) {
      return childIndent;
    }
    break;
  }
  return `${indent}  `;
}

function currentLevel3dSourceLocation() {
  const document = level3dSourceDocument();
  const source = level3dEditorSource(document);
  const name = sanitizeLevel3dName(level3dNameInput?.value || currentLevel3dEntry()?.name || "");
  const bundle = sanitizeLevel3dBundle(level3dBundleInput?.value || "");
  for (const range of findLevels3Ranges(source)) {
    if (bundle && range.bundle !== bundle) {
      continue;
    }
    const entry = findLevel3dDefinitions(source, range).find((candidate) => candidate.name === name);
    if (entry) {
      return { document, start: entry.start, key: `${range.bundle}:${entry.name}` };
    }
  }
  const range = findLevels3InsertionRange(source, bundle);
  return range ? { document, start: range.start, key: `${range.bundle}:levels3` } : null;
}

function loadLevel3dFromSourcePosition(position, options = {}) {
  if (!isPuzzleDocument(activeDocument()) || !isTextDocument(activeDocument())) {
    return null;
  }
  const source = sourceEditor.value || "";
  const entry = findLevel3dDefinitionAtPosition(source, position);
  if (!entry) {
    return null;
  }
  return loadLevel3dSourceDefinition(entry, source, options);
}

function loadLevel3dSourceTarget(target, options = {}) {
  if (!isPuzzleDocument(activeDocument()) || !isTextDocument(activeDocument())) {
    return null;
  }
  const source = sourceEditor.value || "";
  const entry = Number.isInteger(target?.bodyStart) && Number.isInteger(target?.bodyEnd)
    ? sourceEditableEntryFromTarget(source, target, {
      find: findLevel3dDefinitionAtPosition,
      defaultName: "level_1",
      body: (_source, entry) => ({
        bundle: entry.bundle || entry.params?.bundle || "",
        model: entry.model || entry.params?.model || "",
        rows: rowsForLevel3dDefinition(_source, entry),
      }),
    })
    : findLevel3dDefinitionAtPosition(source, target?.start ?? 0);
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
  if (!sourceDocument || sourceDocument.id === activeDocument()?.id) {
    ensurePreviewTargetsActiveDocument();
  }
  const exportData = options.exportData || previewExport || extractPreviewExport(latestHtml);
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
    level3dNameInput.value = entry.name || "level_1";
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

function level3dSourceLinesWithOffsets(source) {
  return editorSourceLinesWithOffsets(source);
}

function level3dScannerCode(line) {
  return String(line || "").split("//", 1)[0].trim();
}

function renderLevel3dRuntime() {
  if (!level3dRuntimeFrame) {
    renderLevel3dStageOverlay();
    return;
  }
  const update = level3dRuntimePreviewUpdate();
  if (!latestHtml || !update) {
    showBlankLevel3dRuntimeFrame(level3dRuntimeFrame);
    level3dRuntimeFrameLoaded = false;
    level3dRuntimeFrameKey = "";
    level3dStageRendererView = null;
    setLevel3dActionStatus(latestHtml ? "Load a 3D level first" : "Run Preview first", "");
    return;
  }
  const key = `${activePreviewDocument()?.id || ""}:${latestHtml.length}:${currentEditableLevelIndex()}`;
  if (level3dRuntimeFrameKey !== key) {
    level3dRuntimeFrameLoaded = false;
    level3dStageRendererView = null;
    level3dRuntimeFrameKey = key;
    level3dRuntimeFrame.addEventListener("load", () => {
      level3dRuntimeFrameLoaded = true;
      sendLevel3dSnapshotToRuntime();
    }, { once: true });
    level3dRuntimeFrame.srcdoc = level3dRuntimePreviewDocument(update);
    return;
  }
  sendLevel3dSnapshotToRuntime();
  sendLevel3dLayerSnapshotToRuntime();
}

function sendLevel3dSnapshotToRuntime() {
  if (!level3dRuntimeFrameLoaded || !level3dRuntimeFrame?.contentWindow) {
    return;
  }
  const update = level3dRuntimePreviewUpdate();
  if (!update) {
    showBlankLevel3dRuntimeFrame(level3dRuntimeFrame);
    level3dRuntimeFrameLoaded = false;
    level3dRuntimeFrameKey = "";
    return;
  }
  level3dRuntimeFrame.contentWindow.postMessage(level3dPreviewSurfaceMessage(update), "*");
}

function refreshLevel3dRuntimePreviews() {
  level3dStageRendererView = null;
  sendLevel3dSnapshotToRuntime();
  sendLevel3dLayerSnapshotToRuntime();
}

function startLevel3dPlaytest() {
  if (level3dPlaytestActive) {
    return;
  }
  const snapshot = level3dRuntimeSnapshot();
  if (!latestHtml || !snapshot) {
    setLevel3dActionStatus(latestHtml ? "Load a 3D level first" : "Run Preview first", "is-error");
    return;
  }
  if (typeof clearSolutionPreview === "function") {
    clearSolutionPreview();
  }
  level3dPlaytestSnapshot = level3dNormalizePlaytestSnapshot(snapshot);
  level3dPlaytestActive = true;
  level3dStageHit = null;
  updateLevel3dPlaytestControls();
  renderLevel3dStageOverlay();
  refreshLevel3dRuntimePreviews();
  requestLevel3dPlaytestState();
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
  level3dPlaytestSnapshot = null;
  level3dStageHit = null;
  updateLevel3dPlaytestControls();
  renderLevel3dStageOverlay();
  if (options.syncPreview !== false) {
    refreshLevel3dRuntimePreviews();
  }
  setLevel3dActionStatus("Stopped 3D level play", "");
}

function toggleLevel3dPlaytest() {
  if (level3dPlaytestActive) {
    stopLevel3dPlaytest();
  } else {
    startLevel3dPlaytest();
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
    level3dCameraZoomScrub,
    level3dOriginXScrub,
    level3dOriginYScrub,
    level3dOriginZScrub,
    level3dResetPreviewButton,
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
  if (level3dStageOverlay) {
    level3dStageOverlay.focus?.({ preventScroll: true });
    return;
  }
  level3dStageCanvas?.focus?.({ preventScroll: true });
}

function level3dPlaytestFrameWindow() {
  if (level3dRuntimeFrame) {
    return level3dRuntimeFrameLoaded ? level3dRuntimeFrame.contentWindow : null;
  }
  return previewFrame?.contentWindow || null;
}

function requestLevel3dPlaytestState() {
  const target = level3dPlaytestFrameWindow();
  if (!target) {
    return false;
  }
  target.postMessage({ type: "PuzzleStudioRequestPuzzle3State" }, "*");
  return true;
}

function sendLevel3dPlaytestKey(event) {
  if (!level3dPlaytestActive) {
    return false;
  }
  const target = level3dPlaytestFrameWindow();
  if (!target) {
    setLevel3dActionStatus("3D play runtime is not ready", "is-error");
    return false;
  }
  if (event.code === "KeyZ") {
    target.postMessage({ type: "PuzzleStudioCommand", command: "undo" }, "*");
  } else if (event.code === "KeyR") {
    target.postMessage({ type: "PuzzleStudioCommand", command: "restart" }, "*");
  } else {
    target.postMessage({
      type: "PuzzleStudioKey",
      key: event.key,
      code: event.code,
    }, "*");
  }
  event.preventDefault();
  event.stopPropagation();
  return true;
}

function handleLevel3dPlaytestStateMessage(event) {
  if (!level3dPlaytestActive || event.data?.type !== "PuzzleStudioPuzzle3State") {
    return;
  }
  const source = String(event.data.source || "");
  if (source && source !== level3dModelPreviewComponent().source) {
    return;
  }
  level3dPlaytestSnapshot = level3dNormalizePlaytestSnapshot(event.data.snapshot);
  renderLevel3dStageOverlay();
  sendLevel3dLayerSnapshotToRuntime();
}

function level3dNormalizePlaytestSnapshot(snapshot) {
  const next = JSON.parse(JSON.stringify(snapshot || {}));
  next.__kind = "puzzle3d";
  const levelCount = Array.isArray(next.levels) && next.levels.length ? next.levels.length : 1;
  const levelIndex = Math.max(0, Math.min(levelCount - 1, Math.trunc(Number(next.levelIndex) || 0)));
  next.levelIndex = levelIndex;
  const size = next.size || next.levels?.[levelIndex]?.size || {};
  const cells = Array.isArray(next.cells)
    ? JSON.parse(JSON.stringify(next.cells))
    : JSON.parse(JSON.stringify(next.levels?.[levelIndex]?.cells || []));
  next.size = { ...size };
  next.cells = cells;
  if (!Array.isArray(next.levels) || !next.levels.length) {
    next.levels = [{
      name: level3dNameInput?.value || "level_1",
      label: level3dNameInput?.value || "Level 1",
      size: { ...next.size },
      cells,
    }];
    next.levelIndex = 0;
  } else if (next.levels[levelIndex]) {
    next.levels[levelIndex] = {
      ...next.levels[levelIndex],
      size: { ...next.size },
      cells,
    };
  }
  return next;
}

function renderLevel3dLayerRuntime() {
  if (!level3dLayerFrame) {
    return;
  }
  const update = level3dLayerRuntimePreviewUpdate();
  if (!latestHtml || !update) {
    showBlankLevel3dRuntimeFrame(level3dLayerFrame);
    level3dLayerFrameLoaded = false;
    level3dLayerFrameKey = "";
    return;
  }
  const key = `${activePreviewDocument()?.id || ""}:puzzle3-layer-renderer:${currentLevel3dLayerZ()}`;
  if (level3dLayerFrameKey !== key) {
    level3dLayerFrameLoaded = false;
    level3dLayerRendererView = null;
    level3dLayerFrameKey = key;
    level3dLayerFrame.addEventListener("load", () => {
      level3dLayerFrameLoaded = true;
      sendLevel3dLayerSnapshotToRuntime();
    }, { once: true });
    level3dLayerFrame.srcdoc = level3dRuntimePreviewDocument(update);
    return;
  }
  sendLevel3dLayerSnapshotToRuntime();
}

function sendLevel3dLayerSnapshotToRuntime() {
  if (!level3dLayerFrameLoaded || !level3dLayerFrame?.contentWindow) {
    return;
  }
  const update = level3dLayerRuntimePreviewUpdate();
  if (!update) {
    showBlankLevel3dRuntimeFrame(level3dLayerFrame);
    level3dLayerFrameLoaded = false;
    level3dLayerFrameKey = "";
    return;
  }
  level3dLayerFrame.contentWindow.postMessage(level3dPreviewSurfaceMessage(update), "*");
}

function level3dRuntimePreviewUpdate() {
  const exportData = previewExport || extractPreviewExport(latestHtml);
  const snapshot = level3dRuntimeSnapshot();
  if (!snapshot && !isPuzzle3dExport(exportData)) {
    return null;
  }
  if (!snapshot) {
    return null;
  }
  const levelCount = Array.isArray(snapshot.levels) && snapshot.levels.length ? snapshot.levels.length : 1;
  const levelIndex = Math.max(0, Math.min(levelCount - 1, Math.trunc(Number(snapshot.levelIndex) || 0)));
  const levelEntry = snapshot.levels?.[levelIndex] || {};
  const size = snapshot.size || levelEntry.size || exportData?.size;
  const cells = Array.isArray(snapshot.cells)
    ? snapshot.cells
    : Array.isArray(levelEntry.cells)
      ? levelEntry.cells
      : exportData?.cells || [];
  return {
    levelIndex,
    level: {
      name: levelEntry.name || level3dNameInput?.value || "level_1",
      label: levelEntry.label || levelEntry.name || level3dNameInput?.value || "Level 1",
      size: size ? { ...size } : undefined,
      cells: JSON.parse(JSON.stringify(cells)),
    },
    resources: level3dRuntimePreviewResources(snapshot),
    camera: level3dRuntimePreviewCamera(snapshot),
    view: level3dRuntimePreviewView(snapshot),
    settings: level3dPreviewSettings(snapshot.settings || {}),
    component: level3dModelPreviewComponent(),
    componentEmbed: true,
  };
}

function level3dRuntimePreviewCamera(source) {
  const camera = level3dPreviewCamera(source);
  return {
    yawDegrees: camera.yawDegrees,
    pitchDegrees: camera.pitchDegrees,
    zoom: camera.zoom,
  };
}

function level3dRuntimePreviewView(source) {
  const camera = level3dPreviewCamera(source);
  const origin = level3dPreviewOriginState();
  return {
    zoom: camera.zoom,
    target: {
      x: origin.x,
      y: origin.y,
      z: origin.z,
    },
  };
}

function level3dLayerRuntimePreviewUpdate() {
  const snapshot = level3dLayerRuntimeSnapshot();
  if (!snapshot) {
    return null;
  }
  return {
    levelIndex: snapshot.levelIndex || 0,
    level: {
      name: snapshot.levels?.[snapshot.levelIndex || 0]?.name || level3dNameInput?.value || "layer",
      label: snapshot.levels?.[snapshot.levelIndex || 0]?.label || level3dNameInput?.value || "Layer",
      size: { ...(snapshot.size || {}) },
      cells: JSON.parse(JSON.stringify(snapshot.cells || [])),
    },
    resources: level3dRuntimePreviewResources(snapshot),
    camera: snapshot.camera,
    settings: snapshot.settings || {},
    component: level3dModelPreviewComponent(),
    componentEmbed: true,
  };
}

function level3dPreviewSurfaceMessage(update) {
  return {
    type: LEVEL3D_PREVIEW_SURFACE_MESSAGE,
    kind: LEVEL3D_PREVIEW_SURFACE_KIND,
    mode: LEVEL3D_PREVIEW_SURFACE_MODE,
    payload: level3dPreviewSurfacePayload(update),
  };
}

function level3dPreviewSurfacePayload(update = {}) {
  const view = update.view || {};
  return {
    levelIndex: update.levelIndex,
    level: update.level,
    resources: update.resources,
    view: {
      ...(update.camera || {}),
      ...(view || {}),
    },
    display: update.settings || {},
  };
}

function level3dModelPreviewComponent() {
  return { kind: "puzzle3", source: "__editor_level3d_preview__" };
}

function level3dRuntimePreviewResources(exportData = previewExport || extractPreviewExport(latestHtml)) {
  return {
    layerCount: exportData?.layerCount,
    objects: exportData?.objects || {},
    sprites: level3dPreviewSprites(exportData),
  };
}

function level3dPreviewSprites(exportData = previewExport || extractPreviewExport(latestHtml), source = level3dEditorSource()) {
  return {
    ...(exportData?.sprites || {}),
    ...sourceLevel3dSprites(source),
  };
}

function sourceLevel3dSprites(source) {
  const sprites = {};
  for (const block of sourceLevel3dSpriteBlocks(source)) {
    Object.assign(sprites, sourceLevel3dSpritesFromBlock(String(source || "").slice(block.bodyStart, block.bodyEnd)));
  }
  return sprites;
}

function sourceLevel3dSpriteBlocks(source) {
  const text = String(source || "");
  const pattern = /(^|\n)([\t ]*)sprites3(?:\s+[^\n{]+)?\s*\{/gm;
  const blocks = [];
  let match = null;
  while ((match = pattern.exec(text))) {
    const start = match.index + match[1].length;
    const openIndex = text.indexOf("{", start);
    const closeIndex = level3dMatchingBrace(text, openIndex);
    if (openIndex < 0 || closeIndex < 0) {
      continue;
    }
    blocks.push({ bodyStart: openIndex + 1, bodyEnd: closeIndex });
    pattern.lastIndex = closeIndex + 1;
  }
  return blocks;
}

function sourceLevel3dSpritesFromBlock(block) {
  const lines = String(block || "")
    .split("\n")
    .map((raw) => level3dScannerCode(raw).trim());
  const sprites = {};
  for (let index = 0; index < lines.length; index += 1) {
    const name = lines[index];
    if (!isLevel3dSpriteName(name) || !isLevel3dSpritePaletteRow(nextLevel3dNonEmptyLine(lines, index + 1))) {
      continue;
    }
    let paletteIndex = index + 1;
    while (paletteIndex < lines.length && !lines[paletteIndex]) {
      paletteIndex += 1;
    }
    const palette = level3dSpritePaletteFromLine(lines[paletteIndex]);
    if (!palette) {
      continue;
    }
    const bitmap = [];
    for (let rowIndex = paletteIndex + 1; rowIndex < lines.length; rowIndex += 1) {
      const row = lines[rowIndex];
      if (isLevel3dSpriteName(row) && isLevel3dSpritePaletteRow(nextLevel3dNonEmptyLine(lines, rowIndex + 1))) {
        break;
      }
      bitmap.push(row);
    }
    if (bitmap.some((row) => row.length > 0) && level3dSpriteBitmapUsesPalette(bitmap, palette)) {
      sprites[name] = { palette, bitmap };
    }
  }
  return sprites;
}

function level3dSpriteBitmapUsesPalette(bitmap, palette) {
  const keys = new Set(Object.keys(palette || {}));
  for (const row of bitmap || []) {
    for (const char of String(row || "")) {
      if (char !== "." && char !== " " && !keys.has(char)) {
        return false;
      }
    }
  }
  return true;
}

function level3dMatchingBrace(text, openIndex) {
  if (openIndex < 0 || text[openIndex] !== "{") {
    return -1;
  }
  let depth = 0;
  for (let index = openIndex; index < text.length; index += 1) {
    const char = text[index];
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

function nextLevel3dNonEmptyLine(lines, start) {
  for (let index = start; index < lines.length; index += 1) {
    if (lines[index]) {
      return lines[index];
    }
  }
  return "";
}

function isLevel3dSpriteName(value) {
  return /^@?[A-Za-z_][\w:]*$/.test(String(value || "")) && !isLevel3dSpritePaletteRow(value);
}

function isLevel3dSpritePaletteRow(value) {
  const tokens = String(value || "").split(/\s+/).filter(Boolean);
  return tokens.length > 0 && tokens.every((token) => Boolean(level3dSpriteColorToken(token)));
}

function level3dSpritePaletteFromLine(line) {
  const keys = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
  const palette = {};
  const tokens = String(line || "").split(/\s+/).filter(Boolean);
  if (!tokens.length || tokens.length > keys.length) {
    return null;
  }
  for (const [index, token] of tokens.entries()) {
    const color = level3dSpriteColorToken(token);
    if (!color) {
      return null;
    }
    palette[keys[index]] = color;
  }
  return palette;
}

function level3dSpriteColorToken(token) {
  const value = String(token || "");
  if (value.toLowerCase() === "transparent") {
    return "transparent";
  }
  return /^#(?:[0-9a-fA-F]{3,4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$/.test(value) ? value : null;
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
    }));
  const next = JSON.parse(JSON.stringify(snapshot));
  next.camera = level3dLayerCamera();
  next.settings = level3dLayerSettings(snapshot.settings || {});
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

function level3dLayerCamera() {
  return { yawDegrees: 0, pitchDegrees: 90, zoom: 1, projection: "orthographic" };
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

function renderLevel3dLayerOverlay() {
  if (!(level3dLayerOverlay instanceof HTMLCanvasElement) || !level3dLayerBoard) {
    return;
  }
  if (level3dLayerBoard.classList.contains("is-grid-board")) {
    const ctx = level3dLayerOverlay.getContext("2d");
    if (ctx) {
      ctx.clearRect(0, 0, level3dLayerOverlay.width, level3dLayerOverlay.height);
    }
    return;
  }
  const metrics = level3dScaledSurfaceMetrics(level3dLayerOverlay);
  const scale = window.devicePixelRatio || 1;
  const width = Math.max(1, Math.floor(metrics.width * scale));
  const height = Math.max(1, Math.floor(metrics.height * scale));
  if (level3dLayerOverlay.width !== width || level3dLayerOverlay.height !== height) {
    level3dLayerOverlay.width = width;
    level3dLayerOverlay.height = height;
  }
  const ctx = level3dLayerOverlay.getContext("2d");
  if (!ctx) {
    return;
  }
  ctx.setTransform(scale, 0, 0, scale, 0, 0);
  ctx.clearRect(0, 0, metrics.width, metrics.height);
  drawLevel3dLayerHover(ctx, metrics.width, metrics.height);
}

function drawLevel3dLayerHover(ctx, width, height) {
  if (!level3dLayerHover || !level3dPositionInBounds(level3dLayerHover)) {
    return;
  }
  const points = level3dLayerCellScreenPoints(level3dLayerHover, width, height);
  if (!points?.length) {
    return;
  }
  ctx.save();
  ctx.fillStyle = level3dSelectedEntry()?.objects?.length
    ? "rgba(77, 171, 218, 0.18)"
    : "rgba(230, 96, 105, 0.16)";
  ctx.strokeStyle = level3dSelectedEntry()?.objects?.length
    ? "rgba(39, 107, 143, 0.88)"
    : "rgba(180, 59, 67, 0.88)";
  ctx.lineWidth = 2;
  ctx.beginPath();
  ctx.moveTo(points[0].x, points[0].y);
  for (const point of points.slice(1)) {
    ctx.lineTo(point.x, point.y);
  }
  ctx.closePath();
  ctx.fill();
  ctx.stroke();
  ctx.restore();
}

function level3dLayerCellScreenPoints(position, width, height) {
  const footprint = level3dLayerCellFootprint(position);
  if (!footprint) {
    return null;
  }
  const view = level3dLayerTopDownView(width, height);
  return footprint.points.map((point) => level3dLayerFootprintPointToScreen(point, view, width, height));
}

function level3dLayerPositionFromEvent(event) {
  if (!level3dLayerBoard) {
    return null;
  }
  const cell = document.elementFromPoint(event.clientX, event.clientY)?.closest?.(".level3d-layer-cell");
  if (cell && level3dLayerBoard.contains(cell)) {
    const position = {
      x: Math.trunc(Number(cell.dataset.x) || 0),
      y: Math.trunc(Number(cell.dataset.y) || 0),
      z: Math.trunc(Number(cell.dataset.z) || currentLevel3dLayerZ()),
    };
    return level3dPositionInBounds(position) ? position : null;
  }
  const surface = level3dLayerOverlay instanceof HTMLCanvasElement
    ? level3dLayerOverlay
    : (level3dLayerFrame || level3dLayerBoard);
  const point = level3dEventPointInScaledSurface(event, surface);
  return level3dLayerPositionAt(point.x, point.y, point.width, point.height);
}

function level3dLayerPositionAt(x, y, width, height) {
  const view = level3dLayerTopDownView(width, height);
  if (!view.cellFootprints?.length) {
    return null;
  }
  const pointer = level3dLayerScreenPointToFootprint({ x, y }, view, width, height);
  for (const footprint of view.cellFootprints) {
    if (!level3dPointInPolygon(pointer, footprint.points)) {
      continue;
    }
    const position = {
      x: footprint.position.x,
      y: footprint.position.y,
      z: currentLevel3dLayerZ(),
    };
    return level3dPositionInBounds(position) ? position : null;
  }
  return null;
}

function level3dLayerCellFootprint(position) {
  if (!level3dLayerRendererView?.cellFootprintMap) {
    return null;
  }
  return level3dLayerRendererView.cellFootprintMap.get(`${position.x},${position.y}`) || null;
}

function level3dLayerUsesRuntimeScreenFootprints(view) {
  return view?.coordinateSpace === "canvas-css-px";
}

function level3dLayerScreenPointToFootprint(point, view, width, height) {
  if (level3dLayerUsesRuntimeScreenFootprints(view)) {
    const sourceWidth = Math.max(1, Number(view.width) || 1);
    const sourceHeight = Math.max(1, Number(view.height) || 1);
    const viewportWidth = Math.max(1, Number(view.viewport?.width) || Number(view.canvasRect?.width) || sourceWidth);
    const viewportHeight = Math.max(1, Number(view.viewport?.height) || Number(view.canvasRect?.height) || sourceHeight);
    const targetWidth = Math.max(1, Number(width) || viewportWidth);
    const targetHeight = Math.max(1, Number(height) || viewportHeight);
    const canvasRect = level3dLayerRuntimeCanvasRect(view, viewportWidth, viewportHeight);
    const viewportPoint = {
      x: Number(point.x) * viewportWidth / targetWidth,
      y: Number(point.y) * viewportHeight / targetHeight,
    };
    return {
      x: (viewportPoint.x - canvasRect.x) * sourceWidth / canvasRect.width,
      y: (viewportPoint.y - canvasRect.y) * sourceHeight / canvasRect.height,
    };
  }
  const transform = level3dLayerViewTransform(view, width, height);
  return {
    x: (Number(point.x) - transform.offsetX) / transform.scale,
    y: (Number(point.y) - transform.offsetY) / transform.scale,
  };
}

function level3dLayerFootprintPointToScreen(point, view, width, height) {
  if (level3dLayerUsesRuntimeScreenFootprints(view)) {
    const sourceWidth = Math.max(1, Number(view.width) || 1);
    const sourceHeight = Math.max(1, Number(view.height) || 1);
    const viewportWidth = Math.max(1, Number(view.viewport?.width) || Number(view.canvasRect?.width) || sourceWidth);
    const viewportHeight = Math.max(1, Number(view.viewport?.height) || Number(view.canvasRect?.height) || sourceHeight);
    const targetWidth = Math.max(1, Number(width) || viewportWidth);
    const targetHeight = Math.max(1, Number(height) || viewportHeight);
    const canvasRect = level3dLayerRuntimeCanvasRect(view, viewportWidth, viewportHeight);
    const viewportPoint = {
      x: canvasRect.x + Number(point.x) * canvasRect.width / sourceWidth,
      y: canvasRect.y + Number(point.y) * canvasRect.height / sourceHeight,
    };
    return {
      x: viewportPoint.x * targetWidth / viewportWidth,
      y: viewportPoint.y * targetHeight / viewportHeight,
    };
  }
  const transform = level3dLayerViewTransform(view, width, height);
  return level3dLayerTransformViewPoint(point, transform);
}

function level3dLayerRuntimeCanvasRect(view, viewportWidth, viewportHeight) {
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

function level3dLayerTransformViewPoint(point, transform) {
  return {
    x: transform.offsetX + point.x * transform.scale,
    y: transform.offsetY + point.y * transform.scale,
  };
}

function level3dPointInPolygon(point, polygon) {
  if (!Array.isArray(polygon) || polygon.length < 3) {
    return false;
  }
  let inside = false;
  for (let index = 0, previous = polygon.length - 1; index < polygon.length; previous = index, index += 1) {
    const a = polygon[index];
    const b = polygon[previous];
    if (!level3dPointOnSegment(point, a, b)) {
      const crosses = (a.y > point.y) !== (b.y > point.y)
        && point.x < ((b.x - a.x) * (point.y - a.y)) / ((b.y - a.y) || 0.000001) + a.x;
      if (crosses) {
        inside = !inside;
      }
    } else {
      return true;
    }
  }
  return inside;
}

function level3dPointOnSegment(point, a, b) {
  const cross = (point.y - a.y) * (b.x - a.x) - (point.x - a.x) * (b.y - a.y);
  if (Math.abs(cross) > 0.001) {
    return false;
  }
  const dot = (point.x - a.x) * (b.x - a.x) + (point.y - a.y) * (b.y - a.y);
  if (dot < -0.001) {
    return false;
  }
  const lengthSquared = (b.x - a.x) ** 2 + (b.y - a.y) ** 2;
  return dot <= lengthSquared + 0.001;
}

function level3dLayerViewTransform(view, width, height) {
  const targetWidth = Math.max(1, width || view.width || 1);
  const targetHeight = Math.max(1, height || view.height || 1);
  const sourceWidth = Math.max(1, Number(view.width) || targetWidth);
  const sourceHeight = Math.max(1, Number(view.height) || targetHeight);
  const scale = Math.max(0.0001, Math.min(targetWidth / sourceWidth, targetHeight / sourceHeight));
  return {
    scale,
    offsetX: (targetWidth - sourceWidth * scale) / 2,
    offsetY: (targetHeight - sourceHeight * scale) / 2,
  };
}

function level3dLayerTopDownView(width, height) {
  if (level3dLayerRendererView) {
    return level3dLayerRendererView;
  }
  const gridWidth = Math.max(1, Math.trunc(Number(level3d.width) || 1));
  const gridDepth = Math.max(1, Math.trunc(Number(level3d.depth) || 1));
  const margin = 18;
  const availableWidth = Math.max(1, width - margin * 2);
  const availableHeight = Math.max(1, height - margin * 2);
  return {
    width,
    height,
    centerX: (gridWidth - 1) / 2,
    centerY: (gridDepth - 1) / 2,
    originX: width / 2,
    originY: height / 2,
    scale: Math.max(0.0001, Math.min(availableWidth / gridWidth, availableHeight / gridDepth)),
  };
}

function handleLevel3dLayerRendererViewMessage(event) {
  if (event.source !== level3dLayerFrame?.contentWindow) {
    return;
  }
  if (event.data?.type !== "PuzzleStudioPuzzle3View") {
    return;
  }
  const view = event.data.view || {};
  const scale = Number(view.scale);
  const width = Number(view.width);
  const height = Number(view.height);
  if (!Number.isFinite(scale) || scale <= 0 || !Number.isFinite(width) || !Number.isFinite(height)) {
    return;
  }
  const cellFootprints = level3dNormalizeLayerCellFootprints(view.cellFootprints);
  level3dLayerRendererView = {
    width: Math.max(1, width),
    height: Math.max(1, height),
    coordinateSpace: view.coordinateSpace === "canvas-css-px" ? "canvas-css-px" : "",
    viewport: {
      width: Math.max(1, Number(view.viewport?.width) || width),
      height: Math.max(1, Number(view.viewport?.height) || height),
    },
    canvasRect: {
      x: Number(view.canvasRect?.x) || 0,
      y: Number(view.canvasRect?.y) || 0,
      width: Math.max(1, Number(view.canvasRect?.width) || width),
      height: Math.max(1, Number(view.canvasRect?.height) || height),
    },
    originX: Number(view.originX) || 0,
    originY: Number(view.originY) || 0,
    scale,
    centerX: Number(view.center?.x) || 0,
    centerY: Number(view.center?.y) || 0,
    cellFootprints,
    cellFootprintMap: new Map(cellFootprints.map((footprint) => [
      `${footprint.position.x},${footprint.position.y}`,
      footprint,
    ])),
  };
  renderLevel3dLayerOverlay();
}

function handleLevel3dStageRendererViewMessage(event) {
  if (event.source !== level3dRuntimeFrame?.contentWindow) {
    return;
  }
  if (event.data?.type !== "PuzzleStudioPuzzle3View") {
    return;
  }
  const normalized = level3dNormalizeRuntimeProjectionView(event.data.view);
  if (!normalized) {
    return;
  }
  level3dStageRendererView = normalized;
  renderLevel3dStageOverlay();
}

function level3dNormalizeRuntimeProjectionView(view) {
  const scale = Number(view?.scale);
  const width = Number(view?.width);
  const height = Number(view?.height);
  if (!Number.isFinite(scale) || scale <= 0 || !Number.isFinite(width) || !Number.isFinite(height)) {
    return null;
  }
  return {
    width: Math.max(1, width),
    height: Math.max(1, height),
    coordinateSpace: view.coordinateSpace === "canvas-css-px" ? "canvas-css-px" : "",
    viewport: {
      width: Math.max(1, Number(view.viewport?.width) || width),
      height: Math.max(1, Number(view.viewport?.height) || height),
    },
    canvasRect: {
      x: Number(view.canvasRect?.x) || 0,
      y: Number(view.canvasRect?.y) || 0,
      width: Math.max(1, Number(view.canvasRect?.width) || width),
      height: Math.max(1, Number(view.canvasRect?.height) || height),
    },
    originX: Number(view.originX) || 0,
    originY: Number(view.originY) || 0,
    scale,
    center: {
      x: Number(view.center?.x) || 0,
      y: Number(view.center?.y) || 0,
      z: Number(view.center?.z) || 0,
    },
    camera: {
      yawDegrees: Number(view.camera?.yawDegrees ?? 0),
      pitchDegrees: Number(view.camera?.pitchDegrees ?? 35),
      zoom: 1,
    },
    threeProjection: level3dNormalizeThreeProjection(view.threeProjection),
  };
}

function level3dNormalizeThreeProjection(raw) {
  if (!raw || typeof raw !== "object") {
    return null;
  }
  const size = raw.size || {};
  const target = raw.target || {};
  const width = Math.max(1, Math.trunc(Number(size.width) || level3d.width || 1));
  const depth = Math.max(1, Math.trunc(Number(size.depth) || level3d.depth || 1));
  const height = Math.max(1, Math.trunc(Number(size.height) || level3d.height || 1));
  const distance = Number(raw.distance);
  const visibleHeight = Number(raw.visibleHeight);
  return {
    size: { width, depth, height },
    target: {
      x: Number(target.x) || 0,
      y: Number(target.y) || 0,
      z: Number(target.z) || 0,
    },
    distance: Number.isFinite(distance) && distance > 0 ? distance : 1,
    visibleHeight: Number.isFinite(visibleHeight) && visibleHeight > 0 ? visibleHeight : 1,
    fovDegrees: Number(raw.fovDegrees) || 34,
    aspect: Math.max(0.01, Number(raw.aspect) || 1),
    projection: String(raw.projection || "").toLowerCase() === "orthographic" ? "orthographic" : "perspective",
  };
}

function level3dNormalizeLayerCellFootprints(raw) {
  if (!Array.isArray(raw)) {
    return [];
  }
  const footprints = [];
  for (const footprint of raw) {
    const position = {
      x: Math.trunc(Number(footprint?.position?.x) || 0),
      y: Math.trunc(Number(footprint?.position?.y) || 0),
    };
    const points = Array.isArray(footprint?.points)
      ? footprint.points
          .map((point) => ({
            x: Number(point?.x),
            y: Number(point?.y),
          }))
          .filter((point) => Number.isFinite(point.x) && Number.isFinite(point.y))
      : [];
    if (points.length >= 3) {
      footprints.push({ position, points });
    }
  }
  return footprints;
}

function ensureLevel3dStageOverlay() {
  if (level3dStageCanvas instanceof HTMLCanvasElement) {
    if (level3dStageOverlay !== level3dStageCanvas) {
      level3dStageOverlay = level3dStageCanvas;
    }
    if (!level3dStageCanvas.dataset.level3dBound) {
      level3dStageCanvas.dataset.level3dBound = "true";
      level3dStageCanvas.addEventListener("pointermove", handleLevel3dStagePointerMove);
      level3dStageCanvas.addEventListener("pointerleave", () => {
        level3dStageHit = null;
        renderLevel3dStageOverlay();
      });
      level3dStageCanvas.addEventListener("pointerdown", handleLevel3dStagePointerDown);
      level3dStageCanvas.addEventListener("keydown", handleLevel3dStageKeydown);
    }
    return level3dStageCanvas;
  }
  if (!level3dRuntimeFrame?.parentElement) {
    return null;
  }
  if (level3dStageOverlay && level3dStageOverlay.parentElement === level3dRuntimeFrame.parentElement) {
    return level3dStageOverlay;
  }
  const overlay = document.createElement("canvas");
  overlay.className = "level3d-stage-overlay";
  overlay.tabIndex = 0;
  overlay.setAttribute("aria-label", "3D placement surface");
  overlay.addEventListener("pointermove", handleLevel3dStagePointerMove);
  overlay.addEventListener("pointerleave", () => {
    level3dStageHit = null;
    renderLevel3dStageOverlay();
  });
  overlay.addEventListener("pointerdown", handleLevel3dStagePointerDown);
  overlay.addEventListener("keydown", handleLevel3dStageKeydown);
  level3dRuntimeFrame.parentElement.append(overlay);
  level3dStageOverlay = overlay;
  return overlay;
}

function renderLevel3dStageOverlay() {
  const overlay = ensureLevel3dStageOverlay();
  if (!overlay) {
    return;
  }
  const resizeMode = level3dStageResizeMode();
  overlay.classList.toggle("is-stage-resize-mode", Boolean(resizeMode));
  overlay.classList.toggle("is-stage-expand-mode", resizeMode === "expand");
  overlay.classList.toggle("is-stage-shrink-mode", resizeMode === "shrink");
  const metrics = level3dScaledSurfaceMetrics(overlay);
  const scale = window.devicePixelRatio || 1;
  const width = Math.max(1, Math.floor(metrics.width * scale));
  const height = Math.max(1, Math.floor(metrics.height * scale));
  if (overlay.width !== width || overlay.height !== height) {
    overlay.width = width;
    overlay.height = height;
  }
  const ctx = overlay.getContext("2d");
  if (!ctx) {
    return;
  }
  ctx.setTransform(scale, 0, 0, scale, 0, 0);
  ctx.clearRect(0, 0, metrics.width, metrics.height);
  if (!level3dStageHit?.polygon?.length) {
    return;
  }
  if (level3dIsStageResizeHit(level3dStageHit)) {
    drawLevel3dStageResizeHint(ctx, level3dStageHit, metrics.width, metrics.height);
    return;
  }
  ctx.save();
  ctx.lineJoin = "round";
  ctx.fillStyle = level3dSelectedEntry()?.objects?.length
    ? "rgba(39, 107, 143, 0.28)"
    : "rgba(180, 59, 67, 0.24)";
  ctx.strokeStyle = level3dSelectedEntry()?.objects?.length
    ? "rgba(77, 171, 218, 0.96)"
    : "rgba(230, 96, 105, 0.96)";
  ctx.lineWidth = 2;
  drawLevel3dPolygonPath(ctx, level3dStageHit.polygon);
  ctx.fill();
  ctx.stroke();
  ctx.restore();
}

function scheduleLevel3dSurfaceResize() {
  if (level3dSurfaceResizeFrame) {
    return;
  }
  level3dSurfaceResizeFrame = requestAnimationFrame(() => {
    level3dSurfaceResizeFrame = 0;
    syncLevel3dFrameLayout();
    if (!level3dBuilder?.hidden) {
      renderLevel3dLayerBoard();
    }
    level3dStageRendererView = null;
    level3dLayerRendererView = null;
    renderLevel3dStageOverlay();
    renderLevel3dLayerOverlay();
    level3dRuntimeFrame?.contentWindow?.postMessage({ type: "PuzzleStudioResize" }, "*");
    level3dLayerFrame?.contentWindow?.postMessage({ type: "PuzzleStudioResize" }, "*");
  });
}

function handleLevel3dStagePointerMove(event) {
  if (level3dPlaytestActive) {
    if (level3dStageHit) {
      level3dStageHit = null;
      renderLevel3dStageOverlay();
    }
    return;
  }
  const hit = level3dStageHitFromEvent(event);
  if (level3dStageHitKey(hit) === level3dStageHitKey(level3dStageHit)) {
    return;
  }
  level3dStageHit = hit;
  renderLevel3dStageOverlay();
}

function handleLevel3dStagePointerDown(event) {
  if (level3dPlaytestActive) {
    level3dStageOverlay?.focus();
    return;
  }
  if (event.button !== 0) {
    return;
  }
  const hit = level3dStageHitFromEvent(event);
  if (!hit) {
    return;
  }
  event.preventDefault();
  level3dStageOverlay?.focus();
  level3dStageHit = hit;
  applyLevel3dStageHit(hit);
}

function handleLevel3dStageKeydown(event) {
  if (level3dPlaytestActive) {
    sendLevel3dPlaytestKey(event);
    return;
  }
  if (handleLevel3dSliceHorizontalInput(event)) {
    return;
  }
  if ((event.key !== "Enter" && event.key !== " ") || !level3dStageHit) {
    return;
  }
  event.preventDefault();
  applyLevel3dStageHit(level3dStageHit);
}

function applyLevel3dStageHit(hit) {
  if (level3dIsStageResizeHit(hit)) {
    applyLevel3dStageResizeHit(hit);
    return;
  }
  const selected = level3dSelectedEntry();
  const selectedChar = selected?.char || level3d.selectedChar || level3dEmptyChar();
  const target = level3dStagePaintTarget(hit, selected);
  if (!target) {
    return;
  }
  if (withVisualEditHistory("level3d", () => paintLevel3dCellAtPosition(target, selectedChar))) {
    level3dStageHit = null;
    setLevel3dActionStatus(level3dCellLabel(selectedChar), "is-ok");
    renderLevel3dStageOverlay();
  }
}

function level3dStagePaintTarget(hit, selected = level3dSelectedEntry()) {
  if (!hit) {
    return null;
  }
  if (!selected?.objects?.length) {
    return hit.remove || null;
  }
  return level3dEditMode() === "add"
    ? hit.place || null
    : hit.replace || hit.remove || hit.place || null;
}

function level3dIsStageResizeHit(hit) {
  return hit?.mode === "expand"
    || hit?.mode === "shrink"
    || hit?.kind?.startsWith("expand-")
    || hit?.kind?.startsWith("shrink-");
}

function applyLevel3dStageResizeHit(hit) {
  const mode = hit?.mode || (hit?.kind?.startsWith("shrink-") ? "shrink" : "expand");
  const delta = mode === "shrink" ? -1 : 1;
  const edge = hit?.resizeEdge || hit?.edge;
  if (hit?.dimension === "width" || hit?.kind?.endsWith("-width")) {
    resizeLevel3dWidth((level3d.width || 1) + delta, { edge: edge || "right" });
    return;
  }
  if (hit?.dimension === "depth" || hit?.kind?.endsWith("-depth")) {
    resizeLevel3dDepth((level3d.depth || 1) + delta, { edge: edge || "back" });
    return;
  }
  if (hit?.dimension === "height" || hit?.kind?.endsWith("-height")) {
    resizeLevel3dHeight((level3d.height || 1) + delta, { edge: edge || "top" });
  }
}

function showBlankLevel3dRuntimeFrame(frame) {
  if (!frame) {
    return;
  }
  frame.srcdoc = level3dBlankRuntimeDocument();
}

function level3dBlankRuntimeDocument() {
  if (typeof emptyPreviewDocument === "function") {
    return emptyPreviewDocument();
  }
  return "<!doctype html><html><head><meta charset=\"utf-8\"></head><body></body></html>";
}

function level3dRuntimePreviewDocument(update) {
  const html = editorPreviewDocument(latestHtml);
  const payload = level3dPreviewSurfaceMessage(update);
  const json = JSON.stringify(payload)
    .replace(/</g, "\\u003c")
    .replace(/\u2028/g, "\\u2028")
    .replace(/\u2029/g, "\\u2029");
  const seedScript = `<script id="puzzle-studio-initial-model-preview">
(() => {
  const update = ${json};
  window.PuzzleStudioInitialPreviewSurface = update;
  window.PuzzleStudioModelComponentPreviewFixture = function(source, incoming = update) {
    const next = JSON.parse(JSON.stringify(source || {}));
    const payload = incoming.payload || incoming;
    const resources = payload.resources || incoming.resources || incoming;
    if (resources.layerCount != null) {
      next.layerCount = Math.max(1, Math.trunc(Number(resources.layerCount) || 1));
    }
    if (resources.objects && typeof resources.objects === "object") {
      next.objects = JSON.parse(JSON.stringify(resources.objects));
    }
    if (resources.sprites && typeof resources.sprites === "object") {
      next.sprites = JSON.parse(JSON.stringify(resources.sprites));
    }
    const level = payload.level || incoming.level || {};
    const size = level.size || payload.size || incoming.size || next.size || {};
    const cells = Array.isArray(level.cells) ? level.cells : Array.isArray(payload.cells) ? payload.cells : Array.isArray(incoming.cells) ? incoming.cells : next.cells || [];
    const rawLevelIndex = payload.levelIndex ?? incoming.levelIndex ?? next.levelIndex ?? 0;
    const levels = Array.isArray(next.levels) && next.levels.length ? next.levels : [{}];
    const levelIndex = Math.max(0, Math.min(levels.length - 1, Math.trunc(Number(rawLevelIndex) || 0)));
    const target = levels[levelIndex] || {};
    levels[levelIndex] = {
      ...target,
      name: level.name || target.name || "level_1",
      label: level.label || target.label || level.name || target.name || "Level 1",
      size: { ...size },
      cells: JSON.parse(JSON.stringify(cells)),
    };
    next.levels = levels;
    next.levelIndex = levelIndex;
    next.size = { ...size };
    next.cells = JSON.parse(JSON.stringify(cells));
    const view = payload.view || incoming.view || {};
    if (payload.camera || incoming.camera || view.yawDegrees != null || view.pitchDegrees != null) {
      next.camera = JSON.parse(JSON.stringify(payload.camera || incoming.camera || {
        yawDegrees: view.yawDegrees,
        pitchDegrees: view.pitchDegrees,
        zoom: view.zoom,
        projection: view.projection,
      }));
    }
    if (payload.view || incoming.view) {
      next.view = JSON.parse(JSON.stringify({
        zoom: view.zoom,
        target: view.target,
      }));
    }
    const display = payload.display || incoming.settings;
    if (display) {
      next.settings = { ...(next.settings || {}), ...JSON.parse(JSON.stringify(display)) };
    }
    const sceneName = incoming.scene || "__editor_model_preview__";
    next.scenes = [{
      name: sceneName,
      components: [incoming.component || { kind: "puzzle3", source: "__editor_model_preview__" }],
    }];
    next.currentScene = sceneName;
    return next;
  };
  let fixtureValue = typeof window.Puzzle3DFixture === "undefined"
    ? undefined
    : window.PuzzleStudioModelComponentPreviewFixture(window.Puzzle3DFixture, update);
  Object.defineProperty(window, "Puzzle3DFixture", {
    configurable: true,
    get() {
      return fixtureValue;
    },
    set(value) {
      fixtureValue = window.PuzzleStudioModelComponentPreviewFixture(value, update);
    },
  });
})();
<\/script>`;
  const bootScript = `<script id="puzzle-studio-initial-model-preview-boot">
(() => {
  const update = window.PuzzleStudioInitialPreviewSurface;
  if (!update || update.type !== "${LEVEL3D_PREVIEW_SURFACE_MESSAGE}") {
    return;
  }
  if (window.PuzzleStudioInitialPreviewSurfaceConsumed === true) {
    return;
  }
  if (typeof window.applyPuzzleStudioPreviewSurfaceUpdate === "function") {
    window.applyPuzzleStudioPreviewSurfaceUpdate(update);
    return;
  }
  if (typeof window.loadSnapshotData !== "function") {
    return;
  }
  const source = typeof window.PuzzleStudioModelComponentPreviewFixture === "function"
    ? window.PuzzleStudioModelComponentPreviewFixture(window.Puzzle3DFixture || {}, update)
    : JSON.parse(JSON.stringify(window.Puzzle3DFixture || {}));
  window.loadSnapshotData(source, { scene: source.scenes[0].name, preferPuzzleScene: false });
})();
<\/script>`;
  let next = html;
  if (next.includes("</head>")) {
    next = next.replace("</head>", `${seedScript}\n  </head>`);
  } else if (next.includes("<body")) {
    next = next.replace("<body", `${seedScript}\n<body`);
  } else {
    next = `${seedScript}\n${next}`;
  }
  if (next.includes("</body>")) {
    return next.replace("</body>", `${bootScript}\n  </body>`);
  }
  return `${next}\n${bootScript}`;
}

function level3dStageHitFromEvent(event) {
  const overlay = ensureLevel3dStageOverlay();
  if (!overlay) {
    return null;
  }
  const point = level3dEventPointInScaledSurface(event, overlay);
  return level3dStageHitAt(point.x, point.y, point.width, point.height);
}

function level3dStageHitKey(hit) {
  if (!hit) {
    return "";
  }
  const place = hit.place ? `${hit.place.x},${hit.place.y},${hit.place.z}` : "";
  const remove = hit.remove ? `${hit.remove.x},${hit.remove.y},${hit.remove.z}` : "";
  return `${hit.mode || ""}:${hit.kind}:${hit.edge || ""}:${place}:${remove}`;
}

function level3dSelectedEntry() {
  return level3dVisiblePaletteEntries().find((entry) => entry.char === level3d.selectedChar)
    || level3dVisiblePaletteEntries().find((entry) => entry.objects.length > 0)
    || level3dVisiblePaletteEntries()[0]
    || { char: level3dEmptyChar(), objects: [] };
}

function level3dRuntimeSnapshot() {
  if (level3dPlaytestActive && level3dPlaytestSnapshot) {
    return level3dSnapshotWithPreviewGrid(level3dPlaytestSnapshot);
  }
  const exportData = previewExport || extractPreviewExport(latestHtml);
  if (!isPuzzle3dExport(exportData)) {
    return fallbackLevel3dRuntimeSnapshot(exportData);
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

function level3dEditedSnapshotAppliesToLevel(exportData = previewExport, levelIndex = currentEditableLevelIndex(exportData)) {
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

function showPuzzle3dSolutionPreview(solution) {
  const steps = Array.isArray(solution.steps) ? solution.steps : [];
  if (!steps.length) {
    setLevelSolveStatus("Solved, but no steps were returned", "is-error");
    return;
  }
  const snapshot = puzzle3dSolutionStepSnapshot(steps[0]);
  if (!snapshot) {
    setLevelSolveStatus("3D solution steps did not include scene data", "is-error");
    return;
  }
  levelSolutionPreview = {
    kind: "puzzle3d",
    steps,
    moves: solutionMoves(solution),
    index: 0,
    snapshot,
  };
  updateSolutionControls();
  renderPuzzle3dSolverPreview();
  setLevelSolveStatus(solution.depth ? `Solved in ${solution.depth} moves` : "Already solved", "is-ok");
}

function setPuzzle3dSolutionStep(index) {
  if (!levelSolutionPreview || levelSolutionPreview.kind !== "puzzle3d") {
    return;
  }
  const nextIndex = Math.max(0, Math.min(levelSolutionPreview.steps.length - 1, index));
  levelSolutionPreview.index = nextIndex;
  levelSolutionPreview.snapshot = puzzle3dSolutionStepSnapshot(levelSolutionPreview.steps[nextIndex]);
  updateSolutionControls();
  renderPuzzle3dSolverPreview();
}

function renderPuzzle3dSolverPreview() {
  if (!solverBoardViewport || !latestHtml) {
    clearPuzzle3dSolverPreview();
    return false;
  }
  const snapshot = puzzle3dSolverPreviewSnapshot();
  const update = level3dPreviewUpdateFromSnapshot(snapshot);
  if (!update) {
    clearPuzzle3dSolverPreview();
    return false;
  }
  if (!level3dSolverFrame) {
    level3dSolverFrame = document.createElement("iframe");
    level3dSolverFrame.className = "solver3d-frame";
    level3dSolverFrame.title = "3D solution preview";
    level3dSolverFrame.sandbox = "allow-scripts";
    level3dSolverFrame.scrolling = "no";
    solverBoardViewport.append(level3dSolverFrame);
  }
  solverBoardViewport.classList.add("is-puzzle3d");
  if (solverBoard) {
    solverBoard.hidden = true;
  }
  const key = `${activePreviewDocument()?.id || ""}:${latestHtml.length}:solver3d`;
  if (level3dSolverFrameKey !== key) {
    level3dSolverFrameLoaded = false;
    level3dSolverFrameKey = key;
    level3dSolverFrame.addEventListener("load", () => {
      level3dSolverFrameLoaded = true;
      sendPuzzle3dSolutionToSolverRuntime();
    }, { once: true });
    level3dSolverFrame.srcdoc = level3dRuntimePreviewDocument(update);
    return true;
  }
  sendPuzzle3dSolutionToSolverRuntime();
  return true;
}

function clearPuzzle3dSolverPreview() {
  if (level3dSolverFrame) {
    level3dSolverFrame.remove();
  }
  level3dSolverFrame = null;
  level3dSolverFrameKey = "";
  level3dSolverFrameLoaded = false;
  solverBoardViewport?.classList.remove("is-puzzle3d");
  if (solverBoard) {
    solverBoard.hidden = false;
  }
}

function sendPuzzle3dSolutionToSolverRuntime() {
  if (!level3dSolverFrameLoaded || !level3dSolverFrame?.contentWindow) {
    return;
  }
  const snapshot = puzzle3dSolverPreviewSnapshot();
  const update = level3dPreviewUpdateFromSnapshot(snapshot);
  if (!update) {
    return;
  }
  level3dSolverFrame.contentWindow.postMessage(level3dPreviewSurfaceMessage(update), "*");
}

function puzzle3dSolverPreviewSnapshot() {
  if (levelSolutionPreview?.kind === "puzzle3d") {
    return levelSolutionPreview.snapshot
      || puzzle3dSolutionStepSnapshot(levelSolutionPreview.steps?.[levelSolutionPreview.index || 0]);
  }
  if (solverObservationPreview?.kind === "puzzle3d") {
    return solverObservationPreview.snapshot || null;
  }
  if (typeof solverPuzzle3dPreviewSnapshot === "function") {
    return solverPuzzle3dPreviewSnapshot();
  }
  return level3dRuntimeSnapshot();
}

function level3dPreviewUpdateFromSnapshot(snapshot) {
  if (!snapshot) {
    return null;
  }
  const levelCount = Array.isArray(snapshot.levels) && snapshot.levels.length ? snapshot.levels.length : 1;
  const levelIndex = Math.max(0, Math.min(levelCount - 1, Math.trunc(Number(snapshot.levelIndex) || 0)));
  const levelEntry = snapshot.levels?.[levelIndex] || {};
  const size = snapshot.size || levelEntry.size || {};
  const cells = Array.isArray(snapshot.cells)
    ? snapshot.cells
    : Array.isArray(levelEntry.cells)
      ? levelEntry.cells
      : [];
  const resources = level3dRuntimePreviewResources(snapshot);
  return {
    levelIndex,
    level: {
      name: levelEntry.name || level3dNameInput?.value || "level_1",
      label: levelEntry.label || levelEntry.name || level3dNameInput?.value || "Level 1",
      size: size ? { ...size } : undefined,
      cells: level3dCellsWithObjectDescriptors(cells, resources.objects),
    },
    resources,
    camera: level3dRuntimePreviewCamera(snapshot),
    view: level3dRuntimePreviewView(snapshot),
    settings: level3dPreviewSettings(snapshot.settings || {}),
    component: level3dModelPreviewComponent(),
    componentEmbed: true,
  };
}

function level3dCellsWithObjectDescriptors(cells, objects = {}) {
  const descriptors = new Map();
  for (const descriptor of Object.values(objects || {})) {
    if (!descriptor) {
      continue;
    }
    if (descriptor.id != null) {
      descriptors.set(`id:${Number(descriptor.id)}`, descriptor);
    }
    if (descriptor.name) {
      descriptors.set(`name:${descriptor.name}`, descriptor);
    }
  }
  return (cells || []).map((cell) => ({
    ...JSON.parse(JSON.stringify(cell || {})),
    objects: (cell?.objects || []).map((object) => {
      const descriptor = descriptors.get(`id:${Number(object?.id)}`)
        || descriptors.get(`name:${object?.name || ""}`)
        || {};
      return {
        ...JSON.parse(JSON.stringify(descriptor)),
        ...JSON.parse(JSON.stringify(object || {})),
        sprite: object?.sprite || descriptor.sprite || object?.name || descriptor.name,
        layer: object?.layer ?? descriptor.layer ?? null,
      };
    }),
  }));
}

function puzzle3dSolutionStepSnapshot(step) {
  const scene = step?.scene;
  if (scene?.kind !== "puzzle3d") {
    return null;
  }
  const base = levelSolutionPreview?.snapshot
    || (() => {
      const previous = levelSolutionPreview;
      levelSolutionPreview = null;
      const snapshot = level3dRuntimeSnapshot();
      levelSolutionPreview = previous;
      return snapshot;
    })()
    || previewExport
    || extractPreviewExport(latestHtml)
    || {};
  const snapshot = JSON.parse(JSON.stringify(base));
  snapshot.__kind = "puzzle3d";
  snapshot.size = { ...(scene.size || snapshot.size || {}) };
  snapshot.cells = JSON.parse(JSON.stringify(scene.cells || []));
  snapshot.layerCount = scene.layerCount || snapshot.layerCount;
  snapshot.completed = Boolean(step.completed);
  snapshot.clearCommands = Array.isArray(step.clearCommands) ? [...step.clearCommands] : [];
  if (Array.isArray(snapshot.levels) && Number.isInteger(snapshot.levelIndex) && snapshot.levels[snapshot.levelIndex]) {
    snapshot.levels[snapshot.levelIndex].size = { ...snapshot.size };
    snapshot.levels[snapshot.levelIndex].cells = JSON.parse(JSON.stringify(snapshot.cells));
  }
  return snapshot;
}

function fallbackLevel3dRuntimeSnapshot(exportData = previewExport || extractPreviewExport(latestHtml)) {
  if (!level3d.slices.length) {
    return null;
  }
  const size = {
    width: Math.max(1, Math.trunc(Number(level3d.width) || 1)),
    depth: Math.max(1, Math.trunc(Number(level3d.depth) || 1)),
    height: Math.max(1, Math.trunc(Number(level3d.height) || 1)),
  };
  const objects = {};
  for (const entry of level3d.palette || []) {
    for (const name of entry.objects || []) {
      objects[name] = level3dObjectDescriptor(name, exportData);
    }
  }
  const snapshot = {
    __kind: "puzzle3d",
    size,
    cells: [],
    levels: [{ name: level3dNameInput?.value || "level_1", size, cells: [] }],
    levelIndex: 0,
    camera: level3dPreviewCamera(exportData),
    sprites: exportData?.sprites || {},
    settings: exportData?.settings || {},
    objects,
  };
  const edited = level3dSnapshotLevelData(snapshot);
  if (edited) {
    snapshot.size = { ...edited.size };
    snapshot.cells = edited.cells;
    snapshot.levels[0].size = { ...edited.size };
    snapshot.levels[0].cells = edited.cells;
  }
  return level3dSnapshotWithPreviewGrid(snapshot);
}

function level3dSnapshotWithPreviewGrid(snapshot) {
  if (!snapshot) {
    return snapshot;
  }
  const next = JSON.parse(JSON.stringify(snapshot));
  next.settings = level3dPreviewSettings(next.settings || {});
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
  const raw = snapshot?.settings?.grid;
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

function level3dSnapshotLevelData(exportData = previewExport) {
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
            y: level3d.depth - 1 - row,
            z: level3d.height - 1 - slice,
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

function level3dObjectsForChar(ch, exportData = previewExport) {
  const entry = level3d.palette.find((candidate) => candidate.char === ch);
  if (!entry?.objects?.length) {
    return [];
  }
  return entry.objects.map((name) => level3dObjectDescriptor(name, exportData)).filter(Boolean);
}

function level3dPaletteObjectDescriptor(name, exportData = previewExport, sprites = level3dPreviewSprites(exportData)) {
  const object = level3dObjectDescriptor(name, exportData);
  if (!object) {
    return null;
  }
  return level3dObjectHasPreviewSprite(object, exportData, sprites) ? object : null;
}

function level3dObjectHasPreviewSprite(object, exportData = previewExport, sprites = exportData?.sprites) {
  return Boolean(object && (
    sprites?.[object.sprite]
    || sprites?.[object.name]
    || exportData?.sprites?.[object.sprite]
    || exportData?.sprites?.[object.name]
  ));
}

function level3dObjectDescriptor(name, exportData = previewExport) {
  const fromObjects = exportData?.objects?.[name];
  if (fromObjects) {
    return { ...fromObjects };
  }
  for (const level of exportData?.levels || []) {
    for (const cell of level.cells || []) {
      const object = (cell.objects || []).find((candidate) => candidate.name === name || candidate.sprite === name);
      if (object) {
        return { ...object };
      }
    }
  }
  return { name, sprite: name };
}

function level3dStageHitAt(x, y, width, height) {
  const snapshot = level3dRuntimeSnapshot();
  if (!snapshot || !level3d.slices.length) {
    return null;
  }
  const view = level3dStagePreviewView(snapshot, width, height);
  const resizeMode = level3dStageResizeMode();
  if (resizeMode) {
    const resizeFaces = level3dStageResizeFaces(snapshot, view, resizeMode);
    return resizeFaces.find((face) => pointInLevel3dPolygon({ x, y }, face.polygon))
      || nearestLevel3dStageResizeFace({ x, y }, resizeFaces, view);
  }
  const faces = level3dUsablePlacementFaces(level3dPlacementFaces(snapshot, view));
  faces.sort((left, right) => level3dPrimitiveOrder(right) - level3dPrimitiveOrder(left));
  return faces.find((face) => pointInLevel3dPolygon({ x, y }, face.polygon))
    || nearestLevel3dPlacementFace({ x, y }, faces, view);
}

function level3dUsablePlacementFaces(faces) {
  const selected = level3dSelectedEntry();
  if (!selected?.objects?.length) {
    return faces.filter((face) => face.remove);
  }
  if (level3dEditMode() === "add") {
    return faces.filter((face) => face.place);
  }
  return faces.filter((face) => face.replace);
}

function level3dPlacementFaces(snapshot, view) {
  const occupied = new Set((snapshot.cells || [])
    .filter((cell) => cell.objects?.length)
    .map((cell) => level3dVoxelKey(cell.position.x, cell.position.y, cell.position.z)));
  const faces = [];
  const directions = [
    { side: "xNeg", normal: { x: -1, y: 0, z: 0 }, delta: { x: -1, y: 0, z: 0 } },
    { side: "xPos", normal: { x: 1, y: 0, z: 0 }, delta: { x: 1, y: 0, z: 0 } },
    { side: "yNeg", normal: { x: 0, y: -1, z: 0 }, delta: { x: 0, y: -1, z: 0 } },
    { side: "yPos", normal: { x: 0, y: 1, z: 0 }, delta: { x: 0, y: 1, z: 0 } },
    { side: "zNeg", normal: { x: 0, y: 0, z: -1 }, delta: { x: 0, y: 0, z: -1 } },
    { side: "zPos", normal: { x: 0, y: 0, z: 1 }, delta: { x: 0, y: 0, z: 1 } },
  ];

  for (const cell of snapshot.cells || []) {
    if (!cell.objects?.length) {
      continue;
    }
    for (const face of directions) {
      if (level3dDirectionDepth(face.normal, snapshot.camera) <= 0) {
        continue;
      }
      const place = {
        x: Number(cell.position.x) + face.delta.x,
        y: Number(cell.position.y) + face.delta.y,
        z: Number(cell.position.z) + face.delta.z,
      };
      if (!level3dPositionInBounds(place) || occupied.has(level3dVoxelKey(place.x, place.y, place.z))) {
        continue;
      }
      faces.push(level3dPlacementFace(face.side, cell.position, view, {
        kind: "occupied",
        place,
        replace: { ...cell.position },
        remove: { ...cell.position },
      }));
    }
  }

  for (let y = 0; y < Math.max(1, level3d.depth || 1); y += 1) {
    for (let x = 0; x < Math.max(1, level3d.width || 1); x += 1) {
      const place = { x, y, z: 0 };
      if (occupied.has(level3dVoxelKey(x, y, 0))) {
        continue;
      }
      faces.push(level3dPlacementFace("zPos", { x, y, z: -1 }, view, {
        kind: "floor",
        place,
        remove: null,
      }));
    }
  }
  return faces;
}

function level3dExpansionFaces(snapshot, view) {
  return level3dStageResizeFaces(snapshot, view, "expand");
}

function level3dStageResizeFaces(snapshot, view, mode = "expand") {
  const size = snapshot?.size || {};
  const width = Math.max(1, Math.trunc(Number(size.width) || level3d.width || 1));
  const depth = Math.max(1, Math.trunc(Number(size.depth) || level3d.depth || 1));
  const height = Math.max(1, Math.trunc(Number(size.height) || level3d.height || 1));
  const faces = level3dStageResizeSliceSpecs(width, depth, height, mode)
    .filter((spec) => mode === "expand" || level3dCanShrinkDimension(spec.dimension, { width, depth, height }))
    .map((spec) => level3dStageResizeFace(spec.faceCorners, view, {
      kind: `${mode}-${spec.dimension}`,
      mode,
      dimension: spec.dimension,
      axis: spec.axis,
      edge: spec.edge,
      resizeEdge: spec.resizeEdge,
      frameBounds: spec.frameBounds,
    }));
  faces.sort((left, right) => level3dPrimitiveOrder(right) - level3dPrimitiveOrder(left));
  return faces;
}

function level3dStageResizeSliceSpecs(width, depth, height, mode) {
  const expand = mode === "expand";
  return [
    {
      dimension: "width",
      axis: "x",
      edge: "left",
      resizeEdge: "left",
      faceCorners: expand
        ? level3dBoundsBottomFace({ x0: -1.5, x1: -0.5, y0: -0.5, y1: depth - 0.5, z: -0.5 })
        : level3dBoundsSideFace("xNeg", { x: -0.5, y0: -0.5, y1: depth - 0.5, z0: -0.5, z1: height - 0.5 }),
      frameBounds: level3dResizeSliceFrameBounds({ width, depth, height }, "width", "left", mode),
    },
    {
      dimension: "width",
      axis: "x",
      edge: "right",
      resizeEdge: "right",
      faceCorners: expand
        ? level3dBoundsBottomFace({ x0: width - 0.5, x1: width + 0.5, y0: -0.5, y1: depth - 0.5, z: -0.5 })
        : level3dBoundsSideFace("xPos", { x: width - 0.5, y0: -0.5, y1: depth - 0.5, z0: -0.5, z1: height - 0.5 }),
      frameBounds: level3dResizeSliceFrameBounds({ width, depth, height }, "width", "right", mode),
    },
    {
      dimension: "depth",
      axis: "y",
      edge: "front",
      resizeEdge: "front",
      faceCorners: expand
        ? level3dBoundsBottomFace({ x0: -0.5, x1: width - 0.5, y0: -1.5, y1: -0.5, z: -0.5 })
        : level3dBoundsSideFace("yNeg", { y: -0.5, x0: -0.5, x1: width - 0.5, z0: -0.5, z1: height - 0.5 }),
      frameBounds: level3dResizeSliceFrameBounds({ width, depth, height }, "depth", "front", mode),
    },
    {
      dimension: "depth",
      axis: "y",
      edge: "back",
      resizeEdge: "back",
      faceCorners: expand
        ? level3dBoundsBottomFace({ x0: -0.5, x1: width - 0.5, y0: depth - 0.5, y1: depth + 0.5, z: -0.5 })
        : level3dBoundsSideFace("yPos", { y: depth - 0.5, x0: -0.5, x1: width - 0.5, z0: -0.5, z1: height - 0.5 }),
      frameBounds: level3dResizeSliceFrameBounds({ width, depth, height }, "depth", "back", mode),
    },
    {
      dimension: "height",
      axis: "z",
      edge: "down",
      resizeEdge: "bottom",
      faceCorners: level3dBoundsHorizontalFace({ x0: -0.5, x1: width - 0.5, y0: -0.5, y1: depth - 0.5, z: expand ? -1.5 : -0.5 }),
      frameBounds: level3dResizeSliceFrameBounds({ width, depth, height }, "height", "down", mode),
    },
    {
      dimension: "height",
      axis: "z",
      edge: "up",
      resizeEdge: "top",
      faceCorners: level3dBoundsHorizontalFace({ x0: -0.5, x1: width - 0.5, y0: -0.5, y1: depth - 0.5, z: expand ? height + 0.5 : height - 0.5 }),
      frameBounds: level3dResizeSliceFrameBounds({ width, depth, height }, "height", "up", mode),
    },
  ];
}

function level3dCanShrinkDimension(dimension, size) {
  if (dimension === "width") {
    return size.width > 1;
  }
  if (dimension === "depth") {
    return size.depth > 1;
  }
  if (dimension === "height") {
    return size.height > 1;
  }
  return false;
}

function level3dStageResizeFace(corners, view, metadata) {
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

function level3dBoundsBottomFace({ x0, x1, y0, y1, z }) {
  return [{ x: x0, y: y0, z }, { x: x1, y: y0, z }, { x: x1, y: y1, z }, { x: x0, y: y1, z }];
}

function level3dBoundsHorizontalFace({ x0, x1, y0, y1, z }) {
  return [{ x: x0, y: y0, z }, { x: x1, y: y0, z }, { x: x1, y: y1, z }, { x: x0, y: y1, z }];
}

function level3dBoundsSideFace(side, bounds) {
  if (side === "xNeg") {
    return [
      { x: bounds.x, y: bounds.y0, z: bounds.z0 },
      { x: bounds.x, y: bounds.y0, z: bounds.z1 },
      { x: bounds.x, y: bounds.y1, z: bounds.z1 },
      { x: bounds.x, y: bounds.y1, z: bounds.z0 },
    ];
  }
  if (side === "xPos") {
    return [
      { x: bounds.x, y: bounds.y0, z: bounds.z1 },
      { x: bounds.x, y: bounds.y0, z: bounds.z0 },
      { x: bounds.x, y: bounds.y1, z: bounds.z0 },
      { x: bounds.x, y: bounds.y1, z: bounds.z1 },
    ];
  }
  if (side === "yNeg") {
    return [
      { x: bounds.x0, y: bounds.y, z: bounds.z0 },
      { x: bounds.x1, y: bounds.y, z: bounds.z0 },
      { x: bounds.x1, y: bounds.y, z: bounds.z1 },
      { x: bounds.x0, y: bounds.y, z: bounds.z1 },
    ];
  }
  return [
    { x: bounds.x0, y: bounds.y, z: bounds.z1 },
    { x: bounds.x1, y: bounds.y, z: bounds.z1 },
    { x: bounds.x1, y: bounds.y, z: bounds.z0 },
    { x: bounds.x0, y: bounds.y, z: bounds.z0 },
  ];
}

function level3dResizeSliceFrameBounds(size, dimension, edge, mode = "shrink") {
  return mode === "expand"
    ? level3dExpandedSliceFrameBounds(size, dimension, edge)
    : level3dSliceFrameBounds(size, dimension, edge);
}

function level3dExpandedSliceFrameBounds(size, dimension, edge) {
  const bounds = level3dFullStageBounds(size);
  if (dimension === "width" && edge === "left") {
    bounds.x1 = bounds.x0;
    bounds.x0 -= 1;
  } else if (dimension === "width" && edge === "right") {
    bounds.x0 = bounds.x1;
    bounds.x1 += 1;
  } else if (dimension === "depth" && (edge === "front" || edge === "forward")) {
    bounds.y1 = bounds.y0;
    bounds.y0 -= 1;
  } else if (dimension === "depth" && (edge === "back" || edge === "backward")) {
    bounds.y0 = bounds.y1;
    bounds.y1 += 1;
  } else if (dimension === "height" && edge === "down") {
    bounds.z1 = bounds.z0;
    bounds.z0 -= 1;
  } else if (dimension === "height" && edge === "up") {
    bounds.z0 = bounds.z1;
    bounds.z1 += 1;
  }
  return bounds;
}

function level3dSliceFrameBounds(size, dimension, edge) {
  const bounds = level3dFullStageBounds(size);
  if (dimension === "width" && edge === "left") {
    bounds.x1 = bounds.x0 + 1;
  } else if (dimension === "width" && edge === "right") {
    bounds.x0 = bounds.x1 - 1;
  } else if (dimension === "depth" && (edge === "front" || edge === "forward")) {
    bounds.y1 = bounds.y0 + 1;
  } else if (dimension === "depth" && (edge === "back" || edge === "backward")) {
    bounds.y0 = bounds.y1 - 1;
  } else if (dimension === "height" && edge === "down") {
    bounds.z1 = bounds.z0 + 1;
  } else if (dimension === "height" && edge === "up") {
    bounds.z0 = bounds.z1 - 1;
  }
  return bounds;
}

function level3dFullStageBounds(size) {
  return {
    x0: -0.5,
    x1: Math.max(1, Number(size?.width) || 1) - 0.5,
    y0: -0.5,
    y1: Math.max(1, Number(size?.depth) || 1) - 0.5,
    z0: -0.5,
    z1: Math.max(1, Number(size?.height) || 1) - 0.5,
  };
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

function nearestLevel3dPlacementFace(point, faces, view) {
  let nearest = null;
  let nearestDistance = Infinity;
  for (const face of faces) {
    const dx = point.x - face.center.x;
    const dy = point.y - face.center.y;
    const distance = Math.hypot(dx, dy);
    if (distance < nearestDistance) {
      nearest = face;
      nearestDistance = distance;
    }
  }
  const tolerance = Math.max(64, Math.min(140, (view.scale || 1) * 1.1));
  return nearestDistance <= tolerance ? nearest : null;
}

function nearestLevel3dExpansionFace(point, faces, view) {
  return nearestLevel3dStageResizeFace(point, faces, view);
}

function nearestLevel3dStageResizeFace(point, faces, view) {
  let nearest = null;
  let nearestDistance = Infinity;
  for (const face of faces) {
    const dx = point.x - face.center.x;
    const dy = point.y - face.center.y;
    const distance = Math.hypot(dx, dy);
    if (distance < nearestDistance) {
      nearest = face;
      nearestDistance = distance;
    }
  }
  const tolerance = Math.max(72, Math.min(190, (view.scale || 1) * 1.35));
  return nearestDistance <= tolerance ? nearest : null;
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

function drawLevel3dExpansionHint(ctx, hit, width, height) {
  drawLevel3dStageResizeHint(ctx, hit, width, height);
}

function drawLevel3dStageResizeHint(ctx, hit, width, height) {
  const snapshot = level3dRuntimeSnapshot();
  if (!snapshot) {
    return;
  }
  const mode = hit?.mode || (hit?.kind?.startsWith("shrink-") ? "shrink" : "expand");
  const view = level3dStagePreviewView(snapshot, width, height);
  const ghostFace = level3dStageResizeFaces(snapshot, view, mode)
    .find((face) => face.kind === hit.kind && face.edge === hit.edge);
  const isShrink = mode === "shrink";
  ctx.save();
  ctx.lineJoin = "round";
  ctx.fillStyle = isShrink ? "rgba(230, 96, 105, 0.14)" : "rgba(77, 171, 218, 0.12)";
  ctx.strokeStyle = isShrink ? "rgba(230, 96, 105, 0.72)" : "rgba(77, 171, 218, 0.62)";
  ctx.lineWidth = 1.5;
  if (ghostFace?.polygon?.length) {
    drawLevel3dPolygonPath(ctx, ghostFace.polygon);
    ctx.fill();
    ctx.stroke();
  }
  const edges = level3dStageResizeFrameEdges(snapshot?.size || {}, hit);
  const lines = edges.map((edge) => {
    const from = level3dProjectPoint(edge.from, view);
    const to = level3dProjectPoint(edge.to, view);
    return {
      from,
      to,
      order: level3dGridOrder(level3dMidpoint3(edge.from, edge.to), view.camera),
      depth: (from.depth + to.depth) / 2,
    };
  }).sort((left, right) => level3dPrimitiveOrder(left) - level3dPrimitiveOrder(right));
  ctx.setLineDash([7, 5]);
  ctx.lineWidth = 2;
  ctx.strokeStyle = isShrink ? "rgba(180, 59, 67, 0.82)" : "rgba(39, 107, 143, 0.72)";
  for (const line of lines) {
    ctx.beginPath();
    ctx.moveTo(line.from.x, line.from.y);
    ctx.lineTo(line.to.x, line.to.y);
    ctx.stroke();
  }
  ctx.restore();
}

function level3dStageResizeFrameEdges(size, hit) {
  if (hit?.frameBounds) {
    return level3dFrameEdgesFromBounds(hit.frameBounds);
  }
  return level3dExpandedFrameEdges(size, hit?.kind, hit?.edge || "right");
}

function level3dExpandedFrameEdges(size, kind, edge) {
  const width = Math.max(1, Number(size?.width) || level3d.width || 1);
  const depth = Math.max(1, Number(size?.depth) || level3d.depth || 1);
  const height = Math.max(1, Number(size?.height) || level3d.height || 1);
  const x0 = kind === "expand-width" && edge === "left" ? -1.5 : -0.5;
  const x1 = kind === "expand-width" && edge === "right" ? width + 0.5 : width - 0.5;
  const y0 = kind === "expand-depth" && (edge === "front" || edge === "forward") ? -1.5 : -0.5;
  const y1 = kind === "expand-depth" && (edge === "back" || edge === "backward") ? depth + 0.5 : depth - 0.5;
  const z0 = kind === "expand-height" && (edge === "bottom" || edge === "down") ? -1.5 : -0.5;
  const z1 = kind === "expand-height" && (edge === "top" || edge === "up") ? height + 0.5 : height - 0.5;
  return level3dFrameEdgesFromBounds({
    x0,
    x1,
    y0,
    y1,
    z0,
    z1,
  });
}

function level3dFrameEdgesFromBounds(bounds) {
  const corners = {
    leftBackBottom: { x: bounds.x0, y: bounds.y0, z: bounds.z0 },
    rightBackBottom: { x: bounds.x1, y: bounds.y0, z: bounds.z0 },
    rightFrontBottom: { x: bounds.x1, y: bounds.y1, z: bounds.z0 },
    leftFrontBottom: { x: bounds.x0, y: bounds.y1, z: bounds.z0 },
    leftBackTop: { x: bounds.x0, y: bounds.y0, z: bounds.z1 },
    rightBackTop: { x: bounds.x1, y: bounds.y0, z: bounds.z1 },
    rightFrontTop: { x: bounds.x1, y: bounds.y1, z: bounds.z1 },
    leftFrontTop: { x: bounds.x0, y: bounds.y1, z: bounds.z1 },
  };
  return [
    { from: corners.leftBackBottom, to: corners.rightBackBottom },
    { from: corners.rightBackBottom, to: corners.rightFrontBottom },
    { from: corners.rightFrontBottom, to: corners.leftFrontBottom },
    { from: corners.leftFrontBottom, to: corners.leftBackBottom },
    { from: corners.leftBackTop, to: corners.rightBackTop },
    { from: corners.rightBackTop, to: corners.rightFrontTop },
    { from: corners.rightFrontTop, to: corners.leftFrontTop },
    { from: corners.leftFrontTop, to: corners.leftBackTop },
    { from: corners.leftBackBottom, to: corners.leftBackTop },
    { from: corners.rightBackBottom, to: corners.rightBackTop },
    { from: corners.rightFrontBottom, to: corners.rightFrontTop },
    { from: corners.leftFrontBottom, to: corners.leftFrontTop },
  ];
}

function level3dStageViewOptions() {
  return { padding: 0.56 };
}

function level3dStagePreviewView(snapshot, width, height) {
  const runtimeView = level3dStageRendererViewForSurface(snapshot, width, height);
  return runtimeView || level3dPreviewView(snapshot, width, height, level3dStageViewOptions());
}

function level3dStageRendererViewForSurface(snapshot, width, height) {
  if (!level3dStageRendererView || level3dStageRendererView.coordinateSpace !== "canvas-css-px") {
    return null;
  }
  const size = snapshot?.size || {};
  const sameSize = Math.max(1, Math.trunc(Number(size.width) || 1)) === Math.max(1, Math.trunc(Number(level3d.width) || 1))
    && Math.max(1, Math.trunc(Number(size.depth) || 1)) === Math.max(1, Math.trunc(Number(level3d.depth) || 1))
    && Math.max(1, Math.trunc(Number(size.height) || 1)) === Math.max(1, Math.trunc(Number(level3d.height) || 1));
  if (!sameSize) {
    return null;
  }
  return {
    ...level3dStageRendererView,
    surfaceWidth: Math.max(1, Number(width) || level3dStageRendererView.viewport.width),
    surfaceHeight: Math.max(1, Number(height) || level3dStageRendererView.viewport.height),
  };
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
  const sprite = snapshot.sprites?.[object.sprite] || snapshot.sprites?.[object.name];
  if (!sprite) {
    return [];
  }
  const blocks = level3dBitmapBlocks(sprite.bitmap || []);
  const spriteHeight = Math.max(1, blocks.length);
  const spriteDepth = Math.max(1, ...blocks.map((rows) => rows.length));
  const spriteWidth = Math.max(1, ...blocks.flatMap((rows) => rows.map((row) => row.length)));
  const scale = 1 / Math.max(spriteWidth, spriteDepth, spriteHeight);
  const voxels = [];
  for (let z = 0; z < blocks.length; z += 1) {
    const rows = blocks[z] || [];
    for (let row = 0; row < rows.length; row += 1) {
      for (let column = 0; column < rows[row].length; column += 1) {
        const fill = sprite.palette?.[rows[row][column]];
        if (!fill || level3dParseColor(fill)?.a <= 0) {
          continue;
        }
        const grid = {
          x: column,
          y: Math.max(0, spriteDepth - 1 - row),
          z: Math.max(0, spriteHeight - 1 - z),
        };
        const voxelPosition = {
          x: Number(position.x) + (grid.x + 0.5) * scale - 0.5,
          y: Number(position.y) + (grid.y + 0.5) * scale - 0.5,
          z: Number(position.z) + (grid.z + 0.5) * scale - 0.5,
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
  level3dPreviewCameraState.zoom = level3dClampNumber(level3dPreviewCameraState.zoom, 0.25, 4);
  return level3dPreviewCameraState;
}

function level3dBasePreviewCamera(source) {
  const camera = source?.camera || previewExport?.camera || {};
  return {
    yawDegrees: Number(camera.yawDegrees ?? 15),
    pitchDegrees: Number(camera.pitchDegrees ?? 55),
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
  const yaw = level3dDegreesToRadians(view.camera?.yawDegrees ?? 0);
  const pitch = level3dDegreesToRadians(view.camera?.pitchDegrees ?? 35);
  const zoom = view.camera?.zoom ?? 1;
  const x = position.x - view.center.x;
  const y = position.y - view.center.y;
  const z = position.z - view.center.z;
  const yawX = x * Math.cos(yaw) - y * Math.sin(yaw);
  const yawY = x * Math.sin(yaw) + y * Math.cos(yaw);
  const scale = view.scale * zoom;
  return {
    x: view.origin.x + yawX * scale,
    y: view.origin.y + (-yawY * Math.sin(pitch) - z * Math.cos(pitch)) * scale,
    depth: -yawY * Math.cos(pitch) + z * Math.sin(pitch),
  };
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
  const yaw = level3dDegreesToRadians(camera.yawDegrees ?? 0);
  const pitch = level3dDegreesToRadians(camera.pitchDegrees ?? 35);
  const distance = Math.max(0.0001, Number(projection.distance) || 1);
  const horizontal = Math.cos(pitch);
  const cameraPosition = {
    x: target.x - Math.sin(yaw) * horizontal * distance,
    y: target.y + Math.sin(pitch) * distance,
    z: target.z + Math.cos(yaw) * horizontal * distance,
  };
  const world = {
    x: Number(position.x) - (Math.max(1, Number(size.width) || 1) - 1) / 2,
    y: Number(position.z) - (Math.max(1, Number(size.height) || 1) - 1) / 2,
    z: (Math.max(1, Number(size.depth) || 1) - 1) / 2 - Number(position.y),
  };
  const forward = level3dNormalizeVector({
    x: target.x - cameraPosition.x,
    y: target.y - cameraPosition.y,
    z: target.z - cameraPosition.z,
  });
  let upSeed = { x: 0, y: 1, z: 0 };
  if (Math.abs(Math.cos(pitch)) < 0.001) {
    upSeed = { x: 0, y: 0, z: -Math.sign(Math.sin(pitch)) || -1 };
  }
  const right = level3dNormalizeVector(level3dCrossVector(forward, upSeed));
  const up = level3dNormalizeVector(level3dCrossVector(right, forward));
  const relative = {
    x: world.x - cameraPosition.x,
    y: world.y - cameraPosition.y,
    z: world.z - cameraPosition.z,
  };
  const cameraX = level3dDotVector(relative, right);
  const cameraY = level3dDotVector(relative, up);
  const cameraDepth = Math.max(0.0001, level3dDotVector(relative, forward));
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

function level3dNormalizeVector(vector) {
  const length = Math.hypot(Number(vector.x) || 0, Number(vector.y) || 0, Number(vector.z) || 0) || 1;
  return {
    x: (Number(vector.x) || 0) / length,
    y: (Number(vector.y) || 0) / length,
    z: (Number(vector.z) || 0) / length,
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
      zoom: 1,
    },
    center: view.center || { x: 0, y: 0, z: 0 },
    origin: {
      x: Number(view.originX) || 0,
      y: Number(view.originY) || 0,
    },
    scale: Math.max(0.0001, Number(view.scale) || 1),
  };
  if (window.Puzzle3VisualCore?.projectOrthographic) {
    return Puzzle3VisualCore.projectOrthographic(position, projectionView);
  }
  const yaw = level3dDegreesToRadians(projectionView.camera.yawDegrees);
  const pitch = level3dDegreesToRadians(projectionView.camera.pitchDegrees);
  const x = position.x - projectionView.center.x;
  const y = position.y - projectionView.center.y;
  const z = position.z - projectionView.center.z;
  const yawX = x * Math.cos(yaw) - y * Math.sin(yaw);
  const yawY = x * Math.sin(yaw) + y * Math.cos(yaw);
  return {
    x: projectionView.origin.x + yawX * projectionView.scale,
    y: projectionView.origin.y + (-yawY * Math.sin(pitch) - z * Math.cos(pitch)) * projectionView.scale,
    depth: -yawY * Math.cos(pitch) + z * Math.sin(pitch),
  };
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

function level3dBitmapBlocks(bitmap) {
  const blocks = [[]];
  for (const row of bitmap || []) {
    if (row === "") {
      blocks.push([]);
    } else {
      blocks[blocks.length - 1].push(String(row));
    }
  }
  return blocks;
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
  const levelName = sanitizeLevel3dName(level3dNameInput?.value || currentLevel3dEntry()?.name || "level_1");
  await copyTextToClipboard(level3dSnippetSource(levelName, level3dSourceData(), "", { bodyIndent: "" }));
  setLevel3dActionStatus("Copied 3D level", "is-ok");
}

function addLevel3dToSource() {
  const sourceDocument = level3dSourceDocument();
  if (!sourceDocument) {
    setLevel3dActionStatus("No game entry for 3D level", "is-error");
    return;
  }
  const levelName = sanitizeLevel3dName(level3dNameInput?.value || "level_1");
  const bundle = sanitizeLevel3dBundle(level3dBundleInput?.value || "");
  const nextSource = insertLevel3d(level3dEditorSource(sourceDocument), levelName, level3dSourceData(), bundle);
  if (!nextSource) {
    setLevel3dActionStatus(`No levels3 block named ${bundle}`, "is-error");
    return;
  }
  applyLevel3dSourceChange(sourceDocument, nextSource);
  level3dNameInput.value = nextLevelName(levelName);
  syncLevel3dNameOptions();
  setLevel3dActionStatus("Added 3D level", "is-ok");
}

function updateLevel3dInSource() {
  const sourceDocument = level3dSourceDocument();
  if (!sourceDocument) {
    setLevel3dActionStatus("No game entry for 3D level", "is-error");
    return;
  }
  const levelName = sanitizeLevel3dName(level3dNameInput?.value || "level_1");
  const bundle = sanitizeLevel3dBundle(level3dBundleInput?.value || "");
  const result = replaceLevel3dByName(level3dEditorSource(sourceDocument), levelName, level3dSourceData(), bundle);
  if (!result) {
    setLevel3dActionStatus(`No 3D level named ${bundle}.${levelName}`, "is-error");
    return;
  }
  applyLevel3dSourceChange(sourceDocument, result.source);
  setLevel3dActionStatus(`Updated 3D level ${levelName}`, "is-ok");
}

function applyLevel3dSourceChange(sourceDocument, source) {
  sourceDocument.source = source;
  level3d.sourceDocumentId = sourceDocument.id || level3d.sourceDocumentId || "";
  level3d.sourceKey = "";
  if (sourceDocument.id === activeDocument()?.id) {
    setSourceEditorValue(source, { resetUndo: false });
  }
  scheduleLocalSave();
  schedulePreview();
}

function resetLevel3dPreviewView() {
  if (level3dPlaytestActive) {
    return;
  }
  resetLevel3dPreviewState(previewExport || extractPreviewExport(latestHtml));
  renderLevel3dPreviewControls();
  level3dStageHit = null;
  renderLevel3dStageOverlay();
  refreshLevel3dRuntimePreviews();
  setLevel3dActionStatus("Reset 3D preview view", "is-ok");
}

function resetLevel3dPreviewState(source = previewExport || extractPreviewExport(latestHtml)) {
  level3dPreviewCameraState = level3dBasePreviewCamera(source);
  level3dPreviewOrigin = level3dDefaultPreviewTarget(source);
}

function level3dDefaultPreviewTarget(source = previewExport || extractPreviewExport(latestHtml)) {
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
  document.documentElement.classList.add("is-sprite3d-camera-scrubbing");
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
  document.documentElement.classList.remove("is-sprite3d-camera-scrubbing");
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
  level3dStageHit = null;
  renderLevel3dStageOverlay();
  refreshLevel3dRuntimePreviews();
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
    level3dActionStatus.className = `sprite-action-status tool-feedback-bar ${className || ""}`.trim();
    level3dActionStatus.textContent = text || "";
  }
  if (text && typeof setStatus === "function") {
    setStatus(text, className);
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
level3dPreviousLayerButton?.addEventListener("click", () => moveLevel3dLayer(-1));
level3dNextLayerButton?.addEventListener("click", () => moveLevel3dLayer(1));
level3dLayerBoard?.addEventListener("pointerdown", startLevel3dLayerPaint);
level3dLayerBoard?.addEventListener("pointermove", continueLevel3dLayerPaint);
level3dLayerBoard?.addEventListener("pointerup", stopLevel3dLayerPaint);
level3dLayerBoard?.addEventListener("pointercancel", stopLevel3dLayerPaint);
level3dLayerBoard?.addEventListener("keydown", (event) => {
  if (handleLevel3dSliceHorizontalInput(event)) {
    return;
  }
  if (event.key !== "Enter" && event.key !== " ") {
    return;
  }
  if (withVisualEditHistory("level3d", paintLevel3dLayerHoverCell)) {
    event.preventDefault();
    event.stopPropagation();
  }
});
level3dLayerBoard?.addEventListener("pointerleave", () => {
  level3dLayerHover = null;
  renderLevel3dLayerOverlay();
});
document.querySelectorAll("[data-level3d-layer-edge]").forEach((button) => {
  button.addEventListener("click", () => {
    const mode = level3dStageResizeMode();
    if (!mode) {
      return;
    }
    resizeLevel3dLayerEdge(button.dataset.level3dLayerEdge, mode);
  });
});
window.addEventListener("message", handleLevel3dLayerRendererViewMessage);
window.addEventListener("message", handleLevel3dStageRendererViewMessage);
window.addEventListener("message", handleLevel3dPlaytestStateMessage);
window.addEventListener("resize", scheduleLevel3dSurfaceResize);
document.addEventListener("keydown", (event) => {
  const tagName = event.target?.tagName || "";
  if (level3dBuilder.hidden || !level3dPlaytestActive || ["INPUT", "TEXTAREA", "SELECT"].includes(tagName)) {
    return;
  }
  sendLevel3dPlaytestKey(event);
});
document.addEventListener("keydown", (event) => {
  handleLevel3dSliceHorizontalInput(event);
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
level3dPlaytestButton?.addEventListener("click", toggleLevel3dPlaytest);
copyLevel3dButton?.addEventListener("click", () => {
  copyLevel3dToClipboard().catch((error) => setLevel3dActionStatus(error?.message || String(error), "is-error"));
});
addLevel3dButton?.addEventListener("click", addLevel3dToSource);
updateLevel3dButton?.addEventListener("click", updateLevel3dInSource);
registerSourceEditableTarget?.("level3d", {
  find: findLevel3dDefinitionAtPosition,
  load: loadLevel3dFromSourcePosition,
});
