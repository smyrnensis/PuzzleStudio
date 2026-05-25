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
let level3dLayerPaintDrag = null;
let level3dPreviewScrubDrag = null;
let level3dSliceScrubDrag = null;
let level3dPreviewCameraState = null;
let level3dPreviewOrigin = { x: 0, y: 0, z: 0 };
let level3dSurfaceResizeFrame = 0;
const LEVEL3D_EDITOR_MAX_SIZE = 256;
const LEVEL3D_PALETTE_PREVIEW_SIZE = 42;
const LEVEL3D_SLICE_SCRUB_STEP_PX = 18;
const LEVEL3D_CAMERA_MIN_PITCH_DEGREES = -90;
const LEVEL3D_CAMERA_MAX_PITCH_DEGREES = 90;
const LEVEL3D_MODEL_COMPONENT_PREVIEW_MESSAGE = "PuzzleStudioRenderPuzzle3ModelComponent";
const LEVEL3D_EMPTY_CHAR = ".";
let level3d = {
  width: 0,
  depth: 0,
  height: 0,
  slice: 0,
  selectedChar: LEVEL3D_EMPTY_CHAR,
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
  syncLevel3dControlsFromPreview();
  renderLevel3dPalette();
  renderLevel3dPreviewControls();
  renderLevel3dLayerControls();
  renderLevel3dLayerBoard();
  renderLevel3dSourcePreview();
  renderLevel3dRuntime();
  renderLevel3dStageOverlay();
}

function syncLevel3dControlsFromPreview() {
  const exportData = previewExport || extractPreviewExport(latestHtml);
  applyDefaultLevel3dSelectionForActiveDocument(exportData);
  const sourceDocument = level3dSourceDocument();
  const source = level3dEditorSource(sourceDocument);
  const sourceDefinition = currentLevel3dSourceDefinition(source);
  const levelEntry = sourceDefinition
    ? level3dExportEntryForSourceDefinition(sourceDefinition, exportData)
    : currentLevel3dEntry(exportData);
  if (!levelEntry && !sourceDefinition) {
    syncLevel3dPaletteWithoutLoadedLevel(source, sourceDocument);
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
  if ((palette || []).some((entry) => entry.char === current)) {
    return current;
  }
  return (palette || []).find((entry) => entry.objects.length > 0)?.char
    || (palette || [])[0]?.char
    || LEVEL3D_EMPTY_CHAR;
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
  if (!level3d.palette.some((entry) => entry.char === level3d.selectedChar)) {
    level3d.selectedChar = level3d.palette.find((entry) => entry.objects.length > 0)?.char
      || level3d.palette[0]?.char
      || LEVEL3D_EMPTY_CHAR;
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
  if (!level3d.palette.some((entry) => entry.char === level3d.selectedChar)) {
    level3d.selectedChar = level3d.palette.find((entry) => entry.objects.length > 0)?.char
      || level3d.palette[0]?.char
      || LEVEL3D_EMPTY_CHAR;
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
  const target = firstLevel3dTargetNearDocument(document);
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
  return Boolean(exportData?.__kind === "puzzle3d" || exportData?.directions?.forward || exportData?.levelBundles);
}

function renderLevel3dSourcePreview() {
  if (!level3dSourcePreview) {
    return;
  }
  syncLevel3dNameOptions();
  const levelName = sanitizeLevel3dName(level3dNameInput?.value || currentLevel3dEntry()?.name || "level_1");
  const sourceData = level3dSourceData();
  level3dSourcePreview.textContent = levelDefinition3dSource(levelName, sourceData, "");
}

function level3dSourceData(source = level3dEditorSource(), exportData = previewExport || extractPreviewExport(latestHtml)) {
  if (level3d.slices.length) {
    return level3dStateLevelData();
  }
  const entry = currentLevel3dEntry(exportData);
  if (!entry) {
    return { rows: [], unknownCells: 0 };
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
  for (const entry of level3d.palette) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "level-token level3d-token";
    button.classList.toggle("is-selected", paintSelectionActive && entry.char === level3d.selectedChar);
    button.dataset.label = level3dPaletteEntryLabel(entry);
    button.title = level3dPaletteEntryLabel(entry);
    button.setAttribute("aria-label", `Paint ${level3dPaletteEntryLabel(entry)}`);

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
      renderLevel3dLayerOverlay();
      renderLevel3dStageOverlay();
    });
    level3dPalette.append(button);
    drawLevel3dPalettePreview(visual, entry, exportData);
  }
}

function level3dPaintSelectionActive() {
  return !level3dStageResizeMode();
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
  button.innerHTML = icon;
  button.addEventListener("click", () => {
    setLevel3dStageResizeMode(level3dStageResizeMode() === mode ? null : mode);
    level3dStageHit = null;
    renderLevel3dPalette();
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
  const objects = (entry.objects || [])
    .map((name) => level3dPaletteObjectDescriptor(name, exportData))
    .filter(Boolean);
  const snapshot = {
    size: { width: 1, depth: 1, height: 1 },
    camera: level3dPalettePreviewCamera(exportData),
    sprites: exportData?.sprites || {},
    settings: exportData?.settings || {},
  };
  if (!objects.length) {
    if ((entry.objects || []).length) {
      drawLevel3dUnavailableTilePreview(ctx, width, height);
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
    padding: 0.96,
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

function drawLevel3dUnavailableTilePreview(ctx, width, height) {
  ctx.save();
  const inset = Math.max(6, Math.min(width, height) * 0.18);
  ctx.strokeStyle = "rgba(157, 163, 170, 0.58)";
  ctx.lineWidth = 1.25;
  ctx.setLineDash([4, 3]);
  ctx.strokeRect(inset, inset, Math.max(1, width - inset * 2), Math.max(1, height - inset * 2));
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
  if (!level3d.slices.length) {
    level3dLayerBoard.classList.add("is-empty");
    renderLevel3dLayerOverlay();
    return;
  }
  level3dLayerBoard.classList.remove("is-empty");
  renderLevel3dLayerRuntime();
  renderLevel3dLayerOverlay();
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
  setLevel3dLayer(currentLevel3dLayerZ() + delta);
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
  if (event.button !== 0) {
    return;
  }
  const cell = level3dLayerCellFromPointer(event);
  if (!cell) {
    return;
  }
  event.preventDefault();
  level3dLayerBoard?.focus();
  level3dLayerPaintDrag = {
    pointerId: event.pointerId,
    lastKey: "",
    beforeSnapshot: visualEditSnapshot("level3d"),
    changed: false,
  };
  level3dLayerBoard?.setPointerCapture?.(event.pointerId);
  continueLevel3dLayerPaint(event);
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

function paintLevel3dCellAtPosition(position, ch = level3d.selectedChar) {
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
  const nextChar = String(ch || level3dEmptyChar()).charAt(0);
  const current = String(slice[row] || "").padEnd(level3d.width, level3dEmptyChar()).slice(0, level3d.width);
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

function levelDefinition3dSource(name, levelData, indent = "", options = {}) {
  const rows = Array.isArray(levelData?.rows) ? levelData.rows : [];
  const bodyIndent = options.bodyIndent || `${indent}  `;
  return [
    `${indent}level ${sanitizeLevel3dName(name)} {`,
    ...rows.map((row) => String(row || "").length ? `${bodyIndent}${row}` : ""),
    `${indent}}`,
  ].join("\n") + (options.trailingNewline === false ? "" : "\n");
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
  const range = findLevels3InsertionRange(source, bundle);
  if (!range) {
    return "";
  }
  const definitions = findLevel3dDefinitions(source, range);
  const indent = definitions[0]?.indent || `${range.indent || ""}  `;
  const bodyIndent = definitions[0]?.bodyIndent || `${indent}  `;
  const levelSource = levelDefinition3dSource(name, levelData, indent, { bodyIndent }).trimEnd();
  return `${source.slice(0, range.bodyEnd).trimEnd()}\n\n${levelSource}\n${source.slice(range.bodyEnd)}`;
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
    const replacement = levelDefinition3dSource(requested, levelData, entry.indent, { bodyIndent: entry.bodyIndent }).trimEnd();
    return {
      source: replaceEditorSourceRangePreservingLineBoundary(source, entry.start, entry.end, replacement),
    };
  }
  return null;
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
    ? {
      name: target.name || "level_1",
      start: target.start ?? target.bodyStart,
      end: target.end ?? target.bodyEnd,
      bodyStart: target.bodyStart,
      bodyEnd: target.bodyEnd,
      bundle: target.bundle || target.params?.bundle || "",
      model: target.model || target.params?.model || "",
      levelIndex: target.levelIndex,
      rows: rowsForLevel3dDefinition(source, {
        bodyStart: target.bodyStart,
        bodyEnd: target.bodyEnd,
      }),
    }
    : findLevel3dDefinitionAtPosition(source, target?.start ?? 0);
  if (!entry) {
    return null;
  }
  return loadLevel3dSourceDefinition(entry, source, options);
}

function loadLevel3dSourceDefinition(entry, source, options = {}) {
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
  if (levels[levelIndex]) {
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
    sendLevel3dSnapshotToPreviewFrame();
    return;
  }
  const update = level3dRuntimePreviewUpdate();
  if (!latestHtml || !update) {
    showBlankLevel3dRuntimeFrame(level3dRuntimeFrame);
    level3dRuntimeFrameLoaded = false;
    level3dRuntimeFrameKey = "";
    setLevel3dActionStatus(latestHtml ? "Load a 3D level first" : "Run Preview first", "");
    return;
  }
  const key = `${activePreviewDocument()?.id || ""}:${latestHtml.length}:${currentEditableLevelIndex()}`;
  if (level3dRuntimeFrameKey !== key) {
    level3dRuntimeFrameLoaded = false;
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
    sendLevel3dSnapshotToPreviewFrame();
    return;
  }
  const update = level3dRuntimePreviewUpdate();
  if (!update) {
    showBlankLevel3dRuntimeFrame(level3dRuntimeFrame);
    level3dRuntimeFrameLoaded = false;
    level3dRuntimeFrameKey = "";
    return;
  }
  level3dRuntimeFrame.contentWindow.postMessage({
    type: LEVEL3D_MODEL_COMPONENT_PREVIEW_MESSAGE,
    ...update,
  }, "*");
  sendLevel3dSnapshotToPreviewFrame(update);
}

function sendLevel3dSnapshotToPreviewFrame(update = level3dRuntimePreviewUpdate()) {
  if (currentPreviewMode !== "level3d" || !previewFrame?.contentWindow || !update) {
    return;
  }
  previewFrameHasEditorLevelState = true;
  previewFrame.contentWindow.postMessage({
    type: LEVEL3D_MODEL_COMPONENT_PREVIEW_MESSAGE,
    ...update,
  }, "*");
}

function refreshLevel3dRuntimePreviews() {
  sendLevel3dSnapshotToRuntime();
  sendLevel3dLayerSnapshotToRuntime();
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
  const key = `${activePreviewDocument()?.id || ""}:puzzle3-layer-renderer`;
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
  level3dLayerFrame.contentWindow.postMessage({
    type: LEVEL3D_MODEL_COMPONENT_PREVIEW_MESSAGE,
    ...update,
  }, "*");
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
    camera: level3dPreviewCamera(snapshot),
    settings: level3dPreviewSettings(snapshot.settings || {}),
    component: level3dModelPreviewComponent(),
    componentEmbed: true,
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

function level3dModelPreviewComponent() {
  return { kind: "puzzle3", source: "__editor_level3d_preview__" };
}

function level3dRuntimePreviewResources(exportData = previewExport || extractPreviewExport(latestHtml)) {
  return {
    layerCount: exportData?.layerCount,
    objects: exportData?.objects || {},
    sprites: exportData?.sprites || {},
  };
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
  return { yawDegrees: 0, pitchDegrees: 90, zoom: 1 };
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
  const rect = level3dLayerSurfaceRect();
  const scale = window.devicePixelRatio || 1;
  const width = Math.max(1, Math.floor(rect.width * scale));
  const height = Math.max(1, Math.floor(rect.height * scale));
  if (level3dLayerOverlay.width !== width || level3dLayerOverlay.height !== height) {
    level3dLayerOverlay.width = width;
    level3dLayerOverlay.height = height;
  }
  const ctx = level3dLayerOverlay.getContext("2d");
  if (!ctx) {
    return;
  }
  ctx.setTransform(scale, 0, 0, scale, 0, 0);
  ctx.clearRect(0, 0, rect.width, rect.height);
  drawLevel3dLayerHover(ctx, rect.width, rect.height);
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
  const rect = level3dLayerSurfaceRect();
  return level3dLayerPositionAt(event.clientX - rect.left, event.clientY - rect.top, rect.width, rect.height);
}

function level3dLayerSurfaceRect() {
  const surface = level3dLayerOverlay instanceof HTMLCanvasElement
    ? level3dLayerOverlay
    : (level3dLayerFrame || level3dLayerBoard);
  return surface.getBoundingClientRect();
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
  const rect = overlay.getBoundingClientRect();
  const scale = window.devicePixelRatio || 1;
  const width = Math.max(1, Math.floor(rect.width * scale));
  const height = Math.max(1, Math.floor(rect.height * scale));
  if (overlay.width !== width || overlay.height !== height) {
    overlay.width = width;
    overlay.height = height;
  }
  const ctx = overlay.getContext("2d");
  if (!ctx) {
    return;
  }
  ctx.setTransform(scale, 0, 0, scale, 0, 0);
  ctx.clearRect(0, 0, rect.width, rect.height);
  drawLevel3dStagePreview(ctx, rect.width, rect.height);
  if (!level3dStageHit?.polygon?.length) {
    return;
  }
  if (level3dIsStageResizeHit(level3dStageHit)) {
    drawLevel3dStageResizeHint(ctx, level3dStageHit, rect.width, rect.height);
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
    level3dLayerRendererView = null;
    renderLevel3dStageOverlay();
    renderLevel3dLayerOverlay();
    level3dRuntimeFrame?.contentWindow?.postMessage({ type: "PuzzleStudioResize" }, "*");
    level3dLayerFrame?.contentWindow?.postMessage({ type: "PuzzleStudioResize" }, "*");
  });
}

function handleLevel3dStagePointerMove(event) {
  const hit = level3dStageHitFromEvent(event);
  if (level3dStageHitKey(hit) === level3dStageHitKey(level3dStageHit)) {
    return;
  }
  level3dStageHit = hit;
  renderLevel3dStageOverlay();
}

function handleLevel3dStagePointerDown(event) {
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
  const target = selected?.objects?.length ? hit.place : hit.remove;
  if (!target) {
    return;
  }
  if (withVisualEditHistory("level3d", () => paintLevel3dCellAtPosition(target, selectedChar))) {
    level3dStageHit = null;
    setLevel3dActionStatus(level3dCellLabel(selectedChar), "is-ok");
    renderLevel3dStageOverlay();
  }
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
  const payload = {
    type: LEVEL3D_MODEL_COMPONENT_PREVIEW_MESSAGE,
    ...update,
  };
  const json = JSON.stringify(payload)
    .replace(/</g, "\\u003c")
    .replace(/\u2028/g, "\\u2028")
    .replace(/\u2029/g, "\\u2029");
  const seedScript = `<script id="puzzle-studio-initial-model-preview">window.PuzzleStudioInitialModelComponentPreview=${json};<\/script>`;
  const bootScript = `<script id="puzzle-studio-initial-model-preview-boot">
(() => {
  const update = window.PuzzleStudioInitialModelComponentPreview;
  if (!update || update.type !== "${LEVEL3D_MODEL_COMPONENT_PREVIEW_MESSAGE}") {
    return;
  }
  if (typeof window.applyPuzzle3ModelComponentPreviewUpdate === "function") {
    window.applyPuzzle3ModelComponentPreviewUpdate(update);
    return;
  }
  if (typeof window.loadSnapshotData !== "function") {
    return;
  }
  const source = JSON.parse(JSON.stringify(window.Puzzle3DFixture || {}));
  const resources = update.resources || update;
  if (resources.layerCount != null) {
    source.layerCount = Math.max(1, Math.trunc(Number(resources.layerCount) || 1));
  }
  if (resources.objects && typeof resources.objects === "object") {
    source.objects = JSON.parse(JSON.stringify(resources.objects));
  }
  if (resources.sprites && typeof resources.sprites === "object") {
    source.sprites = JSON.parse(JSON.stringify(resources.sprites));
  }
  const level = update.level || {};
  const size = level.size || update.size || source.size || {};
  const cells = Array.isArray(level.cells) ? level.cells : Array.isArray(update.cells) ? update.cells : source.cells || [];
  const levelIndex = Math.max(0, Math.trunc(Number(update.levelIndex ?? source.levelIndex) || 0));
  const levels = Array.isArray(source.levels) && source.levels.length ? source.levels : [{}];
  const target = levels[Math.min(levelIndex, levels.length - 1)] || {};
  levels[Math.min(levelIndex, levels.length - 1)] = {
    ...target,
    name: level.name || target.name || "level_1",
    label: level.label || target.label || level.name || target.name || "Level 1",
    size: { ...size },
    cells: JSON.parse(JSON.stringify(cells)),
  };
  source.levels = levels;
  source.levelIndex = Math.min(levelIndex, levels.length - 1);
  source.size = { ...size };
  source.cells = JSON.parse(JSON.stringify(cells));
  if (update.camera) {
    source.camera = JSON.parse(JSON.stringify(update.camera));
  }
  if (update.settings) {
    source.settings = { ...(source.settings || {}), ...JSON.parse(JSON.stringify(update.settings)) };
  }
  source.scenes = [{
    name: update.scene || "__editor_model_preview__",
    components: [update.component || { kind: "puzzle3", source: "__editor_model_preview__" }],
  }];
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
  const rect = overlay.getBoundingClientRect();
  return level3dStageHitAt(event.clientX - rect.left, event.clientY - rect.top, rect.width, rect.height);
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
  return level3d.palette.find((entry) => entry.char === level3d.selectedChar)
    || level3d.palette[0]
    || { char: level3dEmptyChar(), objects: [] };
}

function level3dRuntimeSnapshot() {
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
  const edited = level3dSnapshotLevelData(snapshot);
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
  if (!solverBoardViewport || !latestHtml || levelSolutionPreview?.kind !== "puzzle3d") {
    clearPuzzle3dSolverPreview();
    return false;
  }
  const snapshot = levelSolutionPreview.snapshot
    || puzzle3dSolutionStepSnapshot(levelSolutionPreview.steps?.[levelSolutionPreview.index || 0]);
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
  if (!level3dSolverFrameLoaded || !level3dSolverFrame?.contentWindow || levelSolutionPreview?.kind !== "puzzle3d") {
    return;
  }
  const snapshot = levelSolutionPreview.snapshot
    || puzzle3dSolutionStepSnapshot(levelSolutionPreview.steps?.[levelSolutionPreview.index || 0]);
  const update = level3dPreviewUpdateFromSnapshot(snapshot);
  if (!update) {
    return;
  }
  level3dSolverFrame.contentWindow.postMessage({
    type: LEVEL3D_MODEL_COMPONENT_PREVIEW_MESSAGE,
    ...update,
  }, "*");
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
  return {
    levelIndex,
    level: {
      name: levelEntry.name || level3dNameInput?.value || "level_1",
      label: levelEntry.label || levelEntry.name || level3dNameInput?.value || "Level 1",
      size: size ? { ...size } : undefined,
      cells: JSON.parse(JSON.stringify(cells)),
    },
    resources: level3dRuntimePreviewResources(snapshot),
    camera: level3dPreviewCamera(snapshot),
    settings: level3dPreviewSettings(snapshot.settings || {}),
    component: level3dModelPreviewComponent(),
    componentEmbed: true,
  };
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

function level3dPaletteObjectDescriptor(name, exportData = previewExport) {
  const object = level3dObjectDescriptor(name, exportData);
  if (!object) {
    return null;
  }
  return level3dObjectHasPreviewSprite(object, exportData) ? object : null;
}

function level3dObjectHasPreviewSprite(object, exportData = previewExport) {
  return Boolean(object && (
    exportData?.sprites?.[object.sprite]
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
  const view = level3dPreviewView(snapshot, width, height, level3dStageViewOptions());
  const resizeMode = level3dStageResizeMode();
  if (resizeMode) {
    const resizeFaces = level3dStageResizeFaces(snapshot, view, resizeMode);
    return resizeFaces.find((face) => pointInLevel3dPolygon({ x, y }, face.polygon))
      || nearestLevel3dStageResizeFace({ x, y }, resizeFaces, view);
  }
  const faces = level3dPlacementFaces(snapshot, view);
  faces.sort((left, right) => level3dPrimitiveOrder(right) - level3dPrimitiveOrder(left));
  return faces.find((face) => pointInLevel3dPolygon({ x, y }, face.polygon))
    || nearestLevel3dPlacementFace({ x, y }, faces, view);
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
      edge: "forward",
      resizeEdge: "front",
      faceCorners: expand
        ? level3dBoundsBottomFace({ x0: -0.5, x1: width - 0.5, y0: -1.5, y1: -0.5, z: -0.5 })
        : level3dBoundsSideFace("yNeg", { y: -0.5, x0: -0.5, x1: width - 0.5, z0: -0.5, z1: height - 0.5 }),
      frameBounds: level3dResizeSliceFrameBounds({ width, depth, height }, "depth", "forward", mode),
    },
    {
      dimension: "depth",
      axis: "y",
      edge: "backward",
      resizeEdge: "back",
      faceCorners: expand
        ? level3dBoundsBottomFace({ x0: -0.5, x1: width - 0.5, y0: depth - 0.5, y1: depth + 0.5, z: -0.5 })
        : level3dBoundsSideFace("yPos", { y: depth - 0.5, x0: -0.5, x1: width - 0.5, z0: -0.5, z1: height - 0.5 }),
      frameBounds: level3dResizeSliceFrameBounds({ width, depth, height }, "depth", "backward", mode),
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
  } else if (dimension === "depth" && edge === "forward") {
    bounds.y1 = bounds.y0;
    bounds.y0 -= 1;
  } else if (dimension === "depth" && edge === "backward") {
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
  } else if (dimension === "depth" && edge === "forward") {
    bounds.y1 = bounds.y0 + 1;
  } else if (dimension === "depth" && edge === "backward") {
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
  const selected = level3dSelectedEntry();
  const usableFaces = selected?.objects?.length ? faces.filter((face) => face.place) : faces.filter((face) => face.remove);
  let nearest = null;
  let nearestDistance = Infinity;
  for (const face of usableFaces) {
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

function drawLevel3dStagePreview(ctx, width, height) {
  const snapshot = level3dRuntimeSnapshot();
  if (!snapshot) {
    return;
  }
  const view = level3dPreviewView(snapshot, width, height, level3dStageViewOptions());
  ctx.save();
  ctx.fillStyle = "#f5f3ef";
  ctx.fillRect(0, 0, width, height);
  drawLevel3dCellsPreview(ctx, width, height, snapshot, snapshot.cells || [], level3dStageViewOptions());
  drawLevel3dPreviewGrid(ctx, snapshot, view);
  ctx.restore();
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
  const view = level3dPreviewView(snapshot, width, height, level3dStageViewOptions());
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
    return [{
      fill: "#ffde8a",
      scale: 1,
      position: { ...position },
      bounds: level3dVoxelBounds(position, 1),
    }];
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
      x: (Math.max(1, Number(size.width) || 1) - 1) / 2 + (Number(previewOrigin.x) || 0),
      y: (Math.max(1, Number(size.depth) || 1) - 1) / 2 + (Number(previewOrigin.y) || 0),
      z: (Math.max(1, Number(size.height) || 1) - 1) / 2 + (Number(previewOrigin.z) || 0),
    },
    origin: {
      x: width / 2 - ((bounds.minX + bounds.maxX) / 2) * scale,
      y: height / 2 - ((bounds.minY + bounds.maxY) / 2) * scale,
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
  await copyTextToClipboard(levelDefinition3dSource(levelName, level3dSourceData(), ""));
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
  resetLevel3dPreviewState(previewExport || extractPreviewExport(latestHtml));
  renderLevel3dPreviewControls();
  level3dStageHit = null;
  renderLevel3dStageOverlay();
  refreshLevel3dRuntimePreviews();
  setLevel3dActionStatus("Reset 3D preview view", "is-ok");
}

function resetLevel3dPreviewState(source = previewExport || extractPreviewExport(latestHtml)) {
  level3dPreviewCameraState = level3dBasePreviewCamera(source);
  level3dPreviewOrigin = { x: 0, y: 0, z: 0 };
}

function level3dPreviewScrubTarget(event) {
  return event.target?.closest?.("[data-level3d-preview]") || null;
}

function startLevel3dPreviewScrub(event) {
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
window.addEventListener("message", handleLevel3dLayerRendererViewMessage);
window.addEventListener("resize", scheduleLevel3dSurfaceResize);
if (window.ResizeObserver) {
  const level3dSurfaceObserver = new ResizeObserver(scheduleLevel3dSurfaceResize);
  if (level3dStageCanvas) {
    level3dSurfaceObserver.observe(level3dStageCanvas);
  }
  if (level3dLayerBoard) {
    level3dSurfaceObserver.observe(level3dLayerBoard);
  }
}
copyLevel3dButton?.addEventListener("click", () => {
  copyLevel3dToClipboard().catch((error) => setLevel3dActionStatus(error?.message || String(error), "is-error"));
});
addLevel3dButton?.addEventListener("click", addLevel3dToSource);
updateLevel3dButton?.addEventListener("click", updateLevel3dInSource);
registerSourceEditableTarget?.("level3d", {
  find: findLevel3dDefinitionAtPosition,
  load: loadLevel3dFromSourcePosition,
});
