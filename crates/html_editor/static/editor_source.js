// Source editor state, text-editing commands, highlighting, completions, color editing, and source textarea event binding.
const sourceColorPopover = createSourceColorPopover();
const sourceColorCodeInput = sourceColorPopover?.querySelector("[data-source-color-code-input]");
const sourceColorAdjusterHost = sourceColorPopover?.querySelector("[data-source-color-adjuster]");
const sourceColorPreview = sourceColorPopover?.querySelector("[data-source-color-preview]");
const sourceCompletionPopover = createSourceCompletionPopover();
const sourceCompletionTextEncoder = new TextEncoder();
const sourceBlockSelectionLayer = createSourceBlockSelectionLayer();
const sourceCaretLayer = createSourceCaretLayer();
const sourceImportLinkFrame = createSourceImportLinkFrame();
const sourceTargetHintLayer = createSourceTargetHintLayer();
const SOURCE_EDITABLE_TARGETS = [
  {
    kind: "level3d",
    label: "3D level",
    openOptions: { switchMode: true },
    showInitialFrame: true,
  },
  {
    kind: "level",
    label: "level",
    openOptions: {},
    showInitialFrame: true,
  },
  {
    kind: "sprite3d",
    label: "3D sprite",
    openOptions: { switchMode: true },
    showInitialFrame: true,
  },
  {
    kind: "sprite",
    label: "sprite",
    openOptions: { switchMode: true },
    showInitialFrame: true,
  },
  {
    kind: "sounds",
    label: (entry) => entry?.kind || "sound",
    openOptions: { switchMode: true },
    showInitialFrame: false,
  },
];
const sourceEditableTargetHandlers = new Map();
let sourceHighlightTimer = 0;
let sourceCompletionTimer = 0;
let activeHighlightRequest = null;
let sourceHighlightRequestId = 0;
let sourceCompletionRequestId = 0;
let sourceColorEdit = null;
let sourceCompletionState = null;
let sourceImportLinkState = null;
let sourceTargetHintKey = "";
let sourceTargetHintTimer = 0;
let sourceTargetHintActive = false;
let sourceEditorKillRing = "";
let sourceEditorBlockSelection = null;
let sourceEditorRangeDrag = null;
let sourceEditorPreferredCaretX = null;
let suppressNextSourceClickSelection = false;
let sourceUndoStack = [];
let sourceRedoStack = [];
let sourceUndoApplying = false;
let sourceHighlightSource = "";
let sourceHighlightHtml = "";
let sourceHighlightMode = "";

function sourceEditorSnapshot() {
  return {
    value: sourceEditor.value || "",
    selectionStart: sourceEditor.selectionStart || 0,
    selectionEnd: sourceEditor.selectionEnd || 0,
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

function resetSourceUndoHistory() {
  sourceUndoStack = [sourceEditorSnapshot()];
  sourceRedoStack = [];
}

function ensureSourceUndoHistory() {
  if (!sourceUndoStack.length || sourceUndoStack.at(-1)?.value !== sourceEditor.value) {
    resetSourceUndoHistory();
    return;
  }
  const snapshot = sourceEditorSnapshot();
  if (!sameSourceEditorSnapshot(sourceUndoStack.at(-1), snapshot)) {
    sourceUndoStack[sourceUndoStack.length - 1] = snapshot;
  }
}

function recordSourceUndoSnapshot() {
  if (sourceUndoApplying) {
    return;
  }
  const snapshot = sourceEditorSnapshot();
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
  sourceEditor.value = snapshot.value || "";
  const start = Math.max(0, Math.min(sourceEditor.value.length, snapshot.selectionStart || 0));
  const end = Math.max(0, Math.min(sourceEditor.value.length, snapshot.selectionEnd || start));
  sourceEditor.setSelectionRange(start, end, snapshot.selectionDirection || "none");
  sourceEditorContentChanged();
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
  sourceEditor.value = value || "";
  invalidateSourceTargetHintFrames();
  updateSourceMeta();
  scheduleSourceHighlight(true);
  if (options.resetUndo === false) {
    recordSourceUndoSnapshot();
  } else {
    resetSourceUndoHistory();
  }
  showSourceTargetHintFramesOnce();
}

function scheduleSourceHighlight(immediate = false) {
  if (!renderSourceHighlightWithLoadedWasm()) {
    renderPlainSourceHighlight();
  }
  window.clearTimeout(sourceHighlightTimer);
  sourceHighlightTimer = window.setTimeout(refreshSourceHighlight, immediate ? 0 : 140);
}

function renderPlainSourceHighlight(source = sourceEditor.value) {
  if (!sourceHighlight) {
    return;
  }
  setSourceHighlightHtml(source, escapeHtml(source || " "), "plain");
}

function renderSourceHighlightWithLoadedWasm(source = sourceEditor.value) {
  const document = activeDocument();
  if (
    !sourceHighlight
    || !wasmCompiler
    || typeof wasmCompiler.highlight_source_html !== "function"
    || !isPuzzleDocument(document)
    || !isTextDocument(document)
  ) {
    return false;
  }
  try {
    setSourceHighlightHtml(
      source,
      wasmCompiler.highlight_source_html(source) || escapeHtml(source || " "),
      "wasm",
    );
    return true;
  } catch {
    return false;
  }
}

function setSourceHighlightHtml(source, html, mode) {
  if (!sourceHighlight) {
    return;
  }
  syncSourceHighlightMetrics();
  if (sourceHighlightSource !== source || sourceHighlightHtml !== html) {
    sourceHighlight.innerHTML = html;
    sourceHighlightSource = source;
    sourceHighlightHtml = html;
  }
  sourceHighlightMode = mode;
  syncSourceHighlightScroll();
  renderSourceCaret();
}

function syncSourceHighlightMetrics() {
  if (!sourceHighlight || !sourceEditor) {
    return;
  }
  sourceHighlight.style.width = `${sourceEditor.clientWidth}px`;
  sourceHighlight.style.height = `${sourceEditor.scrollHeight}px`;
}

function syncSourceHighlightScroll() {
  if (!sourceHighlight) {
    return;
  }
  syncSourceHighlightMetrics();
  sourceHighlight.style.transform = `translate(${-sourceEditor.scrollLeft}px, ${-sourceEditor.scrollTop}px)`;
  renderSourceCaret();
  if (sourceTargetHintActive) {
    renderSourceTargetHintFrames();
  }
}

function invalidateSourceTargetHintFrames() {
  sourceTargetHintKey = "";
}

function showSourceTargetHintFramesOnce() {
  if (!sourceTargetHintLayer || !sourceDocumentSupportsEditableTargets()) {
    return;
  }
  sourceTargetHintActive = true;
  invalidateSourceTargetHintFrames();
  renderSourceTargetHintFrames();
  window.clearTimeout(sourceTargetHintTimer);
  sourceTargetHintTimer = window.setTimeout(hideSourceTargetHintFrames, 1800);
}

function hideSourceTargetHintFrames() {
  sourceTargetHintActive = false;
  sourceTargetHintKey = "";
  window.clearTimeout(sourceTargetHintTimer);
  sourceTargetHintTimer = 0;
  if (sourceTargetHintLayer) {
    sourceTargetHintLayer.replaceChildren();
    sourceTargetHintLayer.hidden = true;
  }
}

function sourceDocumentSupportsEditableTargets() {
  return typeof activeDocument === "function"
    && typeof isPuzzleDocument === "function"
    && typeof isTextDocument === "function"
    && isPuzzleDocument(activeDocument())
    && isTextDocument(activeDocument());
}

async function refreshSourceHighlight() {
  const document = activeDocument();
  if (!sourceHighlight || !isPuzzleDocument(document) || !isTextDocument(document)) {
    return;
  }

  if (activeHighlightRequest) {
    activeHighlightRequest.abort();
  }
  const source = sourceEditor.value;
  const requestId = ++sourceHighlightRequestId;
  const controller = new AbortController();
  activeHighlightRequest = controller;

  try {
    const text = await window.PuzzleStudioHost.highlight(
      { source },
      { signal: controller.signal },
    );
    if (requestId !== sourceHighlightRequestId || source !== sourceEditor.value) {
      return;
    }
    syncSourceHighlightMetrics();
    const payload = JSON.parse(text);
    setSourceHighlightHtml(source, payload.html || escapeHtml(source || " "), "server");
  } catch (error) {
    if (error.name === "AbortError") {
      return;
    }
    if (previewBackendUnavailable(error)) {
      try {
        const html = await highlightSourceWithWasm(source);
        if (requestId !== sourceHighlightRequestId || source !== sourceEditor.value) {
          return;
        }
        setSourceHighlightHtml(source, html || escapeHtml(source || " "), "wasm");
        return;
      } catch {
        // Fall through to plain highlighting. Syntax color is optional.
      }
    }
    renderPlainSourceHighlight(source);
  } finally {
    if (activeHighlightRequest === controller) {
      activeHighlightRequest = null;
    }
  }
}

async function highlightSourceWithWasm(source) {
  const compiler = await loadWasmCompiler();
  return compiler.highlight_source_html(source);
}

async function suggestSourceCompletionsWithWasm(source, cursorOffset) {
  const compiler = await loadWasmCompiler();
  if (typeof compiler.suggest_source_completions !== "function") {
    return null;
  }
  const cursorByteOffset = sourceByteOffset(source, cursorOffset);
  const json = compiler.suggest_source_completions(source, cursorByteOffset);
  const list = JSON.parse(json || "{}");
  return {
    replaceStart: sourceUtf16OffsetFromByteOffset(source, Number(list.replaceStart) || 0),
    replaceEnd: sourceUtf16OffsetFromByteOffset(source, Number(list.replaceEnd) || 0),
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

function createSourceCaretLayer() {
  if (!sourceEditorWrap) {
    return null;
  }
  const layer = document.createElement("div");
  layer.className = "source-caret-layer";
  layer.hidden = true;
  sourceEditorWrap.append(layer);
  return layer;
}

function scheduleSourceCompletion(immediate = false) {
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
  if (!options.manual && !shouldAutoRequestSourceCompletion(source, cursor)) {
    hideSourceCompletions();
    return false;
  }
  const requestId = ++sourceCompletionRequestId;
  try {
    const list = await suggestSourceCompletionsWithWasm(source, cursor);
    if (requestId !== sourceCompletionRequestId || source !== sourceEditor.value || cursor !== sourceEditor.selectionStart) {
      return false;
    }
    const items = list?.items || [];
    if (!items.length) {
      hideSourceCompletions();
      return false;
    }
    sourceCompletionState = {
      replaceStart: list.replaceStart,
      replaceEnd: list.replaceEnd,
      items,
      selectedIndex: 0,
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

function shouldAutoRequestSourceCompletion(source, cursor) {
  const before = source.slice(0, cursor);
  const last = before.at(-1);
  if (!last) {
    return false;
  }
  if (last === ":" || /[_@A-Za-z0-9.-]/.test(last)) {
    return true;
  }
  return /\bgoto\s+$/.test(before.slice(Math.max(0, before.length - 80)));
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
  const count = sourceCompletionState.items.length;
  sourceCompletionState.selectedIndex = (sourceCompletionState.selectedIndex + delta + count) % count;
  renderSourceCompletionItems();
}

function acceptSourceCompletion(index = sourceCompletionState?.selectedIndex ?? 0) {
  if (!sourceCompletionState) {
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
    documents[currentDocumentIndex].source = sourceEditor.value;
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
  const style = window.getComputedStyle(sourceEditor);
  const lineHeight = parseFloat(style.lineHeight || "0") || 20;
  const anchor = Math.max(
    0,
    Math.min(sourceEditor.value.length, sourceCompletionState?.replaceStart ?? sourceEditor.selectionStart),
  );
  const editorRect = sourceEditor.getBoundingClientRect();
  const anchorPoint = sourceEditorCaretPoint(anchor);
  const cursorPoint = sourceVisualCaretPoint(sourceEditor.selectionStart) || sourceEditorCaretPoint(sourceEditor.selectionStart);
  const maxLeft = Math.max(8, window.innerWidth - 284);
  const visualAnchorPoint = sourceVisualCaretPoint(anchor) || anchorPoint;
  const left = editorRect.left + visualAnchorPoint.left;
  const top = editorRect.top + cursorPoint.top + lineHeight + 6;
  const availableBelow = Math.max(56, window.innerHeight - top - 8);
  sourceCompletionPopover.style.left = `${Math.max(8, Math.min(maxLeft, left))}px`;
  sourceCompletionPopover.style.top = `${top}px`;
  sourceCompletionPopover.style.maxHeight = `${Math.min(216, availableBelow)}px`;
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
  mirror.style.width = `${sourceEditor.clientWidth}px`;
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
  mirror.textContent = sourceEditor.value.slice(0, offset);
  marker.textContent = "\u200b";
  mirror.append(marker);
  document.body.append(mirror);
  const point = {
    left: marker.offsetLeft - sourceEditor.scrollLeft,
    top: marker.offsetTop - sourceEditor.scrollTop,
  };
  mirror.remove();
  return point;
}

function renderSourceCaret() {
  if (!sourceCaretLayer || !sourceEditor || !sourceHighlight) {
    return;
  }
  sourceCaretLayer.replaceChildren();
  const hasFocus = document.activeElement === sourceEditor;
  const hasSelection = sourceEditor.selectionStart !== sourceEditor.selectionEnd;
  if (!hasFocus || hasSelection || !isTextDocument(activeDocument())) {
    sourceCaretLayer.hidden = true;
    return;
  }
  const rect = sourceCaretRectForOffset(sourceEditor.selectionStart);
  if (!rect) {
    sourceCaretLayer.hidden = true;
    return;
  }
  const caret = document.createElement("div");
  caret.className = "source-caret";
  caret.style.left = `${rect.left}px`;
  caret.style.top = `${rect.top}px`;
  caret.style.height = `${rect.height}px`;
  sourceCaretLayer.append(caret);
  sourceCaretLayer.hidden = false;
}

function sourceCaretRectForOffset(offset) {
  const source = sourceEditor.value || "";
  const safeOffset = Math.max(0, Math.min(source.length, offset || 0));
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
  return {
    left: (useRightEdge ? rect.right : rect.left) - wrapRect.left,
    top: rect.top - wrapRect.top,
    height: Math.max(lineHeight, rect.height || lineHeight),
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
  let best = null;

  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    const text = node.nodeValue || "";
    if (!text.length) {
      continue;
    }
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
        const boundary = clientX <= midX ? sourceOffset + index : sourceOffset + index + 1;
        const horizontalDistance = clientX < rect.left
          ? rect.left - clientX
          : clientX > rect.right
            ? clientX - rect.right
            : 0;
        const score = (lineDistance * 10000) + horizontalDistance;
        if (!best || score < best.score) {
          best = { offset: boundary, score };
        }
        if (lineDistance === 0 && clientX >= rect.left && clientX <= rect.right) {
          range.detach?.();
          return Math.max(0, Math.min(source.length, boundary));
        }
      }
    }
    sourceOffset += text.length;
  }
  range.detach?.();
  return best ? Math.max(0, Math.min(source.length, best.offset)) : null;
}

function sourceOffsetFromVisualPointer(event, source = sourceEditor.value || "") {
  if (!event || !sourceEditorWrap?.contains(event.target)) {
    return null;
  }
  const offset = sourceVisualOffsetFromPoint(event.clientX, event.clientY);
  return Number.isInteger(offset)
    ? Math.max(0, Math.min(String(source || "").length, offset))
    : null;
}

function sourceOffsetFromVisualPoint(clientX, clientY, source = sourceEditor.value || "") {
  const offset = sourceVisualOffsetFromPoint(clientX, clientY);
  return Number.isInteger(offset)
    ? Math.max(0, Math.min(String(source || "").length, offset))
    : null;
}

function setSourceRangeSelection(anchor, focus) {
  const source = sourceEditor.value || "";
  const start = Math.max(0, Math.min(source.length, Math.min(anchor, focus)));
  const end = Math.max(start, Math.min(source.length, Math.max(anchor, focus)));
  sourceEditor.setSelectionRange(start, end, focus < anchor ? "backward" : "forward");
  renderSourceCaret();
}

function syncSourceSelectionFromPointer(event) {
  if (
    !event
    || event.altKey
    || event.ctrlKey
    || event.metaKey
    || sourceEditorBlockSelection
    || !isTextDocument(activeDocument())
  ) {
    return;
  }
  const offset = sourceOffsetFromVisualPointer(event);
  if (!Number.isInteger(offset)) {
    return;
  }
  if (event.shiftKey) {
    const anchor = sourceEditor.selectionDirection === "backward"
      ? sourceEditor.selectionEnd
      : sourceEditor.selectionStart;
    sourceEditor.setSelectionRange(Math.min(anchor, offset), Math.max(anchor, offset), offset < anchor ? "backward" : "forward");
  } else {
    sourceEditor.setSelectionRange(offset, offset);
  }
  renderSourceCaret();
}

function handleSourceVisualMouseDown(event) {
  if (
    !event
    || event.button !== 0
    || event.altKey
    || event.ctrlKey
    || event.metaKey
    || event.detail > 1
    || sourceEditorBlockSelection
    || !isTextDocument(activeDocument())
  ) {
    return;
  }
  const offset = sourceOffsetFromVisualPointer(event);
  if (!Number.isInteger(offset)) {
    return;
  }
  event.preventDefault();
  sourceEditorPreferredCaretX = null;
  sourceEditor.focus({ preventScroll: true });
  const anchor = event.shiftKey
    ? (sourceEditor.selectionDirection === "backward" ? sourceEditor.selectionEnd : sourceEditor.selectionStart)
    : offset;
  sourceEditorRangeDrag = {
    anchor,
    focus: offset,
    moved: false,
  };
  setSourceRangeSelection(anchor, offset);
  window.addEventListener("mousemove", updateSourceRangeDrag, true);
  window.addEventListener("mouseup", finishSourceRangeDrag, true);
  syncPreviewModeFromSourceCursor();
  hideSourceCompletions();
  hideSourceImportLinkFrame();
}

function updateSourceRangeDrag(event) {
  if (!sourceEditorRangeDrag) {
    return;
  }
  const offset = sourceOffsetFromVisualPoint(event.clientX, event.clientY);
  if (!Number.isInteger(offset)) {
    return;
  }
  event.preventDefault();
  if (offset !== sourceEditorRangeDrag.focus) {
    sourceEditorRangeDrag.moved = true;
  }
  sourceEditorRangeDrag.focus = offset;
  setSourceRangeSelection(sourceEditorRangeDrag.anchor, offset);
  syncPreviewModeFromSourceCursor();
}

function finishSourceRangeDrag(event) {
  if (!sourceEditorRangeDrag) {
    return;
  }
  if (event) {
    event.preventDefault();
  }
  suppressNextSourceClickSelection = Boolean(sourceEditorRangeDrag.moved);
  sourceEditorRangeDrag = null;
  window.removeEventListener("mousemove", updateSourceRangeDrag, true);
  window.removeEventListener("mouseup", finishSourceRangeDrag, true);
  renderSourceCaret();
}

function sourceByteOffset(value, utf16Offset) {
  return sourceCompletionTextEncoder.encode(value.slice(0, utf16Offset)).length;
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
  if (!sourceEditorWrap) {
    return null;
  }
  const popover = document.createElement("div");
  popover.className = "source-color-popover";
  popover.hidden = true;
  popover.innerHTML = `
    <span class="source-color-editor-header">
      <input class="source-color-code-input" data-source-color-code-input type="text" spellcheck="false" autocomplete="off" aria-label="Color code">
      <span class="source-color-live-preview sprite-color-swatch" data-source-color-preview aria-hidden="true"></span>
    </span>
    <span class="source-color-adjuster-host" data-source-color-adjuster></span>
  `;
  popover.addEventListener("mousedown", (event) => event.stopPropagation());
  popover.addEventListener("click", (event) => event.stopPropagation());
  sourceEditorWrap.append(popover);
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

function createSourceTargetHintLayer() {
  if (!sourceEditorWrap) {
    return null;
  }
  const layer = document.createElement("div");
  layer.className = "source-target-hint-layer";
  layer.hidden = true;
  sourceEditorWrap.append(layer);
  return layer;
}

function showSourceColorEditor(event = null) {
  if (!sourceColorPopover || !sourceColorCodeInput) {
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
  if (event && !sourceColorEventTargetsToken(event, token)) {
    hideSourceColorEditor();
    return false;
  }
  sourceColorEdit = token;
  sourceColorCodeInput.value = token.value.toLowerCase();
  syncSourceColorPopoverSurfaces(formatHexColorToken(parsed.rgb, parsed.alpha));
  renderSourceColorAdjuster(formatHexColorToken(parsed.rgb, parsed.alpha));
  positionSourceColorPopover(event);
  sourceColorPopover.hidden = false;
  return true;
}

function renderSourceColorAdjuster(color) {
  if (!sourceColorAdjusterHost) {
    return;
  }
  sourceColorAdjusterHost.replaceChildren();
  if (typeof renderSpriteColorAdjuster !== "function") {
    return;
  }
  sourceColorAdjusterHost.append(renderSpriteColorAdjuster({
    color,
    ariaLabel: "Source color",
    onChange: (nextColor) => updateSourceColorFromPopover({ color: nextColor }),
  }));
}

function syncSourceColorPopoverSurfaces(color) {
  const parsed = parseHexColorToken(color);
  if (!parsed) {
    return;
  }
  const normalized = formatHexColorToken(parsed.rgb, parsed.alpha);
  sourceColorCodeInput.value = normalized;
  sourceColorPopover?.style.setProperty("--source-picker-color", normalized);
  sourceColorPreview?.style.setProperty("--sprite-swatch-color", normalized);
  const adjuster = sourceColorAdjusterHost?.querySelector(".sprite-color-adjuster");
  if (typeof adjuster?.syncColor === "function") {
    adjuster.syncColor(normalized);
  }
}

function sourceColorEventTargetsToken(event, token) {
  if (!sourceEditor || !event || typeof document.caretPositionFromPoint !== "function") {
    return true;
  }
  const position = document.caretPositionFromPoint(event.clientX, event.clientY);
  const offset = position?.offsetNode === sourceEditor ? position.offset : null;
  if (!Number.isInteger(offset)) {
    return true;
  }
  return offset >= token.start && offset <= token.end;
}

function hideSourceColorEditor() {
  sourceColorEdit = null;
  if (sourceColorPopover) {
    sourceColorPopover.hidden = true;
  }
}

function positionSourceColorPopover(event = null) {
  if (!sourceColorPopover || !sourceEditorWrap) {
    return;
  }
  const wasHidden = sourceColorPopover.hidden;
  if (wasHidden) {
    sourceColorPopover.style.visibility = "hidden";
    sourceColorPopover.hidden = false;
  }
  const wrapRect = sourceEditorWrap.getBoundingClientRect();
  const popoverWidth = sourceColorPopover.offsetWidth || 274;
  const popoverHeight = sourceColorPopover.offsetHeight || 280;
  const margin = 8;
  const gap = 10;
  const localClearance = (sourceEditorLineHeight() * 2) + gap;
  const maxLeft = Math.max(margin, wrapRect.width - popoverWidth - margin);
  const maxTop = Math.max(margin, wrapRect.height - popoverHeight - margin);
  const anchor = sourceColorTokenAnchorRect(event, wrapRect);
  const textRects = sourceColorTextRects();
  const candidates = [
    { left: anchor.right + gap, top: anchor.bottom + localClearance, preference: 0 },
    { left: anchor.left - popoverWidth - gap, top: anchor.bottom + localClearance, preference: 1 },
    { left: anchor.right + gap, top: anchor.top - popoverHeight - localClearance, preference: 2 },
    { left: anchor.left - popoverWidth - gap, top: anchor.top - popoverHeight - localClearance, preference: 3 },
    { left: anchor.left, top: anchor.bottom + localClearance, preference: 4 },
    { left: anchor.left, top: anchor.top - popoverHeight - localClearance, preference: 5 },
    { left: anchor.right + gap, top: anchor.top - margin, preference: 6 },
    { left: anchor.left - popoverWidth - gap, top: anchor.top - margin, preference: 7 },
  ].map((candidate) => {
    const left = Math.max(margin, Math.min(maxLeft, candidate.left));
    const top = Math.max(margin, Math.min(maxTop, candidate.top));
    const rect = {
      left,
      top,
      right: left + popoverWidth,
      bottom: top + popoverHeight,
    };
    return {
      ...rect,
      score: sourceColorPopoverPositionScore(rect, anchor, wrapRect, textRects) + candidate.preference,
    };
  });
  const best = candidates.sort((a, b) => a.score - b.score)[0];
  sourceColorPopover.style.left = `${best.left}px`;
  sourceColorPopover.style.top = `${best.top}px`;
  if (wasHidden) {
    sourceColorPopover.hidden = true;
    sourceColorPopover.style.visibility = "";
  }
}

function sourceColorTokenAnchorRect(event, wrapRect) {
  const lineHeight = sourceEditorLineHeight();
  if (sourceColorEdit) {
    const startRect = sourceCaretRectForOffset(sourceColorEdit.start);
    const endRect = sourceCaretRectForOffset(sourceColorEdit.end);
    if (startRect && endRect) {
      const top = Math.min(startRect.top, endRect.top);
      const bottom = Math.max(startRect.top + startRect.height, endRect.top + endRect.height);
      return {
        left: Math.min(startRect.left, endRect.left),
        right: Math.max(startRect.left, endRect.left + 1),
        top,
        bottom,
      };
    }
  }
  const left = event?.clientX ? event.clientX - wrapRect.left : 18;
  const top = event?.clientY ? event.clientY - wrapRect.top : 18;
  return {
    left,
    right: left + 1,
    top,
    bottom: top + lineHeight,
  };
}

function sourceColorPopoverPositionScore(rect, anchor, wrapRect, textRects) {
  const lineHeight = sourceEditorLineHeight();
  const protectedRect = {
    left: 0,
    right: wrapRect.width,
    top: Math.max(0, anchor.top - lineHeight),
    bottom: anchor.bottom + (lineHeight * 2.2),
  };
  const textOverlap = sourceColorTextOverlapArea(rect, textRects);
  const protectedOverlap = sourceRectIntersectionArea(rect, protectedRect);
  return (textOverlap * 10) + (protectedOverlap * 20);
}

function sourceColorTextRects() {
  if (!sourceHighlight || !sourceEditorWrap) {
    return [];
  }
  const wrapRect = sourceEditorWrap.getBoundingClientRect();
  const walker = document.createTreeWalker(sourceHighlight, NodeFilter.SHOW_TEXT);
  const range = document.createRange();
  const rects = [];
  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    const length = (node.nodeValue || "").length;
    if (!length) {
      continue;
    }
    range.setStart(node, 0);
    range.setEnd(node, length);
    for (const clientRect of range.getClientRects()) {
      rects.push({
        left: clientRect.left - wrapRect.left,
        right: clientRect.right - wrapRect.left,
        top: clientRect.top - wrapRect.top,
        bottom: clientRect.bottom - wrapRect.top,
      });
    }
  }
  range.detach?.();
  return rects;
}

function sourceColorTextOverlapArea(rect, textRects) {
  let overlap = 0;
  for (const textRect of textRects) {
    overlap += sourceRectIntersectionArea(rect, textRect);
  }
  return overlap;
}

function sourceRectIntersectionArea(a, b) {
  const width = Math.max(0, Math.min(a.right, b.right) - Math.max(a.left, b.left));
  const height = Math.max(0, Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top));
  return width * height;
}

function updateSourceColorFromPopover(options = {}) {
  if (!sourceColorEdit || !sourceColorCodeInput) {
    return;
  }
  const current = hexColorAt(sourceEditor.value, sourceColorEdit.start);
  if (!current || current.start !== sourceColorEdit.start) {
    hideSourceColorEditor();
    return;
  }
  const parsedColor = options.color ? parseHexColorToken(options.color) : null;
  const parsedCode = options.fromCode ? parseHexColorToken(sourceColorCodeInput.value) : null;
  if ((options.fromCode && !parsedCode) || (options.color && !parsedColor)) {
    return;
  }
  const parsedNextColor = parsedCode || parsedColor;
  if (!parsedNextColor) {
    return;
  }
  const next = formatHexColorToken(parsedNextColor.rgb, parsedNextColor.alpha);
  sourceEditor.setRangeText(next, current.start, current.end, "preserve");
  sourceColorEdit = { start: current.start, end: current.start + next.length, value: next };
  sourceEditor.setSelectionRange(sourceColorEdit.start, sourceColorEdit.end);
  recordSourceUndoSnapshot();
  const parsedNext = parseHexColorToken(next);
  if (parsedNext) {
    syncSourceColorPopoverSurfaces(next);
  }
  updateSourceMeta();
  if (documents[currentDocumentIndex]) {
    documents[currentDocumentIndex].source = sourceEditor.value;
  }
  scheduleSourceHighlight(true);
  scheduleLocalSave();
  resetLevelBuilderFromSource(false);
  schedulePreview();
}

function refreshSourceColorEditor() {
  if (!sourceColorEdit || sourceColorPopover?.hidden) {
    return;
  }
  const current = hexColorAt(sourceEditor.value, sourceColorEdit.start);
  if (!current || current.start !== sourceColorEdit.start) {
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

function bindSourceEditorEvents() {
sourceEditor.addEventListener("beforeinput", (event) => {
  if (!isTextDocument(documents[currentDocumentIndex])) {
    return;
  }
  ensureSourceUndoHistory();
  if (event.inputType === "historyUndo" || event.inputType === "historyRedo") {
    event.preventDefault();
    if (event.inputType === "historyUndo") {
      undoSourceEdit();
    } else {
      redoSourceEdit();
    }
  }
});
sourceEditor.addEventListener("input", () => {
  if (!isTextDocument(documents[currentDocumentIndex])) {
    return;
  }
  hideSourceTargetHintFrames();
  invalidateSourceTargetHintFrames();
  hideSourceImportLinkFrame();
  clearSourceBlockSelection();
  sourceEditorPreferredCaretX = null;
  recordSourceUndoSnapshot();
  updateSourceMeta();
  refreshSourceColorEditor();
  scheduleSourceHighlight();
  scheduleSourceCompletion();
  if (documents[currentDocumentIndex]) {
    documents[currentDocumentIndex].source = sourceEditor.value;
  }
  scheduleLocalSave();
  resetLevelBuilderFromSource(false);
  syncPreviewModeFromSourceCursor();
  renderSourceCaret();
  schedulePreview();
});
sourceEditor.addEventListener("click", (event) => {
  hideSourceTargetHintFrames();
  if (suppressNextSourceClickSelection) {
    suppressNextSourceClickSelection = false;
    return;
  }
  sourceEditorPreferredCaretX = null;
  if (openSourceImportLinkFromPointer(event)) {
    return;
  }
  syncSourceSelectionFromPointer(event);
  window.setTimeout(() => showSourceColorEditor(), 40);
  window.setTimeout(() => showSourceCompletions({ manual: false }), 0);
  syncPreviewModeFromSourcePointer(event);
});
sourceEditor.addEventListener("mousedown", handleSourceVisualMouseDown);
sourceEditor.addEventListener("pointerdown", handleSourceBlockSelectionPointerDown);
sourceEditor.addEventListener("mousemove", updateSourceImportLinkFromPointer);
sourceEditor.addEventListener("mouseleave", handleSourceImportEditorMouseLeave);
sourceEditor.addEventListener("pointermove", updateSourceBlockSelectionDrag);
sourceEditor.addEventListener("pointerup", finishSourceBlockSelectionDrag);
sourceEditor.addEventListener("pointercancel", finishSourceBlockSelectionDrag);
sourceEditor.addEventListener("keyup", (event) => {
  if (event.key === "Escape") {
    hideSourceColorEditor();
    hideSourceCompletions();
    return;
  }
  if (event.key.startsWith("Arrow") || event.key === "Home" || event.key === "End") {
    showSourceColorEditor();
    showSourceCompletions({ manual: false });
    syncPreviewModeFromSourceCursor();
  }
  renderSourceCaret();
});
sourceEditor.addEventListener("focus", () => {
  renderSourceCaret();
  syncPreviewModeFromSourceCursor({ force: true });
});
sourceEditor.addEventListener("blur", renderSourceCaret);
document.addEventListener("selectionchange", () => {
  if (document.activeElement !== sourceEditor) {
    renderSourceCaret();
    return;
  }
  syncPreviewModeFromSourceCursor();
  renderSourceCaret();
});
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
      insertAtSelection(nextLineIndent());
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
    renderSourceCaret();
    return true;
  }
  const current = sourceCaretRectForOffset(sourceSelectionFocus());
  if (!current || !sourceEditorWrap) {
    moveSourceSelection(sourceVerticalPosition(event.key === "ArrowDown" ? 1 : -1), event.shiftKey);
    renderSourceCaret();
    return true;
  }
  const wrapRect = sourceEditorWrap.getBoundingClientRect();
  const lineHeight = sourceEditorLineHeight();
  if (!Number.isFinite(sourceEditorPreferredCaretX)) {
    sourceEditorPreferredCaretX = current.left;
  }
  const targetClientX = wrapRect.left + sourceEditorPreferredCaretX;
  const targetClientY = wrapRect.top + current.top + (event.key === "ArrowDown" ? lineHeight + 1 : -1);
  const next = sourceOffsetFromVisualPoint(targetClientX, targetClientY);
  if (Number.isInteger(next)) {
    moveSourceSelection(next, event.shiftKey);
  } else {
    moveSourceSelection(sourceVerticalPosition(event.key === "ArrowDown" ? 1 : -1), event.shiftKey);
  }
  renderSourceCaret();
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
  const lines = Math.max(1, Math.floor(sourceEditor.clientHeight / lineHeight) - 1);
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
    renderSourceCaret();
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

function sourceEditorContentChanged() {
  invalidateSourceTargetHintFrames();
  recordSourceUndoSnapshot();
  updateSourceMeta();
  refreshSourceColorEditor();
  if (documents[currentDocumentIndex]) {
    documents[currentDocumentIndex].source = sourceEditor.value;
  }
  scheduleSourceHighlight();
  scheduleLocalSave();
  resetLevelBuilderFromSource(false);
  schedulePreview();
  hideSourceCompletions();
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

function handleSourceBlockSelectionPointerDown(event) {
  if (!event.altKey || event.ctrlKey || event.metaKey || !isTextDocument(activeDocument())) {
    return;
  }
  const anchor = sourceEditorPositionFromPoint(event.clientX, event.clientY);
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
  renderSourceBlockSelection();
}

function updateSourceImportLinkFromPointer(event) {
  hideSourceTargetHintFrames();
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
  const target = sourceEditableTargetAtPointer(event);
  if (!target) {
    hideSourceImportLinkFrame();
    return;
  }
  sourceImportLinkState = target;
  renderSourceImportLinkFrame();
}

function sourceImportLinkAtPointer(event) {
  const position = sourceEditorPositionFromPoint(event.clientX, event.clientY);
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

function sourceEditableTargetAtPointer(event) {
  if (!event || sourceEditorBlockSelection || !sourceDocumentSupportsEditableTargets()) {
    return null;
  }
  const source = sourceEditor.value || "";
  const offset = sourceOffsetFromEditorPointer(event, source);
  if (!Number.isInteger(offset)) {
    return null;
  }
  return sourceEditableTargetAtOffset(source, offset);
}

function sourceEditableTargetAtOffset(source, offset, options = {}) {
  const hoverLine = options.lineRange || sourceLineRangeAtOffset(source, offset);
  const configs = options.configs || SOURCE_EDITABLE_TARGETS;
  for (const config of configs) {
    const finder = sourceEditableTargetFinder(config);
    const entry = finder ? finder(source, offset) : null;
    if (!entry) {
      continue;
    }
    return sourceEditableTargetFrameState({
      kind: config.kind,
      name: entry.name || "",
      start: hoverLine.start,
      end: hoverLine.end,
      position: offset,
      label: sourceEditableTargetLabel(config, entry),
    });
  }
  return null;
}

function sourceEditableTargetFinder(config) {
  const finder = sourceEditableTargetHandlers.get(config.kind)?.find;
  return typeof finder === "function" ? finder : null;
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
    find: typeof handlers.find === "function" ? handlers.find : null,
    load: typeof handlers.load === "function" ? handlers.load : null,
  });
  invalidateSourceTargetHintFrames();
  if (sourceTargetHintActive && sourceDocumentSupportsEditableTargets()) {
    renderSourceTargetHintFrames();
  }
}

function sourceEditableTargetFrameState(target) {
  const frame = sourceFrameRectForOffsets(target.start, target.end);
  if (!frame) {
    return null;
  }
  return {
    linkType: "source-target",
    targetKind: target.kind,
    name: target.name,
    position: target.position,
    label: target.label,
    ...frame,
  };
}

function renderSourceTargetHintFrames() {
  if (!sourceTargetHintLayer) {
    return;
  }
  const states = sourceInitialEditableTargetFrames();
  const key = [
    activeDocument()?.id || "",
    sourceEditor.value.length,
    sourceEditor.scrollLeft,
    sourceEditor.scrollTop,
    ...states.map((state) => `${state.targetKind}:${state.position}:${state.left}:${state.top}`),
  ].join("|");
  if (key === sourceTargetHintKey) {
    return;
  }
  sourceTargetHintKey = key;
  sourceTargetHintLayer.replaceChildren();
  sourceTargetHintLayer.hidden = states.length === 0;
  for (const state of states) {
    const frame = document.createElement("span");
    frame.className = "source-target-hint-frame";
    frame.style.left = `${state.left}px`;
    frame.style.top = `${state.top}px`;
    frame.style.width = `${state.width}px`;
    frame.style.height = `${state.height}px`;
    frame.dataset.sourceTargetKind = state.targetKind;
    frame.title = state.label;
    frame.setAttribute("aria-label", state.label);
    sourceTargetHintLayer.append(frame);
  }
}

function sourceInitialEditableTargetFrames() {
  if (!sourceDocumentSupportsEditableTargets()) {
    return [];
  }
  const source = sourceEditor.value || "";
  const lines = sourceImportLinesWithOffsets(source);
  const states = [];
  const seenKinds = new Set();
  const configs = SOURCE_EDITABLE_TARGETS.filter((config) => config.showInitialFrame);
  for (const line of lines) {
    if (seenKinds.size >= configs.length) {
      break;
    }
    const firstCode = String(line.raw || "").search(/\S/);
    if (firstCode < 0) {
      continue;
    }
    const offset = line.start + firstCode;
    const lineRange = sourceLineRangeAtOffset(source, offset);
    for (const config of configs) {
      if (seenKinds.has(config.kind)) {
        continue;
      }
      const state = sourceEditableTargetAtOffset(source, offset, {
        configs: [config],
        lineRange,
      });
      if (!state) {
        continue;
      }
      states.push(state);
      seenKinds.add(config.kind);
      break;
    }
  }
  return states;
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
  mirror.style.width = `${sourceEditor.clientWidth}px`;
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
    left: rangeRect.left - mirrorRect.left - sourceEditor.scrollLeft - 2,
    top: rangeRect.top - mirrorRect.top - sourceEditor.scrollTop,
    width: Math.max(8, rangeRect.width + 4),
    height: Math.max(lineHeight, rangeRect.height || lineHeight),
  };
  mirror.remove();
  return rect;
}

function sourceLineRangeAtOffset(source, offset) {
  const lines = sourceImportLinesWithOffsets(source);
  if (!lines.length) {
    return { start: 0, end: 0 };
  }
  const lineIndex = sourceLineIndexAtOffset(lines, offset);
  const line = lines[lineIndex] || lines[0];
  if (!line) {
    return { start: 0, end: 0 };
  }
  const firstCode = String(line.raw || "").search(/\S/);
  if (firstCode < 0) {
    return { start: line.start, end: line.end };
  }
  return {
    start: line.start + firstCode,
    end: Math.max(line.start + firstCode + 1, line.end),
  };
}

function sourceOffsetFromEditorPointer(event, source) {
  if (!event || !sourceEditorWrap?.contains(event.target)) {
    return null;
  }
  const visualOffset = sourceOffsetFromVisualPointer(event, source);
  if (Number.isInteger(visualOffset)) {
    return visualOffset;
  }
  const position = sourceEditorPositionFromPoint(event.clientX, event.clientY);
  if (!position) {
    return null;
  }
  const lines = sourceImportLinesWithOffsets(source);
  return sourceOffsetForLineColumn(lines, position.lineIndex, position.column);
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
  const lineIndex = sourceLineIndexAtOffset(lines, offset);
  const line = lines[lineIndex];
  if (!line) {
    return null;
  }
  const code = stripSourceImportLineComment(line.raw);
  const importMatch = code.match(/^(\s*import\s*)"((?:\\.|[^"\\])*)"/);
  if (importMatch) {
    return sourceQuotedPathLinkForMatch(importMatch, line, lineIndex, offset, "import");
  }
  if (!sourceLineIsInAssetsBlock(lines, lineIndex)) {
    return null;
  }
  const assetMatch = code.match(/^(\s*(?:css|script)\s*)"((?:\\.|[^"\\])*)"/);
  if (assetMatch) {
    return sourceQuotedPathLinkForMatch(assetMatch, line, lineIndex, offset, "asset");
  }
  return null;
}

function sourceQuotedPathLinkForMatch(match, line, lineIndex, offset, kind) {
  if (!match) {
    return null;
  }
  const quoteStart = line.start + match[1].length;
  const frameStart = quoteStart;
  const frameEnd = quoteStart + match[0].length - match[1].length;
  const pathStart = quoteStart + 1;
  const pathEnd = pathStart + match[2].length;
  if (offset < frameStart || offset > frameEnd) {
    return null;
  }
  const baseDir = directoryName(activeDocument()?.puzzlePath || "");
  const resolvedPath = resolveWasmImportPath(baseDir, match[2]);
  return {
    kind,
    rawPath: match[2],
    resolvedPath,
    lineIndex,
    start: frameStart,
    end: frameEnd,
    rect: sourceFrameRectForOffsets(frameStart, frameEnd),
    pathStart,
    pathEnd,
  };
}

function sourceLineIsInAssetsBlock(lines, lineIndex) {
  const stack = [];
  for (let index = 0; index < lineIndex; index += 1) {
    const code = stripSourceImportLineComment(lines[index]?.raw || "").trim();
    if (!code) {
      continue;
    }
    if (code === "}" || code === "end") {
      stack.pop();
      continue;
    }
    if (/^assets(?:\s*\{)?$/.test(code)) {
      stack.push("assets");
      continue;
    }
    if (code.endsWith("{")) {
      stack.push("other");
    }
  }
  return stack.at(-1) === "assets";
}

function stripSourceImportLineComment(line) {
  let quote = "";
  let escaped = false;
  for (let index = 0; index < line.length; index += 1) {
    const char = line[index];
    const next = line[index + 1] || "";
    if (escaped) {
      escaped = false;
      continue;
    }
    if (char === "\\") {
      escaped = true;
      continue;
    }
    if (quote) {
      if (char === quote) {
        quote = "";
      }
      continue;
    }
    if (char === "\"") {
      quote = char;
      continue;
    }
    if (char === "/" && next === "/") {
      return line.slice(0, index);
    }
  }
  return line;
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
  const label = sourceImportLinkState.linkType === "source-target"
    ? sourceImportLinkState.label
    : `Open ${sourceImportLinkState.rawPath}`;
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
    const builtin = link.kind === "import" && typeof builtinThemeImportSource === "function"
      ? builtinThemeImportSource(link.resolvedPath)
      : "";
    setEditorStatus(builtin ? "Built-in import" : `${link.kind === "asset" ? "Asset" : "Import"} not found`, builtin ? "" : "is-error");
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
  if (sourceImportLinkState?.linkType === "source-target") {
    return openSourceEditableTarget();
  }
  return openSourceImportLink();
}

function openSourceEditableTarget() {
  const target = sourceImportLinkState;
  if (!target || !Number.isInteger(target.position)) {
    return false;
  }
  const key = loadSourceEditableTarget(target, { recordHistory: true });
  hideSourceImportLinkFrame();
  if (key) {
    sourceEditor.focus({ preventScroll: true });
    return true;
  }
  return false;
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

function openSourceImportLinkFromPointer(event) {
  if (sourceEditorBlockSelection || !sourceDocumentSupportsEditableTargets()) {
    return false;
  }
  const link = sourceImportLinkAtPointer(event);
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
  const focus = sourceEditorPositionFromPoint(event.clientX, event.clientY);
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
  sourceEditorBlockSelection.draggingPointerId = null;
  renderSourceBlockSelection();
}

function sourceEditorPositionFromPoint(clientX, clientY) {
  const lines = sourceLinesWithOffsets(sourceEditor.value);
  if (!lines.length) {
    return null;
  }
  const visualOffset = sourceOffsetFromVisualPoint(clientX, clientY);
  if (Number.isInteger(visualOffset)) {
    return sourceLineColumnForOffset(lines, visualOffset);
  }
  const rect = sourceEditor.getBoundingClientRect();
  const style = window.getComputedStyle(sourceEditor);
  const paddingLeft = Number.parseFloat(style.paddingLeft) || 0;
  const paddingTop = Number.parseFloat(style.paddingTop) || 0;
  const lineHeight = sourceEditorLineHeight();
  const charWidth = sourceEditorCharWidth();
  const x = clientX - rect.left + sourceEditor.scrollLeft - paddingLeft;
  const y = clientY - rect.top + sourceEditor.scrollTop - paddingTop;
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
  sourceBlockSelectionLayer.hidden = !ranges.length;
  sourceBlockSelectionLayer.replaceChildren();
  if (!ranges.length) {
    return;
  }
  const style = window.getComputedStyle(sourceEditor);
  const paddingLeft = Number.parseFloat(style.paddingLeft) || 0;
  const paddingTop = Number.parseFloat(style.paddingTop) || 0;
  const lineHeight = sourceEditorLineHeight();
  const charWidth = sourceEditorCharWidth();
  for (const range of ranges) {
    const rect = document.createElement("div");
    rect.className = "source-block-selection-range";
    rect.style.left = `${paddingLeft + (range.startCol * charWidth) - sourceEditor.scrollLeft}px`;
    rect.style.top = `${paddingTop + (range.lineIndex * lineHeight) - sourceEditor.scrollTop}px`;
    rect.style.width = `${Math.max(2, (range.endCol - range.startCol) * charWidth)}px`;
    rect.style.height = `${lineHeight}px`;
    sourceBlockSelectionLayer.append(rect);
  }
}

function clearSourceBlockSelection() {
  sourceEditorBlockSelection = null;
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
  if (event.metaKey || event.ctrlKey) {
    return false;
  }
  if (event.key === "Backspace") {
    deleteSourceBlockSelection(-1);
  } else if (event.key === "Delete") {
    deleteSourceBlockSelection(1);
  } else if (event.key === "Tab") {
    replaceSourceBlockSelection("\t");
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

sourceEditor.addEventListener("keydown", (event) => {
  if (!isTextDocument(documents[currentDocumentIndex])) {
    return;
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
    if (event.key === "Enter" || event.key === "Tab") {
      event.preventDefault();
      acceptSourceCompletion();
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      hideSourceCompletions();
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
  if (event.key === "Escape" && sourceColorEdit && !sourceColorPopover?.hidden) {
    hideSourceColorEditor();
    return;
  }
  if (handleSourceEditorEmacsBinding(event)) {
    return;
  }
  if (handleSourceEditorVsCodeShortcut(event)) {
    return;
  }
  if (handleSourceEditorArrowNavigation(event)) {
    return;
  }
  if (handleSourceBraceAssist(event)) {
    return;
  }
  if (event.key === "Tab") {
    event.preventDefault();
    if (event.shiftKey || sourceEditor.value.slice(sourceEditor.selectionStart, sourceEditor.selectionEnd).includes("\n")) {
      indentSourceSelectedLines(event.shiftKey ? -1 : 1);
      return;
    }
    insertAtSelection("\t");
    return;
  }

  if (event.key !== "Enter") {
    return;
  }

  event.preventDefault();
  insertAtSelection(nextLineIndent());
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
    documents[currentDocumentIndex].source = sourceEditor.value;
  }
  scheduleSourceHighlight();
  scheduleLocalSave();
  schedulePreview();
  hideSourceCompletions();
}

function setSourceEditorText(value, selectionStart = null, selectionEnd = selectionStart) {
  sourceEditor.value = value || "";
  invalidateSourceTargetHintFrames();
  hideSourceColorEditor();
  hideSourceCompletions();
  clearSourceBlockSelection();
  if (selectionStart !== null) {
    sourceEditor.setSelectionRange(selectionStart, selectionEnd ?? selectionStart);
  }
  recordSourceUndoSnapshot();
  updateSourceMeta();
  if (documents[currentDocumentIndex]) {
    documents[currentDocumentIndex].source = sourceEditor.value;
  }
  scheduleSourceHighlight();
  resetLevelBuilderFromSource(false);
}

function bindSourceEditorPopoverEvents() {
sourceEditor.addEventListener("scroll", syncSourceHighlightScroll);
sourceEditor.addEventListener("scroll", hideSourceColorEditor);
sourceEditor.addEventListener("scroll", hideSourceCompletions);
sourceEditor.addEventListener("scroll", hideSourceImportLinkFrame);
sourceEditor.addEventListener("scroll", renderSourceBlockSelection);
sourceEditor.addEventListener("scroll", renderSourceCaret);
sourceColorCodeInput?.addEventListener("input", () => updateSourceColorFromPopover({ fromCode: true }));
sourceColorCodeInput?.addEventListener("change", () => updateSourceColorFromPopover({ fromCode: true }));
sourceColorCodeInput?.addEventListener("keydown", (event) => {
  event.stopPropagation();
  if (event.key !== "Enter") {
    return;
  }
  event.preventDefault();
  updateSourceColorFromPopover({ fromCode: true });
});
document.addEventListener("mousedown", (event) => {
  if (sourceColorPopover?.hidden) {
    return;
  }
  if (sourceEditorWrap?.contains(event.target)) {
    return;
  }
  hideSourceColorEditor();
});
window.addEventListener("resize", syncSourceHighlightScroll);
window.addEventListener("resize", hideSourceColorEditor);
window.addEventListener("resize", renderSourceBlockSelection);
window.addEventListener("resize", renderSourceCaret);
}

function nextLineIndent() {
  const source = sourceEditor.value;
  const start = sourceEditor.selectionStart;
  const lineStart = source.lastIndexOf("\n", start - 1) + 1;
  const lineBeforeCursor = source.slice(lineStart, start);
  const currentIndent = lineBeforeCursor.match(/^[\t ]*/)?.[0] || "";
  const extraIndent = lineBeforeCursor.trimEnd().endsWith("{") ? "\t" : "";
  const afterCursor = source.slice(sourceEditor.selectionEnd);
  const nextNonWhitespace = afterCursor.match(/^\s*(.)/)?.[1] || "";

  if (extraIndent && nextNonWhitespace === "}") {
    return `\n${currentIndent}${extraIndent}\n${currentIndent}`;
  }
  return `\n${currentIndent}${extraIndent}`;
}

function sourceLevelNameControlEntries(config = {}) {
  const source = String(config.source || "");
  const requestedScope = String(config.scopeValue || "").trim();
  const findRanges = config.findRanges || (() => []);
  const findDefinitions = config.findDefinitions || (() => []);
  const rangeScope = config.rangeScope || (() => "");
  const entryName = config.entryName || ((entry) => entry?.name || "");
  const optionValue = config.optionValue || ((entry) => entryName(entry));
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
      entries.push({ range, entry, name, value });
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
    if (entry.name !== entry.value) {
      option.label = entry.name;
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
    button.textContent = entry.value;
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
