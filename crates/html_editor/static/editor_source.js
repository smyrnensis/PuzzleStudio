// Source editor state, text-editing commands, highlighting, completions, color editing, and source textarea event binding.
const sourceColorPopover = createSourceColorPopover();
const sourceCompletionPopover = createSourceCompletionPopover();
const sourceCompletionTextEncoder = new TextEncoder();
const sourceBlockSelectionLayer = createSourceBlockSelectionLayer();
const sourceFindMatchLayer = createSourceFindMatchLayer();
const sourceFindPanel = createSourceFindPanel();
const sourceFindInput = sourceFindPanel?.querySelector("[data-source-find-input]");
const sourceReplaceInput = sourceFindPanel?.querySelector("[data-source-replace-input]");
const sourceFindStatus = sourceFindPanel?.querySelector("[data-source-find-status]");
const sourceFindCaseButton = sourceFindPanel?.querySelector("[data-source-find-case]");
const sourceImportLinkFrame = createSourceImportLinkFrame();
const SOURCE_EDITABLE_TARGETS = [
  {
    kind: "level3d",
    label: "3D level",
    openOptions: { switchMode: true },
  },
  {
    kind: "level",
    label: "level",
    openOptions: {},
  },
  {
    kind: "sprite3d",
    label: "3D sprite",
    openOptions: { switchMode: true },
  },
  {
    kind: "sprite",
    label: "sprite",
    openOptions: { switchMode: true },
  },
  {
    kind: "sounds",
    label: (entry) => entry?.soundKind || "sound",
    openOptions: { switchMode: true },
  },
];
const sourceEditableTargetHandlers = new Map();
let sourceHighlightTimer = 0;
let sourceOptimisticHighlightFrame = 0;
let sourceOptimisticHighlightSource = null;
let sourceCompletionTimer = 0;
let sourceOutlineTimer = 0;
let activeHighlightRequest = null;
let sourceHighlightRequestId = 0;
let sourceCompletionRequestId = 0;
let sourceOutlineRequestId = 0;
let sourceOutlineSignature = "";
let sourceOutlineDirty = true;
let sourceColorEdit = null;
let sourceCompletionState = null;
let sourceImportLinkState = null;
let sourceEditorKillRing = "";
let sourceEditorBlockSelection = null;
let sourceEditorPreferredCaretX = null;
let sourceFindState = {
  matches: [],
  selectedIndex: -1,
  matchCase: false,
  replaceVisible: false,
};
let sourceUndoStack = [];
let sourceRedoStack = [];
let sourceUndoApplying = false;
let sourceHighlightSource = "";
let sourceHighlightHtml = "";
let sourceHighlightMode = "";
let sourceHighlightRuns = [];
let sourceHighlightUnavailableStatusShown = false;
let sourcePlainTextModeActive = false;
let sourceLayoutSyncFrame = 0;
let sourceCompositionPreviewSource = "";
let sourceCompositionRange = null;
let sourceLevelBuilderResetFrame = 0;
let sourceLevelBuilderResetCells = false;
let sourceLevelBuilderResetSignature = null;
let sourceHighlightClientWidth = 0;
let sourceHighlightScrollHeight = 0;
let sourceFoldedBlockKeys = new Set();
let sourceFoldBaseSource = null;
let sourceFoldViewMap = [];
let sourceFoldBlockCacheSource = "";
let sourceFoldBlockCache = [];
let sourceFoldEditSnapshot = null;
let sourceOutlineItems = [];
let sourceOutlineExpandedItemIds = new Set();

function sourcePuzzleLevelName(value, defaultName = "") {
  const text = String(value ?? "").trim();
  return text || String(defaultName ?? "").trim();
}

function sourcePuzzleQuotedText(value, context = "source text") {
  const text = String(value ?? "");
  if (/[\r\n]/.test(text)) {
    throw new Error(`${context} cannot contain line breaks`);
  }
  return `"${text.replace(/"/g, "\\\"")}"`;
}

function parseSourcePuzzleQuotedText(value) {
  const text = String(value ?? "").trim();
  if (!text.startsWith("\"") || !text.endsWith("\"")) {
    return null;
  }
  return text.slice(1, -1).replace(/\\"/g, "\"");
}

function sourcePuzzleLevelHeaderName(code) {
  const text = String(code || "").trim();
  if (!/^level(?:\s|$)/.test(text)) {
    return null;
  }
  let rest = text.slice("level".length).trim();
  if (rest.endsWith("{")) {
    rest = rest.slice(0, -1).trim();
  }
  if (!rest) {
    return "";
  }
  return parseSourcePuzzleQuotedText(rest);
}

function sourcePuzzleLevelHeaderSource(name, indent = "", options = {}) {
  const levelName = sourcePuzzleLevelName(name, options.defaultName || "");
  const opensBlock = options.openBlock === true;
  if (!levelName) {
    return opensBlock ? `${indent}{` : `${indent}level`;
  }
  return `${indent}level ${sourcePuzzleQuotedText(levelName, "level name")}${opensBlock ? " {" : ""}`;
}

function sourceEditorDocumentValue() {
  return sourceFoldBaseSource !== null
    ? sourceFoldBaseSource
    : sourceEditor.value || "";
}

function sourceFoldsActive() {
  return sourceFoldBaseSource !== null && sourceFoldedBlockKeys.size > 0;
}

function sourceFoldedLineCount() {
  if (!sourceFoldsActive()) {
    return 0;
  }
  return sourceFoldRangesForSource(sourceEditorDocumentValue())
    .reduce((total, range) => total + Math.max(0, range.hiddenLineCount), 0);
}

function resetSourceFoldingState() {
  sourceFoldedBlockKeys = new Set();
  sourceFoldBaseSource = null;
  sourceFoldViewMap = [];
  sourceFoldBlockCacheSource = "";
  sourceFoldBlockCache = [];
}

function sourceFoldableBlocks(source) {
  // CodeMirror owns generic folding. Puzzle syntax recognition must stay in
  // Rust, so the removed textarea folding path cannot infer brace blocks here.
  void source;
  return [];
}

function sourceFoldRangesForSource(source) {
  if (!sourceFoldedBlockKeys.size) {
    return [];
  }
  const selected = sourceFoldableBlocks(source)
    .filter((block) => sourceFoldedBlockKeys.has(block.key))
    .sort((left, right) => (
      left.sourceStart - right.sourceStart
      || right.sourceEnd - left.sourceEnd
    ));
  const ranges = [];
  let coveredUntil = -1;
  for (const block of selected) {
    if (block.sourceStart < coveredUntil) {
      continue;
    }
    ranges.push(block);
    coveredUntil = block.sourceEnd;
  }
  return ranges;
}

function sourceFoldStateForSource(source = sourceEditorDocumentValue()) {
  if (sourceEditor.sourceEditorPort?.kind === "codemirror") {
    return [];
  }
  return sourceFoldableBlocks(source)
    .filter((block) => sourceFoldedBlockKeys.has(block.key))
    .map((block) => block.key);
}

function restoreSourceFoldState(keys = []) {
  if (sourceEditor.sourceEditorPort?.kind === "codemirror") {
    resetSourceFoldingState();
    return false;
  }
  const validKeys = new Set(sourceFoldableBlocks(sourceEditorDocumentValue()).map((block) => block.key));
  sourceFoldedBlockKeys = new Set(
    (Array.isArray(keys) ? keys : []).filter((key) => typeof key === "string" && validKeys.has(key)),
  );
  applySourceFoldingView();
  return sourceFoldedBlockKeys.size > 0;
}

function buildSourceFoldedView(source) {
  const text = String(source || "");
  const ranges = sourceFoldRangesForSource(text);
  const map = [];
  let view = "";
  let sourceCursor = 0;
  let viewCursor = 0;

  const appendVisible = (sourceStart, sourceEnd) => {
    if (sourceEnd <= sourceStart) {
      return;
    }
    const chunk = text.slice(sourceStart, sourceEnd);
    view += chunk;
    map.push({
      kind: "visible",
      viewStart: viewCursor,
      viewEnd: viewCursor + chunk.length,
      sourceStart,
      sourceEnd,
    });
    viewCursor += chunk.length;
  };

  for (const range of ranges) {
    appendVisible(sourceCursor, range.sourceStart);
    const marker = sourceFoldMarker(range);
    view += marker;
    map.push({
      kind: "fold",
      viewStart: viewCursor,
      viewEnd: viewCursor + marker.length,
      sourceStart: range.sourceStart,
      sourceEnd: range.sourceEnd,
    });
    viewCursor += marker.length;
    sourceCursor = range.sourceEnd;
  }
  appendVisible(sourceCursor, text.length);
  return { view, map, ranges };
}

function sourceFoldMarker(range) {
  return `${sourceFoldMarkerPrefix()}${sourceFoldMarkerBrace()}${range?.hasTrailingNewline ? "\n" : ""}`;
}

function sourceFoldMarkerPrefix() {
  return " ⋯ ";
}

function sourceFoldMarkerBrace() {
  return "}";
}

function sourceViewOffsetToDocumentOffset(offset, affinity = "start") {
  if (!sourceFoldsActive()) {
    return Math.max(0, Math.min((sourceEditor.value || "").length, offset || 0));
  }
  return sourceViewOffsetToDocumentOffsetWithMap(
    sourceFoldViewMap,
    sourceEditorDocumentValue().length,
    offset,
    affinity,
  );
}

function sourceViewOffsetToDocumentOffsetWithMap(map, documentLength, offset, affinity = "start") {
  const viewOffset = Math.max(0, offset || 0);
  for (const segment of map || []) {
    if (viewOffset < segment.viewStart) {
      return segment.sourceStart;
    }
    if (viewOffset <= segment.viewEnd) {
      if (segment.kind === "fold") {
        if (affinity === "end" || viewOffset >= segment.viewEnd) {
          return segment.sourceEnd;
        }
        return segment.sourceStart;
      }
      return segment.sourceStart + Math.max(0, Math.min(segment.sourceEnd - segment.sourceStart, viewOffset - segment.viewStart));
    }
  }
  return Math.max(0, documentLength || 0);
}

function sourceDocumentOffsetToViewOffset(offset, affinity = "start") {
  if (!sourceFoldsActive()) {
    return Math.max(0, Math.min((sourceEditor.value || "").length, offset || 0));
  }
  const sourceOffset = Math.max(0, offset || 0);
  for (const segment of sourceFoldViewMap) {
    if (sourceOffset < segment.sourceStart) {
      return segment.viewStart;
    }
    if (sourceOffset <= segment.sourceEnd) {
      if (segment.kind === "fold") {
        return affinity === "end" && sourceOffset >= segment.sourceEnd
          ? segment.viewEnd
          : segment.viewStart;
      }
      return segment.viewStart + Math.max(0, Math.min(segment.viewEnd - segment.viewStart, sourceOffset - segment.sourceStart));
    }
  }
  return sourceEditor.value.length;
}

function applySourceFoldingView(options = {}) {
  const documentSource = sourceEditorDocumentValue();
  const previousStart = sourceViewOffsetToDocumentOffset(sourceEditor.selectionStart || 0, "start");
  const previousEnd = sourceViewOffsetToDocumentOffset(sourceEditor.selectionEnd || sourceEditor.selectionStart || 0, "end");
  const previousDirection = sourceEditor.selectionDirection || "none";
  const folded = buildSourceFoldedView(documentSource);
  if (!folded.ranges.length) {
    sourceFoldBaseSource = null;
    sourceFoldViewMap = [];
    sourceEditor.value = documentSource;
  } else {
    sourceFoldBaseSource = documentSource;
    sourceFoldViewMap = folded.map;
    sourceEditor.value = folded.view;
  }
  const selectionStart = sourceDocumentOffsetToViewOffset(previousStart, "start");
  const selectionEnd = sourceDocumentOffsetToViewOffset(previousEnd, "end");
  sourceEditor.setSelectionRange(selectionStart, selectionEnd, previousDirection);
  if (options.refresh !== false) {
    scheduleSourceHighlight(true, { preserveCurrent: false });
    updateSourceMeta();
    hideSourceColorEditor();
    hideSourceCompletions();
    hideSourceImportLinkFrame();
    clearSourceBlockSelection();
  }
}

function captureSourceFoldEditSnapshot() {
  if (!sourceFoldsActive()) {
    sourceFoldEditSnapshot = null;
    return;
  }
  const source = sourceEditorDocumentValue();
  sourceFoldEditSnapshot = {
    source,
    view: sourceEditor.value || "",
    map: sourceFoldViewMap.map((segment) => ({ ...segment })),
    ranges: sourceFoldRangesForSource(source).map((range) => ({ ...range })),
  };
}

function expandSourceFoldsForEdit() {
  if (!sourceFoldsActive()) {
    return false;
  }
  const documentSource = sourceEditorDocumentValue();
  const selectionStart = sourceViewOffsetToDocumentOffset(sourceEditor.selectionStart || 0, "start");
  const selectionEnd = sourceViewOffsetToDocumentOffset(sourceEditor.selectionEnd || sourceEditor.selectionStart || 0, "end");
  const selectionDirection = sourceEditor.selectionDirection || "none";
  sourceFoldedBlockKeys = new Set();
  sourceFoldBaseSource = null;
  sourceFoldViewMap = [];
  sourceEditor.value = documentSource;
  sourceEditor.setSelectionRange(selectionStart, selectionEnd, selectionDirection);
  scheduleSourceHighlight(true, { preserveCurrent: false });
  updateSourceMeta();
  hideSourceColorEditor();
  hideSourceCompletions();
  hideSourceImportLinkFrame();
  clearSourceBlockSelection();
  return true;
}

function commitSourceFoldedDisplayEdit() {
  if (!sourceFoldsActive()) {
    sourceFoldEditSnapshot = null;
    return true;
  }
  const snapshot = sourceFoldEditSnapshot || {
    source: sourceFoldBaseSource || "",
    view: buildSourceFoldedView(sourceFoldBaseSource || "").view,
    map: sourceFoldViewMap.map((segment) => ({ ...segment })),
    ranges: sourceFoldRangesForSource(sourceFoldBaseSource || "").map((range) => ({ ...range })),
  };
  const before = snapshot.view || "";
  const after = sourceEditor.value || "";
  let prefix = 0;
  const maxPrefix = Math.min(before.length, after.length);
  while (prefix < maxPrefix && before[prefix] === after[prefix]) {
    prefix += 1;
  }
  let suffix = 0;
  const maxSuffix = Math.min(before.length - prefix, after.length - prefix);
  while (
    suffix < maxSuffix
    && before[before.length - suffix - 1] === after[after.length - suffix - 1]
  ) {
    suffix += 1;
  }

  const oldViewStart = prefix;
  const oldViewEnd = before.length - suffix;
  const inserted = after.slice(prefix, after.length - suffix);
  if (sourceFoldEditTouchesMarker(snapshot.map, oldViewStart, oldViewEnd)) {
    sourceFoldBaseSource = snapshot.source;
    sourceFoldedBlockKeys = new Set(
      (snapshot.ranges || [])
        .filter((range) => !sourceFoldRangeMarkerTouched(snapshot.map, range, oldViewStart, oldViewEnd))
        .map((range) => range.key),
    );
    sourceFoldEditSnapshot = null;
    applySourceFoldingView({ refresh: false });
    return false;
  }
  const sourceStart = sourceViewOffsetToDocumentOffsetWithMap(
    snapshot.map,
    snapshot.source.length,
    oldViewStart,
    "start",
  );
  const sourceEnd = sourceViewOffsetToDocumentOffsetWithMap(
    snapshot.map,
    snapshot.source.length,
    oldViewEnd,
    "end",
  );
  const nextSource = `${snapshot.source.slice(0, sourceStart)}${inserted}${snapshot.source.slice(sourceEnd)}`;
  sourceFoldBaseSource = nextSource;
  sourceFoldedBlockKeys = sourceFoldKeysAfterEdit(snapshot, nextSource, sourceStart, sourceEnd, inserted);
  sourceFoldEditSnapshot = null;
  applySourceFoldingView({ refresh: false });
  return true;
}

function sourceFoldEditTouchesMarker(map, viewStart, viewEnd) {
  return (map || []).some((segment) => (
    segment.kind === "fold"
    && viewStart < segment.viewEnd
    && viewEnd > segment.viewStart
    && !(viewStart === segment.viewStart && viewEnd === segment.viewStart)
    && !(viewStart === segment.viewEnd && viewEnd === segment.viewEnd)
  ));
}

function sourceFoldRangeMarkerTouched(map, range, viewStart, viewEnd) {
  return (map || []).some((segment) => (
    segment.kind === "fold"
    && segment.sourceStart === range.sourceStart
    && segment.sourceEnd === range.sourceEnd
    && viewStart < segment.viewEnd
    && viewEnd > segment.viewStart
  ));
}

function sourceFoldKeysAfterEdit(snapshot, nextSource, sourceStart, sourceEnd, inserted) {
  const lineDelta = sourceFoldLineDelta(snapshot.source.slice(sourceStart, sourceEnd), inserted);
  const nextBlocks = sourceFoldableBlocks(nextSource);
  const nextKeys = new Set();
  for (const range of snapshot.ranges || []) {
    const overlaps = sourceStart < range.sourceEnd && sourceEnd > range.sourceStart;
    if (overlaps) {
      continue;
    }
    const targetOpenLine = range.openLine + (sourceEnd <= range.sourceStart ? lineDelta : 0);
    const candidate = nextBlocks
      .filter((block) => block.openLine === targetOpenLine)
      .sort((left, right) => Math.abs(left.hiddenLineCount - range.hiddenLineCount) - Math.abs(right.hiddenLineCount - range.hiddenLineCount))[0];
    if (candidate) {
      nextKeys.add(candidate.key);
    }
  }
  return nextKeys;
}

function sourceFoldLineDelta(removed, inserted) {
  return sourceFoldNewlineCount(inserted) - sourceFoldNewlineCount(removed);
}

function sourceFoldNewlineCount(value) {
  return (String(value || "").match(/\n/g) || []).length;
}

function sourceKeydownWillEdit(event) {
  if (!event || event.defaultPrevented || event.isComposing) {
    return false;
  }
  if (!event.metaKey && !event.ctrlKey && !event.altKey) {
    return event.key.length === 1
      || event.key === "Enter"
      || event.key === "Tab"
      || event.key === "Backspace"
      || event.key === "Delete";
  }
  const key = event.key.length === 1 ? event.key.toLowerCase() : event.key;
  if (event.altKey && !event.ctrlKey && !event.metaKey && (event.key === "ArrowUp" || event.key === "ArrowDown")) {
    return true;
  }
  if ((event.metaKey || event.ctrlKey) && !event.altKey) {
    return key === "z"
      || key === "y"
      || key === "/"
      || key === "["
      || key === "]"
      || key === "Enter"
      || (event.shiftKey && key === "k");
  }
  return false;
}

function sourceEditorSnapshot() {
  return {
    value: sourceEditorDocumentValue(),
    selectionStart: sourceViewOffsetToDocumentOffset(sourceEditor.selectionStart || 0, "start"),
    selectionEnd: sourceViewOffsetToDocumentOffset(sourceEditor.selectionEnd || 0, "end"),
    selectionDirection: sourceEditor.selectionDirection || "none",
  };
}

function sameSourceEditorSnapshot(a, b) {
  return Boolean(a && b)
    && a.value === b.value
    && a.selectionStart === b.selectionStart
    && a.selectionEnd === b.selectionEnd
    && a.selectionDirection === b.selectionDirection;
}

function sourceChangedRange(before, after) {
  const beforeText = String(before || "");
  const afterText = String(after || "");
  let prefix = 0;
  const maxPrefix = Math.min(beforeText.length, afterText.length);
  while (prefix < maxPrefix && beforeText[prefix] === afterText[prefix]) {
    prefix += 1;
  }

  let suffix = 0;
  const maxSuffix = Math.min(beforeText.length - prefix, afterText.length - prefix);
  while (
    suffix < maxSuffix
    && beforeText[beforeText.length - suffix - 1] === afterText[afterText.length - suffix - 1]
  ) {
    suffix += 1;
  }

  return {
    start: prefix,
    end: beforeText.length - suffix,
  };
}

function sourceSnapshotWithChangedRangeSelection(snapshot, nextValue) {
  const range = sourceChangedRange(snapshot?.value || "", nextValue || "");
  return {
    ...snapshot,
    selectionStart: range.start,
    selectionEnd: range.end,
    selectionDirection: range.start === range.end ? "none" : "forward",
  };
}

function resetSourceUndoHistory() {
  if (sourceEditor.sourceEditorPort?.kind === "codemirror") {
    sourceUndoStack = [];
    sourceRedoStack = [];
    return;
  }
  sourceUndoStack = [sourceEditorSnapshot()];
  sourceRedoStack = [];
}

function ensureSourceUndoHistory() {
  if (sourceEditor.sourceEditorPort?.kind === "codemirror") {
    return;
  }
  if (!sourceUndoStack.length || sourceUndoStack.at(-1)?.value !== sourceEditorDocumentValue()) {
    resetSourceUndoHistory();
    return;
  }
  const snapshot = sourceEditorSnapshot();
  if (!sameSourceEditorSnapshot(sourceUndoStack.at(-1), snapshot)) {
    sourceUndoStack[sourceUndoStack.length - 1] = snapshot;
  }
}

function recordSourceUndoSnapshot() {
  if (sourceEditor.sourceEditorPort?.kind === "codemirror" || sourceUndoApplying) {
    return;
  }
  const snapshot = sourceEditorSnapshot();
  const previousSnapshot = sourceUndoStack.at(-1);
  if (previousSnapshot && previousSnapshot.value !== snapshot.value) {
    sourceUndoStack[sourceUndoStack.length - 1] = sourceSnapshotWithChangedRangeSelection(previousSnapshot, snapshot.value);
  }
  if (sameSourceEditorSnapshot(sourceUndoStack.at(-1), snapshot)) {
    return;
  }
  sourceUndoStack.push(snapshot);
  if (sourceUndoStack.length > 200) {
    sourceUndoStack.shift();
  }
  sourceRedoStack = [];
}

function restoreSourceEditorSnapshot(snapshot) {
  if (!snapshot) {
    return;
  }
  sourceUndoApplying = true;
  resetSourceFoldingState();
  sourceEditor.value = snapshot.value || "";
  const start = Math.max(0, Math.min(sourceEditor.value.length, snapshot.selectionStart || 0));
  const end = Math.max(0, Math.min(sourceEditor.value.length, snapshot.selectionEnd || start));
  sourceEditor.setSelectionRange(start, end, snapshot.selectionDirection || "none");
  sourceEditorContentChanged();
  scrollSourceOffsetIntoView(start);
  sourceUndoApplying = false;
}

function undoSourceEdit() {
  if (sourceUndoStack.length <= 1) {
    return false;
  }
  sourceRedoStack.push(sourceUndoStack.pop());
  restoreSourceEditorSnapshot(sourceUndoStack.at(-1));
  return true;
}

function redoSourceEdit() {
  const snapshot = sourceRedoStack.pop();
  if (!snapshot) {
    return false;
  }
  sourceUndoStack.push(snapshot);
  restoreSourceEditorSnapshot(snapshot);
  return true;
}

function handleSourceUndoShortcut(event) {
  if (event.altKey) {
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
  ensureSourceUndoHistory();
  event.preventDefault();
  event.stopPropagation();
  if (redo) {
    redoSourceEdit();
  } else {
    undoSourceEdit();
  }
  return true;
}

function setSourceEditorValue(value, options = {}) {
  const nextValue = value || "";
  const currentValue = sourceEditorDocumentValue();
  const preservesUndo = options.preserveUndoOnSameValue === true && currentValue === nextValue;
  const preserveCurrentHighlight = options.preserveHighlight !== false;
  const sameUnfoldedValue = sourceFoldBaseSource === null && currentValue === nextValue;
  if (sameUnfoldedValue && preserveCurrentHighlight && sourceHighlightSource === nextValue) {
    updateSourceMeta();
    if (sourceDocumentSupportsEditableTargets()) {
      scheduleSourceOutlineRefresh(true, { force: true });
    } else {
      resetSourcePuzzleAnalysisState();
    }
    if (preservesUndo) {
      ensureSourceUndoHistory();
    } else if (options.resetUndo === false) {
      recordSourceUndoSnapshot();
    } else {
      resetSourceUndoHistory();
    }
    return;
  }
  resetSourceFoldingState();
  if (sourceEditor.sourceEditorPort?.kind === "codemirror") {
    sourceEditor.sourceEditorPort.replaceDocument(nextValue, {
      preserveHistory: preservesUndo,
    });
  } else {
    sourceEditor.value = nextValue;
  }
  updateSourceMeta();
  if (sourceDocumentSupportsEditableTargets()) {
    scheduleSourceHighlight(true, { preserveCurrent: preserveCurrentHighlight });
    scheduleSourceOutlineRefresh(true, { force: true });
  } else {
    resetSourcePuzzleAnalysisState();
  }
  if (preservesUndo) {
    ensureSourceUndoHistory();
  } else if (options.resetUndo === false) {
    recordSourceUndoSnapshot();
  } else {
    resetSourceUndoHistory();
  }
}

function scheduleSourceHighlight(immediate = false, options = {}) {
  if (!sourceDocumentSupportsEditableTargets()) {
    resetSourcePuzzleAnalysisState();
    return;
  }
  setSourcePlainTextMode(false);
  if (sourceEditor.sourceEditorPort?.kind === "codemirror") {
    window.cancelAnimationFrame(sourceOptimisticHighlightFrame);
    sourceOptimisticHighlightFrame = 0;
    sourceOptimisticHighlightSource = null;
    window.clearTimeout(sourceHighlightTimer);
    sourceHighlightTimer = window.setTimeout(() => {
      sourceHighlightTimer = 0;
      refreshSourceHighlight();
    }, immediate ? 0 : 140);
    return;
  }
  const preserveCurrent = options.preserveCurrent !== false;
  if (immediate) {
    window.cancelAnimationFrame(sourceOptimisticHighlightFrame);
    sourceOptimisticHighlightFrame = 0;
    sourceOptimisticHighlightSource = null;
    if (preserveCurrent && sourceHighlightMode) {
      if (!renderOptimisticSourceHighlight()) {
        syncSourceHighlightScroll();
      }
    } else {
      renderPlainSourceHighlight();
    }
  } else if (preserveCurrent && sourceHighlightMode) {
    scheduleOptimisticSourceHighlight();
  } else {
    schedulePlainSourceHighlight();
  }
  window.clearTimeout(sourceHighlightTimer);
  sourceHighlightTimer = window.setTimeout(() => {
    sourceHighlightTimer = 0;
    refreshSourceHighlight();
  }, immediate ? 0 : 140);
}

function scheduleOptimisticSourceHighlight(source = null) {
  sourceOptimisticHighlightSource = typeof source === "string" ? source : null;
  if (sourceOptimisticHighlightFrame) {
    return;
  }
  sourceOptimisticHighlightFrame = window.requestAnimationFrame(() => {
    sourceOptimisticHighlightFrame = 0;
    const expectedSource = sourceOptimisticHighlightSource;
    sourceOptimisticHighlightSource = null;
    const currentSource = sourceEditor.value || "";
    if (expectedSource !== null && expectedSource !== currentSource) {
      return;
    }
    if (!renderOptimisticSourceHighlight(expectedSource ?? currentSource)) {
      syncSourceHighlightScroll();
    }
  });
}

function schedulePlainSourceHighlight() {
  sourceOptimisticHighlightSource = null;
  if (sourceOptimisticHighlightFrame) {
    return;
  }
  sourceOptimisticHighlightFrame = window.requestAnimationFrame(() => {
    sourceOptimisticHighlightFrame = 0;
    renderPlainSourceHighlight();
  });
}

function resetSourcePuzzleAnalysisState() {
  const plainModeChanged = setSourcePlainTextMode(true);
  window.clearTimeout(sourceHighlightTimer);
  sourceHighlightTimer = 0;
  window.cancelAnimationFrame(sourceOptimisticHighlightFrame);
  sourceOptimisticHighlightFrame = 0;
  sourceOptimisticHighlightSource = null;
  window.clearTimeout(sourceOutlineTimer);
  sourceOutlineTimer = 0;
  window.clearTimeout(sourceCompletionTimer);
  sourceCompletionTimer = 0;
  if (activeHighlightRequest) {
    activeHighlightRequest.abort();
    activeHighlightRequest = null;
  }
  sourceHighlightRequestId += 1;
  sourceOutlineRequestId += 1;
  sourceCompletionRequestId += 1;
  sourceOutlineItems = [];
  sourceOutlineDirty = true;
  sourceOutlineSignature = "";
  sourceCursorPreviewKey = "";
  sourceCursorResolveSignature = null;
  sourceCursorResolveRegion = null;
  hideSourceCompletions();
  if (sourceEditor.sourceEditorPort?.kind === "codemirror") {
    sourceEditor.sourceEditorPort.clearHighlights();
    sourceHighlightSource = "";
    sourceHighlightHtml = "";
    sourceHighlightRuns = [];
    sourceHighlightMode = "plain-text";
  }
  if (plainModeChanged && sourceHighlight) {
    sourceHighlight.innerHTML = "";
    sourceHighlightSource = "";
    sourceHighlightHtml = "";
    sourceHighlightRuns = [];
    sourceHighlightMode = "plain-text";
    syncSourceHighlightScroll();
  }
  if (plainModeChanged) {
    renderSourceOutlineEmpty("No outline");
  }
}

function setSourcePlainTextMode(enabled) {
  const next = Boolean(enabled);
  if (sourcePlainTextModeActive === next) {
    return false;
  }
  sourcePlainTextModeActive = next;
  sourceEditorWrap?.classList.toggle("is-plain-source", next);
  return true;
}

function renderOptimisticSourceHighlight(source = sourceEditor.value) {
  if (!sourceHighlight || !sourceHighlightMode || sourceHighlightSource === source) {
    return false;
  }
  const previous = sourceHighlightSource || "";
  let runs = sourceHighlightRuns;
  if (!runs.length) {
    if ((sourceHighlight.textContent || "") !== previous) {
      return false;
    }
    runs = sourceHighlightRunsFromDom();
    sourceHighlightRuns = runs;
  }
  if (!runs.length) {
    return false;
  }
  let prefix = 0;
  const maxPrefix = Math.min(previous.length, source.length);
  while (prefix < maxPrefix && previous[prefix] === source[prefix]) {
    prefix += 1;
  }
  let suffix = 0;
  const maxSuffix = Math.min(previous.length - prefix, source.length - prefix);
  while (
    suffix < maxSuffix
    && previous[previous.length - suffix - 1] === source[source.length - suffix - 1]
  ) {
    suffix += 1;
  }

  const inserted = source.slice(prefix, source.length - suffix);
  const nextRuns = [
    ...sourceHighlightRunsSlice(runs, 0, prefix),
  ];
  if (inserted) {
    const style = sourceHighlightStyleAtOffset(runs, prefix);
    nextRuns.push({
      text: inserted,
      className: style.className,
      style: style.style,
    });
  }
  nextRuns.push(...sourceHighlightRunsSlice(runs, previous.length - suffix, previous.length));
  setSourceHighlightHtml(source, sourceHighlightRunsToHtml(nextRuns), "optimistic", nextRuns, {
    deferLayout: true,
  });
  return true;
}

function sourceHighlightRunsFromDom() {
  const runs = [];
  if (!sourceHighlight) {
    return runs;
  }
  return sourceHighlightRunsFromRoot(sourceHighlight);
}

function sourceHighlightRunsFromRoot(root) {
  const runs = [];
  if (!root) {
    return runs;
  }
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    const text = node.nodeValue || "";
    if (!text) {
      continue;
    }
    const element = node.parentElement && node.parentElement !== sourceHighlight
      ? node.parentElement
      : null;
    runs.push({
      text,
      className: element?.getAttribute("class") || "",
      style: element?.getAttribute("style") || "",
    });
  }
  return sourceHighlightMergeRuns(runs);
}

function sourceFoldHighlightRuns(runs, source) {
  if (!sourceFoldsActive()) {
    return sourceHighlightMergeRuns(runs);
  }
  const foldedRuns = [];
  let sourceCursor = 0;
  for (const range of sourceFoldRangesForSource(source)) {
    foldedRuns.push(...sourceHighlightRunsSlice(runs, sourceCursor, range.sourceStart));
    foldedRuns.push(...sourceFoldMarkerHighlightRuns(runs, range));
    sourceCursor = range.sourceEnd;
  }
  foldedRuns.push(...sourceHighlightRunsSlice(runs, sourceCursor, String(source || "").length));
  return sourceHighlightMergeRuns(foldedRuns);
}

function sourceFoldMarkerHighlightRuns(runs, range) {
  const markerRuns = [{
    text: sourceFoldMarkerPrefix(),
    className: "source-fold-marker",
    style: "",
  }];
  const braceRuns = sourceHighlightRunsSlice(runs, range.closeOffset, range.closeOffset + 1);
  if (braceRuns.map((run) => run.text).join("") === sourceFoldMarkerBrace()) {
    markerRuns.push(...braceRuns);
  } else {
    markerRuns.push({
      text: sourceFoldMarkerBrace(),
      className: "",
      style: "",
    });
  }
  if (range.hasTrailingNewline) {
    markerRuns.push({
      text: "\n",
      className: "",
      style: "",
    });
  }
  return markerRuns;
}

function sourceHighlightRunsSlice(runs, start, end) {
  const result = [];
  let offset = 0;
  for (const run of runs) {
    const runStart = offset;
    const runEnd = runStart + run.text.length;
    offset = runEnd;
    if (runEnd <= start || runStart >= end) {
      continue;
    }
    const sliceStart = Math.max(start, runStart) - runStart;
    const sliceEnd = Math.min(end, runEnd) - runStart;
    result.push({
      text: run.text.slice(sliceStart, sliceEnd),
      className: run.className,
      style: run.style,
    });
  }
  return sourceHighlightMergeRuns(result);
}

function sourceHighlightStyleAtOffset(runs, offset) {
  const before = Math.max(0, offset - 1);
  let position = 0;
  let firstAfter = null;
  for (const run of runs) {
    const next = position + run.text.length;
    if (!firstAfter && next >= offset) {
      firstAfter = run;
    }
    if (before >= position && before < next) {
      return { className: run.className, style: run.style };
    }
    position = next;
  }
  const run = firstAfter || runs.at(-1) || {};
  return { className: run.className || "", style: run.style || "" };
}

function sourceHighlightRunsToHtml(runs) {
  return sourceHighlightMergeRuns(runs).map((run) => {
    const text = escapeHtml(run.text);
    const className = run.className ? ` class="${escapeHtml(run.className)}"` : "";
    const style = run.style ? ` style="${escapeHtml(run.style)}"` : "";
    return className || style ? `<span${className}${style}>${text}</span>` : text;
  }).join("") || " ";
}

function sourceHighlightMergeRuns(runs) {
  const merged = [];
  for (const run of runs) {
    if (!run?.text) {
      continue;
    }
    const previous = merged.at(-1);
    if (previous && previous.className === run.className && previous.style === run.style) {
      previous.text += run.text;
    } else {
      merged.push({
        text: run.text,
        className: run.className || "",
        style: run.style || "",
      });
    }
  }
  return merged;
}

function sourcePredictedBeforeInputValue(event) {
  if (
    !event
    || sourceEditorBlockSelection?.ranges?.length
  ) {
    return null;
  }
  if (
    !["insertText", "insertCompositionText"].includes(event.inputType)
    || typeof event.data !== "string"
  ) {
    return null;
  }
  const source = sourceEditor.value || "";
  const start = Math.max(0, Math.min(source.length, sourceEditor.selectionStart || 0));
  const end = Math.max(start, Math.min(source.length, sourceEditor.selectionEnd || start));
  return `${source.slice(0, start)}${event.data}${source.slice(end)}`;
}

function renderPredictedSourceHighlight(source) {
  if (!renderOptimisticSourceHighlight(source)) {
    setSourceHighlightHtml(
      source,
      escapeHtml(source || " "),
      "optimistic",
      [{ text: source || " ", className: "", style: "" }],
      { deferLayout: true },
    );
  }
}

function beginSourceCompositionPreview(source) {
  sourceCompositionPreviewSource = source;
  if (activeHighlightRequest) {
    activeHighlightRequest.abort();
  }
  sourceHighlightRequestId += 1;
  renderPredictedSourceHighlight(source);
}

function sourceCompositionPreviewValue(data) {
  const text = String(data ?? "");
  const source = sourceEditor.value || "";
  const range = sourceCompositionRange || {
    start: sourceEditor.selectionStart || 0,
    end: sourceEditor.selectionEnd || sourceEditor.selectionStart || 0,
  };
  const start = Math.max(0, Math.min(source.length, range.start || 0));
  const end = Math.max(start, Math.min(source.length, range.end || start));
  return `${source.slice(0, start)}${text}${source.slice(end)}`;
}

function clearSourceCompositionPreview() {
  sourceCompositionPreviewSource = "";
  sourceCompositionRange = null;
}

function scheduleLevelBuilderResetFromSource(resetCells = false) {
  // The level builder is rebuilt from the last *compiled* preview export, which
  // typing alone never changes, so a reset here only matters while the level
  // pane is actually on screen. When it is hidden the board is re-rendered the
  // moment the pane is shown (mode switch / compile), so skipping the per-frame
  // render avoids a full board + solver re-render on every keystroke.
  if (!(isPaneVisible("level") && levelBuilder && !levelBuilder.hidden)) {
    return;
  }
  // The board is rebuilt from the compiled preview export, which typing never
  // changes (a recompile does). When no export exists yet the palette falls back
  // to the live source, so include that in the signature. Skipping unchanged
  // resets avoids a full board + solver re-render on every keystroke while the
  // level pane is open. Cell-resetting requests always run.
  if (!resetCells && !sourceLevelBuilderResetCells) {
    const exportData = currentPreviewExportData();
    const signature = exportData || `live:${sourceEditorDocumentValue()}`;
    if (signature === sourceLevelBuilderResetSignature) {
      return;
    }
    sourceLevelBuilderResetSignature = signature;
  } else {
    sourceLevelBuilderResetSignature = null;
  }
  sourceLevelBuilderResetCells = sourceLevelBuilderResetCells || Boolean(resetCells);
  if (sourceLevelBuilderResetFrame) {
    return;
  }
  sourceLevelBuilderResetFrame = window.requestAnimationFrame(() => {
    const shouldResetCells = sourceLevelBuilderResetCells;
    sourceLevelBuilderResetFrame = 0;
    sourceLevelBuilderResetCells = false;
    resetLevelBuilderFromSource(shouldResetCells);
  });
}

function renderPlainSourceHighlight(source = sourceEditor.value, reason = null) {
  if (sourceEditor.sourceEditorPort?.kind === "codemirror") {
    sourceEditor.sourceEditorPort.clearHighlights();
    sourceHighlightSource = String(source || "");
    sourceHighlightHtml = "";
    sourceHighlightRuns = [];
    sourceHighlightMode = "plain-text";
    if (reason) {
      const message = `Source highlighting unavailable: ${userFacingRuntimeError(reason)}`;
      if (!sourceHighlightUnavailableStatusShown && typeof setEditorStatus === "function") {
        sourceHighlightUnavailableStatusShown = true;
        setEditorStatus(message, "is-error");
      }
      console.warn(message);
    }
    return;
  }
  if (!sourceHighlight) {
    return;
  }
  setSourceHighlightHtml(source, escapeHtml(source || " "), "plain");
  if (reason) {
    const message = `Source highlighting unavailable: ${userFacingRuntimeError(reason)}`;
    sourceHighlight.dataset.highlightError = message;
    sourceHighlight.title = message;
    if (!sourceHighlightUnavailableStatusShown && typeof setEditorStatus === "function") {
      sourceHighlightUnavailableStatusShown = true;
      setEditorStatus(message, "is-error");
    }
    console.warn(message);
  }
}

function setSourceHighlightHtml(source, html, mode, runs = null, options = {}) {
  if (!sourceHighlight) {
    return;
  }
  if (sourceHighlightSource !== source || sourceHighlightHtml !== html) {
    sourceHighlight.innerHTML = html;
    sourceHighlightSource = source;
    sourceHighlightHtml = html;
    sourceHighlightRuns = Array.isArray(runs)
      ? sourceHighlightMergeRuns(runs)
      : sourceHighlightRunsFromDom();
  } else if (Array.isArray(runs)) {
    sourceHighlightRuns = sourceHighlightMergeRuns(runs);
  }
  delete sourceHighlight.dataset.highlightError;
  sourceHighlight.removeAttribute("title");
  sourceHighlightUnavailableStatusShown = false;
  sourceHighlightMode = mode;
  if (options.deferLayout) {
    syncSourceHighlightTransform();
    scheduleSourceEditorLayoutSync();
  } else {
    syncSourceHighlightScroll();
  }
  renderSourceBlockSelection();
}

function syncSourceHighlightMetrics() {
  if (!sourceHighlight || !sourceEditor) {
    return;
  }
  sourceEditor.style.height = "auto";
  const clientWidth = sourceEditorWrap.clientWidth;
  const scrollHeight = Math.max(sourceEditorWrap.clientHeight, sourceEditor.scrollHeight);
  sourceEditor.style.height = `${scrollHeight}px`;
  if (sourceHighlightClientWidth !== clientWidth) {
    sourceHighlightClientWidth = clientWidth;
    sourceHighlight.style.width = `${clientWidth}px`;
  }
  if (sourceHighlightScrollHeight !== scrollHeight) {
    sourceHighlightScrollHeight = scrollHeight;
    sourceHighlight.style.height = `${scrollHeight}px`;
  }
  syncSourceOverlayLayerMetrics(clientWidth, scrollHeight);
}

function syncSourceOverlayLayerMetrics(clientWidth, scrollHeight) {
  for (const layer of [sourceBlockSelectionLayer, sourceFindMatchLayer]) {
    if (!layer) {
      continue;
    }
    layer.style.width = `${clientWidth}px`;
    layer.style.height = `${scrollHeight}px`;
  }
}

function syncSourceHighlightScroll() {
  if (!sourceHighlight) {
    return;
  }
  syncSourceHighlightMetrics();
  syncSourceHighlightTransform();
}

function syncSourceHighlightTransform() {
  if (!sourceHighlight) {
    return;
  }
  sourceHighlight.style.transform = "";
  if (sourceBlockSelectionLayer) {
    sourceBlockSelectionLayer.style.transform = "";
  }
  if (sourceFindMatchLayer) {
    sourceFindMatchLayer.style.transform = "";
  }
}

function sourceScrollTop() {
  if (sourceEditor.sourceEditorPort?.kind === "codemirror") {
    return sourceEditor.sourceEditorPort.scrollTop();
  }
  return sourceEditorWrap.scrollTop || 0;
}

function sourceScrollLeft() {
  if (sourceEditor.sourceEditorPort?.kind === "codemirror") {
    return sourceEditor.sourceEditorPort.scrollLeft();
  }
  return sourceEditorWrap.scrollLeft || 0;
}

function setSourceScrollTop(value) {
  if (sourceEditor.sourceEditorPort?.kind === "codemirror") {
    sourceEditor.sourceEditorPort.scrollTop(value);
    return;
  }
  sourceEditorWrap.scrollTop = Math.max(0, value || 0);
}

function setSourceScrollLeft(value) {
  if (sourceEditor.sourceEditorPort?.kind === "codemirror") {
    sourceEditor.sourceEditorPort.scrollLeft(value);
    return;
  }
  sourceEditorWrap.scrollLeft = Math.max(0, value || 0);
}

function sourceViewportHeight() {
  if (sourceEditor.sourceEditorPort?.kind === "codemirror") {
    return sourceEditor.sourceEditorPort.viewportSize().height;
  }
  return sourceEditorWrap.clientHeight;
}

function sourceViewportWidth() {
  if (sourceEditor.sourceEditorPort?.kind === "codemirror") {
    return sourceEditor.sourceEditorPort.viewportSize().width;
  }
  return sourceEditorWrap.clientWidth;
}

function scheduleSourceEditorLayoutSync(frameCount = 1) {
  if (sourceLayoutSyncFrame) {
    return;
  }
  let remainingFrames = Math.max(1, frameCount || 1);
  const sync = () => {
    sourceLayoutSyncFrame = 0;
    syncSourceHighlightScroll();
    if (remainingFrames > 1) {
      remainingFrames -= 1;
      sourceLayoutSyncFrame = window.requestAnimationFrame(sync);
    }
  };
  sourceLayoutSyncFrame = window.requestAnimationFrame(sync);
}

function sourceDocumentSupportsEditableTargets() {
  return typeof activeDocument === "function"
    && typeof isPuzzleDocument === "function"
    && typeof isTextDocument === "function"
    && isPuzzleDocument(activeDocument())
    && isTextDocument(activeDocument());
}

function sourceOutlineVisible() {
  if (!sourceOutlineList) {
    return false;
  }
  const section = sourceOutlineList.closest("[data-explorer-section='outline']");
  if (section?.classList.contains("is-collapsed")) {
    return false;
  }
  const sections = sourceOutlineList.closest(".explorer-sections");
  return !sections?.classList.contains("is-outline-collapsed");
}

function stripSourceStructureLineComment(line) {
  let quote = "";
  let escaped = false;
  for (let index = 0; index < line.length; index += 1) {
    const ch = line[index];
    if (quote) {
      if (escaped) {
        escaped = false;
      } else if (ch === "\\") {
        escaped = true;
      } else if (ch === quote) {
        quote = "";
      }
      continue;
    }
    if (ch === "\"" || ch === "'") {
      quote = ch;
      continue;
    }
    if (ch === "/" && line[index + 1] === "/") {
      return line.slice(0, index);
    }
  }
  return line;
}

function sourceLineHasStructuralBrace(line) {
  let quote = "";
  let escaped = false;
  for (let index = 0; index < line.length; index += 1) {
    const ch = line[index];
    if (quote) {
      if (escaped) {
        escaped = false;
      } else if (ch === "\\") {
        escaped = true;
      } else if (ch === quote) {
        quote = "";
      }
      continue;
    }
    if (ch === "\"" || ch === "'") {
      quote = ch;
      continue;
    }
    if (ch === "/" && line[index + 1] === "/") {
      return false;
    }
    if (ch === "{" || ch === "}") {
      return true;
    }
  }
  return false;
}

function sourceOutlineStructuralSignature(source) {
  // Labels and offsets are both derived from one exact source snapshot. A
  // non-structural line inserted before an item changes its location, so a
  // brace-only signature leaves outline navigation stale.
  return String(source || "");
}

function markSourceOutlineDirtyForSource(source, options = {}) {
  const signature = sourceOutlineStructuralSignature(source);
  if (options.force === true || signature !== sourceOutlineSignature) {
    sourceOutlineSignature = signature;
    sourceOutlineDirty = true;
    return true;
  }
  return false;
}

function sourceOutlineShouldRefreshForSource(source, options = {}) {
  markSourceOutlineDirtyForSource(source, options);
  return sourceOutlineDirty && sourceOutlineVisible();
}

async function refreshSourceHighlight() {
  const document = activeDocument();
  const codeMirror = sourceEditor.sourceEditorPort?.kind === "codemirror";
  if (!isPuzzleDocument(document) || !isTextDocument(document)) {
    return;
  }
  if (!codeMirror) {
    throw new Error("Source highlighting requires the CodeMirror source editor port.");
  }
  if (sourceCompositionPreviewSource) {
    return;
  }

  if (activeHighlightRequest) {
    activeHighlightRequest.abort();
  }
  const source = sourceEditorDocumentValue();
  const displaySource = sourceEditor.value || "";
  const range = sourceEditor.sourceEditorPort.highlightViewportRange();
  const includeOutline = sourceOutlineShouldRefreshForSource(source);
  const requestId = ++sourceHighlightRequestId;
  const controller = new AbortController();
  activeHighlightRequest = controller;

  try {
    const text = await window.PuzzleStudioHost.highlight(
      {
        source,
        rangeStart: range.from,
        rangeEnd: range.to,
        includeOutline,
      },
      { signal: controller.signal },
    );
    if (
      requestId !== sourceHighlightRequestId
      || source !== sourceEditorDocumentValue()
      || displaySource !== (sourceEditor.value || "")
    ) {
      return;
    }
    const payload = JSON.parse(text);
    if (payload.outline) {
      applySourceOutlinePayload(payload.outline, source);
    }
    sourceEditor.sourceEditorPort.applyHighlightRange(source, range, payload);
    sourceHighlightSource = source;
    sourceHighlightHtml = "";
    sourceHighlightRuns = [];
    sourceHighlightMode = "server";
    sourceHighlightUnavailableStatusShown = false;
    renderSourceBlockSelection();
  } catch (error) {
    if (error.name === "AbortError") {
      return;
    }
    if (
      requestId !== sourceHighlightRequestId
      || source !== sourceEditorDocumentValue()
      || displaySource !== (sourceEditor.value || "")
    ) {
      return;
    }
    renderPlainSourceHighlight(displaySource, error);
  } finally {
    if (activeHighlightRequest === controller) {
      activeHighlightRequest = null;
    }
  }
}

function scheduleSourceOutlineRefresh(immediate = false, options = {}) {
  if (!sourceDocumentSupportsEditableTargets()) {
    resetSourcePuzzleAnalysisState();
    return;
  }
  const source = sourceEditorDocumentValue();
  if (!sourceOutlineShouldRefreshForSource(source, options)) {
    return;
  }
  if (sourceHighlightTimer) {
    return;
  }
  window.clearTimeout(sourceOutlineTimer);
  sourceOutlineTimer = window.setTimeout(() => {
    sourceOutlineTimer = 0;
    refreshSourceOutline();
  }, immediate ? 0 : 160);
}

async function refreshSourceOutline() {
  const document = activeDocument();
  const requestId = ++sourceOutlineRequestId;
  if (!sourceOutlineList) {
    return;
  }
  if (!document || !isPuzzleDocument(document) || !isTextDocument(document)) {
    sourceOutlineItems = [];
    renderSourceOutlineEmpty("No outline");
    return;
  }
  const source = sourceEditorDocumentValue();
  if (!sourceOutlineShouldRefreshForSource(source)) {
    return;
  }
  try {
    const text = await window.PuzzleStudioHost.sourceOutline({ source });
    if (requestId !== sourceOutlineRequestId || source !== sourceEditorDocumentValue()) {
      return;
    }
    const payload = JSON.parse(text || "{}");
    applySourceOutlinePayload(payload, source);
  } catch (error) {
    if (error.name === "AbortError") {
      return;
    }
    if (requestId !== sourceOutlineRequestId) {
      return;
    }
    sourceOutlineItems = [];
    renderSourceOutlineEmpty(`Outline unavailable: ${userFacingRuntimeError(error)}`);
  }
}

function applySourceOutlinePayload(payload, source) {
  const nextItems = normalizeSourceOutlineItems(payload?.items, source);
  const structureChanged = sourceOutlineStructureSignature(sourceOutlineItems)
    !== sourceOutlineStructureSignature(nextItems);
  sourceOutlineItems = nextItems;
  pruneSourceOutlineExpandedItems();
  sourceOutlineDirty = false;
  sourceOutlineSignature = sourceOutlineStructuralSignature(source);
  if (structureChanged) {
    renderSourceOutline();
  } else {
    syncSourceOutlineRowOffsets();
  }
  syncSourceOutlineActiveItem();
}

function sourceOutlineStructureSignature(items) {
  return items.map((item) => [
    item.id,
    item.kind,
    item.label,
    item.depth,
    item.parent,
  ].join("\u0000")).join("\u0001");
}

function syncSourceOutlineRowOffsets() {
  if (!sourceOutlineList) {
    return;
  }
  const itemsById = sourceOutlineItemById();
  for (const row of sourceOutlineList.querySelectorAll("[data-source-outline-id]")) {
    const item = itemsById.get(row.dataset.sourceOutlineId || "");
    if (item && row.dataset.sourceOutlineStart !== String(item.start)) {
      row.dataset.sourceOutlineStart = String(item.start);
    }
  }
}

function normalizeSourceOutlineItems(items, source) {
  const utf16ByUtf8 = sourceUtf16OffsetsByUtf8Byte(source);
  return (Array.isArray(items) ? items : []).map((item) => {
    const byteStart = Number(item?.start);
    const byteEnd = Number(item?.end);
    const start = utf16ByUtf8.get(byteStart);
    const end = utf16ByUtf8.get(byteEnd);
    if (
      !Number.isInteger(byteStart)
      || !Number.isInteger(byteEnd)
      || byteStart < 0
      || byteStart > byteEnd
      || start === undefined
      || end === undefined
    ) {
      throw new Error("Rust source outline contains an invalid UTF-8 source range.");
    }
    return {
      id: String(item?.id || ""),
      kind: String(item?.kind || "item"),
      label: String(item?.label || item?.kind || "item"),
      start,
      end,
      depth: Math.max(0, Math.min(8, Number(item?.depth) || 0)),
      parent: item?.parent == null ? "" : String(item.parent),
    };
  }).filter((item) => item.id && Number.isFinite(item.start));
}

function sourceUtf16OffsetsByUtf8Byte(source) {
  const offsets = new Map([[0, 0]]);
  let byteOffset = 0;
  for (let utf16Offset = 0; utf16Offset < source.length;) {
    const codePoint = source.codePointAt(utf16Offset);
    const utf16Length = codePoint > 0xffff ? 2 : 1;
    const utf8Length = codePoint <= 0x7f
      ? 1
      : codePoint <= 0x7ff
        ? 2
        : codePoint <= 0xffff
          ? 3
          : 4;
    byteOffset += utf8Length;
    utf16Offset += utf16Length;
    offsets.set(byteOffset, utf16Offset);
  }
  return offsets;
}

function sourceOutlineItemById() {
  return new Map(sourceOutlineItems.map((item) => [item.id, item]));
}

function sourceOutlineParentIdsWithChildren() {
  const parentIds = new Set();
  for (const item of sourceOutlineItems) {
    if (item.parent) {
      parentIds.add(item.parent);
    }
  }
  return parentIds;
}

function pruneSourceOutlineExpandedItems() {
  const ids = new Set(sourceOutlineItems.map((item) => item.id));
  sourceOutlineExpandedItemIds = new Set(
    [...sourceOutlineExpandedItemIds].filter((id) => ids.has(id)),
  );
}

function sourceOutlineItemHiddenByCollapsedParent(item, itemsById = sourceOutlineItemById()) {
  let parentId = item?.parent || "";
  while (parentId) {
    if (!sourceOutlineExpandedItemIds.has(parentId)) {
      return true;
    }
    parentId = itemsById.get(parentId)?.parent || "";
  }
  return false;
}

function visibleSourceOutlineItems() {
  const itemsById = sourceOutlineItemById();
  return sourceOutlineItems.filter((item) => !sourceOutlineItemHiddenByCollapsedParent(item, itemsById));
}

function renderSourceOutline() {
  if (!sourceOutlineList) {
    return;
  }
  if (!sourceOutlineItems.length) {
    renderSourceOutlineEmpty("No outline");
    return;
  }
  const parentIdsWithChildren = sourceOutlineParentIdsWithChildren();
  sourceOutlineList.replaceChildren(...visibleSourceOutlineItems().map((item) => {
    const hasChildren = parentIdsWithChildren.has(item.id);
    const expanded = sourceOutlineExpandedItemIds.has(item.id);
    const button = document.createElement("button");
    button.type = "button";
    button.className = "source-outline-row";
    button.dataset.sourceOutlineId = item.id;
    button.dataset.sourceOutlineStart = String(item.start);
    button.style.setProperty("--depth", String(item.depth));
    button.setAttribute("role", "treeitem");
    button.setAttribute("aria-level", String(item.depth + 1));
    button.setAttribute("aria-label", item.label);
    if (hasChildren) {
      button.setAttribute("aria-expanded", String(expanded));
    }
    const chevron = document.createElement("span");
    chevron.className = hasChildren
      ? "source-outline-chevron"
      : "source-outline-chevron source-outline-chevron-spacer";
    if (hasChildren) {
      chevron.dataset.sourceOutlineToggle = item.id;
      chevron.innerHTML = sourceOutlineChevronSvg(expanded);
    }
    const kind = document.createElement("span");
    kind.className = "source-outline-kind";
    kind.innerHTML = sourceOutlineKindIconSvg(item.kind);
    const label = document.createElement("span");
    label.className = "source-outline-label";
    label.textContent = item.label;
    button.append(chevron, kind, label);
    return button;
  }));
}

function renderSourceOutlineEmpty(message) {
  if (!sourceOutlineList) {
    return;
  }
  const empty = document.createElement("div");
  empty.className = "source-outline-empty";
  empty.textContent = message;
  sourceOutlineList.replaceChildren(empty);
}

function sourceOutlineChevronSvg(expanded) {
  const path = expanded ? "M4 6l4 4 4-4" : "M6 4l4 4-4 4";
  return `<svg class="source-outline-chevron-icon" viewBox="0 0 16 16" aria-hidden="true"><path d="${path}"></path></svg>`;
}

const SOURCE_OUTLINE_KIND_ICON_NAMES = Object.freeze({
  "puzzle": "puzzle",
  "puzzle3": "puzzle",
  "levels": "map",
  "levels3": "map",
  "level": "map",
  "sprites": "image",
  "sprites3": "image",
  "sprite": "image",
  "objects": "boxes",
  "object": "box",
  "groups": "group",
  "tags": "tag",
  "marks": "bookmark",
  "render": "scan-eye",
  "camera": "camera",
  "animation": "circle-play",
  "tween": "chart-spline",
  "row": "rows-3",
  "column": "columns-3",
  "choice": "mouse-pointer-click",
  "button": "square-mouse-pointer",
  "text": "message-square",
  "message": "message-square",
  "title": "file-text",
  "subtitle": "file-text",
  "author": "file-text",
  "homepage": "file-text",
  "import": "import",
  "rules": "list-checks",
  "rule": "list-checks",
  "routine": "workflow",
  "win_conditions": "flag",
  "lose_conditions": "flag-off",
  "scene": "clapperboard",
  "screen": "panels-top-left",
  "layout": "panels-top-left",
  "level_menu": "panels-top-left",
  "assets": "package",
  "resources": "package",
  "legend": "move-horizontal",
  "map": "arrow-right",
  "theme": "swatch-book",
  "colors": "palette",
  "shapes": "shapes",
  "sounds": "volume-2",
  "keys": "keyboard",
  "layers": "layers",
  "collision_layers": "layers",
  "metadata": "info",
  "fix": "wrench",
});

const SOURCE_OUTLINE_LIFECYCLE_ICON_NAME = "zap";
const SOURCE_OUTLINE_DEFAULT_ICON_NAME = "file-code-2";

function sourceOutlineKindIconName(kind) {
  const text = String(kind || "").trim();
  if (Object.prototype.hasOwnProperty.call(SOURCE_OUTLINE_KIND_ICON_NAMES, text)) {
    return SOURCE_OUTLINE_KIND_ICON_NAMES[text];
  }
  if (text.startsWith("on_")) {
    return SOURCE_OUTLINE_LIFECYCLE_ICON_NAME;
  }
  return SOURCE_OUTLINE_DEFAULT_ICON_NAME;
}

function sourceOutlineKindIconSvg(kind) {
  const name = sourceOutlineKindIconName(kind);
  const icons = {
    puzzle: `
      <path d="M15.39 4.39a1 1 0 0 0 1.68-.474 2.5 2.5 0 1 1 3.014 3.015 1 1 0 0 0-.474 1.68l1.683 1.682a2.414 2.414 0 0 1 0 3.414L19.61 15.39a1 1 0 0 1-1.68-.474 2.5 2.5 0 1 0-3.014 3.015 1 1 0 0 1 .474 1.68l-1.683 1.682a2.414 2.414 0 0 1-3.414 0L8.61 19.61a1 1 0 0 0-1.68.474 2.5 2.5 0 1 1-3.014-3.015 1 1 0 0 0 .474-1.68l-1.683-1.682a2.414 2.414 0 0 1 0-3.414L4.39 8.61a1 1 0 0 1 1.68.474 2.5 2.5 0 1 0 3.014-3.015 1 1 0 0 1-.474-1.68l1.683-1.682a2.414 2.414 0 0 1 3.414 0z"></path>
    `,
    map: `
      <path d="M14.106 5.553a2 2 0 0 0 1.788 0l3.659-1.83A1 1 0 0 1 21 4.619v12.764a1 1 0 0 1-.553.894l-4.553 2.277a2 2 0 0 1-1.788 0l-4.212-2.106a2 2 0 0 0-1.788 0l-3.659 1.83A1 1 0 0 1 3 19.381V6.618a1 1 0 0 1 .553-.894l4.553-2.277a2 2 0 0 1 1.788 0z"></path>
      <path d="M15 5.764v15"></path>
      <path d="M9 3.236v15"></path>
    `,
    box: `
      <path d="M21 8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16Z"></path>
      <path d="m3.3 7 8.7 5 8.7-5"></path>
      <path d="M12 22V12"></path>
    `,
    boxes: `
      <path d="M2.97 12.92A2 2 0 0 0 2 14.63v3.24a2 2 0 0 0 .97 1.71l3 1.8a2 2 0 0 0 2.06 0L12 19v-5.5l-5-3-4.03 2.42Z"></path>
      <path d="m7 16.5-4.74-2.85"></path>
      <path d="m7 16.5 5-3"></path>
      <path d="M7 16.5v5.17"></path>
      <path d="M12 13.5V19l3.97 2.38a2 2 0 0 0 2.06 0l3-1.8a2 2 0 0 0 .97-1.71v-3.24a2 2 0 0 0-.97-1.71L17 10.5l-5 3Z"></path>
      <path d="m17 16.5-5-3"></path>
      <path d="m17 16.5 4.74-2.85"></path>
      <path d="M17 16.5v5.17"></path>
      <path d="M7.97 4.42A2 2 0 0 0 7 6.13v4.37l5 3 5-3V6.13a2 2 0 0 0-.97-1.71l-3-1.8a2 2 0 0 0-2.06 0l-3 1.8Z"></path>
      <path d="M12 8 7.26 5.15"></path>
      <path d="m12 8 4.74-2.85"></path>
      <path d="M12 13.5V8"></path>
    `,
    image: `
      <rect width="18" height="18" x="3" y="3" rx="2" ry="2"></rect>
      <circle cx="9" cy="9" r="2"></circle>
      <path d="m21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21"></path>
    `,
    group: `
      <path d="M3 7V5c0-1.1.9-2 2-2h2"></path>
      <path d="M17 3h2c1.1 0 2 .9 2 2v2"></path>
      <path d="M21 17v2c0 1.1-.9 2-2 2h-2"></path>
      <path d="M7 21H5c-1.1 0-2-.9-2-2v-2"></path>
      <rect width="7" height="5" x="7" y="7" rx="1"></rect>
      <rect width="7" height="5" x="10" y="12" rx="1"></rect>
    `,
    tag: `
      <path d="M12.586 2.586A2 2 0 0 0 11.172 2H4a2 2 0 0 0-2 2v7.172a2 2 0 0 0 .586 1.414l8.704 8.704a2.426 2.426 0 0 0 3.42 0l6.58-6.58a2.426 2.426 0 0 0 0-3.42z"></path>
      <circle cx="7.5" cy="7.5" r=".5" fill="currentColor"></circle>
    `,
    bookmark: `
      <path d="M17 3a2 2 0 0 1 2 2v15a1 1 0 0 1-1.496.868l-4.512-2.578a2 2 0 0 0-1.984 0l-4.512 2.578A1 1 0 0 1 5 20V5a2 2 0 0 1 2-2z"></path>
    `,
    "scan-eye": `
      <path d="M3 7V5a2 2 0 0 1 2-2h2"></path>
      <path d="M17 3h2a2 2 0 0 1 2 2v2"></path>
      <path d="M21 17v2a2 2 0 0 1-2 2h-2"></path>
      <path d="M7 21H5a2 2 0 0 1-2-2v-2"></path>
      <circle cx="12" cy="12" r="1"></circle>
      <path d="M18.944 12.33a1 1 0 0 0 0-.66 7.5 7.5 0 0 0-13.888 0 1 1 0 0 0 0 .66 7.5 7.5 0 0 0 13.888 0"></path>
    `,
    camera: `
      <path d="M13.997 4a2 2 0 0 1 1.76 1.05l.486.9A2 2 0 0 0 18.003 7H20a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V9a2 2 0 0 1 2-2h1.997a2 2 0 0 0 1.759-1.048l.489-.904A2 2 0 0 1 10.004 4z"></path>
      <circle cx="12" cy="13" r="3"></circle>
    `,
    "chart-spline": `
      <path d="M3 3v16a2 2 0 0 0 2 2h16"></path>
      <path d="M7 16c.5-2 1.5-7 4-7 2 0 2 3 4 3 2.5 0 4.5-5 5-7"></path>
    `,
    "circle-play": `
      <path d="M9 9.003a1 1 0 0 1 1.517-.859l4.997 2.997a1 1 0 0 1 0 1.718l-4.997 2.997A1 1 0 0 1 9 14.996z"></path>
      <circle cx="12" cy="12" r="10"></circle>
    `,
    "rows-3": `
      <rect width="18" height="18" x="3" y="3" rx="2"></rect>
      <path d="M21 9H3"></path>
      <path d="M21 15H3"></path>
    `,
    "columns-3": `
      <rect width="18" height="18" x="3" y="3" rx="2"></rect>
      <path d="M9 3v18"></path>
      <path d="M15 3v18"></path>
    `,
    "mouse-pointer-click": `
      <path d="M14 4.1 12 6"></path>
      <path d="m5.1 8-2.9-.8"></path>
      <path d="m6 12-1.9 2"></path>
      <path d="M7.2 2.2 8 5.1"></path>
      <path d="M9.037 9.69a.498.498 0 0 1 .653-.653l11 4.5a.5.5 0 0 1-.074.949l-4.349 1.041a1 1 0 0 0-.74.739l-1.04 4.35a.5.5 0 0 1-.95.074z"></path>
    `,
    "square-mouse-pointer": `
      <path d="M12.034 12.681a.498.498 0 0 1 .647-.647l9 3.5a.5.5 0 0 1-.033.943l-3.444 1.068a1 1 0 0 0-.66.66l-1.067 3.443a.5.5 0 0 1-.943.033z"></path>
      <path d="M21 11V5a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h6"></path>
    `,
    "message-square": `
      <path d="M22 17a2 2 0 0 1-2 2H6.828a2 2 0 0 0-1.414.586l-2.202 2.202A.71.71 0 0 1 2 21.286V5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2z"></path>
    `,
    "file-text": `
      <path d="M6 22a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h8a2.4 2.4 0 0 1 1.704.706l3.588 3.588A2.4 2.4 0 0 1 20 8v12a2 2 0 0 1-2 2z"></path>
      <path d="M14 2v5a1 1 0 0 0 1 1h5"></path>
      <path d="M10 9H8"></path>
      <path d="M16 13H8"></path>
      <path d="M16 17H8"></path>
    `,
    "import": `
      <path d="M12 3v12"></path>
      <path d="m8 11 4 4 4-4"></path>
      <path d="M8 5H4a2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V7a2 2 0 0 0-2-2h-4"></path>
    `,
    "list-checks": `
      <path d="M13 5h8"></path>
      <path d="M13 12h8"></path>
      <path d="M13 19h8"></path>
      <path d="m3 17 2 2 4-4"></path>
      <path d="m3 7 2 2 4-4"></path>
    `,
    workflow: `
      <rect width="8" height="8" x="3" y="3" rx="2"></rect>
      <path d="M7 11v4a2 2 0 0 0 2 2h4"></path>
      <rect width="8" height="8" x="13" y="13" rx="2"></rect>
    `,
    flag: `
      <path d="M4 22V4a1 1 0 0 1 .4-.8A6 6 0 0 1 8 2c3 0 5 2 7.333 2q2 0 3.067-.8A1 1 0 0 1 20 4v10a1 1 0 0 1-.4.8A6 6 0 0 1 16 16c-3 0-5-2-8-2a6 6 0 0 0-4 1.528"></path>
    `,
    "flag-off": `
      <path d="M16 16c-3 0-5-2-8-2a6 6 0 0 0-4 1.528"></path>
      <path d="m2 2 20 20"></path>
      <path d="M4 22V4"></path>
      <path d="M7.656 2H8c3 0 5 2 7.333 2q2 0 3.067-.8A1 1 0 0 1 20 4v10.347"></path>
    `,
    "panels-top-left": `
      <rect width="18" height="18" x="3" y="3" rx="2"></rect>
      <path d="M3 9h18"></path>
      <path d="M9 21V9"></path>
    `,
    clapperboard: `
      <path d="m12.296 3.464 3.02 3.956"></path>
      <path d="M20.2 6 3 11l-.9-2.4c-.3-1.1.3-2.2 1.3-2.5l13.5-4c1.1-.3 2.2.3 2.5 1.3z"></path>
      <path d="M3 11h18v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"></path>
      <path d="m6.18 5.276 3.1 3.899"></path>
    `,
    package: `
      <path d="m7.5 4.27 9 5.15"></path>
      <path d="M21 8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16Z"></path>
      <path d="m3.3 7 8.7 5 8.7-5"></path>
      <path d="M12 22V12"></path>
    `,
    palette: `
      <circle cx="13.5" cy="6.5" r=".5" fill="currentColor"></circle>
      <circle cx="17.5" cy="10.5" r=".5" fill="currentColor"></circle>
      <circle cx="8.5" cy="7.5" r=".5" fill="currentColor"></circle>
      <circle cx="6.5" cy="12.5" r=".5" fill="currentColor"></circle>
      <path d="M12 22C6.477 22 2 17.523 2 12S6.477 2 12 2s10 4.477 10 10c0 1.657-1.343 3-3 3h-1.5a2.5 2.5 0 0 0 0 5H19a3 3 0 0 1-3 3z"></path>
    `,
    "swatch-book": `
      <path d="M11 17a4 4 0 0 1-8 0V5a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2Z"></path>
      <path d="M16.7 13H19a2 2 0 0 1 2 2v4a2 2 0 0 1-2 2H7"></path>
      <path d="M 7 17h.01"></path>
      <path d="m11 8 2.3-2.3a2.4 2.4 0 0 1 3.404.004L18.6 7.6a2.4 2.4 0 0 1 .026 3.434L9.9 19.8"></path>
    `,
    shapes: `
      <path d="M8.3 10a.7.7 0 0 1-.626-1.079L11.4 3a.7.7 0 0 1 1.198-.043L16.3 8.9a.7.7 0 0 1-.572 1.1Z"></path>
      <rect x="3" y="14" width="7" height="7" rx="1"></rect>
      <circle cx="17.5" cy="17.5" r="3.5"></circle>
    `,
    "move-horizontal": `
      <path d="m18 8 4 4-4 4"></path>
      <path d="M2 12h20"></path>
      <path d="m6 8-4 4 4 4"></path>
    `,
    "arrow-right": `
      <path d="M5 12h14"></path>
      <path d="m12 5 7 7-7 7"></path>
    `,
    "volume-2": `
      <path d="M11 4.702a1 1 0 0 0-1.664-.747L5.23 7.5H3a1 1 0 0 0-1 1v7a1 1 0 0 0 1 1h2.23l4.106 3.545A1 1 0 0 0 11 19.298z"></path>
      <path d="M16 9a5 5 0 0 1 0 6"></path>
      <path d="M19.364 18.364a9 9 0 0 0 0-12.728"></path>
    `,
    keyboard: `
      <path d="M10 8h.01"></path>
      <path d="M12 12h.01"></path>
      <path d="M14 8h.01"></path>
      <path d="M16 12h.01"></path>
      <path d="M18 8h.01"></path>
      <path d="M6 8h.01"></path>
      <path d="M7 16h10"></path>
      <path d="M8 12h.01"></path>
      <rect width="20" height="16" x="2" y="4" rx="2"></rect>
    `,
    layers: `
      <path d="m12.83 2.18a2 2 0 0 0-1.66 0L2.6 6.08a1 1 0 0 0 0 1.83l8.58 3.91a2 2 0 0 0 1.66 0l8.58-3.9a1 1 0 0 0 0-1.83z"></path>
      <path d="m22 12.5-9.17 4.18a2 2 0 0 1-1.66 0L2 12.5"></path>
      <path d="m22 17.5-9.17 4.18a2 2 0 0 1-1.66 0L2 17.5"></path>
    `,
    info: `
      <circle cx="12" cy="12" r="10"></circle>
      <path d="M12 16v-4"></path>
      <path d="M12 8h.01"></path>
    `,
    wrench: `
      <path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94z"></path>
    `,
    zap: `
      <path d="M4 14a1 1 0 0 1-.78-1.63l9.9-10.2a.5.5 0 0 1 .86.46l-1.92 6.02A1 1 0 0 0 13 10h7a1 1 0 0 1 .78 1.63l-9.9 10.2a.5.5 0 0 1-.86-.46l1.92-6.02A1 1 0 0 0 11 14z"></path>
    `,
    "file-code-2": `
      <path d="M4 22h14a2 2 0 0 0 2-2V7l-5-5H6a2 2 0 0 0-2 2v4"></path>
      <path d="M14 2v4a2 2 0 0 0 2 2h4"></path>
      <path d="m5 12-3 3 3 3"></path>
      <path d="m9 18 3-3-3-3"></path>
    `,
  };
  const paths = icons[name];
  if (!paths) {
    throw new Error(`Unknown source outline lucide icon ${name}`);
  }
  return `
    <svg xmlns="http://www.w3.org/2000/svg" class="source-outline-icon lucide lucide-${name}-icon lucide-${name}" viewBox="0 0 24 24" aria-hidden="true">
      ${paths}
    </svg>
  `;
}

function toggleSourceOutlineItem(itemId, expanded = null) {
  const parentIdsWithChildren = sourceOutlineParentIdsWithChildren();
  if (!parentIdsWithChildren.has(itemId)) {
    return false;
  }
  const nextExpanded = expanded ?? !sourceOutlineExpandedItemIds.has(itemId);
  if (nextExpanded) {
    sourceOutlineExpandedItemIds.add(itemId);
  } else {
    sourceOutlineExpandedItemIds.delete(itemId);
  }
  renderSourceOutline();
  syncSourceOutlineActiveItem();
  sourceOutlineList
    ?.querySelector(`[data-source-outline-id="${CSS.escape(itemId)}"]`)
    ?.focus({ preventScroll: true });
  return true;
}

function openSourceOutlineItem(itemId) {
  const item = sourceOutlineItems.find((entry) => entry.id === itemId);
  const document = activeDocument();
  if (!item || !document) {
    return false;
  }
  const opened = revealSourceLocation({
    document,
    start: item.start,
  });
  if (opened) {
    syncSourceOutlineActiveItem();
    sourceEditor.focus({ preventScroll: true });
  }
  return opened;
}

function syncSourceOutlineActiveItem(options = {}) {
  if (!sourceOutlineList || !sourceOutlineItems.length) {
    return;
  }
  const cursor = Number.isInteger(options.position)
    ? options.position
    : sourceViewOffsetToDocumentOffset(sourceEditor.selectionStart || 0, "start");
  let active = null;
  for (const item of sourceOutlineItems) {
    if (cursor >= item.start && cursor <= Math.max(item.end, item.start)) {
      if (!active || item.start >= active.start) {
        active = item;
      }
    } else if (!active && cursor >= item.start) {
      active = item;
    } else if (cursor >= item.start && item.start > active.start) {
      active = item;
    }
  }
  const itemsById = sourceOutlineItemById();
  let activeId = active?.id || "";
  let parentId = active?.parent || "";
  while (parentId) {
    if (!sourceOutlineExpandedItemIds.has(parentId)) {
      activeId = parentId;
    }
    parentId = itemsById.get(parentId)?.parent || "";
  }
  for (const row of sourceOutlineList.querySelectorAll("[data-source-outline-id]")) {
    row.classList.toggle("is-active", activeId === row.dataset.sourceOutlineId);
  }
}

async function suggestSourceCompletionsWithWasm(source, cursorOffset) {
  if (typeof window.PuzzleStudioRuntime?.suggestSourceCompletions !== "function") {
    return null;
  }
  const json = await window.PuzzleStudioRuntime.suggestSourceCompletions(source, cursorOffset);
  const list = JSON.parse(json || "{}");
  return {
    replaceStart: Number(list.replaceStart) || 0,
    replaceEnd: Number(list.replaceEnd) || 0,
    items: Array.isArray(list.items) ? list.items : [],
  };
}

function createSourceCompletionPopover() {
  if (!sourceEditorWrap) {
    return null;
  }
  const popover = document.createElement("div");
  popover.className = "source-completion-popover";
  popover.hidden = true;
  popover.addEventListener("mousedown", (event) => {
    event.preventDefault();
    event.stopPropagation();
  });
  popover.addEventListener("click", (event) => {
    event.stopPropagation();
    const item = event.target.closest("[data-source-completion-index]");
    if (!item || !sourceCompletionState) {
      return;
    }
    acceptSourceCompletion(Number(item.dataset.sourceCompletionIndex));
  });
  sourceEditorWrap.append(popover);
  return popover;
}

function createSourceBlockSelectionLayer() {
  if (!sourceEditorWrap) {
    return null;
  }
  const layer = document.createElement("div");
  layer.className = "source-block-selection-layer";
  layer.hidden = true;
  sourceEditorWrap.append(layer);
  return layer;
}

function createSourceFindMatchLayer() {
  if (!sourceEditorWrap) {
    return null;
  }
  const layer = document.createElement("div");
  layer.className = "source-find-match-layer";
  layer.hidden = true;
  sourceEditorWrap.append(layer);
  return layer;
}

function createSourceFindPanel() {
  if (!sourceEditorWrap) {
    return null;
  }
  const panel = document.createElement("div");
  panel.className = "source-find-panel";
  panel.hidden = true;
  panel.innerHTML = `
    <div class="source-find-row">
      <input class="source-find-input" data-source-find-input type="search" autocomplete="off" autocapitalize="off" spellcheck="false" placeholder="Find" aria-label="Find in source">
      <button class="source-find-icon-button" data-source-find-case type="button" aria-label="Match case" title="Match case" aria-pressed="false">Aa</button>
      <button class="source-find-icon-button" data-source-find-previous type="button" aria-label="Previous match" title="Previous match">
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m18 15-6-6-6 6"></path></svg>
      </button>
      <button class="source-find-icon-button" data-source-find-next type="button" aria-label="Next match" title="Next match">
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m6 9 6 6 6-6"></path></svg>
      </button>
      <button class="source-find-icon-button" data-source-find-close type="button" aria-label="Close find" title="Close">
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M18 6 6 18"></path><path d="m6 6 12 12"></path></svg>
      </button>
    </div>
    <div class="source-find-row source-replace-row">
      <input class="source-find-input" data-source-replace-input type="text" autocomplete="off" autocapitalize="off" spellcheck="false" placeholder="Replace" aria-label="Replace with">
      <button class="source-find-icon-button" data-source-replace-current type="button" aria-label="Replace" title="Replace">
        <svg class="lucide lucide-replace" viewBox="0 0 24 24" aria-hidden="true"><path d="M14 4a1 1 0 0 1 1-1"></path><path d="M15 10a1 1 0 0 1-1-1"></path><path d="M21 4a1 1 0 0 0-1-1"></path><path d="M21 9a1 1 0 0 1-1 1"></path><path d="m3 7 3 3 3-3"></path><path d="M6 10V5a2 2 0 0 1 2-2h2"></path><rect x="3" y="14" width="7" height="7" rx="1"></rect></svg>
      </button>
      <button class="source-find-icon-button" data-source-replace-all type="button" aria-label="Replace all" title="Replace all">
        <svg class="lucide lucide-replace-all" viewBox="0 0 24 24" aria-hidden="true"><path d="M14 14a1 1 0 0 1 1 1v5a1 1 0 0 1-1 1"></path><path d="M14 4a1 1 0 0 1 1-1"></path><path d="M15 10a1 1 0 0 1-1-1"></path><path d="M19 14a1 1 0 0 1 1 1v5a1 1 0 0 1-1 1"></path><path d="M21 4a1 1 0 0 0-1-1"></path><path d="M21 9a1 1 0 0 1-1 1"></path><path d="m3 7 3 3 3-3"></path><path d="M6 10V5a2 2 0 0 1 2-2h2"></path><rect x="3" y="14" width="7" height="7" rx="1"></rect></svg>
      </button>
    </div>
    <div class="source-find-status" data-source-find-status aria-live="polite">No query</div>
  `;
  sourceEditorWrap.append(panel);
  return panel;
}

function scheduleSourceCompletion(immediate = false) {
  if (!sourceDocumentSupportsEditableTargets()) {
    hideSourceCompletions();
    return;
  }
  window.clearTimeout(sourceCompletionTimer);
  sourceCompletionTimer = window.setTimeout(() => {
    showSourceCompletions({ manual: false });
  }, immediate ? 0 : 120);
}

async function showSourceCompletions(options = {}) {
  const document = activeDocument();
  if (!sourceCompletionPopover || !isPuzzleDocument(document) || !isTextDocument(document)) {
    hideSourceCompletions();
    return false;
  }
  const source = sourceEditor.value;
  const cursor = sourceEditor.selectionStart;
  if (!options.manual && !sourceAutoCompletionEligible(source, cursor)) {
    hideSourceCompletions();
    return false;
  }
  const requestId = ++sourceCompletionRequestId;
  try {
    const list = await suggestSourceCompletionsWithWasm(source, cursor);
    if (requestId !== sourceCompletionRequestId || source !== sourceEditor.value || cursor !== sourceEditor.selectionStart) {
      return false;
    }
    const items = filterSourceCompletionsForTypedReplacement(
      filterSourceCompletionsForDocument(list?.items || [], document),
      list,
      source,
      cursor,
    );
    if (!items.length) {
      hideSourceCompletions();
      return false;
    }
    const mode = sourceCompletionMode(options, list, source, cursor);
    const previousState = sourceCompletionState;
    const selectedIndex = sourceCompletionSelectedIndexForSession(previousState, {
      source,
      cursor,
      replaceStart: list.replaceStart,
      replaceEnd: list.replaceEnd,
      items,
      mode,
    });
    sourceCompletionState = {
      mode,
      source,
      cursor,
      replaceStart: list.replaceStart,
      replaceEnd: list.replaceEnd,
      items,
      selectedIndex,
    };
    renderSourceCompletionItems();
    positionSourceCompletionPopover();
    sourceCompletionPopover.hidden = false;
    return true;
  } catch {
    hideSourceCompletions();
    return false;
  }
}

function filterSourceCompletionsForDocument(items, document) {
  const profile = typeof puzzleSourceProfile === "function" ? puzzleSourceProfile(document) : "";
  const hidden = profile === "puzzle3d"
    ? new Set(["puzzle", "levels", "sprites"])
    : profile === "puzzle2d"
      ? new Set(["puzzle3", "levels3", "sprites3"])
      : null;
  if (!hidden) {
    return items;
  }
  return items.filter((item) => !hidden.has(item?.label || ""));
}

function filterSourceCompletionsForTypedReplacement(items, list, source, cursor) {
  const replaceStart = Math.max(0, Math.min(source.length, Number(list?.replaceStart) || 0));
  const safeCursor = Math.max(replaceStart, Math.min(source.length, Number(cursor) || replaceStart));
  const replaceEnd = Math.max(safeCursor, Math.min(source.length, Number(list?.replaceEnd) || safeCursor));
  // The full token under the caret, not just the prefix before it: when the
  // cursor lands inside an existing word we must still suppress a suggestion
  // that would replace it with the identical text.
  const current = source.slice(replaceStart, replaceEnd);
  if (!current) {
    return items;
  }
  return items.filter((item) => (item?.insertText || item?.label || "") !== current);
}

function sourceAutoCompletionEligible(source, cursor) {
  if (
    sourceEditor.selectionStart !== sourceEditor.selectionEnd
    || sourceEditorBlockSelection?.ranges?.length
    || sourceCursorBeforeSyntaxBoundaryWithoutPrefix(source, cursor)
  ) {
    return false;
  }
  return sourceCursorHasCompletionPrefix(source, cursor)
    || sourceCursorAfterSelectorTagSeparator(source, cursor);
}

function sourceCursorHasCompletionPrefix(source, cursor) {
  return /[_@A-Za-z0-9.-]$/.test(source.slice(0, cursor));
}

function sourceCursorAfterSelectorTagSeparator(source, cursor) {
  return /(?:^|[^\w@.-])[@A-Za-z_][\w@.-]*(?::[@A-Za-z_][\w@.-]*)*:$/.test(source.slice(0, cursor));
}

function sourceCursorBeforeSyntaxBoundaryWithoutPrefix(source, cursor) {
  const before = source.slice(0, cursor);
  if (
    /[_@A-Za-z0-9.-]$/.test(before)
    || sourceCursorAfterSelectorTagSeparator(source, cursor)
  ) {
    return false;
  }
  const lineEnd = source.indexOf("\n", cursor);
  const safeLineEnd = lineEnd < 0 ? source.length : lineEnd;
  const after = stripSourceImportLineComment(source.slice(cursor, safeLineEnd));
  return /^[\t ]*[\]{}]/.test(after);
}

function sourceCompletionMode(options, list, source, cursor) {
  if (options.manual) {
    return "completion";
  }
  const replaceStart = Math.max(0, Math.min(source.length, Number(list?.replaceStart) || 0));
  const prefix = source.slice(replaceStart, cursor);
  return /[_@A-Za-z0-9.-]/.test(prefix) ? "completion" : "hint";
}

function sourceCompletionSelectedIndexForSession(previousState, nextState) {
  if (nextState.mode !== "completion") {
    return null;
  }
  if (!sourceCompletionSessionMatches(previousState, nextState)) {
    return 0;
  }
  const previousIndex = Number.isInteger(previousState.selectedIndex)
    ? previousState.selectedIndex
    : 0;
  const previousItem = previousState.items?.[previousIndex];
  const matchingIndex = nextState.items.findIndex((item) => sourceCompletionItemsMatch(item, previousItem));
  if (matchingIndex >= 0) {
    return matchingIndex;
  }
  return Math.max(0, Math.min(nextState.items.length - 1, previousIndex));
}

function sourceCompletionSessionMatches(previousState, nextState) {
  return Boolean(
    previousState
    && previousState.source === nextState.source
    && previousState.cursor === nextState.cursor
    && previousState.replaceStart === nextState.replaceStart
    && previousState.replaceEnd === nextState.replaceEnd
  );
}

function sourceCompletionItemsMatch(left, right) {
  return Boolean(
    left
    && right
    && (left.label || "") === (right.label || "")
    && (left.insertText || "") === (right.insertText || "")
    && (left.kind || "") === (right.kind || "")
    && (left.detail || "") === (right.detail || "")
  );
}

function hideSourceCompletions() {
  sourceCompletionState = null;
  sourceCompletionRequestId += 1;
  window.clearTimeout(sourceCompletionTimer);
  if (sourceCompletionPopover) {
    sourceCompletionPopover.hidden = true;
    sourceCompletionPopover.innerHTML = "";
  }
}

function keepSourceCompletionsVisibleDuringEdit() {
  return Boolean(sourceCompletionPopover && !sourceCompletionPopover.hidden && sourceCompletionState);
}

function renderSourceCompletionItems() {
  if (!sourceCompletionPopover || !sourceCompletionState) {
    return;
  }
  sourceCompletionPopover.innerHTML = sourceCompletionState.items.map((item, index) => {
    const selected = index === sourceCompletionState.selectedIndex ? " is-selected" : "";
    return `
      <button class="source-completion-item${selected}" data-source-completion-index="${index}" type="button">
        <span class="source-completion-label">${escapeHtml(item.label || "")}</span>
        <span class="source-completion-kind source-completion-kind-${escapeHtml(item.kind || "keyword")}">${escapeHtml(sourceCompletionKindLabel(item))}</span>
      </button>
    `;
  }).join("");
  sourceCompletionPopover
    .querySelector(".source-completion-item.is-selected")
    ?.scrollIntoView({ block: "nearest" });
}

function sourceCompletionKindLabel(item) {
  return item.detail || item.kind || "";
}

function moveSourceCompletionSelection(delta) {
  if (!sourceCompletionState?.items?.length) {
    return;
  }
  sourceCompletionState.mode = "completion";
  if (!Number.isInteger(sourceCompletionState.selectedIndex)) {
    sourceCompletionState.selectedIndex = 0;
  }
  const count = sourceCompletionState.items.length;
  sourceCompletionState.selectedIndex = (sourceCompletionState.selectedIndex + delta + count) % count;
  renderSourceCompletionItems();
}

function sourceCompletionMatchesCurrentCursor() {
  return Boolean(
    sourceCompletionState
    && sourceEditor.value === sourceCompletionState.source
    && sourceEditor.selectionStart === sourceEditor.selectionEnd
    && sourceEditor.selectionStart === sourceCompletionState.cursor
  );
}

function sourceCursorInLineLeadingWhitespace() {
  const source = sourceEditor.value || "";
  const cursor = sourceEditor.selectionStart;
  const lineStart = source.lastIndexOf("\n", cursor - 1) + 1;
  return /^[\t ]*$/.test(source.slice(lineStart, cursor));
}

function acceptSourceCompletion(index = sourceCompletionState?.selectedIndex ?? 0) {
  if (!sourceDocumentSupportsEditableTargets()) {
    hideSourceCompletions();
    return false;
  }
  if (!sourceCompletionState || !sourceCompletionMatchesCurrentCursor()) {
    return false;
  }
  const item = sourceCompletionState.items[index];
  if (!item) {
    return false;
  }
  const insertText = item.insertText || item.label || "";
  const replaceStart = Math.max(0, Math.min(sourceEditor.value.length, sourceCompletionState.replaceStart));
  const replaceEnd = Math.max(replaceStart, Math.min(sourceEditor.value.length, sourceCompletionState.replaceEnd));
  sourceEditor.setRangeText(insertText, replaceStart, replaceEnd, "end");
  recordSourceUndoSnapshot();
  hideSourceCompletions();
  updateSourceMeta();
  if (documents[currentDocumentIndex]) {
    documents[currentDocumentIndex].source = sourceEditorDocumentValue();
  }
  scheduleSourceHighlight();
  scheduleLocalSave();
  resetLevelBuilderFromSource(false);
  schedulePreview();
  return true;
}

function positionSourceCompletionPopover() {
  if (!sourceCompletionPopover || !sourceEditorWrap || !sourceEditor) {
    return;
  }
  const anchor = Math.max(
    0,
    Math.min(sourceEditor.value.length, sourceCompletionState?.replaceStart ?? sourceEditor.selectionStart),
  );
  const wrapRect = sourceEditorWrap.getBoundingClientRect();
  const anchorRect = sourceCaretRectForOffset(anchor);
  const cursorRect = sourceCaretRectForOffset(sourceEditor.selectionStart);
  if (!anchorRect || !cursorRect) {
    return;
  }
  const maxLeft = Math.max(8, window.innerWidth - 284);
  const left = wrapRect.left + anchorRect.left;
  const top = wrapRect.top + cursorRect.top + cursorRect.height + 6;
  const availableBelow = Math.max(56, window.innerHeight - top - 8);
  sourceCompletionPopover.style.left = `${Math.max(8, Math.min(maxLeft, left))}px`;
  sourceCompletionPopover.style.top = `${top}px`;
  sourceCompletionPopover.style.maxHeight = `${Math.min(216, availableBelow)}px`;
}

function sourceFindShortcutRequested(event) {
  if (sourceBlockSelectionOwnsControlShortcut(event)) {
    return false;
  }
  const modifier = (event.metaKey && !event.ctrlKey) || (event.ctrlKey && !event.metaKey);
  if (!modifier || event.shiftKey) {
    return false;
  }
  const key = event.key.length === 1 ? event.key.toLowerCase() : event.key;
  return key === "f" || event.code === "KeyF";
}

function handleSourceFindShortcut(event) {
  if (!sourceFindShortcutRequested(event) || !isTextDocument(documents[currentDocumentIndex])) {
    return false;
  }
  event.preventDefault();
  event.stopPropagation();
  event.stopImmediatePropagation?.();
  openSourceFindPanel({ replace: event.altKey });
  return true;
}

function sourceFindMoveShortcutRequested(event) {
  if (!event.metaKey || event.ctrlKey || event.altKey) {
    return false;
  }
  const key = event.key.length === 1 ? event.key.toLowerCase() : event.key;
  return key === "g" || event.code === "KeyG";
}

function handleSourceFindMoveShortcut(event) {
  if (!sourceFindMoveShortcutRequested(event) || !isTextDocument(documents[currentDocumentIndex])) {
    return false;
  }
  event.preventDefault();
  event.stopPropagation();
  event.stopImmediatePropagation?.();
  moveSourceFindSelection(event.shiftKey ? -1 : 1);
  return true;
}

function isSourceFindPanelOpen() {
  return Boolean(sourceFindPanel && !sourceFindPanel.hidden);
}

function openSourceFindPanel(options = {}) {
  if (!sourceFindPanel || !sourceFindInput || !isTextDocument(documents[currentDocumentIndex])) {
    return false;
  }
  hideSourceColorEditor();
  hideSourceCompletions();
  setSourceFindReplaceVisible(Boolean(options.replace) || sourceFindState.replaceVisible);
  const selected = sourceFindSeedFromSelection();
  if (selected) {
    sourceFindInput.value = selected;
  }
  sourceFindPanel.hidden = false;
  syncSourceFindMatches({ select: Boolean(sourceFindInput.value), anchor: sourceEditor.selectionStart });
  window.setTimeout(() => {
    sourceFindInput.focus();
    sourceFindInput.select();
  }, 0);
  return true;
}

function closeSourceFindPanel(options = {}) {
  if (!sourceFindPanel) {
    return;
  }
  sourceFindPanel.hidden = true;
  sourceFindState.matches = [];
  sourceFindState.selectedIndex = -1;
  renderSourceFindMatches();
  if (options.focusEditor !== false) {
    sourceEditor.focus({ preventScroll: true });
  }
}

function sourceFindSeedFromSelection() {
  const start = Math.min(sourceEditor.selectionStart, sourceEditor.selectionEnd);
  const end = Math.max(sourceEditor.selectionStart, sourceEditor.selectionEnd);
  const value = sourceEditor.value.slice(start, end);
  if (!value || value.length > 160 || value.includes("\n")) {
    return "";
  }
  return value;
}

function setSourceFindReplaceVisible(visible) {
  sourceFindState.replaceVisible = Boolean(visible);
  sourceFindPanel?.classList.toggle("has-replace", sourceFindState.replaceVisible);
  if (sourceFindState.replaceVisible) {
    sourceReplaceInput?.removeAttribute("tabindex");
  } else {
    sourceReplaceInput?.setAttribute("tabindex", "-1");
  }
}

function syncSourceFindMatches(options = {}) {
  if (!isSourceFindPanelOpen() || !sourceFindInput) {
    return;
  }
  const query = sourceFindInput.value || "";
  sourceFindState.matches = findSourceMatches(query, sourceFindState.matchCase);
  if (!query) {
    sourceFindState.selectedIndex = -1;
    setSourceFindStatus("No query");
    renderSourceFindMatches();
    return;
  }
  if (!sourceFindState.matches.length) {
    sourceFindState.selectedIndex = -1;
    setSourceFindStatus("No results");
    renderSourceFindMatches();
    return;
  }

  const exactIndex = sourceFindState.matches.findIndex((match) =>
    match.start === sourceEditor.selectionStart && match.end === sourceEditor.selectionEnd
  );
  if (exactIndex >= 0) {
    sourceFindState.selectedIndex = exactIndex;
  } else if (options.keepIndex && sourceFindState.matches[sourceFindState.selectedIndex]) {
    sourceFindState.selectedIndex = Math.max(0, Math.min(sourceFindState.matches.length - 1, sourceFindState.selectedIndex));
  } else {
    const anchor = Number.isInteger(options.anchor) ? options.anchor : sourceEditor.selectionEnd;
    const nextIndex = sourceFindState.matches.findIndex((match) => match.start >= anchor);
    sourceFindState.selectedIndex = nextIndex >= 0 ? nextIndex : 0;
  }

  if (options.select !== false) {
    selectSourceFindMatch(sourceFindState.selectedIndex, { focusEditor: false });
  }
  updateSourceFindStatus();
  renderSourceFindMatches();
}

function findSourceMatches(query, matchCase) {
  const needle = String(query || "");
  if (!needle) {
    return [];
  }
  const source = sourceEditor.value || "";
  const haystack = matchCase ? source : source.toLocaleLowerCase();
  const normalizedNeedle = matchCase ? needle : needle.toLocaleLowerCase();
  const matches = [];
  let index = haystack.indexOf(normalizedNeedle);
  while (index >= 0) {
    matches.push({ start: index, end: index + needle.length });
    index = haystack.indexOf(normalizedNeedle, index + Math.max(1, needle.length));
  }
  return matches;
}

function selectSourceFindMatch(index, options = {}) {
  const match = sourceFindState.matches[index];
  if (!match) {
    return false;
  }
  sourceFindState.selectedIndex = index;
  sourceEditor.setSelectionRange(match.start, match.end);
  scrollSourceOffsetIntoView(match.start);
  if (options.focusEditor) {
    sourceEditor.focus({ preventScroll: true });
  }
  updateSourceFindStatus();
  renderSourceBlockSelection();
  renderSourceFindMatches();
  return true;
}

function moveSourceFindSelection(delta) {
  if (!isSourceFindPanelOpen()) {
    openSourceFindPanel();
  }
  syncSourceFindMatches({ select: false });
  if (!sourceFindState.matches.length) {
    return false;
  }
  const count = sourceFindState.matches.length;
  const current = sourceFindState.selectedIndex >= 0 ? sourceFindState.selectedIndex : 0;
  return selectSourceFindMatch((current + delta + count) % count, { focusEditor: false });
}

function replaceCurrentSourceFindMatch() {
  if (!isSourceFindPanelOpen() || !sourceReplaceInput) {
    return false;
  }
  syncSourceFindMatches({ select: false });
  const match = sourceFindState.matches[sourceFindState.selectedIndex];
  if (!match) {
    return false;
  }
  const replacement = sourceReplaceInput.value || "";
  sourceEditor.setRangeText(replacement, match.start, match.end, "select");
  const nextAnchor = match.start + replacement.length;
  sourceEditor.setSelectionRange(match.start, nextAnchor);
  sourceEditorContentChanged();
  syncSourceFindMatches({ anchor: nextAnchor });
  return true;
}

function replaceAllSourceFindMatches() {
  if (!isSourceFindPanelOpen() || !sourceReplaceInput || !sourceFindState.matches.length) {
    return false;
  }
  if (expandSourceFoldsForEdit()) {
    syncSourceFindMatches({ anchor: sourceEditor.selectionStart, select: false });
    if (!sourceFindState.matches.length) {
      return false;
    }
  }
  const matches = [...sourceFindState.matches];
  const replacement = sourceReplaceInput.value || "";
  const source = sourceEditor.value || "";
  let output = "";
  let cursor = 0;
  for (const match of matches) {
    output += source.slice(cursor, match.start);
    output += replacement;
    cursor = match.end;
  }
  output += source.slice(cursor);
  const firstStart = matches[0]?.start ?? 0;
  const firstEnd = firstStart + replacement.length;
  sourceEditor.value = output;
  sourceEditor.setSelectionRange(firstStart, firstEnd);
  sourceEditorContentChanged();
  syncSourceFindMatches({ anchor: firstEnd, select: false });
  setSourceFindStatus(`Replaced ${matches.length}`);
  renderSourceFindMatches();
  return true;
}

function refreshSourceFindAfterSourceChange() {
  if (!isSourceFindPanelOpen()) {
    return;
  }
  syncSourceFindMatches({ keepIndex: true, select: false });
}

function syncSourceFindIndexFromSelection() {
  if (!isSourceFindPanelOpen() || !sourceFindState.matches.length) {
    return;
  }
  const index = sourceFindState.matches.findIndex((match) =>
    match.start === sourceEditor.selectionStart && match.end === sourceEditor.selectionEnd
  );
  if (index < 0 || index === sourceFindState.selectedIndex) {
    return;
  }
  sourceFindState.selectedIndex = index;
  updateSourceFindStatus();
  renderSourceFindMatches();
}

function updateSourceFindStatus() {
  const count = sourceFindState.matches.length;
  const index = sourceFindState.selectedIndex;
  setSourceFindStatus(count ? `${index + 1} / ${count}` : "No results");
}

function setSourceFindStatus(text) {
  if (sourceFindStatus) {
    sourceFindStatus.textContent = text;
  }
}

function renderSourceFindMatches() {
  if (!sourceFindMatchLayer) {
    return;
  }
  sourceFindMatchLayer.replaceChildren();
  if (!isSourceFindPanelOpen() || !sourceFindState.matches.length || !isTextDocument(activeDocument())) {
    sourceFindMatchLayer.hidden = true;
    return;
  }
  sourceFindState.matches.slice(0, 600).forEach((match, index) => {
    for (const item of sourceSelectionRectsForOffsets(match.start, match.end)) {
      const rect = document.createElement("div");
      rect.className = `source-find-match${index === sourceFindState.selectedIndex ? " is-current" : ""}`;
      rect.style.left = `${item.left}px`;
      rect.style.top = `${item.top}px`;
      rect.style.width = `${item.width}px`;
      rect.style.height = `${item.height}px`;
      sourceFindMatchLayer.append(rect);
    }
  });
  sourceFindMatchLayer.hidden = sourceFindMatchLayer.childElementCount === 0;
}

function scrollSourceOffsetIntoView(offset, alignment = "nearest") {
  if (sourceEditor.sourceEditorPort?.kind === "codemirror") {
    sourceEditor.sourceEditorPort.scrollIntoView(offset, alignment);
    return;
  }
  const rect = sourceCaretRectForOffset(offset);
  if (!rect) {
    return;
  }
  const margin = 32;
  if (rect.top < margin) {
    setSourceScrollTop(sourceScrollTop() + rect.top - margin);
  } else if (rect.top + rect.height > sourceViewportHeight() - margin) {
    setSourceScrollTop(sourceScrollTop() + rect.top + rect.height - sourceViewportHeight() + margin);
  }
  if (rect.left < margin) {
    setSourceScrollLeft(sourceScrollLeft() + rect.left - margin);
  } else if (rect.left > sourceViewportWidth() - margin) {
    setSourceScrollLeft(sourceScrollLeft() + rect.left - sourceViewportWidth() + margin);
  }
  syncSourceHighlightScroll();
}

function sourceVisualCaretPoint(offset) {
  const rect = sourceCaretRectForOffset(offset);
  return rect ? { left: rect.left, top: rect.top } : null;
}

function sourceEditorCaretPoint(offset) {
  const style = window.getComputedStyle(sourceEditor);
  const mirror = document.createElement("div");
  const marker = document.createElement("span");
  mirror.style.position = "absolute";
  mirror.style.visibility = "hidden";
  mirror.style.pointerEvents = "none";
  mirror.style.boxSizing = "border-box";
  mirror.style.width = `${sourceViewportWidth()}px`;
  mirror.style.minHeight = "0";
  mirror.style.padding = style.padding;
  mirror.style.border = style.border;
  mirror.style.font = style.font;
  mirror.style.lineHeight = style.lineHeight;
  mirror.style.letterSpacing = style.letterSpacing;
  mirror.style.fontVariantLigatures = style.fontVariantLigatures;
  mirror.style.fontFeatureSettings = style.fontFeatureSettings;
  mirror.style.tabSize = style.tabSize;
  mirror.style.whiteSpace = "pre-wrap";
  mirror.style.overflowWrap = "break-word";
  mirror.style.wordBreak = style.wordBreak;
  mirror.textContent = sourceEditor.value.slice(0, offset);
  marker.textContent = "\u200b";
  mirror.append(marker);
  document.body.append(mirror);
  const point = {
    left: marker.offsetLeft - sourceScrollLeft(),
    top: marker.offsetTop - sourceScrollTop(),
  };
  mirror.remove();
  return point;
}

function sourceCaretRectForOffset(offset) {
  const source = sourceEditor.value || "";
  const safeOffset = Math.max(0, Math.min(source.length, offset || 0));
  if (sourceEditor.sourceEditorPort?.kind === "codemirror") {
    const rect = sourceEditor.sourceEditorPort.coordsAtOffset(safeOffset);
    if (!rect) {
      return null;
    }
    const wrapRect = sourceEditorWrap.getBoundingClientRect();
    return {
      left: rect.left - wrapRect.left,
      top: rect.top - wrapRect.top,
      height: rect.height,
    };
  }
  const domPosition = sourceHighlightDomPositionForOffset(safeOffset);
  if (!domPosition) {
    const fallback = sourceEditorCaretPoint(safeOffset);
    return {
      left: fallback.left,
      top: fallback.top,
      height: sourceEditorLineHeight(),
    };
  }

  const range = document.createRange();
  const wrapRect = sourceEditorWrap.getBoundingClientRect();
  const lineHeight = sourceEditorLineHeight();
  let rect = null;

  range.setStart(domPosition.node, domPosition.offset);
  range.setEnd(domPosition.node, domPosition.offset);
  rect = Array.from(range.getClientRects()).find((item) => item.height > 0) || null;

  if (!rect) {
    rect = sourceAdjacentTextRect(domPosition.node, domPosition.offset, 1)
      || sourceAdjacentTextRect(domPosition.node, domPosition.offset, -1);
  }
  range.detach?.();

  if (!rect) {
    const fallback = sourceEditorCaretPoint(safeOffset);
    return {
      left: fallback.left,
      top: fallback.top,
      height: lineHeight,
    };
  }
  const useRightEdge = domPosition.edge === "right";
  const rectHeight = rect.height || lineHeight;
  const height = Math.max(lineHeight, rectHeight);
  const top = rect.top - wrapRect.top - Math.max(0, (lineHeight - rectHeight) / 2);
  return {
    left: (useRightEdge ? rect.right : rect.left) - wrapRect.left,
    top,
    height,
  };
}

function sourceAdjacentTextRect(node, offset, direction) {
  if (!node || node.nodeType !== Node.TEXT_NODE) {
    return null;
  }
  const text = node.nodeValue || "";
  const start = direction > 0 ? offset : offset - 1;
  const end = start + 1;
  if (start < 0 || end > text.length) {
    return null;
  }
  const range = document.createRange();
  range.setStart(node, start);
  range.setEnd(node, end);
  const rect = Array.from(range.getClientRects()).find((item) => item.height > 0) || null;
  range.detach?.();
  if (!rect) {
    return null;
  }
  return direction > 0
    ? { left: rect.left, right: rect.left, top: rect.top, height: rect.height }
    : { left: rect.right, right: rect.right, top: rect.top, height: rect.height };
}

function sourceHighlightDomPositionForOffset(offset) {
  if (!sourceHighlight) {
    return null;
  }
  const walker = document.createTreeWalker(sourceHighlight, NodeFilter.SHOW_TEXT);
  let remaining = Math.max(0, offset || 0);
  let lastText = null;
  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    const length = (node.nodeValue || "").length;
    if (remaining < length) {
      return { node, offset: remaining, edge: "left" };
    }
    if (remaining === length) {
      const text = node.nodeValue || "";
      const nextNode = walker.nextNode();
      const nextText = nextNode?.nodeValue || "";
      if (nextNode && text.endsWith("\n")) {
        return { node: nextNode, offset: 0, edge: "left" };
      }
      if (nextNode && nextText.startsWith("\n")) {
        return { node, offset: length, edge: "right" };
      }
      if (nextNode) {
        return { node: nextNode, offset: 0, edge: "left" };
      }
      return { node, offset: length, edge: "right" };
    }
    remaining -= length;
    lastText = node;
  }
  if (lastText) {
    return { node: lastText, offset: (lastText.nodeValue || "").length, edge: "right" };
  }
  return null;
}

function sourceVisualOffsetFromPoint(clientX, clientY) {
  if (sourceEditor.sourceEditorPort?.kind === "codemirror") {
    const offset = sourceEditor.sourceEditorPort.offsetAtCoords(clientX, clientY);
    return Number.isInteger(offset) ? offset : null;
  }
  if (!sourceHighlight || !sourceEditor) {
    return null;
  }
  const source = sourceEditor.value || "";
  if (!source.length) {
    return 0;
  }
  const walker = document.createTreeWalker(sourceHighlight, NodeFilter.SHOW_TEXT);
  const range = document.createRange();
  let sourceOffset = 0;
  let nearestLineDistance = Number.POSITIVE_INFINITY;
  const textNodes = [];
  let best = null;
  let lineHit = null;
  let bestInLine = null;

  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    const text = node.nodeValue || "";
    if (!text.length) {
      continue;
    }
    range.selectNodeContents(node);
    const lineDistance = Array.from(range.getClientRects()).reduce((distance, rect) => {
      if (rect.width <= 0 && rect.height <= 0) {
        return distance;
      }
      const next = clientY < rect.top
        ? rect.top - clientY
        : clientY > rect.bottom
          ? clientY - rect.bottom
          : 0;
      return Math.min(distance, next);
    }, Number.POSITIVE_INFINITY);
    textNodes.push({ node, text, sourceOffset, lineDistance });
    nearestLineDistance = Math.min(nearestLineDistance, lineDistance);
    sourceOffset += text.length;
  }

  // A click belongs to one visual line. Locate that line from whole text-node
  // rectangles first, then measure only characters belonging to that line.
  for (const entry of textNodes) {
    if (entry.lineDistance > nearestLineDistance + 0.5) {
      continue;
    }
    const { node, text } = entry;
    for (let index = 0; index < text.length; index += 1) {
      range.setStart(node, index);
      range.setEnd(node, index + 1);
      for (const rect of range.getClientRects()) {
        if (rect.width <= 0 && rect.height <= 0) {
          continue;
        }
        const lineDistance = clientY < rect.top
          ? rect.top - clientY
          : clientY > rect.bottom
            ? clientY - rect.bottom
            : 0;
        const midX = rect.left + (rect.width / 2);
        const char = text[index];
        const charStart = entry.sourceOffset + index;
        const charEnd = char === "\n" ? charStart : charStart + 1;
        const boundary = clientX <= midX ? charStart : charEnd;
        const horizontalDistance = clientX < rect.left
          ? rect.left - clientX
          : clientX > rect.right
            ? clientX - rect.right
            : 0;
        const score = (lineDistance * 10000) + horizontalDistance;
        if (!best || score < best.score) {
          best = { offset: boundary, score };
        }
        if (lineDistance === 0 && char !== "\n") {
          if (!lineHit) {
            lineHit = {
              left: rect.left,
              right: rect.right,
              startOffset: charStart,
              endOffset: charEnd,
            };
          } else {
            lineHit.left = Math.min(lineHit.left, rect.left);
            lineHit.right = Math.max(lineHit.right, rect.right);
            lineHit.startOffset = Math.min(lineHit.startOffset, charStart);
            lineHit.endOffset = Math.max(lineHit.endOffset, charEnd);
          }
          if (!bestInLine || horizontalDistance < bestInLine.score) {
            bestInLine = { offset: boundary, score: horizontalDistance };
          }
        }
      }
    }
  }
  range.detach?.();
  if (lineHit) {
    if (clientX <= lineHit.left) {
      return Math.max(0, Math.min(source.length, lineHit.startOffset));
    }
    if (clientX >= lineHit.right) {
      return Math.max(0, Math.min(source.length, lineHit.endOffset));
    }
    if (bestInLine) {
      return Math.max(0, Math.min(source.length, bestInLine.offset));
    }
  }
  return best ? Math.max(0, Math.min(source.length, best.offset)) : null;
}

function sourceOffsetFromVisualPointer(event, source = sourceEditorDocumentValue()) {
  if (!event || !sourceEditorWrap?.contains(event.target)) {
    return null;
  }
  const offset = sourceVisualOffsetFromPoint(event.clientX, event.clientY);
  if (!Number.isInteger(offset)) {
    return null;
  }
  const documentOffset = sourceFoldsActive()
    ? sourceViewOffsetToDocumentOffset(offset, "start")
    : offset;
  return Math.max(0, Math.min(String(source || "").length, documentOffset));
}

function sourceViewOffsetFromVisualPoint(clientX, clientY) {
  const offset = sourceVisualOffsetFromPoint(clientX, clientY);
  if (!Number.isInteger(offset)) {
    return null;
  }
  return Math.max(0, Math.min((sourceEditor.value || "").length, offset));
}

function sourceOffsetFromVisualPoint(clientX, clientY, source = sourceEditorDocumentValue()) {
  const offset = sourceViewOffsetFromVisualPoint(clientX, clientY);
  if (!Number.isInteger(offset)) {
    return null;
  }
  const documentOffset = sourceFoldsActive()
    ? sourceViewOffsetToDocumentOffset(offset, "start")
    : offset;
  return Math.max(0, Math.min(String(source || "").length, documentOffset));
}

function sourceUtf16OffsetFromByteOffset(value, byteOffset) {
  let bytes = 0;
  for (let index = 0; index < value.length;) {
    if (bytes >= byteOffset) {
      return index;
    }
    const codePoint = value.codePointAt(index);
    const char = String.fromCodePoint(codePoint);
    const nextBytes = sourceCompletionTextEncoder.encode(char).length;
    if (bytes + nextBytes > byteOffset) {
      return index;
    }
    bytes += nextBytes;
    index += codePoint > 0xffff ? 2 : 1;
  }
  return value.length;
}

function createSourceColorPopover() {
  if (!document.body) {
    return null;
  }
  const popover = document.createElement("span");
  popover.className = "source-color-popover";
  popover.hidden = true;
  popover.addEventListener("mousedown", (event) => {
    event.stopPropagation();
  });
  popover.addEventListener("click", (event) => {
    event.stopPropagation();
  });
  document.body.append(popover);
  return popover;
}

function createSourceImportLinkFrame() {
  if (!sourceEditorWrap) {
    return null;
  }
  const frame = document.createElement("button");
  frame.className = "source-import-link-frame";
  frame.type = "button";
  frame.hidden = true;
  frame.addEventListener("mousedown", (event) => {
    event.preventDefault();
    event.stopPropagation();
  });
  frame.addEventListener("click", (event) => {
    event.preventDefault();
    event.stopPropagation();
    openSourceFrameLink();
  });
  frame.addEventListener("mouseleave", handleSourceImportFrameMouseLeave);
  sourceEditorWrap.append(frame);
  return frame;
}

function showSourceColorEditor(event = null, visualOffset = null) {
  if (!sourceColorPopover) {
    return false;
  }
  if (!isTextDocument(activeDocument())) {
    hideSourceColorEditor();
    return false;
  }
  const token = hexColorAt(sourceEditor.value, sourceEditor.selectionStart);
  if (!token) {
    hideSourceColorEditor();
    return false;
  }
  const parsed = parseHexColorToken(token.value);
  if (!parsed) {
    hideSourceColorEditor();
    return false;
  }
  if (event && !sourceColorEventTargetsToken(event, token, visualOffset)) {
    hideSourceColorEditor();
    return false;
  }
  sourceColorEdit = token;
  renderSourceColorPopover(formatHexColorToken(parsed.rgb, parsed.alpha), token);
  return true;
}

function renderSourceColorPopover(color, token) {
  if (!sourceColorPopover || !window.PuzzleStudioColorEditor) {
    return;
  }
  sourceColorPopover.replaceChildren(window.PuzzleStudioColorEditor.create({
    color,
    ariaLabel: "Source color",
    onInput: applySourceColorRgb,
  }));
  positionSourceColorPopoverForToken(token);
  sourceColorPopover.hidden = false;
}

function positionSourceColorPopoverForToken(token) {
  if (!sourceColorPopover || !sourceEditorWrap || !token) {
    return;
  }
  const startRect = sourceCaretRectForOffset(token.start);
  if (!startRect) {
    return;
  }
  const wrapRect = sourceEditorWrap.getBoundingClientRect();
  const menuRect = sourceColorPopover.getBoundingClientRect();
  const margin = 8;
  const gap = 6;
  const width = menuRect.width || 238;
  const height = menuRect.height || 220;
  const viewportWidth = document.documentElement.clientWidth || window.innerWidth;
  const viewportHeight = document.documentElement.clientHeight || window.innerHeight;
  const anchorLeft = wrapRect.left + startRect.left;
  const anchorTop = wrapRect.top + startRect.top + (startRect.height || sourceEditorLineHeight());
  let left = anchorLeft;
  let top = anchorTop + gap;
  if (top + height > viewportHeight - margin) {
    top = wrapRect.top + startRect.top - height - gap;
  }
  left = Math.max(margin, Math.min(left, viewportWidth - width - margin));
  top = Math.max(margin, Math.min(top, viewportHeight - height - margin));
  sourceColorPopover.style.left = `${Math.round(left)}px`;
  sourceColorPopover.style.top = `${Math.round(top)}px`;
}

function sourceColorEventTargetsToken(event, token, visualOffset = null) {
  if (!event || !token) {
    return true;
  }
  const offset = Number.isInteger(visualOffset)
    ? visualOffset
    : sourceViewOffsetFromVisualPoint(event.clientX, event.clientY);
  if (!Number.isInteger(offset)) {
    return true;
  }
  return offset >= token.start && offset <= token.end;
}

function hideSourceColorEditor() {
  sourceColorEdit = null;
  if (sourceColorPopover) {
    sourceColorPopover.hidden = true;
    sourceColorPopover.replaceChildren();
  }
}

function hideSourceColorEditorForOutsidePointer(event) {
  if (!sourceColorEdit || sourceColorPopover?.hidden) {
    return;
  }
  const target = event.target;
  if (sourceColorPopover?.contains(target) || sourceEditorWrap?.contains(target)) {
    return;
  }
  hideSourceColorEditor();
}

function sourceColorSelectionTargetsToken(token) {
  if (!token) {
    return false;
  }
  const start = sourceEditor.selectionStart || 0;
  const end = sourceEditor.selectionEnd || start;
  return start >= token.start && end <= token.end;
}

function applySourceColorRgb(rgb) {
  if (!sourceColorEdit) {
    return;
  }
  const current = hexColorAt(sourceEditor.value, sourceColorEdit.start);
  if (!current || current.start !== sourceColorEdit.start) {
    hideSourceColorEditor();
    return;
  }
  const parsedColor = parseHexColorToken(rgb);
  if (!parsedColor) {
    return;
  }
  const next = formatHexColorToken(parsedColor.rgb, parsedColor.alpha);
  sourceEditor.setRangeText(next, current.start, current.end, "preserve");
  sourceColorEdit = { start: current.start, end: current.start + next.length, value: next };
  sourceEditor.setSelectionRange(sourceColorEdit.start, sourceColorEdit.end);
  recordSourceUndoSnapshot();
  const parsedNext = parseHexColorToken(next);
  sourceColorPopover?.querySelector(".color-editor")?.syncColor?.(parsedNext ? next : rgb);
  positionSourceColorPopoverForToken(sourceColorEdit);
  updateSourceMeta();
  if (documents[currentDocumentIndex]) {
    documents[currentDocumentIndex].source = sourceEditorDocumentValue();
  }
  if (!sourceDocumentSupportsEditableTargets()) {
    resetSourcePuzzleAnalysisState();
    scheduleLocalSave();
    return;
  }
  scheduleSourceHighlight(true);
  scheduleLocalSave();
  resetLevelBuilderFromSource(false);
  schedulePreview();
}

function refreshSourceColorEditor() {
  if (!sourceColorEdit) {
    return;
  }
  const current = hexColorAt(sourceEditor.value, sourceColorEdit.start);
  if (!current || current.start !== sourceColorEdit.start) {
    hideSourceColorEditor();
    return;
  }
  if (document.activeElement === sourceEditor && !sourceColorSelectionTargetsToken(current)) {
    hideSourceColorEditor();
  }
}

function hexColorAt(source, index) {
  const start = Math.max(0, Math.min(index, source.length));
  const lineStart = source.lastIndexOf("\n", start - 1) + 1;
  const nextLine = source.indexOf("\n", start);
  const lineEnd = nextLine >= 0 ? nextLine : source.length;
  const line = source.slice(lineStart, lineEnd);
  const pattern = /#[0-9a-fA-F]{3}(?:[0-9a-fA-F]{1})?(?:[0-9a-fA-F]{2})?(?:[0-9a-fA-F]{2})?(?![_a-zA-Z0-9])/g;
  for (const match of line.matchAll(pattern)) {
    const value = match[0];
    if (![4, 5, 7, 9].includes(value.length)) {
      continue;
    }
    const tokenStart = lineStart + match.index;
    const tokenEnd = tokenStart + value.length;
    if (start >= tokenStart && start <= tokenEnd) {
      return { start: tokenStart, end: tokenEnd, value };
    }
  }
  return null;
}

function parseHexColorToken(token) {
  const hex = String(token || "").trim();
  if (!/^#[0-9a-fA-F]{3,4}$|^#[0-9a-fA-F]{6}$|^#[0-9a-fA-F]{8}$/.test(hex)) {
    return null;
  }
  const body = hex.slice(1);
  if (body.length === 3 || body.length === 4) {
    const r = body[0] + body[0];
    const g = body[1] + body[1];
    const b = body[2] + body[2];
    const a = body.length === 4 ? body[3] + body[3] : "ff";
    return { rgb: `#${r}${g}${b}`.toLowerCase(), alpha: parseInt(a, 16) };
  }
  return {
    rgb: `#${body.slice(0, 6)}`.toLowerCase(),
    alpha: body.length === 8 ? parseInt(body.slice(6, 8), 16) : 255,
  };
}

function formatHexColorToken(rgb, alpha) {
  const color = /^#[0-9a-fA-F]{6}$/.test(rgb) ? rgb.toLowerCase() : "#000000";
  if (alpha >= 255) {
    return color;
  }
  return `${color}${alpha.toString(16).padStart(2, "0")}`;
}

function handleSourceBeforeInputTextInsert(event) {
  if (!isTextDocument(documents[currentDocumentIndex])) {
    return;
  }
  if (sourceEditor.sourceEditorPort?.kind === "codemirror") {
    return;
  }
  if (sourceFoldsActive() && event.inputType !== "historyUndo" && event.inputType !== "historyRedo") {
    captureSourceFoldEditSnapshot();
  }
  ensureSourceUndoHistory();
  if (event.inputType === "historyUndo" || event.inputType === "historyRedo") {
    event.preventDefault();
    if (event.inputType === "historyUndo") {
      undoSourceEdit();
    } else {
      redoSourceEdit();
    }
    return;
  }
  const predicted = sourceDocumentSupportsEditableTargets()
    ? sourcePredictedBeforeInputValue(event)
    : null;
  if (predicted !== null) {
    if (event.isComposing || event.inputType === "insertCompositionText") {
      beginSourceCompositionPreview(predicted);
    } else {
      scheduleOptimisticSourceHighlight(predicted);
    }
  }
}

function applySourceAnalysisEditorChanges(changes, source) {
  if (!Array.isArray(changes)) {
    throw new Error("CodeMirror edit must provide source analysis changes.");
  }
  window.PuzzleStudioRuntime
    .applySourceAnalysisEdits(changes, source)
    .catch((error) => console.error("Source analysis edit failed", error));
}

function bindSourceEditorEvents() {
sourceEditor.addEventListener("beforeinput", handleSourceBeforeInputTextInsert);
sourceEditor.addEventListener("sourceanalysisreset", () => {
  const source = sourceEditorDocumentValue();
  window.PuzzleStudioRuntime.resetSourceAnalysis(source).catch((error) => {
    console.error("Source analysis reset failed", error);
  });
});
sourceEditor.addEventListener("sourceanalysisedit", (event) => {
  applySourceAnalysisEditorChanges(event.detail?.changes, event.detail?.source);
});
sourceEditor.addEventListener("compositionstart", () => {
  sourceCompositionRange = {
    start: sourceEditor.selectionStart || 0,
    end: sourceEditor.selectionEnd || sourceEditor.selectionStart || 0,
  };
});
sourceEditor.addEventListener("compositionupdate", (event) => {
  if (!isTextDocument(documents[currentDocumentIndex])) {
    return;
  }
  if (!sourceDocumentSupportsEditableTargets()) {
    return;
  }
  beginSourceCompositionPreview(sourceCompositionPreviewValue(event.data));
});
sourceEditor.addEventListener("input", (event) => {
  if (!isTextDocument(documents[currentDocumentIndex])) {
    return;
  }
  const sourceChanges = event.detail?.changes;
  const editedSource = sourceEditorDocumentValue();
  applySourceAnalysisEditorChanges(sourceChanges, editedSource);
  if (typeof refreshSurfaceEntriesForActiveSource === "function") {
    void refreshSurfaceEntriesForActiveSource(editedSource).catch((error) => {
      console.error("Source entries refresh failed", error);
    });
  }
  if (sourceFoldsActive()) {
    const changed = commitSourceFoldedDisplayEdit();
    if (!changed) {
      clearSourceCompositionPreview();
      if (sourceDocumentSupportsEditableTargets()) {
        scheduleSourceHighlight(true, { preserveCurrent: false });
      } else {
        resetSourcePuzzleAnalysisState();
      }
      updateSourceMeta();
      return;
    }
  }
  clearSourceCompositionPreview();
  const puzzleSource = sourceDocumentSupportsEditableTargets();
  if (puzzleSource) {
    scheduleSourceHighlight();
    scheduleSourceOutlineRefresh();
  } else {
    resetSourcePuzzleAnalysisState();
  }
  hideSourceImportLinkFrame();
  clearSourceBlockSelection();
  sourceEditorPreferredCaretX = null;
  recordSourceUndoSnapshot();
  updateSourceMeta();
  if (puzzleSource) {
    refreshSourceColorEditor();
  }
  refreshSourceFindAfterSourceChange();
  if (documents[currentDocumentIndex]) {
    documents[currentDocumentIndex].source = sourceEditorDocumentValue();
  }
  scheduleLocalSave();
  if (puzzleSource) {
    scheduleSourceCompletion();
    scheduleLevelBuilderResetFromSource(false);
    scheduleSourceCursorPreviewSync();
    schedulePreview();
  }
});
sourceEditor.addEventListener("sourceviewportchange", () => {
  if (sourceDocumentSupportsEditableTargets()) {
    scheduleSourceHighlight(true);
  }
});
sourceEditor.addEventListener("compositionend", () => {
  const previewSource = sourceCompositionPreviewSource;
  sourceCompositionRange = null;
  if (!sourceDocumentSupportsEditableTargets()) {
    clearSourceCompositionPreview();
    return;
  }
  window.requestAnimationFrame(() => {
    if (sourceCompositionPreviewSource === previewSource) {
      clearSourceCompositionPreview();
      scheduleSourceHighlight();
    }
  });
});
sourceEditor.addEventListener("click", (event) => {
  sourceEditorPreferredCaretX = null;
  const interaction = sourceInteractionFromPointer(event);
  if (!interaction) {
    return;
  }
  if (openSourceImportLinkFromPointer(event, interaction.position)) {
    return;
  }
  if (sourceDocumentSupportsEditableTargets()) {
    showSourceColorEditor(event, interaction.viewOffset);
    window.setTimeout(() => showSourceCompletions({ manual: false }), 0);
    syncPreviewModeFromSourceCursor({
      recordHistory: true,
      allowInactiveMode: true,
      position: interaction.documentOffset,
    });
  }
});
sourceEditor.addEventListener("pointerdown", handleSourceBlockSelectionPointerDown);
sourceEditor.addEventListener("mouseleave", handleSourceImportEditorMouseLeave);
sourceEditor.addEventListener("pointermove", updateSourceBlockSelectionDrag);
sourceEditor.addEventListener("pointerup", finishSourceBlockSelectionDrag);
sourceEditor.addEventListener("pointercancel", finishSourceBlockSelectionDrag);
document.addEventListener("pointerup", endSourceNativeSelectionDrag);
document.addEventListener("pointercancel", endSourceNativeSelectionDrag);
sourceEditor.addEventListener("keyup", (event) => {
  if (event.key === "Escape") {
    hideSourceColorEditor();
    hideSourceCompletions();
    return;
  }
  if (
    (event.key === "ArrowDown" || event.key === "ArrowUp")
    && sourceCompletionState
    && !sourceCompletionPopover?.hidden
    && sourceCompletionMatchesCurrentCursor()
  ) {
    renderSourceBlockSelection();
    return;
  }
  if (event.key.startsWith("Arrow") || event.key === "Home" || event.key === "End") {
    if (sourceDocumentSupportsEditableTargets()) {
      showSourceColorEditor();
      showSourceCompletions({ manual: false });
      scheduleSourceCursorPreviewSync();
    }
  }
  renderSourceBlockSelection();
});
sourceEditor.addEventListener("focus", () => {
  renderSourceBlockSelection();
  if (sourceDocumentSupportsEditableTargets()) {
    scheduleSourceCursorPreviewSync();
  }
});
sourceEditor.addEventListener("blur", () => {
  clearSourceCompositionPreview();
  endSourceNativeSelectionDrag();
  renderSourceBlockSelection();
});
document.addEventListener("selectionchange", () => {
  if (document.activeElement !== sourceEditor) {
    renderSourceBlockSelection();
    return;
  }
  if (sourceDocumentSupportsEditableTargets()) {
    scheduleSourceCursorPreviewSync();
    syncSourceOutlineActiveItem();
  }
  syncSourceFindIndexFromSelection();
  renderSourceBlockSelection();
});
document.addEventListener("keydown", (event) => {
  if (event.defaultPrevented) {
    return;
  }
  if (handleSourceFindShortcut(event)) {
    return;
  }
  handleSourceFindMoveShortcut(event);
}, true);
sourceFindPanel?.addEventListener("mousedown", (event) => {
  if (event.target.closest("button")) {
    event.preventDefault();
  }
  event.stopPropagation();
});
sourceFindInput?.addEventListener("input", () => syncSourceFindMatches({ anchor: sourceEditor.selectionStart }));
sourceFindInput?.addEventListener("keydown", (event) => {
  if (event.key === "Escape") {
    event.preventDefault();
    closeSourceFindPanel();
    return;
  }
  if (event.key === "Enter") {
    event.preventDefault();
    moveSourceFindSelection(event.shiftKey ? -1 : 1);
  }
});
sourceReplaceInput?.addEventListener("keydown", (event) => {
  if (event.key === "Escape") {
    event.preventDefault();
    closeSourceFindPanel();
    return;
  }
  if (event.key === "Enter") {
    event.preventDefault();
    replaceCurrentSourceFindMatch();
  }
});
sourceFindCaseButton?.addEventListener("click", () => {
  sourceFindState.matchCase = !sourceFindState.matchCase;
  sourceFindCaseButton.classList.toggle("is-active", sourceFindState.matchCase);
  sourceFindCaseButton.setAttribute("aria-pressed", String(sourceFindState.matchCase));
  syncSourceFindMatches({ anchor: sourceEditor.selectionStart });
});
sourceFindPanel?.querySelector("[data-source-find-previous]")?.addEventListener("click", () => moveSourceFindSelection(-1));
sourceFindPanel?.querySelector("[data-source-find-next]")?.addEventListener("click", () => moveSourceFindSelection(1));
sourceFindPanel?.querySelector("[data-source-find-close]")?.addEventListener("click", () => closeSourceFindPanel());
sourceFindPanel?.querySelector("[data-source-replace-current]")?.addEventListener("click", replaceCurrentSourceFindMatch);
sourceFindPanel?.querySelector("[data-source-replace-all]")?.addEventListener("click", replaceAllSourceFindMatches);
}

function handleSourceEditorEmacsBinding(event) {
  if (event.metaKey || !(event.ctrlKey || event.altKey)) {
    return false;
  }

  const key = event.key.length === 1 ? event.key.toLowerCase() : event.key;
  const ctrl = event.ctrlKey && !event.altKey;
  const alt = event.altKey && !event.ctrlKey;

  if (ctrl) {
    if (key === "a") {
      moveSourceSelection(sourceLineStart(sourceSelectionFocus()), event.shiftKey);
      event.preventDefault();
      event.stopPropagation();
      return true;
    }
    if (key === "e") {
      moveSourceSelection(sourceLineEnd(sourceSelectionFocus()), event.shiftKey);
      event.preventDefault();
      event.stopPropagation();
      return true;
    }
    if (key === "f") {
      moveSourceSelection(sourceForwardCharPosition(), event.shiftKey);
      event.preventDefault();
      event.stopPropagation();
      return true;
    }
    if (key === "b") {
      moveSourceSelection(sourceBackwardCharPosition(), event.shiftKey);
      event.preventDefault();
      event.stopPropagation();
      return true;
    }
    if (key === "n") {
      moveSourceSelection(sourceVerticalPosition(1), event.shiftKey);
      event.preventDefault();
      event.stopPropagation();
      return true;
    }
    if (key === "p") {
      moveSourceSelection(sourceVerticalPosition(-1), event.shiftKey);
      event.preventDefault();
      event.stopPropagation();
      return true;
    }
    if (key === "d") {
      deleteSourceRange(sourceEditor.selectionStart, sourceEditor.selectionEnd, sourceForwardCharPosition());
      event.preventDefault();
      event.stopPropagation();
      return true;
    }
    if (key === "h") {
      deleteSourceRange(sourceEditor.selectionStart, sourceEditor.selectionEnd, sourceBackwardCharPosition());
      event.preventDefault();
      event.stopPropagation();
      return true;
    }
    if (key === "k") {
      killSourceLineEnd();
      event.preventDefault();
      event.stopPropagation();
      return true;
    }
    if (key === "y") {
      if (sourceEditorKillRing) {
        insertAtSelection(sourceEditorKillRing);
      }
      event.preventDefault();
      event.stopPropagation();
      return true;
    }
    if (key === "m" || key === "j") {
      insertSourceNewlineAtSelection();
      event.preventDefault();
      event.stopPropagation();
      return true;
    }
    if (key === "g") {
      hideSourceColorEditor();
      hideSourceCompletions();
      event.preventDefault();
      event.stopPropagation();
      return true;
    }
    if (key === "v") {
      moveSourceSelection(sourcePagePosition(1), event.shiftKey);
      event.preventDefault();
      event.stopPropagation();
      return true;
    }
  }

  if (alt) {
    if (key === "f") {
      moveSourceSelection(sourceWordPosition(1), event.shiftKey);
      event.preventDefault();
      event.stopPropagation();
      return true;
    }
    if (key === "b") {
      moveSourceSelection(sourceWordPosition(-1), event.shiftKey);
      event.preventDefault();
      event.stopPropagation();
      return true;
    }
    if (key === "d") {
      deleteSourceRange(sourceEditor.selectionStart, sourceEditor.selectionEnd, sourceWordPosition(1));
      event.preventDefault();
      event.stopPropagation();
      return true;
    }
    if (key === "v") {
      moveSourceSelection(sourcePagePosition(-1), event.shiftKey);
      event.preventDefault();
      event.stopPropagation();
      return true;
    }
  }

  return false;
}

function sourceSelectionFocus() {
  return sourceEditor.selectionDirection === "backward"
    ? sourceEditor.selectionStart
    : sourceEditor.selectionEnd;
}

function sourceSelectionAnchor() {
  return sourceEditor.selectionDirection === "backward"
    ? sourceEditor.selectionEnd
    : sourceEditor.selectionStart;
}

function moveSourceSelection(position, extend = false) {
  const next = clampSourcePosition(position);
  if (extend) {
    const anchor = sourceSelectionAnchor();
    sourceEditor.setSelectionRange(
      Math.min(anchor, next),
      Math.max(anchor, next),
      next < anchor ? "backward" : "forward",
    );
  } else {
    sourceEditor.setSelectionRange(next, next);
  }
  updateSourceMeta();
  showSourceColorEditor();
  showSourceCompletions({ manual: false });
}

function handleSourceEditorArrowNavigation(event) {
  if (event.altKey || event.ctrlKey || event.metaKey || sourceEditorBlockSelection) {
    return false;
  }
  if (!["ArrowUp", "ArrowDown", "Home", "End"].includes(event.key)) {
    sourceEditorPreferredCaretX = null;
    return false;
  }
  event.preventDefault();
  event.stopPropagation();
  if (event.key === "Home" || event.key === "End") {
    sourceEditorPreferredCaretX = null;
    moveSourceSelection(
      event.key === "Home"
        ? sourceLineStart(sourceSelectionFocus())
        : sourceLineEnd(sourceSelectionFocus()),
      event.shiftKey,
    );
    return true;
  }
  const current = sourceCaretRectForOffset(sourceSelectionFocus());
  if (!current || !sourceEditorWrap) {
    moveSourceSelection(sourceVerticalPosition(event.key === "ArrowDown" ? 1 : -1), event.shiftKey);
    return true;
  }
  const wrapRect = sourceEditorWrap.getBoundingClientRect();
  const lineHeight = sourceEditorLineHeight();
  if (!Number.isFinite(sourceEditorPreferredCaretX)) {
    sourceEditorPreferredCaretX = current.left;
  }
  const targetClientX = wrapRect.left + sourceEditorPreferredCaretX;
  const targetClientY = wrapRect.top + current.top + (event.key === "ArrowDown" ? lineHeight + 1 : -1);
  const next = sourceViewOffsetFromVisualPoint(targetClientX, targetClientY);
  if (Number.isInteger(next)) {
    moveSourceSelection(next, event.shiftKey);
  } else {
    moveSourceSelection(sourceVerticalPosition(event.key === "ArrowDown" ? 1 : -1), event.shiftKey);
  }
  return true;
}

function clampSourcePosition(position) {
  return Math.max(0, Math.min(sourceEditor.value.length, position));
}

function sourceLineStart(position) {
  return sourceEditor.value.lastIndexOf("\n", clampSourcePosition(position) - 1) + 1;
}

function sourceLineEnd(position) {
  const source = sourceEditor.value;
  const lineEnd = source.indexOf("\n", clampSourcePosition(position));
  return lineEnd < 0 ? source.length : lineEnd;
}

function sourceForwardCharPosition() {
  if (sourceEditor.selectionStart !== sourceEditor.selectionEnd) {
    return sourceEditor.selectionEnd;
  }
  return clampSourcePosition(sourceEditor.selectionEnd + 1);
}

function sourceBackwardCharPosition() {
  if (sourceEditor.selectionStart !== sourceEditor.selectionEnd) {
    return sourceEditor.selectionStart;
  }
  return clampSourcePosition(sourceEditor.selectionStart - 1);
}

function sourceVerticalPosition(delta) {
  const source = sourceEditor.value;
  const position = sourceSelectionFocus();
  const currentLineStart = sourceLineStart(position);
  const column = position - currentLineStart;

  if (delta > 0) {
    const currentLineEnd = sourceLineEnd(position);
    if (currentLineEnd >= source.length) {
      return source.length;
    }
    const nextLineStart = currentLineEnd + 1;
    return Math.min(nextLineStart + column, sourceLineEnd(nextLineStart));
  }

  if (currentLineStart <= 0) {
    return 0;
  }
  const previousLineEnd = currentLineStart - 1;
  const previousLineStart = sourceLineStart(previousLineEnd);
  return Math.min(previousLineStart + column, previousLineEnd);
}

function sourcePagePosition(delta) {
  const lineHeight = sourceEditorLineHeight();
  const lines = Math.max(1, Math.floor(sourceViewportHeight() / lineHeight) - 1);
  let position = sourceSelectionFocus();
  for (let step = 0; step < lines; step += 1) {
    const next = sourceVerticalPositionFrom(position, delta);
    if (next === position) {
      break;
    }
    position = next;
  }
  return position;
}

function sourceVerticalPositionFrom(position, delta) {
  const savedStart = sourceEditor.selectionStart;
  const savedEnd = sourceEditor.selectionEnd;
  const savedDirection = sourceEditor.selectionDirection;
  sourceEditor.setSelectionRange(position, position);
  const next = sourceVerticalPosition(delta);
  sourceEditor.setSelectionRange(savedStart, savedEnd, savedDirection);
  return next;
}

function sourceEditorLineHeight() {
  const lineHeight = Number.parseFloat(getComputedStyle(sourceEditor).lineHeight);
  return Number.isFinite(lineHeight) && lineHeight > 0 ? lineHeight : 18;
}

function sourceWordPosition(delta) {
  const source = sourceEditor.value;
  let position = sourceSelectionFocus();
  if (sourceEditor.selectionStart !== sourceEditor.selectionEnd) {
    return delta > 0 ? sourceEditor.selectionEnd : sourceEditor.selectionStart;
  }

  if (delta > 0) {
    while (position < source.length && !isSourceWordChar(source[position])) {
      position += 1;
    }
    while (position < source.length && isSourceWordChar(source[position])) {
      position += 1;
    }
    return position;
  }

  position -= 1;
  while (position > 0 && !isSourceWordChar(source[position])) {
    position -= 1;
  }
  while (position > 0 && isSourceWordChar(source[position - 1])) {
    position -= 1;
  }
  return clampSourcePosition(position);
}

function isSourceWordChar(char) {
  return /[A-Za-z0-9_]/.test(char || "");
}

function deleteSourceRange(selectionStart, selectionEnd, fallbackPosition) {
  const start = Math.min(selectionStart, selectionEnd, fallbackPosition);
  const end = Math.max(selectionStart, selectionEnd, fallbackPosition);
  if (start === end) {
    return;
  }
  sourceEditor.setRangeText("", start, end, "start");
  sourceEditorContentChanged();
}

function handleSourceIndentBackspace(event) {
  if (event.key !== "Backspace" || event.altKey || event.ctrlKey || event.metaKey || event.isComposing) {
    return false;
  }
  const start = sourceEditor.selectionStart;
  const end = sourceEditor.selectionEnd;
  if (start !== end) {
    return false;
  }
  const source = sourceEditor.value;
  const lineStart = sourceLineStart(start);
  const linePrefix = source.slice(lineStart, start);
  if (!linePrefix || !/^[\t ]+$/.test(linePrefix)) {
    return false;
  }

  let removeStart = start;
  if (source[start - 1] === "\t") {
    removeStart = start - 1;
  } else if (source[start - 1] === " ") {
    const column = sourceIndentColumn(linePrefix);
    const targetColumn = Math.max(0, column - (column % 4 || 4));
    const removeCount = Math.max(1, Math.min(sourceTrailingSpaceCount(linePrefix), column - targetColumn));
    removeStart = start - removeCount;
  } else {
    return false;
  }

  event.preventDefault();
  event.stopPropagation();
  sourceEditor.setRangeText("", removeStart, start, "start");
  sourceEditorContentChanged();
  return true;
}

function sourceIndentColumn(indent) {
  let column = 0;
  for (const char of indent) {
    if (char === "\t") {
      const offset = column % 4;
      column += offset === 0 ? 4 : 4 - offset;
    } else {
      column += 1;
    }
  }
  return column;
}

function sourceTrailingSpaceCount(value) {
  return value.match(/ *$/)?.[0].length || 0;
}

function killSourceLineEnd() {
  const start = sourceEditor.selectionStart;
  const end = sourceEditor.selectionEnd;
  if (start !== end) {
    sourceEditorKillRing = sourceEditor.value.slice(start, end);
    sourceEditor.setRangeText("", start, end, "start");
    sourceEditorContentChanged();
    return;
  }

  const source = sourceEditor.value;
  const lineEnd = sourceLineEnd(start);
  const killEnd = lineEnd > start ? lineEnd : Math.min(source.length, lineEnd + 1);
  if (killEnd === start) {
    return;
  }
  sourceEditorKillRing = source.slice(start, killEnd);
  sourceEditor.setRangeText("", start, killEnd, "start");
  sourceEditorContentChanged();
}

function handleSourceBraceAssist(event) {
  if (event.altKey || event.ctrlKey || event.metaKey || event.isComposing) {
    return false;
  }

  if (event.key === "{") {
    event.preventDefault();
    event.stopPropagation();
    insertSourceBracePair();
    return true;
  }

  if (event.key === "}") {
    return handleSourceClosingBrace(event);
  }

  if (event.key === "Backspace") {
    return handleSourceBraceBackspace(event);
  }

  return false;
}

function handleSourceRewriteLhsBracketAssist(event) {
  if (event.altKey || event.ctrlKey || event.metaKey || event.isComposing || event.key !== "[") {
    return false;
  }
  const start = sourceEditor.selectionStart;
  const end = sourceEditor.selectionEnd;
  const source = sourceEditor.value || "";
  if (source.slice(start, end).includes("\n")) {
    return false;
  }
  const lineStart = source.lastIndexOf("\n", start - 1) + 1;
  const lineEnd = source.indexOf("\n", end);
  const safeLineEnd = lineEnd < 0 ? source.length : lineEnd;
  const lineBeforeSelection = source.slice(lineStart, start);
  const lineAfterSelection = source.slice(end, safeLineEnd);
  const codeBeforeSelection = stripSourceImportLineComment(lineBeforeSelection);
  const codeAfterSelection = stripSourceImportLineComment(lineAfterSelection);
  if (codeBeforeSelection.length !== lineBeforeSelection.length) {
    return false;
  }
  if (codeBeforeSelection.includes("->")) {
    return false;
  }
  const arrowAfterSelection = codeAfterSelection.indexOf("->");
  if (arrowAfterSelection >= 0 && codeAfterSelection.slice(0, arrowAfterSelection).includes("[")) {
    return false;
  }
  if (!/(^|[\t ])$/.test(codeBeforeSelection)) {
    return false;
  }

  event.preventDefault();
  event.stopPropagation();
  insertSourceRewritePatternCell(start, end);
  return true;
}

function insertSourceRewritePatternCell(start, end) {
  clearSourceBlockSelection();
  const selection = sourceEditor.value.slice(start, end);
  const replacement = `[ ${selection} ]`;
  sourceEditor.setRangeText(replacement, start, end, "end");
  const innerStart = start + 2;
  const innerEnd = innerStart + selection.length;
  sourceEditor.setSelectionRange(innerStart, innerEnd, sourceEditor.selectionDirection || "none");
  sourceEditorContentChanged();
}

function insertSourceBracePair() {
  const start = sourceEditor.selectionStart;
  const end = sourceEditor.selectionEnd;
  const selection = sourceEditor.value.slice(start, end);
  sourceEditor.setRangeText(`{${selection}}`, start, end, "end");
  const innerStart = start + 1;
  const innerEnd = innerStart + selection.length;
  sourceEditor.setSelectionRange(innerStart, innerEnd, sourceEditor.selectionDirection || "none");
  sourceEditorContentChanged();
}

function handleSourceClosingBrace(event) {
  const start = sourceEditor.selectionStart;
  const end = sourceEditor.selectionEnd;
  const source = sourceEditor.value;
  if (start === end && source[start] === "}") {
    event.preventDefault();
    event.stopPropagation();
    sourceEditor.setSelectionRange(start + 1, start + 1);
    return true;
  }

  const lineStart = source.lastIndexOf("\n", start - 1) + 1;
  const linePrefix = source.slice(lineStart, start);
  if (start === end && /^[\t ]+$/.test(linePrefix)) {
    const indentStart = sourceClosingBraceIndentStart(lineStart, start, linePrefix);
    event.preventDefault();
    event.stopPropagation();
    sourceEditor.setRangeText("}", indentStart, end, "end");
    sourceEditorContentChanged();
    return true;
  }

  return false;
}

function sourceClosingBraceIndentStart(lineStart, cursor, linePrefix) {
  if (linePrefix.endsWith("\t")) {
    return cursor - 1;
  }
  const spaces = linePrefix.match(/ {1,2}$/)?.[0] || "";
  return spaces ? cursor - spaces.length : lineStart;
}

function handleSourceBraceBackspace(event) {
  const start = sourceEditor.selectionStart;
  const end = sourceEditor.selectionEnd;
  const source = sourceEditor.value;
  if (start !== end || source[start - 1] !== "{" || source[start] !== "}") {
    return false;
  }
  event.preventDefault();
  event.stopPropagation();
  sourceEditor.setRangeText("", start - 1, start + 1, "start");
  sourceEditorContentChanged();
  return true;
}

function handleSourceRewriteRhsPatternAssist(event) {
  if (event.altKey || event.ctrlKey || event.metaKey || event.isComposing || event.key !== "[") {
    return false;
  }
  const cursor = sourceEditor.selectionStart;
  if (cursor !== sourceEditor.selectionEnd) {
    return false;
  }
  const source = sourceEditor.value || "";
  const lineStart = source.lastIndexOf("\n", cursor - 1) + 1;
  const lineEnd = source.indexOf("\n", cursor);
  const safeLineEnd = lineEnd < 0 ? source.length : lineEnd;
  const line = source.slice(lineStart, safeLineEnd);
  const cursorColumn = cursor - lineStart;
  const code = stripSourceImportLineComment(line);
  if (cursorColumn > code.length) {
    return false;
  }
  const statementBounds = sourceRewriteStatementBounds(code, cursorColumn);
  const codeBeforeCursor = code.slice(statementBounds.start, cursorColumn);
  const arrow = codeBeforeCursor.lastIndexOf("->");
  if (arrow < 0 || !/^[\t ]*$/.test(codeBeforeCursor.slice(arrow + 2))) {
    return false;
  }
  const statementEnd = statementBounds.end;
  if (code.slice(cursorColumn, statementEnd).trim()) {
    return false;
  }
  const pattern = sourceRewritePatternBeforeArrow(codeBeforeCursor.slice(0, arrow));
  if (!pattern) {
    return false;
  }
  event.preventDefault();
  event.stopPropagation();
  clearSourceBlockSelection();
  const rhsPattern = sourceEmptyRewritePattern(pattern);
  const firstSlot = sourceRewritePatternSlotOffsets(rhsPattern)[0];
  sourceEditor.setRangeText(rhsPattern, cursor, cursor, "end");
  if (Number.isInteger(firstSlot)) {
    const slot = cursor + firstSlot;
    sourceEditor.setSelectionRange(slot, slot);
  }
  sourceEditorContentChanged();
  return true;
}

function sourceRewriteStatementBounds(code, cursorColumn) {
  let start = 0;
  let squareDepth = 0;
  let parenDepth = 0;
  for (let index = 0; index < code.length; index += 1) {
    const char = code[index];
    if (char === "[") {
      squareDepth += 1;
    } else if (char === "]") {
      squareDepth = Math.max(0, squareDepth - 1);
    } else if (char === "(") {
      parenDepth += 1;
    } else if (char === ")") {
      parenDepth = Math.max(0, parenDepth - 1);
    } else if (char === ";" && squareDepth === 0 && parenDepth === 0) {
      if (index < cursorColumn) {
        start = index + 1;
      } else {
        return { start, end: index };
      }
    }
  }
  return { start, end: code.length };
}

function sourceRewritePatternBeforeArrow(lineBeforeArrow) {
  const line = String(lineBeforeArrow || "").replace(/[ \t]+$/, "");
  if (!line.endsWith("]")) {
    return "";
  }
  const end = line.length;
  let scanEnd = end;
  let patternStart = -1;
  while (scanEnd > 0 && line[scanEnd - 1] === "]") {
    const open = sourceMatchingPatternOpen(line, scanEnd - 1);
    if (open < 0) {
      break;
    }
    patternStart = open;
    const before = line.slice(0, open);
    const gap = before.match(/[ \t]*$/)?.[0] || "";
    scanEnd = open - gap.length;
  }
  return patternStart >= 0 ? line.slice(patternStart, end) : "";
}

function sourceMatchingPatternOpen(text, closeIndex) {
  let depth = 0;
  for (let index = closeIndex; index >= 0; index -= 1) {
    if (text[index] === "]") {
      depth += 1;
    } else if (text[index] === "[") {
      depth -= 1;
      if (depth === 0) {
        return index;
      }
    }
  }
  return -1;
}

function sourcePatternCellSeparator(char) {
  return char === "|" || char === ";";
}

function sourceEmptyRewritePattern(pattern) {
  return String(pattern || "").replace(/\[[^\]\[]*\]/g, (cell) => {
    const body = cell.slice(1, -1);
    const separators = Array.from(body).filter(sourcePatternCellSeparator);
    if (!separators.length) {
      return "[  ]";
    }
    const emptyBody = separators.map((separator) => separator === "|" ? " | " : ";").join("");
    return `[ ${emptyBody} ]`;
  });
}

function sourceRewritePatternSlotOffsets(pattern) {
  const slots = [];
  const text = String(pattern || "");
  const cellPattern = /\[[^\]\[]*\]/g;
  for (const match of text.matchAll(cellPattern)) {
    const cell = match[0];
    const body = cell.slice(1, -1);
    let segmentStart = 0;
    const pushSegment = (segmentEnd) => {
      const segment = body.slice(segmentStart, segmentEnd);
      if (/^[\t ]*$/.test(segment)) {
        slots.push((match.index || 0) + 1 + segmentStart + Math.ceil(segment.length / 2));
      }
      segmentStart = segmentEnd + 1;
    };
    for (let index = 0; index <= body.length; index += 1) {
      if (index === body.length || sourcePatternCellSeparator(body[index])) {
        pushSegment(index);
      }
    }
  }
  return slots;
}

function sourceRuleBracketCellSlots(source, cursor) {
  const lineStart = source.lastIndexOf("\n", cursor - 1) + 1;
  const lineEnd = source.indexOf("\n", cursor);
  const safeLineEnd = lineEnd < 0 ? source.length : lineEnd;
  const lineBeforeCursor = source.slice(lineStart, cursor);
  if (stripSourceImportLineComment(lineBeforeCursor).length !== lineBeforeCursor.length) {
    return null;
  }
  const open = source.lastIndexOf("[", cursor - 1);
  const close = source.indexOf("]", cursor);
  if (open < lineStart || close < 0 || close > safeLineEnd || cursor <= open || cursor >= close) {
    return null;
  }
  const body = source.slice(open + 1, close);
  if (body.includes("[") || body.includes("]")) {
    return null;
  }
  const slots = [];
  let segmentStart = 0;
  const pushSegment = (segmentEnd) => {
    const segment = body.slice(segmentStart, segmentEnd);
    if (/^[\t ]*$/.test(segment)) {
      slots.push({
        start: open + 1 + segmentStart,
        end: open + 1 + segmentEnd,
        cursor: open + 1 + segmentStart + Math.ceil(segment.length / 2),
      });
    }
    segmentStart = segmentEnd + 1;
  };
  for (let index = 0; index <= body.length; index += 1) {
    if (index === body.length || sourcePatternCellSeparator(body[index])) {
      pushSegment(index);
    }
  }
  return slots;
}

function handleSourceRuleBracketCellSlotTab(event) {
  if (
    event.key !== "Tab"
    || event.altKey
    || event.ctrlKey
    || event.metaKey
    || event.isComposing
    || sourceEditor.selectionStart !== sourceEditor.selectionEnd
  ) {
    return false;
  }
  const source = sourceEditor.value || "";
  const cursor = sourceEditor.selectionStart;
  const slots = sourceRuleBracketCellSlots(source, cursor);
  if (!slots || slots.length < 2) {
    return false;
  }
  const currentIndex = slots.findIndex((slot) => cursor >= slot.start && cursor <= slot.end);
  if (currentIndex < 0) {
    return false;
  }
  const targetIndex = event.shiftKey
    ? (currentIndex + slots.length - 1) % slots.length
    : (currentIndex + 1) % slots.length;
  const target = slots[targetIndex]?.cursor;
  if (!Number.isInteger(target)) {
    return false;
  }
  event.preventDefault();
  event.stopPropagation();
  sourceEditor.setSelectionRange(target, target);
  updateSourceMeta();
  hideSourceCompletions();
  return true;
}

function handleSourceRuleBracketCellTabExit(event) {
  if (
    event.key !== "Tab"
    || event.shiftKey
    || event.altKey
    || event.ctrlKey
    || event.metaKey
    || event.isComposing
    || sourceEditor.selectionStart !== sourceEditor.selectionEnd
  ) {
    return false;
  }
  const source = sourceEditor.value || "";
  const cursor = sourceEditor.selectionStart;
  const lineStart = source.lastIndexOf("\n", cursor - 1) + 1;
  const lineEnd = source.indexOf("\n", cursor);
  const safeLineEnd = lineEnd < 0 ? source.length : lineEnd;
  const lineBeforeCursor = source.slice(lineStart, cursor);
  if (stripSourceImportLineComment(lineBeforeCursor).length !== lineBeforeCursor.length) {
    return false;
  }
  const open = source.lastIndexOf("[", cursor - 1);
  const close = source.indexOf("]", cursor);
  if (open < lineStart || close < 0 || close > safeLineEnd || cursor <= open || cursor >= close) {
    return false;
  }
  const body = source.slice(open + 1, close);
  if (body.includes("[") || body.includes("]")) {
    return false;
  }
  const afterClose = source[close + 1] || "";
  const hasTrailingHorizontalSpace = afterClose === " " || afterClose === "\t";
  const bodyBeforeCursor = source.slice(open + 1, cursor);
  const bodyAfterCursor = source.slice(cursor, close);
  if (!/^[\t ]*$/.test(body)) {
    if (!bodyBeforeCursor.trim() || !/^[\t ]*$/.test(bodyAfterCursor)) {
      return false;
    }
    event.preventDefault();
    event.stopPropagation();
    clearSourceBlockSelection();
    if (!hasTrailingHorizontalSpace) {
      sourceEditor.setRangeText(" ", close + 1, close + 1, "end");
    }
    const cursorAfterCell = close + 2;
    sourceEditor.setSelectionRange(cursorAfterCell, cursorAfterCell);
    sourceEditorContentChanged();
    return true;
  }
  const replacement = hasTrailingHorizontalSpace ? "[  ]" : "[  ] ";
  event.preventDefault();
  event.stopPropagation();
  clearSourceBlockSelection();
  sourceEditor.setRangeText(replacement, open, close + 1, "end");
  const cursorAfterCell = open + replacement.length + (hasTrailingHorizontalSpace ? 1 : 0);
  sourceEditor.setSelectionRange(cursorAfterCell, cursorAfterCell);
  sourceEditorContentChanged();
  return true;
}

function handleSourceRewritePatternTab(event) {
  if (
    event.key !== "Tab"
    || event.altKey
    || event.ctrlKey
    || event.metaKey
    || sourceEditor.selectionStart !== sourceEditor.selectionEnd
  ) {
    return false;
  }
  const source = sourceEditor.value || "";
  const cursor = sourceEditor.selectionStart;
  const lineStart = source.lastIndexOf("\n", cursor - 1) + 1;
  const lineEnd = source.indexOf("\n", cursor);
  const safeLineEnd = lineEnd < 0 ? source.length : lineEnd;
  const line = source.slice(lineStart, safeLineEnd);
  const code = stripSourceImportLineComment(line);
  const cursorColumn = cursor - lineStart;
  if (cursorColumn > code.length) {
    return false;
  }
  const statementBounds = sourceRewriteStatementBounds(code, cursorColumn);
  const statement = code.slice(statementBounds.start, statementBounds.end);
  const arrow = statement.indexOf("->");
  if (arrow < 0) {
    return false;
  }
  const rhsStart = lineStart + statementBounds.start + arrow + 2;
  const rhsEnd = lineStart + statementBounds.end;
  if (cursor < rhsStart || cursor > rhsEnd) {
    return false;
  }
  const rhsTextBeforeCursor = source.slice(rhsStart, cursor);
  const rhsTextAfterCursor = source.slice(cursor, rhsEnd);
  if (!event.shiftKey && /^[\t ]*$/.test(rhsTextBeforeCursor) && /^[\t ]*$/.test(rhsTextAfterCursor)) {
    const lhsPattern = sourceRewritePatternBeforeArrow(statement.slice(0, arrow));
    if (lhsPattern) {
      event.preventDefault();
      event.stopPropagation();
      clearSourceBlockSelection();
      sourceEditor.setRangeText(lhsPattern, cursor, cursor, "end");
      sourceEditorContentChanged();
      return true;
    }
  }
  const slots = sourceRewritePatternSlotOffsets(source.slice(rhsStart, rhsEnd))
    .map((slot) => rhsStart + slot);
  if (!slots.length) {
    return false;
  }
  const target = event.shiftKey
    ? slots.slice().reverse().find((slot) => slot < cursor) ?? slots.at(-1)
    : slots.find((slot) => slot > cursor);
  if (!event.shiftKey && target == null && handleSourceRuleBracketCellTabExit(event)) {
    return true;
  }
  const fallbackTarget = target ?? slots[0];
  event.preventDefault();
  event.stopPropagation();
  sourceEditor.setSelectionRange(fallbackTarget, fallbackTarget);
  updateSourceMeta();
  hideSourceCompletions();
  return true;
}

function sourceEditorContentChanged(options = {}) {
  const preserveCompletions = Boolean(options.preserveSourceCompletions && keepSourceCompletionsVisibleDuringEdit());
  const puzzleSource = sourceDocumentSupportsEditableTargets();
  if (sourceFoldsActive()) {
    const changed = commitSourceFoldedDisplayEdit();
    if (!changed) {
      if (puzzleSource) {
        scheduleSourceHighlight(true, { preserveCurrent: false });
      } else {
        resetSourcePuzzleAnalysisState();
      }
      updateSourceMeta();
      if (puzzleSource) {
        refreshSourceColorEditor();
      }
      refreshSourceFindAfterSourceChange();
      scheduleLocalSave();
      if (preserveCompletions) {
        positionSourceCompletionPopover();
      } else {
        hideSourceCompletions();
      }
      return;
    }
  }
  if (puzzleSource) {
    scheduleSourceHighlight();
    scheduleSourceOutlineRefresh();
  } else {
    resetSourcePuzzleAnalysisState();
  }
  recordSourceUndoSnapshot();
  updateSourceMeta();
  if (puzzleSource) {
    refreshSourceColorEditor();
  }
  refreshSourceFindAfterSourceChange();
  if (documents[currentDocumentIndex]) {
    documents[currentDocumentIndex].source = sourceEditorDocumentValue();
  }
  scheduleLocalSave();
  if (puzzleSource) {
    scheduleLevelBuilderResetFromSource(false);
    schedulePreview();
  }
  if (preserveCompletions) {
    positionSourceCompletionPopover();
  } else {
    hideSourceCompletions();
  }
}

function sourceCompletionCanStayVisibleForKeydownEdit(event) {
  return Boolean(
    event
    && !event.defaultPrevented
    && !event.isComposing
    && !event.altKey
    && !event.ctrlKey
    && !event.metaKey
    && !sourceEditorBlockSelection?.ranges?.length
    && (event.key.length === 1 || event.key === "Backspace" || event.key === "Delete")
  );
}

function sourcePrintableKeydownEdit(event) {
  const source = sourceEditor.value || "";
  const start = Math.max(0, Math.min(source.length, sourceEditor.selectionStart || 0));
  const end = Math.max(start, Math.min(source.length, sourceEditor.selectionEnd || start));
  if (event.key !== "\"") {
    return {
      replacement: event.key,
      start,
      end,
      selectionStart: start + event.key.length,
      selectionEnd: start + event.key.length,
    };
  }
  const selection = source.slice(start, end);
  const replacement = `"${selection}"`;
  return {
    replacement,
    start,
    end,
    selectionStart: start + 1,
    selectionEnd: start + 1 + selection.length,
  };
}

function handleSourcePrintableKeydownInput(event) {
  if (
    !event
    || event.defaultPrevented
    || event.isComposing
    || event.altKey
    || event.ctrlKey
    || event.metaKey
    || event.key.length !== 1
    || sourceEditorBlockSelection?.ranges?.length
  ) {
    return false;
  }
  event.preventDefault();
  event.stopPropagation();
  hideSourceImportLinkFrame();
  sourceEditorPreferredCaretX = null;
  const edit = sourcePrintableKeydownEdit(event);
  sourceEditor.setRangeText(
    edit.replacement,
    edit.start,
    edit.end,
    "end",
  );
  sourceEditor.setSelectionRange(edit.selectionStart, edit.selectionEnd);
  sourceEditorContentChanged({ preserveSourceCompletions: true });
  scheduleSourceCompletion();
  syncPreviewModeFromSourceCursor();
  return true;
}

function handleSourceEditorVsCodeShortcut(event) {
  if (event.altKey && !event.ctrlKey && !event.metaKey && (event.key === "ArrowUp" || event.key === "ArrowDown")) {
    event.preventDefault();
    event.stopPropagation();
    if (event.shiftKey) {
      duplicateSourceSelectedLines(event.key === "ArrowUp" ? -1 : 1);
    } else {
      moveSourceSelectedLines(event.key === "ArrowUp" ? -1 : 1);
    }
    return true;
  }

  if (!event.metaKey || event.ctrlKey || event.altKey) {
    return false;
  }

  const key = event.key.length === 1 ? event.key.toLowerCase() : event.key;
  if (!event.shiftKey && key === "/") {
    event.preventDefault();
    event.stopPropagation();
    toggleSourceLineComments();
    return true;
  }
  if (!event.shiftKey && key === "l") {
    event.preventDefault();
    event.stopPropagation();
    selectSourceLines();
    return true;
  }
  if (!event.shiftKey && key === "]") {
    event.preventDefault();
    event.stopPropagation();
    indentSourceSelectedLines(1);
    return true;
  }
  if (!event.shiftKey && key === "[") {
    event.preventDefault();
    event.stopPropagation();
    indentSourceSelectedLines(-1);
    return true;
  }
  if (!event.shiftKey && key === "Enter") {
    event.preventDefault();
    event.stopPropagation();
    insertSourceLineAroundSelection(1);
    return true;
  }
  if (event.shiftKey && key === "Enter") {
    event.preventDefault();
    event.stopPropagation();
    insertSourceLineAroundSelection(-1);
    return true;
  }
  if (event.shiftKey && key === "k") {
    event.preventDefault();
    event.stopPropagation();
    deleteSourceSelectedLines();
    return true;
  }
  return false;
}

function selectedSourceLineRange() {
  const source = sourceEditor.value;
  const lines = sourceLinesWithOffsets(source);
  if (!lines.length) {
    return { lines, first: 0, last: 0 };
  }
  const start = Math.min(sourceEditor.selectionStart, sourceEditor.selectionEnd);
  let end = Math.max(sourceEditor.selectionStart, sourceEditor.selectionEnd);
  if (end > start && source[end - 1] === "\n") {
    end -= 1;
  }
  return {
    lines,
    first: sourceLineIndexAtOffset(lines, start),
    last: sourceLineIndexAtOffset(lines, end),
  };
}

function sourceLineIndexAtOffset(lines, offset) {
  const position = Math.max(0, offset);
  for (let index = 0; index < lines.length; index += 1) {
    if (position <= lines[index].end || index === lines.length - 1) {
      return index;
    }
  }
  return Math.max(0, lines.length - 1);
}

function sourceOffsetForLineColumn(lines, lineIndex, column) {
  const line = lines[Math.max(0, Math.min(lines.length - 1, lineIndex))];
  if (!line) {
    return 0;
  }
  return line.start + Math.max(0, Math.min(line.raw.length, column));
}

function sourceLineColumnAtOffset(lines, offset) {
  const lineIndex = sourceLineIndexAtOffset(lines, offset);
  const line = lines[lineIndex];
  return {
    lineIndex,
    column: Math.max(0, Math.min(line.raw.length, offset - line.start)),
  };
}

function sourceOffsetForRawLines(lines, lineIndex, column = 0) {
  let offset = 0;
  const clampedLine = Math.max(0, Math.min(lines.length - 1, lineIndex));
  for (let index = 0; index < clampedLine; index += 1) {
    offset += lines[index].length + 1;
  }
  return offset + Math.max(0, Math.min(lines[clampedLine]?.length || 0, column));
}

function replaceSourceValue(value, selectionStart = null, selectionEnd = selectionStart, selectionDirection = "none") {
  resetSourceFoldingState();
  sourceEditor.value = value;
  if (selectionStart !== null) {
    sourceEditor.setSelectionRange(selectionStart, selectionEnd ?? selectionStart, selectionDirection);
  }
  sourceEditorContentChanged();
}

function selectedSourceRawLineSpan() {
  const { first, last } = selectedSourceLineRange();
  const rawLines = sourceEditor.value.split("\n");
  return {
    rawLines,
    first,
    last: Math.max(first, Math.min(last, rawLines.length - 1)),
  };
}

function selectSourceLines() {
  const { lines, first, last } = selectedSourceLineRange();
  const start = lines[first]?.start ?? 0;
  const end = lines[last]?.absoluteEnd ?? sourceEditor.value.length;
  sourceEditor.setSelectionRange(start, end);
  updateSourceMeta();
}

function deleteSourceSelectedLines() {
  const { rawLines, first, last } = selectedSourceRawLineSpan();
  rawLines.splice(first, last - first + 1);
  const nextLines = rawLines.length ? rawLines : [""];
  const nextLine = Math.max(0, Math.min(first, nextLines.length - 1));
  const nextOffset = sourceOffsetForRawLines(nextLines, nextLine);
  replaceSourceValue(nextLines.join("\n"), nextOffset, nextOffset);
}

function duplicateSourceSelectedLines(direction) {
  const { rawLines, first, last } = selectedSourceRawLineSpan();
  const block = rawLines.slice(first, last + 1);
  const insertAt = direction < 0 ? first : last + 1;
  rawLines.splice(insertAt, 0, ...block);
  const selectedFirst = direction < 0 ? first : first + block.length;
  const selectedLast = direction < 0 ? last : last + block.length;
  const start = sourceOffsetForRawLines(rawLines, selectedFirst);
  const end = sourceOffsetForRawLines(rawLines, selectedLast, rawLines[selectedLast]?.length || 0);
  replaceSourceValue(rawLines.join("\n"), start, end);
}

function moveSourceSelectedLines(direction) {
  const { rawLines, first, last } = selectedSourceRawLineSpan();
  if ((direction < 0 && first === 0) || (direction > 0 && last >= rawLines.length - 1)) {
    return;
  }
  const block = rawLines.splice(first, last - first + 1);
  const insertAt = direction < 0 ? first - 1 : first + 1;
  rawLines.splice(insertAt, 0, ...block);
  const selectedFirst = insertAt;
  const selectedLast = insertAt + block.length - 1;
  const start = sourceOffsetForRawLines(rawLines, selectedFirst);
  const end = sourceOffsetForRawLines(rawLines, selectedLast, rawLines[selectedLast]?.length || 0);
  replaceSourceValue(rawLines.join("\n"), start, end);
}

function mapSourceOffsetThroughEdits(offset, edits) {
  let shift = 0;
  for (const edit of edits) {
    const editEnd = edit.start + edit.removeLength;
    const delta = edit.insertLength - edit.removeLength;
    if (offset < edit.start) {
      break;
    }
    if (edit.removeLength === 0) {
      shift += delta;
    } else if (offset <= edit.start) {
      continue;
    } else if (offset < editEnd) {
      return edit.start + shift;
    } else {
      shift += delta;
    }
  }
  return offset + shift;
}

function toggleSourceLineComments() {
  const originalSource = sourceEditor.value;
  const originalLines = sourceLinesWithOffsets(originalSource);
  const selectionStart = sourceEditor.selectionStart;
  const selectionEnd = sourceEditor.selectionEnd;
  const selectionDirection = sourceEditor.selectionDirection;
  const { rawLines, first, last } = selectedSourceRawLineSpan();
  const indexes = [];
  for (let index = first; index <= last; index += 1) {
    if ((rawLines[index] || "").trim()) {
      indexes.push(index);
    }
  }
  if (!indexes.length) {
    return;
  }
  const uncomment = indexes.every((index) => /^[\t ]*\/\//.test(rawLines[index]));
  const edits = [];
  for (const index of indexes) {
    const line = originalLines[index];
    const indentLength = (rawLines[index].match(/^[\t ]*/) || [""])[0].length;
    if (uncomment) {
      const marker = rawLines[index].slice(indentLength).match(/^\/\/ ?/)?.[0] || "";
      rawLines[index] = rawLines[index].replace(/^([\t ]*)\/\/ ?/, "$1");
      edits.push({
        start: (line?.start || 0) + indentLength,
        removeLength: marker.length,
        insertLength: 0,
      });
    } else {
      rawLines[index] = rawLines[index].replace(/^([\t ]*)/, "$1// ");
      edits.push({
        start: (line?.start || 0) + indentLength,
        removeLength: 0,
        insertLength: 3,
      });
    }
  }
  const start = mapSourceOffsetThroughEdits(selectionStart, edits);
  const end = mapSourceOffsetThroughEdits(selectionEnd, edits);
  replaceSourceValue(rawLines.join("\n"), start, end, selectionDirection);
}

function indentSourceSelectedLines(direction) {
  const { rawLines, first, last } = selectedSourceRawLineSpan();
  for (let index = first; index <= last; index += 1) {
    if (direction > 0) {
      rawLines[index] = `\t${rawLines[index] || ""}`;
    } else if ((rawLines[index] || "").startsWith("\t")) {
      rawLines[index] = rawLines[index].slice(1);
    } else {
      rawLines[index] = (rawLines[index] || "").replace(/^ {1,2}/, "");
    }
  }
  const start = sourceOffsetForRawLines(rawLines, first);
  const end = sourceOffsetForRawLines(rawLines, last, rawLines[last]?.length || 0);
  replaceSourceValue(rawLines.join("\n"), start, end);
}

function insertSourceLineAroundSelection(direction) {
  const { rawLines, first, last } = selectedSourceRawLineSpan();
  const lineIndex = direction < 0 ? first : last + 1;
  const reference = rawLines[direction < 0 ? first : last] || "";
  const indent = lineIndent(reference);
  rawLines.splice(lineIndex, 0, indent);
  const offset = sourceOffsetForRawLines(rawLines, lineIndex, indent.length);
  replaceSourceValue(rawLines.join("\n"), offset, offset);
}

function beginSourceNativeSelectionDrag(event) {
  if (
    event.button !== 0
    || event.altKey
    || event.ctrlKey
    || event.metaKey
    || !isTextDocument(activeDocument())
  ) {
    return;
  }
  sourceEditorWrap?.classList.add("is-source-selection-dragging");
}

function endSourceNativeSelectionDrag() {
  sourceEditorWrap?.classList.remove("is-source-selection-dragging");
}

function handleSourceBlockSelectionPointerDown(event) {
  if (sourceEditorBlockSelection && !event.altKey) {
    clearSourceBlockSelection();
    beginSourceNativeSelectionDrag(event);
    return;
  }
  if (!event.altKey || event.ctrlKey || event.metaKey || !isTextDocument(activeDocument())) {
    beginSourceNativeSelectionDrag(event);
    return;
  }
  const anchor = sourceEditorPositionFromPoint(event.clientX, event.clientY, { preserveColumn: true });
  if (!anchor) {
    return;
  }
  event.preventDefault();
  event.stopPropagation();
  sourceEditor.focus();
  sourceEditor.setPointerCapture(event.pointerId);
  sourceEditorBlockSelection = {
    anchor,
    focus: anchor,
    draggingPointerId: event.pointerId,
    ranges: sourceBlockRangesFromPoints(anchor, anchor),
  };
  const first = sourceEditorBlockSelection.ranges[0];
  if (first) {
    sourceEditor.setSelectionRange(first.start, first.end);
  }
  renderSourceBlockSelection();
}

function updateSourceImportLinkFromPointer(event) {
  if (!sourceImportLinkFrame || sourceEditorBlockSelection || !sourceDocumentSupportsEditableTargets()) {
    hideSourceImportLinkFrame();
    return;
  }
  const link = sourceImportLinkAtPointer(event);
  if (link) {
    sourceImportLinkState = link;
    renderSourceImportLinkFrame();
    return;
  }
  hideSourceImportLinkFrame();
}

function sourceImportLinkAtPointer(event, resolvedPosition = null) {
  const position = resolvedPosition || sourceEditorPositionFromPoint(event.clientX, event.clientY);
  if (!position) {
    return null;
  }
  const lines = sourceImportLinesWithOffsets(sourceEditor.value || "");
  const line = lines[position.lineIndex];
  if (!line) {
    return null;
  }
  const column = Math.max(0, Math.min(line.raw.length, position.column));
  const offset = line.start + column;
  return sourceImportLinkAtOffset(sourceEditor.value || "", offset, lines);
}

function sourceEditableTargetAtOffset(source, offset) {
  if (typeof surfaceEntriesForSource !== "function") {
    return null;
  }
  for (const entry of surfaceEntriesForSource(source)) {
    if (!Number.isInteger(entry?.start) || !Number.isInteger(entry?.end)) {
      continue;
    }
    if (offset < entry.start || offset > entry.end) {
      continue;
    }
    const config = SOURCE_EDITABLE_TARGETS.find((item) => item.kind === entry.kind);
    if (!config) {
      continue;
    }
    return {
      targetKind: config.kind,
      name: entry.name || "",
      position: offset,
      label: sourceEditableTargetLabel(config, entry),
    };
  }
  return null;
}

function sourceEditableTargetLoader(config) {
  const loader = sourceEditableTargetHandlers.get(config.kind)?.load;
  return typeof loader === "function" ? loader : null;
}

function sourceEditableTargetLabel(config, entry) {
  const targetLabel = typeof config.label === "function" ? config.label(entry) : config.label;
  return `Edit ${targetLabel || "source target"} ${entry?.name || ""}`.trim();
}

function registerSourceEditableTarget(kind, handlers = {}) {
  if (!SOURCE_EDITABLE_TARGETS.some((config) => config.kind === kind)) {
    return;
  }
  sourceEditableTargetHandlers.set(kind, {
    load: typeof handlers.load === "function" ? handlers.load : null,
  });
}

function sourceFrameRectForOffsets(start, end) {
  if (!sourceEditor || !Number.isInteger(start) || !Number.isInteger(end)) {
    return null;
  }
  const source = sourceEditor.value || "";
  const safeStart = Math.max(0, Math.min(source.length, start));
  const safeEnd = Math.max(safeStart, Math.min(source.length, end));
  const style = window.getComputedStyle(sourceEditor);
  const mirror = document.createElement("div");
  const before = document.createTextNode(source.slice(0, safeStart));
  const range = document.createElement("span");
  range.textContent = source.slice(safeStart, safeEnd) || "\u200b";
  mirror.style.position = "absolute";
  mirror.style.visibility = "hidden";
  mirror.style.pointerEvents = "none";
  mirror.style.boxSizing = "border-box";
  mirror.style.width = `${sourceViewportWidth()}px`;
  mirror.style.minHeight = "0";
  mirror.style.padding = style.padding;
  mirror.style.border = style.border;
  mirror.style.font = style.font;
  mirror.style.lineHeight = style.lineHeight;
  mirror.style.letterSpacing = style.letterSpacing;
  mirror.style.tabSize = style.tabSize;
  mirror.style.whiteSpace = "pre-wrap";
  mirror.style.overflowWrap = "break-word";
  mirror.style.wordBreak = style.wordBreak;
  mirror.append(before, range, document.createTextNode(source.slice(safeEnd)));
  document.body.append(mirror);
  const mirrorRect = mirror.getBoundingClientRect();
  const rangeRect = range.getBoundingClientRect();
  const lineHeight = sourceEditorLineHeight();
  const rect = {
    left: rangeRect.left - mirrorRect.left - sourceScrollLeft() - 2,
    top: rangeRect.top - mirrorRect.top - sourceScrollTop(),
    width: Math.max(8, rangeRect.width + 4),
    height: Math.max(lineHeight, rangeRect.height || lineHeight),
  };
  mirror.remove();
  return rect;
}

function sourceImportLinesWithOffsets(source) {
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

function sourceImportLinkAtOffset(source, offset, lines = sourceImportLinesWithOffsets(source)) {
  const reference = window.PuzzleStudioRuntime?.sourceImportReference?.(
    source,
    activeDocument()?.puzzlePath || "game.puzzle",
    offset,
  );
  if (!reference?.range || !reference?.pathRange || !reference.resolvedPath) {
    return null;
  }
  return {
    rawPath: reference.rawPath,
    resolvedPath: reference.resolvedPath,
    lineIndex: sourceLineIndexAtOffset(lines, reference.range.start),
    start: reference.range.start,
    end: reference.range.end,
    rect: sourceFrameRectForOffsets(reference.range.start, reference.range.end),
    pathStart: reference.pathRange.start,
    pathEnd: reference.pathRange.end,
  };
}

function renderSourceImportLinkFrame() {
  if (!sourceImportLinkFrame || !sourceImportLinkState) {
    return;
  }
  const rect = sourceImportLinkState.rect;
  if (!rect) {
    hideSourceImportLinkFrame();
    return;
  }
  sourceImportLinkFrame.style.left = `${rect.left}px`;
  sourceImportLinkFrame.style.top = `${rect.top}px`;
  sourceImportLinkFrame.style.width = `${rect.width}px`;
  sourceImportLinkFrame.style.height = `${rect.height}px`;
  const label = `Open ${sourceImportLinkState.rawPath}`;
  sourceImportLinkFrame.title = label;
  sourceImportLinkFrame.setAttribute("aria-label", label);
  sourceImportLinkFrame.hidden = false;
}

function hideSourceImportLinkFrame() {
  sourceImportLinkState = null;
  if (sourceImportLinkFrame) {
    sourceImportLinkFrame.hidden = true;
  }
}

function handleSourceImportEditorMouseLeave(event) {
  if (sourceImportLinkFrame && event.relatedTarget === sourceImportLinkFrame) {
    return;
  }
  hideSourceImportLinkFrame();
}

function handleSourceImportFrameMouseLeave(event) {
  if (event.relatedTarget === sourceEditor) {
    return;
  }
  hideSourceImportLinkFrame();
}

function openSourceImportLink() {
  const link = sourceImportLinkState;
  if (!link) {
    return false;
  }
  const target = documentByPath(link.resolvedPath);
  if (!target || !isTextDocument(target)) {
    setEditorStatus("Import not found", "is-error");
    hideSourceImportLinkFrame();
    return false;
  }
  hideSourceImportLinkFrame();
  revealSourceLocation({ document: target, start: 0 });
  sourceEditor.focus({ preventScroll: true });
  setEditorStatus(`Opened ${target.name || fileName(target.puzzlePath)}`, "is-ok");
  return true;
}

function openSourceFrameLink() {
  return openSourceImportLink();
}

function loadSourceEditableTargetFromPosition(position, options = {}) {
  if (!sourceDocumentSupportsEditableTargets()) {
    return "";
  }
  const source = sourceEditor.value || "";
  const target = sourceEditableTargetAtOffset(source, position);
  return target ? loadSourceEditableTarget(target, options) : "";
}

function loadSourceEditableTarget(target, options = {}) {
  const config = SOURCE_EDITABLE_TARGETS.find((entry) => entry.kind === target.targetKind);
  const loader = config ? sourceEditableTargetLoader(config) : null;
  if (!loader || !Number.isInteger(target.position)) {
    return "";
  }
  return loader(target.position, {
    ...(config.openOptions || {}),
    ...options,
  }) || "";
}

function sourceEditableEntryFromTarget(source, target, options = {}) {
  if (!Number.isInteger(target?.bodyStart) || !Number.isInteger(target?.bodyEnd)) {
    return target;
  }
  const start = Number.isInteger(target.start) ? target.start : target.bodyStart;
  const end = Number.isInteger(target.end) && target.end > target.bodyStart ? target.end : target.bodyEnd;
  const find = typeof options.find === "function" ? options.find : null;
  const localEntry = find ? find(source, start) : null;
  const entry = {
    ...(localEntry || {}),
    ...target,
    start,
    end,
    name: target.name || localEntry?.name || options.defaultName || "",
    levelIndex: Number.isInteger(target.levelIndex) ? target.levelIndex : localEntry?.levelIndex,
  };
  const body = typeof options.body === "function" ? options.body(source, entry, localEntry) : null;
  return body && typeof body === "object" ? { ...entry, ...body } : entry;
}

function openSourceImportLinkFromPointer(event, position = null) {
  if (sourceEditorBlockSelection || !sourceDocumentSupportsEditableTargets()) {
    return false;
  }
  const link = sourceImportLinkAtPointer(event, position);
  if (!link) {
    return false;
  }
  event.preventDefault();
  event.stopPropagation();
  sourceImportLinkState = link;
  return openSourceImportLink();
}

function updateSourceBlockSelectionDrag(event) {
  if (!sourceEditorBlockSelection || sourceEditorBlockSelection.draggingPointerId !== event.pointerId) {
    return;
  }
  const focus = sourceEditorPositionFromPoint(event.clientX, event.clientY, { preserveColumn: true });
  if (!focus) {
    return;
  }
  event.preventDefault();
  sourceEditorBlockSelection.focus = focus;
  sourceEditorBlockSelection.ranges = sourceBlockRangesFromPoints(sourceEditorBlockSelection.anchor, focus);
  const last = sourceEditorBlockSelection.ranges.at(-1);
  if (last) {
    sourceEditor.setSelectionRange(last.end, last.end);
  }
  renderSourceBlockSelection();
}

function finishSourceBlockSelectionDrag(event) {
  if (!sourceEditorBlockSelection || sourceEditorBlockSelection.draggingPointerId !== event.pointerId) {
    return;
  }
  event.preventDefault();
  if (sourceEditor.hasPointerCapture?.(event.pointerId)) {
    sourceEditor.releasePointerCapture(event.pointerId);
  }
  sourceEditorBlockSelection.draggingPointerId = null;
  renderSourceBlockSelection();
}

function sourceEditorPositionFromPoint(clientX, clientY, options = {}) {
  const lines = sourceLinesWithOffsets(sourceEditor.value);
  if (!lines.length) {
    return null;
  }
  const rawPosition = sourceEditorRawPositionFromPoint(clientX, clientY, lines);
  const visualOffset = Number.isInteger(options.visualOffset)
    ? options.visualOffset
    : sourceViewOffsetFromVisualPoint(clientX, clientY);
  if (Number.isInteger(visualOffset)) {
    const visualPosition = sourceLineColumnForOffset(lines, visualOffset);
    return options.preserveColumn
      ? { lineIndex: visualPosition.lineIndex, column: rawPosition.column }
      : visualPosition;
  }
  return rawPosition;
}

function sourceInteractionFromPointer(event, source = sourceEditorDocumentValue()) {
  if (!event || !sourceEditorWrap?.contains(event.target)) {
    return null;
  }
  const viewOffset = sourceViewOffsetFromVisualPoint(event.clientX, event.clientY);
  if (!Number.isInteger(viewOffset)) {
    return null;
  }
  const position = sourceEditorPositionFromPoint(event.clientX, event.clientY, {
    visualOffset: viewOffset,
  });
  const documentOffset = sourceFoldsActive()
    ? sourceViewOffsetToDocumentOffset(viewOffset, "start")
    : viewOffset;
  return {
    viewOffset,
    documentOffset: Math.max(0, Math.min(String(source || "").length, documentOffset)),
    position,
  };
}

function sourceEditorRawPositionFromPoint(clientX, clientY, lines) {
  const rect = sourceEditor.getBoundingClientRect();
  const style = window.getComputedStyle(sourceEditor);
  const paddingLeft = Number.parseFloat(style.paddingLeft) || 0;
  const paddingTop = Number.parseFloat(style.paddingTop) || 0;
  const lineHeight = sourceEditorLineHeight();
  const charWidth = sourceEditorCharWidth();
  const x = clientX - rect.left + sourceScrollLeft() - paddingLeft;
  const y = clientY - rect.top + sourceScrollTop() - paddingTop;
  return {
    lineIndex: Math.max(0, Math.min(lines.length - 1, Math.floor(y / lineHeight))),
    column: Math.max(0, Math.round(x / charWidth)),
  };
}

function sourceLineColumnForOffset(lines, offset) {
  const safeOffset = Math.max(0, offset || 0);
  for (let lineIndex = 0; lineIndex < lines.length; lineIndex += 1) {
    const line = lines[lineIndex];
    if (safeOffset <= line.end || lineIndex === lines.length - 1) {
      return {
        lineIndex,
        column: Math.max(0, Math.min(line.raw.length, safeOffset - line.start)),
      };
    }
  }
  return { lineIndex: 0, column: 0 };
}

function sourceEditorCharWidth() {
  const style = window.getComputedStyle(sourceEditor);
  const context = sourceEditorCharWidth.context || (sourceEditorCharWidth.context = document.createElement("canvas").getContext("2d"));
  context.font = `${style.fontWeight} ${style.fontSize} ${style.fontFamily}`;
  const width = context.measureText("M").width;
  return Number.isFinite(width) && width > 0 ? width : 8;
}

function sourceBlockRangesFromPoints(anchor, focus) {
  const lines = sourceLinesWithOffsets(sourceEditor.value);
  const firstLine = Math.max(0, Math.min(anchor.lineIndex, focus.lineIndex));
  const lastLine = Math.min(lines.length - 1, Math.max(anchor.lineIndex, focus.lineIndex));
  const startCol = Math.max(0, Math.min(anchor.column, focus.column));
  const endCol = Math.max(startCol, Math.max(anchor.column, focus.column));
  const ranges = [];
  for (let lineIndex = firstLine; lineIndex <= lastLine; lineIndex += 1) {
    const line = lines[lineIndex];
    const start = line.start + Math.min(line.raw.length, startCol);
    const end = line.start + Math.min(line.raw.length, endCol);
    ranges.push({ lineIndex, startCol, endCol, start, end });
  }
  return ranges;
}

function renderSourceBlockSelection() {
  if (!sourceBlockSelectionLayer) {
    return;
  }
  const ranges = sourceEditorBlockSelection?.ranges || [];
  sourceEditor?.classList.toggle("has-source-block-selection", ranges.length > 0);
  sourceBlockSelectionLayer.replaceChildren();
  if (ranges.length) {
    for (const range of ranges) {
      appendSourceBlockRange(range);
    }
    sourceBlockSelectionLayer.hidden = sourceBlockSelectionLayer.childElementCount === 0;
    return;
  }
  sourceBlockSelectionLayer.hidden = true;
}

function appendSourceBlockRange(range) {
  if (!range || range.start !== range.end) {
    appendSourceSelectionRects(range?.start, range?.end);
    return;
  }
  const rect = sourceBlockCaretRectForRange(range);
  if (!rect) {
    return;
  }
  const caret = document.createElement("div");
  caret.className = "source-block-selection-caret";
  caret.style.left = `${rect.left + sourceScrollLeft()}px`;
  caret.style.top = `${rect.top + sourceScrollTop()}px`;
  caret.style.height = `${rect.height}px`;
  sourceBlockSelectionLayer.append(caret);
}

function appendSourceSelectionRects(start, end) {
  const rects = sourceSelectionRectsForOffsets(start, end);
  for (const item of rects) {
    const rect = document.createElement("div");
    rect.className = "source-block-selection-range";
    rect.style.left = `${item.left}px`;
    rect.style.top = `${item.top}px`;
    rect.style.width = `${item.width}px`;
    rect.style.height = `${item.height}px`;
    sourceBlockSelectionLayer.append(rect);
  }
}

function sourceSelectionRectsForOffsets(start, end) {
  if (!sourceHighlight || !sourceEditorWrap) {
    return [];
  }
  const source = sourceEditor.value || "";
  const safeStart = Math.max(0, Math.min(source.length, Math.min(start || 0, end || 0)));
  const safeEnd = Math.max(safeStart, Math.min(source.length, Math.max(start || 0, end || 0)));
  if (safeStart === safeEnd) {
    return [];
  }
  const startPosition = sourceHighlightDomPositionForOffset(safeStart);
  const endPosition = sourceHighlightDomPositionForOffset(safeEnd);
  if (!startPosition || !endPosition) {
    return [];
  }
  const range = document.createRange();
  range.setStart(startPosition.node, startPosition.offset);
  range.setEnd(endPosition.node, endPosition.offset);
  const wrapRect = sourceEditorWrap.getBoundingClientRect();
  const lineHeight = sourceEditorLineHeight();
  const rects = Array.from(range.getClientRects())
    .filter((rect) => rect.width > 0 || rect.height > 0)
    .map((rect) => {
      const rectHeight = rect.height || lineHeight;
      const height = Math.max(lineHeight, rectHeight);
      return {
        left: rect.left - wrapRect.left + sourceScrollLeft(),
        right: rect.right - wrapRect.left + sourceScrollLeft(),
        top: rect.top - wrapRect.top + sourceScrollTop() - Math.max(0, (lineHeight - rectHeight) / 2),
        height,
      };
    });
  range.detach?.();
  const merged = [];
  for (const rect of rects) {
    const existing = merged.find((item) => Math.abs(item.top - rect.top) < 1);
    if (existing) {
      existing.left = Math.min(existing.left, rect.left);
      existing.right = Math.max(existing.right, rect.right);
      existing.height = Math.max(existing.height, rect.height);
    } else {
      merged.push({ ...rect });
    }
  }
  return merged.map((rect) => ({
    left: rect.left,
    top: rect.top,
    width: Math.max(2, rect.right - rect.left),
    height: rect.height,
  }));
}

function sourceBlockCaretRectForRange(range) {
  if (!sourceEditor || !sourceEditorWrap || !Number.isInteger(range?.lineIndex)) {
    return null;
  }
  const lines = sourceLinesWithOffsets(sourceEditor.value || "");
  const line = lines[range.lineIndex];
  if (!line) {
    return null;
  }
  const lineStartRect = sourceCaretRectForOffset(line.start);
  if (!lineStartRect) {
    return null;
  }
  if (range.startCol <= line.raw.length) {
    const caretRect = sourceCaretRectForOffset(range.start);
    if (caretRect) {
      return {
        left: caretRect.left,
        top: caretRect.top,
        height: caretRect.height || sourceEditorLineHeight(),
      };
    }
  }
  return {
    left: lineStartRect.left + (sourceEditorCharWidth() * Math.max(0, range.startCol || 0)),
    top: lineStartRect.top,
    height: lineStartRect.height || sourceEditorLineHeight(),
  };
}

function clearSourceBlockSelection() {
  sourceEditorBlockSelection = null;
  sourceEditor?.classList.remove("has-source-block-selection");
  if (sourceBlockSelectionLayer) {
    sourceBlockSelectionLayer.hidden = true;
    sourceBlockSelectionLayer.replaceChildren();
  }
}

function sourceBlockSelectionText() {
  if (!sourceEditorBlockSelection?.ranges?.length) {
    return "";
  }
  return sourceEditorBlockSelection.ranges
    .map((range) => sourceEditor.value.slice(range.start, range.end))
    .join("\n");
}

function moveSourceBlockSelectionHorizontal(delta, extend = false) {
  const ranges = sourceEditorBlockSelection?.ranges || [];
  if (!ranges.length) {
    return false;
  }
  const direction = delta < 0 ? -1 : 1;
  const firstLine = Math.min(...ranges.map((range) => range.lineIndex));
  const lastLine = Math.max(...ranges.map((range) => range.lineIndex));
  let anchor = null;
  let focus = null;

  if (extend) {
    anchor = sourceEditorBlockSelection.anchor || {
      lineIndex: firstLine,
      column: ranges[0]?.startCol || 0,
    };
    const previousFocus = sourceEditorBlockSelection.focus || {
      lineIndex: lastLine,
      column: direction > 0 ? ranges.at(-1)?.endCol || 0 : ranges.at(-1)?.startCol || 0,
    };
    focus = {
      lineIndex: previousFocus.lineIndex,
      column: Math.max(0, (previousFocus.column || 0) + direction),
    };
  } else {
    const collapsed = ranges.every((range) => range.startCol === range.endCol);
    const edgeColumn = direction > 0
      ? Math.max(...ranges.map((range) => range.endCol))
      : Math.min(...ranges.map((range) => range.startCol));
    const nextColumn = collapsed
      ? Math.max(0, edgeColumn + direction)
      : edgeColumn;
    anchor = { lineIndex: firstLine, column: nextColumn };
    focus = { lineIndex: lastLine, column: nextColumn };
  }

  sourceEditorBlockSelection = {
    anchor,
    focus,
    draggingPointerId: null,
    ranges: sourceBlockRangesFromPoints(anchor, focus),
  };
  syncSourceBlockSelectionNativeCaret();
  renderSourceBlockSelection();
  updateSourceMeta();
  hideSourceColorEditor();
  hideSourceCompletions();
  hideSourceImportLinkFrame();
  sourceEditorPreferredCaretX = null;
  return true;
}

function syncSourceBlockSelectionNativeCaret() {
  const last = sourceEditorBlockSelection?.ranges?.at(-1);
  if (!last) {
    return;
  }
  const caret = Math.max(0, Math.min(sourceEditor.value.length, last.end));
  sourceEditor.setSelectionRange(caret, caret);
}

function sourceBlockSelectionOwnsControlShortcut(event) {
  if (
    !sourceEditorBlockSelection?.ranges?.length
    || document.activeElement !== sourceEditor
    || !event.ctrlKey
    || event.metaKey
    || event.altKey
  ) {
    return false;
  }
  const key = event.key.length === 1 ? event.key.toLowerCase() : event.key;
  return key === "f" || key === "b";
}

function sourceBlockSelectionControlShortcutDelta(event) {
  if (!sourceBlockSelectionOwnsControlShortcut(event)) {
    return 0;
  }
  const key = event.key.length === 1 ? event.key.toLowerCase() : event.key;
  if (key === "f") {
    return 1;
  }
  if (key === "b") {
    return -1;
  }
  return 0;
}

function handleSourceBlockSelectionKeydown(event) {
  if (!sourceEditorBlockSelection?.ranges?.length) {
    return false;
  }
  if (event.key === "Escape") {
    clearSourceBlockSelection();
    event.preventDefault();
    event.stopPropagation();
    return true;
  }
  const controlDelta = sourceBlockSelectionControlShortcutDelta(event);
  if (controlDelta) {
    moveSourceBlockSelectionHorizontal(controlDelta, event.shiftKey);
  } else if (event.metaKey || event.ctrlKey) {
    return false;
  } else if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
    moveSourceBlockSelectionHorizontal(event.key === "ArrowRight" ? 1 : -1, event.shiftKey);
  } else if (event.key === "Backspace") {
    deleteSourceBlockSelection(-1);
  } else if (event.key === "Delete") {
    deleteSourceBlockSelection(1);
  } else if (event.key === "Tab") {
    // Tab is a source-editor command; the standard source style does not insert raw tabs.
  } else if (event.key === "Enter") {
    replaceSourceBlockSelection("\n", { keepSelection: false });
  } else if (event.key.length === 1 && !event.altKey) {
    replaceSourceBlockSelection(event.key);
  } else {
    return false;
  }
  event.preventDefault();
  event.stopPropagation();
  return true;
}

function replaceSourceBlockSelection(text, options = {}) {
  applySourceBlockSelectionReplacement((range, index, ranges) => {
    if (Array.isArray(text)) {
      return text[index] ?? text.at(-1) ?? "";
    }
    if (text.includes("\n") && ranges.length > 1) {
      const lines = text.split(/\r?\n/);
      return lines[index] ?? lines.at(-1) ?? "";
    }
    return text;
  }, options);
}

function deleteSourceBlockSelection(direction) {
  applySourceBlockSelectionReplacement((range) => {
    if (range.start !== range.end) {
      return "";
    }
    const source = sourceEditor.value;
    if (direction < 0 && range.start > 0 && source[range.start - 1] !== "\n") {
      return { start: range.start - 1, end: range.start, text: "", column: Math.max(0, range.startCol - 1) };
    }
    if (direction > 0 && range.end < source.length && source[range.end] !== "\n") {
      return { start: range.end, end: range.end + 1, text: "", column: range.startCol };
    }
    return { start: range.start, end: range.end, text: "", column: range.startCol };
  });
}

function applySourceBlockSelectionReplacement(replacer, options = {}) {
  const ranges = sourceEditorBlockSelection?.ranges || [];
  if (!ranges.length) {
    return;
  }
  const originalLines = sourceLinesWithOffsets(sourceEditor.value);
  let source = sourceEditor.value;
  const nextAnchors = [];
  for (let index = ranges.length - 1; index >= 0; index -= 1) {
    const range = ranges[index];
    const line = originalLines[range.lineIndex];
    const result = replacer(range, index, ranges);
    const replacement = typeof result === "object" && result !== null ? String(result.text || "") : String(result ?? "");
    const paddedPrefix = range.startCol > line.raw.length ? " ".repeat(range.startCol - line.raw.length) : "";
    const start = typeof result === "object" && result !== null ? result.start : range.start;
    const end = typeof result === "object" && result !== null ? result.end : range.end;
    const insert = (typeof result === "object" && result !== null) ? replacement : `${paddedPrefix}${replacement}`;
    source = `${source.slice(0, start)}${insert}${source.slice(end)}`;
    nextAnchors.unshift({
      lineIndex: range.lineIndex,
      column: typeof result === "object" && result !== null && Number.isFinite(result.column)
        ? Math.max(0, result.column)
        : range.startCol + replacement.length,
    });
  }
  sourceEditor.value = source;
  sourceEditorContentChanged();
  if (options.keepSelection === false || nextAnchors.some((anchor) => anchor.column < 0)) {
    clearSourceBlockSelection();
    return;
  }
  const anchor = nextAnchors[0];
  const focus = nextAnchors.at(-1) || anchor;
  sourceEditorBlockSelection = {
    anchor,
    focus,
    draggingPointerId: null,
    ranges: sourceBlockRangesFromPoints(anchor, focus),
  };
  const last = sourceEditorBlockSelection.ranges.at(-1);
  if (last) {
    sourceEditor.setSelectionRange(last.end, last.end);
  }
  renderSourceBlockSelection();
}

sourceEditor.addEventListener("sourcecompletioncommand", (event) => {
  if (!isTextDocument(documents[currentDocumentIndex])) {
    return;
  }
  const command = event.detail?.command;
  if (command === "show") {
    event.preventDefault();
    showSourceCompletions({ manual: true });
    return;
  }
  if (
    !sourceCompletionState
    || sourceCompletionPopover?.hidden
    || !sourceCompletionMatchesCurrentCursor()
  ) {
    return;
  }
  if (command === "close") {
    event.preventDefault();
    hideSourceCompletions();
    return;
  }
  if (command === "next" || command === "previous") {
    event.preventDefault();
    moveSourceCompletionSelection(command === "next" ? 1 : -1);
    return;
  }
  if (command === "commit" && sourceCompletionState.mode === "completion") {
    if (acceptSourceCompletion()) {
      event.preventDefault();
    }
  }
});

sourceEditor.addEventListener("keydown", (event) => {
  if (!isTextDocument(documents[currentDocumentIndex])) {
    return;
  }
  if (handleSourceFindShortcut(event)) {
    return;
  }
  if (handleSourceFindMoveShortcut(event)) {
    return;
  }
  if (event.key === "Escape" && isSourceFindPanelOpen()) {
    event.preventDefault();
    event.stopPropagation();
    closeSourceFindPanel();
    return;
  }
  if (sourceEditor.sourceEditorPort?.kind === "codemirror") {
    return;
  }
  if (sourceFoldsActive() && sourceKeydownWillEdit(event)) {
    captureSourceFoldEditSnapshot();
  }
  if (handleSourceUndoShortcut(event)) {
    return;
  }
  if (handleSourceBlockSelectionKeydown(event)) {
    return;
  }
  if ((event.ctrlKey || event.metaKey) && event.key === " ") {
    event.preventDefault();
    showSourceCompletions({ manual: true });
    return;
  }
  if (sourceCompletionState && !sourceCompletionPopover?.hidden) {
    if (!sourceCompletionMatchesCurrentCursor()) {
      if (!sourceCompletionCanStayVisibleForKeydownEdit(event)) {
        hideSourceCompletions();
      }
    } else {
      if (event.key === "Escape") {
        event.preventDefault();
        hideSourceCompletions();
        return;
      }
      if (sourceCompletionState.mode === "completion") {
        if (event.key === "ArrowDown") {
          event.preventDefault();
          moveSourceCompletionSelection(1);
          return;
        }
        if (event.key === "ArrowUp") {
          event.preventDefault();
          moveSourceCompletionSelection(-1);
          return;
        }
        if (event.key === "Tab") {
          event.preventDefault();
          acceptSourceCompletion();
          return;
        }
        if (event.key === "Enter") {
          event.preventDefault();
          acceptSourceCompletion();
          return;
        }
        if (event.ctrlKey && !event.metaKey && !event.altKey && event.key.toLowerCase() === "n") {
          event.preventDefault();
          event.stopPropagation();
          moveSourceCompletionSelection(1);
          return;
        }
        if (event.ctrlKey && !event.metaKey && !event.altKey && event.key.toLowerCase() === "p") {
          event.preventDefault();
          event.stopPropagation();
          moveSourceCompletionSelection(-1);
          return;
        }
      }
    }
  }
  if (event.key === "Escape" && sourceColorEdit) {
    hideSourceColorEditor();
    return;
  }
  if (handleSourceEditorEmacsBinding(event)) {
    return;
  }
  if (handleSourceEditorVsCodeShortcut(event)) {
    return;
  }
  if (handleSourceBraceAssist(event)) {
    return;
  }
  if (handleSourceRewriteLhsBracketAssist(event)) {
    return;
  }
  if (handleSourceIndentBackspace(event)) {
    return;
  }
  if (handleSourceRewriteRhsPatternAssist(event)) {
    return;
  }
  if (handleSourceRuleBracketCellSlotTab(event)) {
    return;
  }
  if (handleSourceRuleBracketCellTabExit(event)) {
    return;
  }
  if (handleSourceRewritePatternTab(event)) {
    return;
  }
  if (event.key === "Tab") {
    event.preventDefault();
    if (event.shiftKey || sourceEditor.value.slice(sourceEditor.selectionStart, sourceEditor.selectionEnd).includes("\n")) {
      indentSourceSelectedLines(event.shiftKey ? -1 : 1);
      return;
    }
    if (sourceCursorInLineLeadingWhitespace()) {
      insertAtSelection("\t");
      return;
    }
    return;
  }
  if (handleSourcePrintableKeydownInput(event)) {
    return;
  }

  if (event.key !== "Enter") {
    return;
  }

  event.preventDefault();
  insertSourceNewlineAtSelection();
});
sourceEditor.addEventListener("copy", (event) => {
  if (!sourceEditorBlockSelection?.ranges?.length) {
    return;
  }
  event.clipboardData?.setData("text/plain", sourceBlockSelectionText());
  event.preventDefault();
});
sourceEditor.addEventListener("cut", (event) => {
  if (!sourceEditorBlockSelection?.ranges?.length) {
    return;
  }
  event.clipboardData?.setData("text/plain", sourceBlockSelectionText());
  event.preventDefault();
  replaceSourceBlockSelection("", { keepSelection: false });
});
sourceEditor.addEventListener("paste", (event) => {
  if (!sourceEditorBlockSelection?.ranges?.length) {
    return;
  }
  event.preventDefault();
  replaceSourceBlockSelection(event.clipboardData?.getData("text/plain") || "", { keepSelection: true });
});

function insertAtSelection(value) {
  clearSourceBlockSelection();
  sourceEditor.setRangeText(
    value,
    sourceEditor.selectionStart,
    sourceEditor.selectionEnd,
    "end",
  );
  recordSourceUndoSnapshot();
  updateSourceMeta();
  if (documents[currentDocumentIndex]) {
    documents[currentDocumentIndex].source = sourceEditorDocumentValue();
  }
  if (sourceDocumentSupportsEditableTargets()) {
    scheduleSourceHighlight();
    scheduleSourceOutlineRefresh();
    schedulePreview();
  } else {
    resetSourcePuzzleAnalysisState();
  }
  scheduleLocalSave();
  hideSourceCompletions();
}

function setSourceEditorText(value, selectionStart = null, selectionEnd = selectionStart) {
  resetSourceFoldingState();
  sourceEditor.value = value || "";
  hideSourceColorEditor();
  hideSourceCompletions();
  clearSourceBlockSelection();
  if (selectionStart !== null) {
    sourceEditor.setSelectionRange(selectionStart, selectionEnd ?? selectionStart);
  }
  recordSourceUndoSnapshot();
  updateSourceMeta();
  if (documents[currentDocumentIndex]) {
    documents[currentDocumentIndex].source = sourceEditorDocumentValue();
  }
  if (sourceDocumentSupportsEditableTargets()) {
    scheduleSourceHighlight();
    scheduleSourceOutlineRefresh(true);
    resetLevelBuilderFromSource(false);
  } else {
    resetSourcePuzzleAnalysisState();
  }
}

function bindSourceEditorPopoverEvents() {
sourceEditorWrap?.addEventListener("scroll", hideSourceColorEditor);
sourceEditorWrap?.addEventListener("scroll", hideSourceCompletions);
sourceEditorWrap?.addEventListener("scroll", hideSourceImportLinkFrame);
sourceEditor.addEventListener("click", syncSourceOutlineActiveItem);
sourceEditor.addEventListener("keyup", syncSourceOutlineActiveItem);
sourceOutlineList?.addEventListener("click", (event) => {
  const row = event.target.closest("[data-source-outline-id]");
  if (!row || !sourceOutlineList.contains(row)) {
    return;
  }
  if (event.target.closest("[data-source-outline-toggle]")) {
    event.preventDefault();
    toggleSourceOutlineItem(row.dataset.sourceOutlineId);
    return;
  }
  openSourceOutlineItem(row.dataset.sourceOutlineId);
});
sourceOutlineList?.addEventListener("keydown", (event) => {
  if (!["Enter", " ", "ArrowRight", "ArrowLeft"].includes(event.key)) {
    return;
  }
  const row = event.target.closest("[data-source-outline-id]");
  if (!row || !sourceOutlineList.contains(row)) {
    return;
  }
  event.preventDefault();
  if (event.key === "ArrowRight" || event.key === "ArrowLeft") {
    toggleSourceOutlineItem(row.dataset.sourceOutlineId, event.key === "ArrowRight");
    return;
  }
  openSourceOutlineItem(row.dataset.sourceOutlineId);
});
document.addEventListener("pointerdown", hideSourceColorEditorForOutsidePointer);
window.addEventListener("resize", syncSourceHighlightScroll);
window.addEventListener("resize", hideSourceColorEditor);
window.addEventListener("resize", () => {
  if (sourceEditorBlockSelection?.ranges?.length) {
    renderSourceBlockSelection();
  }
});
if (window.ResizeObserver && sourceEditorWrap) {
  const sourceEditorWrapObserver = new ResizeObserver(() => scheduleSourceEditorLayoutSync(2));
  sourceEditorWrapObserver.observe(sourceEditorWrap);
}
}

function nextLineIndent() {
  const source = sourceEditor.value;
  const start = sourceEditor.selectionStart;
  const lineStart = source.lastIndexOf("\n", start - 1) + 1;
  const lineBeforeCursor = source.slice(lineStart, start);
  const currentIndent = lineBeforeCursor.match(/^[\t ]*/)?.[0] || "";
  const extraIndent = "";
  const afterCursor = source.slice(sourceEditor.selectionEnd);
  const nextNonWhitespace = afterCursor.match(/^\s*(.)/)?.[1] || "";

  if (lineBeforeCursor.trimEnd().endsWith("{") && nextNonWhitespace === "}") {
    return `\n${currentIndent}${extraIndent}\n${currentIndent}`;
  }
  return `\n${currentIndent}${extraIndent}`;
}

function insertSourceNewlineAtSelection() {
  const insert = nextLineIndent();
  const start = sourceEditor.selectionStart;
  const end = sourceEditor.selectionEnd;
  const cursorOffset = sourceNewlineCursorOffset(insert);
  sourceEditor.setRangeText(insert, start, end, "end");
  if (cursorOffset !== null) {
    const cursor = start + cursorOffset;
    sourceEditor.setSelectionRange(cursor, cursor);
  }
  sourceEditorContentChanged();
}

function sourceNewlineCursorOffset(insert) {
  const firstNewline = insert.indexOf("\n");
  const lastNewline = insert.lastIndexOf("\n");
  return firstNewline >= 0 && lastNewline > firstNewline ? lastNewline : null;
}

function sourceLevelNameControlEntries(config = {}) {
  const source = String(config.source || "");
  if (typeof config.collectEntries === "function") {
    return config.collectEntries({ ...config, source }) || [];
  }
  const requestedScope = String(config.scopeValue || "").trim();
  const findRanges = config.findRanges || (() => []);
  const findDefinitions = config.findDefinitions || (() => []);
  const rangeScope = config.rangeScope || (() => "");
  const entryName = config.entryName || ((entry) => entry?.name || "");
  const optionValue = config.optionValue || ((entry) => entryName(entry));
  const optionLabel = config.optionLabel || null;
  const ranges = findRanges(source).filter((range) => {
    const scope = String(rangeScope(range) || "").trim();
    return requestedScope ? scope === requestedScope : scope === "";
  });
  const entries = [];
  const seen = new Set();
  for (const range of ranges) {
    for (const entry of findDefinitions(source, range) || []) {
      const name = String(entryName(entry, range) || "").trim();
      const value = String(optionValue(entry, range) || name).trim();
      if (!name || !value) {
        continue;
      }
      const key = `${value}\u0000${name}`;
      if (seen.has(key)) {
        continue;
      }
      seen.add(key);
      const label = typeof optionLabel === "function" ? String(optionLabel(entry, range) || "").trim() : "";
      entries.push({ range, entry, name, value, label });
    }
  }
  return entries;
}

function syncSourceLevelNameDatalist(config = {}) {
  const datalist = config.datalist;
  if (!(datalist instanceof HTMLDataListElement)) {
    return [];
  }
  const entries = sourceLevelNameControlEntries(config);
  datalist.replaceChildren(...entries.map((entry) => {
    const option = document.createElement("option");
    option.value = entry.value;
    const label = entry.label || entry.name;
    if (label && label !== entry.value) {
      option.label = label;
    }
    return option;
  }));
  return entries;
}

function loadSourceLevelNameSelection(config = {}) {
  const input = config.nameInput;
  if (!(input instanceof HTMLInputElement)) {
    return false;
  }
  const value = String(input.value || "").trim();
  if (!value) {
    return false;
  }
  const entries = sourceLevelNameControlEntries(config);
  const match = entries.find((entry) => entry.value === value || entry.name === value);
  if (!match || typeof config.load !== "function") {
    return false;
  }
  return Boolean(config.load(match));
}

function showSourceLevelNameMenu(config = {}) {
  const input = config.nameInput;
  if (!(input instanceof HTMLInputElement)) {
    return [];
  }
  const entries = sourceLevelNameControlEntries(config);
  const menu = ensureSourceLevelNameMenu(input);
  if (!entries.length || !menu) {
    hideSourceLevelNameMenu(input);
    return entries;
  }
  const current = String(input.value || "").trim();
  menu.replaceChildren(...entries.map((entry) => {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "source-level-name-option";
    button.classList.toggle("is-current", entry.value === current || entry.name === current);
    button.textContent = entry.label || entry.value;
    button.title = entry.name === entry.value ? entry.value : entry.name;
    button.addEventListener("mousedown", (event) => {
      event.preventDefault();
    });
    button.addEventListener("click", () => {
      input.value = entry.value;
      hideSourceLevelNameMenu(input);
      if (typeof config.load === "function") {
        config.load(entry);
      }
      input.focus();
    });
    return button;
  }));
  menu.hidden = false;
  return entries;
}

function ensureSourceLevelNameMenu(input) {
  const label = input?.closest?.("label");
  if (!label) {
    return null;
  }
  let menu = label.querySelector(".source-level-name-menu");
  if (!menu) {
    menu = document.createElement("div");
    menu.className = "source-level-name-menu";
    menu.hidden = true;
    menu.addEventListener("mousedown", (event) => {
      event.preventDefault();
    });
    label.append(menu);
  }
  return menu;
}

function hideSourceLevelNameMenu(input) {
  const label = input?.closest?.("label");
  const menu = label?.querySelector?.(".source-level-name-menu");
  if (menu) {
    menu.hidden = true;
  }
}
