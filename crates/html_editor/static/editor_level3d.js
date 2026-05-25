// 3D level editor source roundtrip and runtime bridge. Rendering stays in puzzle3_app.js.
let level3dRuntimeFrameKey = "";
let level3dRuntimeFrameLoaded = false;
let level3dStageOverlay = null;
let level3dStageHit = null;
let level3d = {
  width: 0,
  depth: 0,
  height: 0,
  slice: 0,
  selectedChar: "_",
  palette: [],
  slices: [],
  sourceKey: "",
};

function renderLevel3dBuilder() {
  if (!level3dBuilder) {
    return;
  }
  syncLevel3dControlsFromPreview();
  renderLevel3dPalette();
  renderLevel3dSourcePreview();
  renderLevel3dRuntime();
  renderLevel3dStageOverlay();
}

function syncLevel3dControlsFromPreview() {
  const exportData = previewExport || extractPreviewExport(latestHtml);
  const levelEntry = currentLevel3dEntry(exportData);
  const sourceDefinition = levelEntry ? null : currentLevel3dSourceDefinition(activePreviewSource());
  if (!levelEntry && !sourceDefinition && level3d.sourceKey) {
    level3d = {
      width: 0,
      depth: 0,
      height: 0,
      slice: 0,
      selectedChar: "_",
      palette: [{ char: "_", objects: [] }],
      slices: [],
      sourceKey: "",
    };
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
    ? currentLevel3dEditorSourceKey(levelEntry)
    : currentLevel3dEditorSourceKey(sourceDefinition);
  if (levelEntry && sourceKey !== level3d.sourceKey) {
    loadLevel3dFromEntry(levelEntry, activePreviewSource(), exportData, sourceKey);
  } else if (sourceDefinition && sourceKey !== level3d.sourceKey) {
    loadLevel3dFromSourceDefinition(sourceDefinition, activePreviewSource(), sourceKey);
  }
  const size = levelEntry?.size || sourceDefinition?.size || exportData?.size || {};
  if (level3dSizeLabel) {
    const width = level3d.width || Number(size.width) || 0;
    const depth = level3d.depth || Number(size.depth) || 0;
    const height = level3d.height || Number(size.height) || 0;
    level3dSizeLabel.textContent = `${width} x ${depth} x ${height}`;
  }
}

function currentLevel3dEditorSourceKey(levelSource = currentLevel3dEntry() || currentLevel3dSourceDefinition(activePreviewSource())) {
  if (!levelSource) {
    return "";
  }
  const documentId = activePreviewDocument()?.id || "";
  const index = currentEditableLevelIndex(previewExport || extractPreviewExport(latestHtml));
  const sourcePosition = Number.isInteger(levelSource.start) ? levelSource.start : index;
  return `${documentId}:${sourcePosition}:${levelSource.name || ""}:${activePreviewSource().length}`;
}

function loadLevel3dFromEntry(entry, source, exportData = previewExport, sourceKey = "") {
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
      || "_";
  }
  level3d.sourceKey = sourceKey || currentLevel3dEditorSourceKey(entry);
}

function loadLevel3dFromSourceDefinition(definition, source, sourceKey = "") {
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
      || "_";
  }
  level3d.sourceKey = sourceKey || currentLevel3dEditorSourceKey(definition);
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
    unique.unshift({ char: "_", objects: [] });
  }
  return unique.length ? unique : [{ char: "_", objects: [] }];
}

function level3dEmptyChar(entries = level3d.palette) {
  return (entries || []).find((entry) => entry.objects.length === 0)?.char || "_";
}

function level3dSlicesFromRows(rows, emptyChar = "_") {
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

function currentLevel3dBundleName(exportData = previewExport) {
  const sourceRange = findLevels3InsertionRange(activePreviewSource(), "");
  if (sourceRange?.bundle) {
    return sourceRange.bundle;
  }
  const bundles = Object.keys(exportData?.levelBundles || {}).filter((name) => !["default", "levels"].includes(name));
  return bundles[0] || "levels";
}

function isPuzzle3dExport(exportData) {
  return Boolean(exportData?.__kind === "puzzle3d" || exportData?.directions?.forward || exportData?.levelBundles);
}

function renderLevel3dSourcePreview() {
  if (!level3dSourcePreview) {
    return;
  }
  const levelName = sanitizeLevel3dName(level3dNameInput?.value || currentLevel3dEntry()?.name || "level_1");
  const sourceData = level3dSourceData();
  level3dSourcePreview.textContent = levelDefinition3dSource(levelName, sourceData, "");
}

function level3dSourceData(source = activePreviewSource(), exportData = previewExport || extractPreviewExport(latestHtml)) {
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
          text += exactByObjects.get("") || "_";
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
    const block = source.slice(range.bodyStart, range.bodyEnd);
    const legendMatch = block.match(/(^|\n)([\t ]*)legend\s*\{\n([\s\S]*?)\n\2\}/m);
    if (!legendMatch) {
      continue;
    }
    for (const raw of legendMatch[3].split("\n")) {
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
  }
  if (!entries.some((entry) => entry.objects.length === 0)) {
    entries.unshift({ char: "_", objects: [] });
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
    const definition = exact || definitions[0];
    return {
      ...definition,
      bundle: range.bundle,
      model: range.model,
      rows: rowsForLevel3dDefinition(source, definition),
    };
  }
  const range = ranges[0];
  const definitions = findLevel3dDefinitions(source, range);
  const definition = definitions[0];
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
  for (const entry of level3d.palette) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "level-token level3d-token";
    button.classList.toggle("is-selected", entry.char === level3d.selectedChar);
    button.dataset.label = level3dPaletteEntryLabel(entry);
    button.title = level3dPaletteEntryLabel(entry);
    button.setAttribute("aria-label", `Paint ${level3dPaletteEntryLabel(entry)}`);

    const visual = document.createElement("canvas");
    visual.className = "level3d-token-preview";
    visual.width = 64;
    visual.height = 48;
    visual.setAttribute("aria-hidden", "true");
    button.append(visual);

    const label = document.createElement("span");
    label.className = "tile-label level3d-token-label";
    label.textContent = entry.objects.length ? entry.objects.join(" ") : "empty";
    button.append(label);

    button.addEventListener("click", () => {
      level3d.selectedChar = entry.char;
      renderLevel3dPalette();
      renderLevel3dStageOverlay();
    });
    level3dPalette.append(button);
    drawLevel3dPalettePreview(visual, entry, exportData);
  }
}

function level3dPaletteEntryLabel(entry) {
  return entry.objects.length ? `${entry.char} = ${entry.objects.join(" ")}` : `${entry.char} = empty`;
}

function drawLevel3dPalettePreview(canvas, entry, exportData = previewExport) {
  if (!(canvas instanceof HTMLCanvasElement)) {
    return;
  }
  const ctx = canvas.getContext("2d");
  if (!ctx) {
    return;
  }
  const width = canvas.width;
  const height = canvas.height;
  ctx.clearRect(0, 0, width, height);
  const objects = (entry.objects || [])
    .map((name) => level3dObjectDescriptor(name, exportData))
    .filter(Boolean);
  if (!objects.length) {
    drawLevel3dEmptyTilePreview(ctx, width, height);
    return;
  }
  const snapshot = {
    size: { width: 1, depth: 1, height: 1 },
    camera: level3dPreviewCamera(exportData),
    sprites: exportData?.sprites || {},
    settings: exportData?.settings || {},
  };
  drawLevel3dCellsPreview(ctx, width, height, snapshot, [{
    position: { x: 0, y: 0, z: 0 },
    objects,
  }], { padding: 0.92 });
}

function drawLevel3dEmptyTilePreview(ctx, width, height) {
  ctx.save();
  ctx.translate(width / 2, height / 2 + 4);
  ctx.strokeStyle = "rgba(157, 163, 170, 0.78)";
  ctx.lineWidth = 1.5;
  ctx.beginPath();
  ctx.moveTo(0, -15);
  ctx.lineTo(23, -2);
  ctx.lineTo(0, 12);
  ctx.lineTo(-23, -2);
  ctx.closePath();
  ctx.stroke();
  ctx.restore();
}

function level3dCellLabel(ch) {
  const entry = level3d.palette.find((candidate) => candidate.char === ch);
  return entry ? level3dPaletteEntryLabel(entry) : ch;
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
  syncPreviewStateFromLevel3d();
  renderLevel3dStageOverlay();
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
  return [
    `${indent}level ${sanitizeLevel3dName(name)} {`,
    ...rows.map((row) => `${indent}${row}`),
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
    });
    index = close.index;
  }
  return definitions;
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
  const levelSource = levelDefinition3dSource(name, levelData, indent).trimEnd();
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
    const replacement = levelDefinition3dSource(requested, levelData, entry.indent).trimEnd();
    return {
      source: `${source.slice(0, entry.start)}${replacement}${source.slice(entry.end)}`,
    };
  }
  return null;
}

function currentLevel3dSourceLocation() {
  const document = activePreviewDocument();
  const source = activePreviewSource();
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

function level3dSourceLinesWithOffsets(source) {
  const lines = [];
  let start = 0;
  for (const raw of String(source || "").split("\n")) {
    const end = start + raw.length;
    lines.push({ raw, start, end, absoluteEnd: end });
    start = end + 1;
  }
  return lines;
}

function level3dScannerCode(line) {
  return String(line || "").split("//", 1)[0].trim();
}

function renderLevel3dRuntime() {
  if (!level3dRuntimeFrame) {
    return;
  }
  if (!latestHtml) {
    level3dRuntimeFrame.removeAttribute("srcdoc");
    setLevel3dActionStatus("Run Preview first", "");
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
    level3dRuntimeFrame.srcdoc = editorPreviewDocument(latestHtml);
    return;
  }
  sendLevel3dSnapshotToRuntime();
}

function sendLevel3dSnapshotToRuntime() {
  if (!level3dRuntimeFrameLoaded || !level3dRuntimeFrame?.contentWindow) {
    return;
  }
  const snapshot = level3dRuntimeSnapshot();
  if (!snapshot) {
    return;
  }
  level3dRuntimeFrame.contentWindow.postMessage({
    type: "PuzzleStudioSetPuzzle3Snapshot",
    snapshot,
    preferPuzzleScene: true,
    componentEmbed: true,
  }, "*");
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
  const selected = level3dSelectedEntry();
  const selectedChar = selected?.char || level3d.selectedChar || level3dEmptyChar();
  const target = selected?.objects?.length ? hit.place : hit.remove;
  if (!target) {
    return;
  }
  if (paintLevel3dCellAtPosition(target, selectedChar)) {
    level3dStageHit = null;
    setLevel3dActionStatus(level3dCellLabel(selectedChar), "is-ok");
    renderLevel3dStageOverlay();
  }
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
  return `${hit.kind}:${place}:${remove}`;
}

function level3dSelectedEntry() {
  return level3d.palette.find((entry) => entry.char === level3d.selectedChar)
    || level3d.palette[0]
    || { char: level3dEmptyChar(), objects: [] };
}

function level3dRuntimeSnapshot() {
  const exportData = previewExport || extractPreviewExport(latestHtml);
  if (!isPuzzle3dExport(exportData)) {
    return null;
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
  return snapshot;
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
  primitives.sort((left, right) => level3dPrimitiveOrder(left) - level3dPrimitiveOrder(right));
  for (const primitive of primitives) {
    ctx.fillStyle = primitive.fill;
    drawLevel3dPolygonPath(ctx, primitive.points);
    ctx.fill();
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
  drawLevel3dStageFloor(ctx, snapshot, view);
  drawLevel3dCellsPreview(ctx, width, height, snapshot, snapshot.cells || [], level3dStageViewOptions());
  ctx.restore();
}

function drawLevel3dStageFloor(ctx, snapshot, view) {
  const size = snapshot?.size || {};
  const width = Math.max(1, Math.trunc(Number(size.width) || level3d.width || 1));
  const depth = Math.max(1, Math.trunc(Number(size.depth) || level3d.depth || 1));
  const cells = [];
  for (let y = 0; y < depth; y += 1) {
    for (let x = 0; x < width; x += 1) {
      cells.push(level3dPlacementFace("zPos", { x, y, z: -1 }, view, { kind: "floor" }));
    }
  }
  cells.sort((left, right) => level3dPrimitiveOrder(left) - level3dPrimitiveOrder(right));
  ctx.save();
  ctx.lineWidth = 1;
  ctx.fillStyle = "rgba(255, 255, 255, 0.38)";
  ctx.strokeStyle = "rgba(87, 93, 99, 0.18)";
  for (const cell of cells) {
    drawLevel3dPolygonPath(ctx, cell.polygon);
    ctx.fill();
    ctx.stroke();
  }
  ctx.restore();
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
  const camera = level3dPreviewCamera(snapshot);
  const size = snapshot?.size || { width: 1, depth: 1, height: 1 };
  const bounds = level3dProjectedBoundsUnit(size, camera);
  const boundsWidth = Math.max(0.001, bounds.maxX - bounds.minX);
  const boundsHeight = Math.max(0.001, bounds.maxY - bounds.minY);
  const padding = Number(options.padding) || 0.72;
  const scale = Math.min(width / boundsWidth, height / boundsHeight) * padding;
  return {
    camera,
    center: {
      x: (Math.max(1, Number(size.width) || 1) - 1) / 2,
      y: (Math.max(1, Number(size.depth) || 1) - 1) / 2,
      z: (Math.max(1, Number(size.height) || 1) - 1) / 2,
    },
    origin: {
      x: width / 2 - ((bounds.minX + bounds.maxX) / 2) * scale,
      y: height / 2 - ((bounds.minY + bounds.maxY) / 2) * scale,
    },
    scale,
  };
}

function level3dPreviewCamera(source) {
  const camera = source?.camera || previewExport?.camera || {};
  return {
    yawDegrees: Number(camera.yawDegrees ?? 15),
    pitchDegrees: Number(camera.pitchDegrees ?? 55),
    zoom: Number(camera.zoom ?? 1),
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

function syncPreviewStateFromLevel3d() {
  const exportData = previewExport || extractPreviewExport(latestHtml);
  const edited = level3dSnapshotLevelData(exportData);
  if (!exportData || !edited) {
    sendLevel3dSnapshotToRuntime();
    return;
  }
  const levelIndex = currentEditableLevelIndex(exportData);
  if (previewExport?.levels?.[levelIndex]) {
    const nextExport = JSON.parse(JSON.stringify(previewExport));
    nextExport.levels[levelIndex].size = edited.size;
    nextExport.levels[levelIndex].cells = edited.cells;
    nextExport.levelIndex = levelIndex;
    nextExport.size = { ...edited.size };
    nextExport.cells = edited.cells;
    previewExport = nextExport;
    const nextHtml = replacePreviewExport(latestHtml, nextExport);
    if (nextHtml) {
      latestHtml = nextHtml;
      const previewDocument = activePreviewDocument();
      if (previewDocument) {
        previewDocument.previewHtml = nextHtml;
      }
      scheduleLocalSave();
    }
  }
  sendLevel3dSnapshotToRuntime();
}

async function copyLevel3dToClipboard() {
  const levelName = sanitizeLevel3dName(level3dNameInput?.value || currentLevel3dEntry()?.name || "level_1");
  await copyTextToClipboard(levelDefinition3dSource(levelName, level3dSourceData(), ""));
  setLevel3dActionStatus("Copied 3D level", "is-ok");
}

function addLevel3dToSource() {
  ensurePreviewTargetsActiveDocument();
  const previewDocument = activePreviewDocument();
  if (!previewDocument) {
    setLevel3dActionStatus("No game entry for 3D level", "is-error");
    return;
  }
  const levelName = sanitizeLevel3dName(level3dNameInput?.value || "level_1");
  const bundle = sanitizeLevel3dBundle(level3dBundleInput?.value || "");
  const nextSource = insertLevel3d(activePreviewSource(), levelName, level3dSourceData(), bundle);
  if (!nextSource) {
    setLevel3dActionStatus(`No levels3 block named ${bundle}`, "is-error");
    return;
  }
  applyLevel3dSourceChange(previewDocument, nextSource);
  level3dNameInput.value = nextLevelName(levelName);
  setLevel3dActionStatus("Added 3D level", "is-ok");
}

function updateLevel3dInSource() {
  ensurePreviewTargetsActiveDocument();
  const previewDocument = activePreviewDocument();
  if (!previewDocument) {
    setLevel3dActionStatus("No game entry for 3D level", "is-error");
    return;
  }
  const levelName = sanitizeLevel3dName(level3dNameInput?.value || "level_1");
  const bundle = sanitizeLevel3dBundle(level3dBundleInput?.value || "");
  const result = replaceLevel3dByName(activePreviewSource(), levelName, level3dSourceData(), bundle);
  if (!result) {
    setLevel3dActionStatus(`No 3D level named ${bundle}.${levelName}`, "is-error");
    return;
  }
  applyLevel3dSourceChange(previewDocument, result.source);
  setLevel3dActionStatus(`Updated 3D level ${levelName}`, "is-ok");
}

function applyLevel3dSourceChange(previewDocument, source) {
  previewDocument.source = source;
  level3d.sourceKey = "";
  if (previewDocument.id === activeDocument()?.id) {
    setSourceEditorValue(source, { resetUndo: false });
  }
  scheduleLocalSave();
  schedulePreview();
}

function setLevel3dActionStatus(text, className = "") {
  if (level3dActionStatus) {
    level3dActionStatus.className = `sprite-action-status ${className || ""}`.trim();
    level3dActionStatus.textContent = text || "";
  }
  if (text && typeof setStatus === "function") {
    setStatus(text, className);
  }
}

level3dBundleInput?.addEventListener("input", () => {
  level3dBundleInput.dataset.userEdited = "true";
  renderLevel3dSourcePreview();
});
level3dNameInput?.addEventListener("input", () => {
  level3dNameInput.dataset.userEdited = "true";
  renderLevel3dSourcePreview();
});
copyLevel3dButton?.addEventListener("click", () => {
  copyLevel3dToClipboard().catch((error) => setLevel3dActionStatus(error?.message || String(error), "is-error"));
});
addLevel3dButton?.addEventListener("click", addLevel3dToSource);
updateLevel3dButton?.addEventListener("click", updateLevel3dInSource);
