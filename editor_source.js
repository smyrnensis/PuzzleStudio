// @ts-check
/**
 * Analysis operations supplied by static/editor_runtime.js after bootstrap.
 * Their transport results remain external to SourceEditorPort's buffer/event
 * contract. Bootstrap exposes unknown because it can precede runtime startup.
 * @typedef {object} SourceAnalysisHost
 * @property {(source:string,cursorOffset:number) => Promise<string>} suggestSourceCompletions
 * @property {(source:string) => Promise<unknown>} resetSourceAnalysis
 * @property {(changes:import('../web/src/source_editor_port').SourceChange[],source:string) => Promise<unknown>} applySourceAnalysisEdits
 */
// Source editor state, text-editing commands, highlighting, completions, color editing, and SourceEditorPort event binding.
const sourceColorPopover = createSourceColorPopover();
const sourceCompletionPopover = createSourceCompletionPopover();
const sourceCompletionTextEncoder = new TextEncoder();
const sourceFindPanel = createSourceFindPanel();
const sourceFindInput = /** @type {HTMLInputElement | null} */ (sourceFindPanel?.querySelector("input[data-source-find-input]"));
const sourceReplaceInput = /** @type {HTMLInputElement | null} */ (sourceFindPanel?.querySelector("input[data-source-replace-input]"));
const sourceFindStatus = sourceFindPanel?.querySelector("[data-source-find-status]");
const sourceFindCaseButton = sourceFindPanel?.querySelector("[data-source-find-case]");
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
    kind: "visual3d",
    label: "3D visual",
    openOptions: { switchMode: true },
  },
  {
    kind: "visual",
    label: "visual",
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
let sourceCompletionTimer = 0;
let sourceOutlineTimer = 0;
let activeHighlightRequest = null;
let sourceHighlightRequestId = 0;
let sourceCompletionRequestId = 0;
let sourceLineAddRequestId = 0;
let sourceEntriesRefreshRequestId = 0;
/** @type {{source:string,promise:Promise<unknown>}} */
let sourceAnalysisRevision = {
  source: "",
  promise: Promise.resolve(),
};
let sourceOutlineRequestId = 0;
let sourceOutlineSignature = "";
let sourceOutlineDirty = true;
let sourceColorEdit = null;
let sourceCompletionState = null;
let sourceFindState = {
  matches: [],
  selectedIndex: -1,
  matchCase: false,
  replaceVisible: false,
};
let sourceHighlightSource = "";
let sourceHighlightUnavailableStatusShown = false;
let sourceLevelBuilderResetFrame = 0;
let sourceLevelBuilderResetCells = false;
let sourceLevelBuilderResetSignature = null;
let sourceOutlineItems = [];
let sourceOutlineExpandedItemIds = new Set();

function sourcePuzzleLevelName(value, defaultName = "") {
  const text = String(value ?? "").trim();
  return text || String(defaultName ?? "").trim();
}

function sourceEditorDocumentValue() {
  return sourceEditor.documentText();
}

function setSourceEditorValue(value, options = {}) {
  const nextValue = value || "";
  const currentValue = sourceEditorDocumentValue();
  const preservesUndo = options.preserveUndoOnSameValue === true && currentValue === nextValue;
  const preserveCurrentHighlight = options.preserveHighlight !== false;
  if (currentValue === nextValue && preserveCurrentHighlight && sourceHighlightSource === nextValue) {
    updateSourceMeta();
    if (sourceDocumentSupportsEditableTargets()) {
      scheduleSourceOutlineRefresh(true, { force: true });
    } else {
      resetSourcePuzzleAnalysisState();
    }
    return;
  }
  sourceEditor.replaceDocument(nextValue, {
    preserveHistory: preservesUndo,
  });
  updateSourceMeta();
  if (sourceDocumentSupportsEditableTargets()) {
    scheduleSourceHighlight(true, { preserveCurrent: preserveCurrentHighlight });
    scheduleSourceOutlineRefresh(true, { force: true });
  } else {
    resetSourcePuzzleAnalysisState();
  }
}

function scheduleSourceHighlight(immediate = false, options = {}) {
  void options;
  if (!sourceDocumentSupportsEditableTargets()) {
    resetSourcePuzzleAnalysisState();
    return;
  }
  window.clearTimeout(sourceHighlightTimer);
  sourceHighlightTimer = window.setTimeout(() => {
    sourceHighlightTimer = 0;
    refreshSourceHighlight();
  }, immediate ? 0 : 140);
}

function resetSourcePuzzleAnalysisState() {
  window.clearTimeout(sourceHighlightTimer);
  sourceHighlightTimer = 0;
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
  sourceEntriesRefreshRequestId += 1;
  sourceOutlineItems = [];
  sourceOutlineDirty = true;
  sourceOutlineSignature = "";
  sourceCursorPreviewKey = "";
  sourceCursorResolveSignature = null;
  sourceCursorResolveRegion = null;
  hideSourceCompletions();
  hideSourceLineAdd();
  sourceEditor.clearHighlights();
  sourceHighlightSource = "";
  renderSourceOutlineEmpty("No outline");
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
    sourceLevelBuilderResetFrame = 0;
    sourceLevelBuilderResetCells = false;
    resetLevelBuilderFromSource();
  });
}

function renderPlainSourceHighlight(source = sourceEditor.documentText(), reason = null) {
  sourceEditor.clearHighlights();
  sourceHighlightSource = String(source || "");
  if (reason) {
    const message = `Source highlighting unavailable: ${userFacingRuntimeError(reason)}`;
    if (!sourceHighlightUnavailableStatusShown && typeof setEditorStatus === "function") {
      sourceHighlightUnavailableStatusShown = true;
      setEditorStatus(message, "is-error");
    }
    console.warn(message);
  }
}

function sourceScrollTop() {
  return sourceEditor.scrollTop();
}

function sourceScrollLeft() {
  return sourceEditor.scrollLeft();
}

function setSourceScrollTop(value) {
  sourceEditor.scrollTop(value);
}

function setSourceScrollLeft(value) {
  sourceEditor.scrollLeft(value);
}

function sourceViewportHeight() {
  return sourceEditor.viewportSize().height;
}

function sourceViewportWidth() {
  return sourceEditor.viewportSize().width;
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
  if (!isPuzzleDocument(document) || !isTextDocument(document)) {
    return;
  }

  if (activeHighlightRequest) {
    activeHighlightRequest.abort();
  }
  const source = sourceEditorDocumentValue();
  const displaySource = sourceEditor.documentText() || "";
  const range = sourceEditor.highlightViewportRange();
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
      || displaySource !== (sourceEditor.documentText() || "")
    ) {
      return;
    }
    const payload = JSON.parse(text);
    if (payload.outline) {
      applySourceOutlinePayload(payload.outline, source);
    }
    sourceEditor.applyHighlightRange(source, range, payload);
    sourceHighlightSource = source;
    sourceHighlightUnavailableStatusShown = false;
  } catch (error) {
    if (error.name === "AbortError") {
      return;
    }
    if (
      requestId !== sourceHighlightRequestId
      || source !== sourceEditorDocumentValue()
      || displaySource !== (sourceEditor.documentText() || "")
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
    const text = await window.PuzzleStudioHost.sourceOutline({
      source,
    });
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
  try {
    sourceEditor.applyFoldRanges(source, payload);
  } catch (error) {
    const message = `Source folding unavailable: ${userFacingRuntimeError(error)}`;
    if (typeof setEditorStatus === "function") {
      setEditorStatus(message, "is-error");
    }
    console.warn(message);
  }
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
  for (const row of /** @type {NodeListOf<HTMLElement>} */ (sourceOutlineList.querySelectorAll("[data-source-outline-id]"))) {
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
    button.className = "navigation-row source-outline-row";
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
  return editorIconSvg(expanded ? "chevron-down" : "chevron-right", {
    className: "source-outline-chevron-icon",
  });
}

const SOURCE_OUTLINE_KIND_ICON_NAMES = Object.freeze({
  "puzzle": "puzzle",
  "puzzle3": "puzzle",
  "levels": "map",
  "level": "map",
  "visuals": "image",
  "visual": "image",
  "objects": "boxes",
  "object": "box",
  "groups": "group",
  "tags": "tag",
  "marks": "bookmark",
  "render": "scan-eye",
  "camera": "camera",
  "grid": "grid-2x2",
  "viewport": "view",
  "state": "database",
  "input_buffer": "settings",
  "lighting": "settings",
  "pixelate": "settings",
  "animation": "circle-play",
  "tween": "chart-spline",
  "row": "rows-3",
  "column": "columns-3",
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
  "asset": "package",
  "resources": "package",
  "legend": "move-horizontal",
  "map": "arrow-right",
  "theme": "swatch-book",
  "colors": "palette",
  "shapes": "shapes",
  "shape": "shapes",
  "sounds": "volume-2",
  "sfx": "volume-2",
  "music": "volume-2",
  "keys": "keyboard",
  "layers": "layers",
  "collision_layers": "layers",
  "metadata": "info",
  "slots": "square-dashed",
  "fix": "wrench",
  "before_rules": "list-checks",
  "after_rules": "list-checks",
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
  return editorIconSvg(sourceOutlineKindIconName(kind), {
    className: "source-outline-icon",
  });
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
  const row = /** @type {HTMLElement | null} */ (sourceOutlineList
    ?.querySelector(`[data-source-outline-id="${CSS.escape(itemId)}"]`));
  row?.focus({ preventScroll: true });
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
  }, {
    scrollAlignment: "center",
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
    : sourceEditor.selection().from;
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
  for (const row of /** @type {NodeListOf<HTMLElement>} */ (sourceOutlineList.querySelectorAll("[data-source-outline-id]"))) {
    row.classList.toggle("is-active", activeId === row.dataset.sourceOutlineId);
  }
}

async function suggestSourceCompletionsWithWasm(source, cursorOffset) {
  if (typeof /** @type {SourceAnalysisHost} */ (window.PuzzleStudioRuntime)?.suggestSourceCompletions !== "function") {
    return null;
  }
  const json = await /** @type {SourceAnalysisHost} */ (window.PuzzleStudioRuntime).suggestSourceCompletions(source, cursorOffset);
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
    const item = /** @type {HTMLElement | null} */ ((event.target instanceof Element ? event.target : null)?.closest("[data-source-completion-index]"));
    if (!item || !sourceCompletionState) {
      return;
    }
    acceptSourceCompletion(Number(item.dataset.sourceCompletionIndex));
  });
  sourceEditorWrap.append(popover);
  return popover;
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
      <button class="icon-button source-find-icon-button" data-source-find-case type="button" aria-label="Match case" title="Match case" aria-pressed="false">Aa</button>
      <button class="icon-button source-find-icon-button" data-source-find-previous type="button" aria-label="Previous match" title="Previous match">
        ${editorIconSvg("chevron-up")}
      </button>
      <button class="icon-button source-find-icon-button" data-source-find-next type="button" aria-label="Next match" title="Next match">
        ${editorIconSvg("chevron-down")}
      </button>
      <button class="icon-button source-find-icon-button" data-source-find-close type="button" aria-label="Close find" title="Close">
        ${editorIconSvg("x")}
      </button>
    </div>
    <div class="source-find-row source-replace-row">
      <input class="source-find-input" data-source-replace-input type="text" autocomplete="off" autocapitalize="off" spellcheck="false" placeholder="Replace" aria-label="Replace with">
      <button class="icon-button source-find-icon-button" data-source-replace-current type="button" aria-label="Replace" title="Replace">
        ${editorIconSvg("replace")}
      </button>
      <button class="icon-button source-find-icon-button" data-source-replace-all type="button" aria-label="Replace all" title="Replace all">
        ${editorIconSvg("replace-all")}
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

function setSourceLineAddVisible(source, cursor, visible) {
  sourceEditor.setAddLineOverlay(source, cursor, visible);
}

function hideSourceLineAdd() {
  sourceLineAddRequestId += 1;
  setSourceLineAddVisible(sourceEditor.documentText(), sourceEditor.selection().from, false);
}

function sourceLineAddEligible(source, cursor) {
  if (
    sourceEditor.isReadOnly()
    || !sourceEditor.hasFocus()
    || sourceEditor.selection().from !== sourceEditor.selection().to
  ) {
    return false;
  }
  const lineStart = source.lastIndexOf("\n", Math.max(0, cursor - 1)) + 1;
  const nextLine = source.indexOf("\n", cursor);
  const lineEnd = nextLine < 0 ? source.length : nextLine;
  return source.slice(lineStart, lineEnd).trim() === "";
}

function sourceCompletionItemsForRequest(list, source, cursor) {
  const items = filterSourceCompletionsForTypedReplacement(
    list?.items || [],
    list,
    source,
    cursor,
  );
  return items;
}

async function refreshSourceLineAdd() {
  if (!sourceDocumentSupportsEditableTargets()) {
    hideSourceLineAdd();
    return;
  }
  const document = activeDocument();
  const source = sourceEditor.documentText();
  const cursor = sourceEditor.selection().from;
  if (!isPuzzleDocument(document) || !isTextDocument(document) || !sourceLineAddEligible(source, cursor)) {
    hideSourceLineAdd();
    return;
  }
  const requestId = ++sourceLineAddRequestId;
  try {
    const list = await suggestSourceCompletionsWithWasm(source, cursor);
    if (
      requestId !== sourceLineAddRequestId
      || source !== sourceEditor.documentText()
      || cursor !== sourceEditor.selection().from
    ) {
      return;
    }
    const items = sourceCompletionItemsForRequest(list, source, cursor);
    setSourceLineAddVisible(source, cursor, items.length > 0);
  } catch (error) {
    if (
      requestId !== sourceLineAddRequestId
      || source !== sourceEditor.documentText()
      || cursor !== sourceEditor.selection().from
    ) {
      return;
    }
    hideSourceLineAdd();
    console.error("Source line additions unavailable", error);
  }
}

async function showSourceCompletions(options = {}) {
  const document = activeDocument();
  if (!sourceCompletionPopover || !isPuzzleDocument(document) || !isTextDocument(document)) {
    hideSourceCompletions();
    return false;
  }
  const source = sourceEditor.documentText();
  const cursor = sourceEditor.selection().from;
  if (!options.manual && !sourceAutoCompletionEligible(source, cursor)) {
    hideSourceCompletions();
    return false;
  }
  const requestId = ++sourceCompletionRequestId;
  try {
    const list = await suggestSourceCompletionsWithWasm(source, cursor);
    if (requestId !== sourceCompletionRequestId || source !== sourceEditor.documentText() || cursor !== sourceEditor.selection().from) {
      return false;
    }
    const items = sourceCompletionItemsForRequest(list, source, cursor);
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
    sourceEditor.selection().from !== sourceEditor.selection().to
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
  const after = stripSourceStructureLineComment(source.slice(cursor, safeLineEnd));
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
    && sourceEditor.documentText() === sourceCompletionState.source
    && sourceEditor.selection().from === sourceEditor.selection().to
    && sourceEditor.selection().from === sourceCompletionState.cursor
  );
}

function sourceCursorInLineLeadingWhitespace() {
  const source = sourceEditor.documentText() || "";
  const cursor = sourceEditor.selection().from;
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
  const replaceStart = Math.max(0, Math.min(sourceEditor.documentText().length, sourceCompletionState.replaceStart));
  const replaceEnd = Math.max(replaceStart, Math.min(sourceEditor.documentText().length, sourceCompletionState.replaceEnd));
  sourceEditor.replaceRange(insertText, replaceStart, replaceEnd, "end");
  hideSourceCompletions();
  updateSourceMeta();
  if (documents[currentDocumentIndex]) {
    documents[currentDocumentIndex].source = sourceEditorDocumentValue();
  }
  scheduleSourceHighlight();
  scheduleLocalSave();
  resetLevelBuilderFromSource();
  schedulePreview();
  return true;
}

function positionSourceCompletionPopover() {
  if (!sourceCompletionPopover || !sourceEditorWrap || !sourceEditor) {
    return;
  }
  const anchor = Math.max(
    0,
    Math.min(sourceEditor.documentText().length, sourceCompletionState?.replaceStart ?? sourceEditor.selection().from),
  );
  const wrapRect = sourceEditorWrap.getBoundingClientRect();
  const anchorRect = sourceCaretRectForOffset(anchor);
  const cursorRect = sourceCaretRectForOffset(sourceEditor.selection().from);
  if (!anchorRect || !cursorRect) {
    return;
  }
  const margin = 8;
  const gap = 6;
  const popoverWidth = Math.min(276, Math.max(0, wrapRect.width - margin * 2));
  const maxLeft = Math.max(wrapRect.left + margin, wrapRect.right - popoverWidth - margin);
  const left = wrapRect.left + anchorRect.left;
  const caretTop = wrapRect.top + cursorRect.top;
  const caretBottom = caretTop + cursorRect.height;
  const viewportTop = Math.max(margin, wrapRect.top + margin);
  const viewportBottom = Math.min(window.innerHeight - margin, wrapRect.bottom - margin);
  const availableBelow = Math.max(0, viewportBottom - caretBottom - gap);
  const availableAbove = Math.max(0, caretTop - viewportTop - gap);
  const desiredHeight = Math.min(216, Math.max(38, (sourceCompletionState?.items?.length || 1) * 28 + 10));
  const placeBelow = availableBelow >= desiredHeight || availableBelow >= availableAbove;
  const available = placeBelow ? availableBelow : availableAbove;
  const height = Math.max(0, Math.min(desiredHeight, available));
  const top = placeBelow
    ? caretBottom + gap
    : caretTop - gap - height;
  sourceCompletionPopover.dataset.placement = placeBelow ? "below" : "above";
  sourceCompletionPopover.style.left = `${Math.max(wrapRect.left + margin, Math.min(maxLeft, left))}px`;
  sourceCompletionPopover.style.top = `${Math.max(viewportTop, top)}px`;
  sourceCompletionPopover.style.maxHeight = `${height}px`;
}

function sourceFindShortcutRequested(event) {
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
  syncSourceFindPanelLayout();
  syncSourceFindMatches({ select: Boolean(sourceFindInput.value), anchor: sourceEditor.selection().from });
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
  syncSourceFindPanelLayout();
  sourceFindState.matches = [];
  sourceFindState.selectedIndex = -1;
  renderSourceFindMatches();
  if (options.focusEditor !== false) {
    sourceEditor.focus({ preventScroll: true });
  }
}

function sourceFindSeedFromSelection() {
  const start = Math.min(sourceEditor.selection().from, sourceEditor.selection().to);
  const end = Math.max(sourceEditor.selection().from, sourceEditor.selection().to);
  const value = sourceEditor.documentText().slice(start, end);
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
  if (isSourceFindPanelOpen()) {
    syncSourceFindPanelLayout();
  }
}

function syncSourceFindPanelLayout() {
  const open = isSourceFindPanelOpen();
  sourceEditorWrap?.classList.toggle("has-source-find-panel", open);
  if (!sourceEditorWrap) {
    return;
  }
  if (!open) {
    sourceEditorWrap.style.removeProperty("--source-find-panel-space");
    return;
  }
  sourceEditorWrap.style.setProperty(
    "--source-find-panel-space",
    `${Math.ceil(sourceFindPanel.getBoundingClientRect().height)}px`,
  );
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
    match.start === sourceEditor.selection().from && match.end === sourceEditor.selection().to
  );
  if (exactIndex >= 0) {
    sourceFindState.selectedIndex = exactIndex;
  } else if (options.keepIndex && sourceFindState.matches[sourceFindState.selectedIndex]) {
    sourceFindState.selectedIndex = Math.max(0, Math.min(sourceFindState.matches.length - 1, sourceFindState.selectedIndex));
  } else {
    const anchor = Number.isInteger(options.anchor) ? options.anchor : sourceEditor.selection().to;
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
  const source = sourceEditor.documentText() || "";
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
  sourceEditor.setSelection(match.start, match.end);
  scrollSourceOffsetIntoView(match.start, "start");
  if (options.focusEditor) {
    sourceEditor.focus({ preventScroll: true });
  }
  updateSourceFindStatus();
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
  sourceEditor.replaceRange(replacement, match.start, match.end, "select");
  const nextAnchor = match.start + replacement.length;
  sourceEditor.setSelection(match.start, nextAnchor);
  sourceEditorContentChanged();
  syncSourceFindMatches({ anchor: nextAnchor });
  return true;
}

function replaceAllSourceFindMatches() {
  if (!isSourceFindPanelOpen() || !sourceReplaceInput || !sourceFindState.matches.length) {
    return false;
  }
  const matches = [...sourceFindState.matches];
  const replacement = sourceReplaceInput.value || "";
  const source = sourceEditor.documentText() || "";
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
  sourceEditor.replaceRange(output, 0, source.length, "start");
  sourceEditor.setSelection(firstStart, firstEnd);
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
    match.start === sourceEditor.selection().from && match.end === sourceEditor.selection().to
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
  const matches = isSourceFindPanelOpen() && isTextDocument(activeDocument())
    ? sourceFindState.matches
    : [];
  sourceEditor.applyFindMatches(
    sourceEditor.documentText(),
    matches,
    sourceFindState.selectedIndex,
  );
}

/** @param {number} offset
 * @param {Parameters<import("../web/src/source_editor_port").SourceEditorPort["scrollIntoView"]>[1]} alignment */
function scrollSourceOffsetIntoView(offset, alignment = "nearest") {
  sourceEditor.scrollIntoView(offset, alignment);
}

function sourceVisualCaretPoint(offset) {
  const rect = sourceCaretRectForOffset(offset);
  return rect ? { left: rect.left, top: rect.top } : null;
}

function sourceCaretRectForOffset(offset) {
  const source = sourceEditor.documentText();
  const safeOffset = Math.max(0, Math.min(source.length, offset || 0));
  const rect = sourceEditor.coordsAtOffset(safeOffset);
  if (!rect) {
    return null;
  }
  const wrapRect = sourceEditorWrap.getBoundingClientRect();
  return {
    left: rect.left - wrapRect.left,
    top: rect.top - wrapRect.top,
    height: rect.bottom - rect.top,
  };
}

function sourceVisualOffsetFromPoint(clientX, clientY) {
  const offset = sourceEditor.offsetAtCoords(clientX, clientY);
  return Number.isInteger(offset) ? offset : null;
}

function sourceOffsetFromVisualPointer(event, source = sourceEditorDocumentValue()) {
  if (!event || !sourceEditorWrap?.contains(event.target)) {
    return null;
  }
  const offset = sourceVisualOffsetFromPoint(event.clientX, event.clientY);
  if (!Number.isInteger(offset)) {
    return null;
  }
  return Math.max(0, Math.min(String(source || "").length, offset));
}

function sourceViewOffsetFromVisualPoint(clientX, clientY) {
  const offset = sourceVisualOffsetFromPoint(clientX, clientY);
  if (!Number.isInteger(offset)) {
    return null;
  }
  return Math.max(0, Math.min((sourceEditor.documentText() || "").length, offset));
}

function sourceOffsetFromVisualPoint(clientX, clientY, source = sourceEditorDocumentValue()) {
  const offset = sourceViewOffsetFromVisualPoint(clientX, clientY);
  if (!Number.isInteger(offset)) {
    return null;
  }
  return Math.max(0, Math.min(String(source || "").length, offset));
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

function showSourceColorEditor(event = null, visualOffset = null) {
  if (!sourceColorPopover) {
    return false;
  }
  if (!isTextDocument(activeDocument())) {
    hideSourceColorEditor();
    return false;
  }
  const token = sourceColorTokenAt(sourceEditor.selection().from);
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
  const start = sourceEditor.selection().from || 0;
  const end = sourceEditor.selection().to || start;
  return start >= token.start && end <= token.end;
}

function sourceColorTokenAt(offset) {
  const span = sourceEditor.highlightSpanAt(offset, "color");
  if (!span) {
    return null;
  }
  const value = sourceEditor.documentText().slice(span.start, span.end);
  return parseHexColorToken(value)
    ? { start: span.start, end: span.end, value }
    : null;
}

function currentSourceColorEdit() {
  if (!sourceColorEdit) {
    return null;
  }
  return sourceEditor.documentText().slice(sourceColorEdit.start, sourceColorEdit.end)
    === sourceColorEdit.value
    ? sourceColorEdit
    : null;
}

function applySourceColorRgb(rgb) {
  if (!sourceColorEdit) {
    return;
  }
  const current = currentSourceColorEdit();
  if (!current) {
    hideSourceColorEditor();
    return;
  }
  const parsedColor = parseHexColorToken(rgb);
  if (!parsedColor) {
    return;
  }
  const next = formatHexColorToken(parsedColor.rgb, parsedColor.alpha);
  sourceEditor.replaceRange(next, current.start, current.end, "preserve");
  sourceColorEdit = { start: current.start, end: current.start + next.length, value: next };
  sourceEditor.setSelection(sourceColorEdit.start, sourceColorEdit.end);
  const parsedNext = parseHexColorToken(next);
  const colorEditor = /** @type {PuzzleStudioColorEditorElement | null} */ (sourceColorPopover?.querySelector(".color-editor"));
  colorEditor?.syncColor?.(parsedNext ? next : rgb);
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
  resetLevelBuilderFromSource();
  schedulePreview();
}

function refreshSourceColorEditor() {
  if (!sourceColorEdit) {
    return;
  }
  const current = currentSourceColorEdit();
  if (!current) {
    hideSourceColorEditor();
    return;
  }
  if (sourceEditor.hasFocus() && !sourceColorSelectionTargetsToken(current)) {
    hideSourceColorEditor();
  }
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

function syncSourceAnalysisEditorChanges(changes, source) {
  if (!Array.isArray(changes)) {
    throw new Error("CodeMirror edit must provide source analysis changes.");
  }
  const editedSource = String(source || "");
  const entriesRefreshRequestId = ++sourceEntriesRefreshRequestId;
  const analysisEdit = /** @type {SourceAnalysisHost} */ (window.PuzzleStudioRuntime).applySourceAnalysisEdits(changes, editedSource);
  const completion = analysisEdit.then(() => {
    if (
      entriesRefreshRequestId !== sourceEntriesRefreshRequestId
      || editedSource !== sourceEditorDocumentValue()
      || typeof refreshSurfaceEntriesForActiveSource !== "function"
    ) {
      return null;
    }
    return refreshSurfaceEntriesForActiveSource(editedSource);
  });
  sourceAnalysisRevision = {
    source: editedSource,
    promise: completion,
  };
  void completion.catch((error) => {
    if (
      entriesRefreshRequestId === sourceEntriesRefreshRequestId
      && editedSource === sourceEditorDocumentValue()
    ) {
      console.error("Source analysis revision failed", error);
    }
  });
  return completion;
}

function sourceEditorAnalysisRevisionReady(source = sourceEditorDocumentValue()) {
  const expectedSource = String(source || "");
  const pending = sourceAnalysisRevision.source === expectedSource
    ? sourceAnalysisRevision.promise
    : Promise.resolve();
  return pending.then(() => {
    if (sourceEditorDocumentValue() !== expectedSource) {
      throw new Error("Source changed before its analysis revision became ready.");
    }
    return loadSurfaceEntriesForSource(expectedSource, { reportUnavailable: true });
  });
}

function bindSourceEditorEvents() {
sourceEditor.on("sourceanalysisreset", () => {
  const source = sourceEditorDocumentValue();
  sourceEntriesRefreshRequestId += 1;
  const reset = /** @type {SourceAnalysisHost} */ (window.PuzzleStudioRuntime).resetSourceAnalysis(source);
  sourceAnalysisRevision = {
    source,
    promise: reset,
  };
  void reset.then(() => {
    if (source === sourceEditorDocumentValue()) {
      void refreshSourceLineAdd();
    }
  }).catch((error) => {
    console.error("Source analysis reset failed", error);
  });
});
sourceEditor.on("sourceanalysisedit", (event) => {
  syncSourceAnalysisEditorChanges(event.detail?.changes, event.detail?.source);
});
sourceEditor.on("change", (event) => {
  if (!isTextDocument(documents[currentDocumentIndex])) {
    return;
  }
  const sourceChanges = event.detail?.changes;
  const editedSource = sourceEditorDocumentValue();
  syncSourceAnalysisEditorChanges(sourceChanges, editedSource);
  const puzzleSource = sourceDocumentSupportsEditableTargets();
  if (puzzleSource) {
    scheduleSourceHighlight();
    scheduleSourceOutlineRefresh();
  } else {
    resetSourcePuzzleAnalysisState();
  }
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
    void refreshSourceLineAdd();
    scheduleLevelBuilderResetFromSource(false);
    scheduleSourceCursorPreviewSync();
    schedulePreview();
  }
});
sourceEditor.on("sourceviewportchange", () => {
  if (sourceDocumentSupportsEditableTargets()) {
    scheduleSourceHighlight(true);
  }
});
sourceEditor.on("click", async (event) => {
  const interaction = sourceInteractionFromPointer(event);
  if (!interaction) {
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
sourceEditor.on("keyup", (event) => {
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
    return;
  }
  if (event.key.startsWith("Arrow") || event.key === "Home" || event.key === "End") {
    if (sourceDocumentSupportsEditableTargets()) {
      showSourceColorEditor();
      showSourceCompletions({ manual: false });
      scheduleSourceCursorPreviewSync();
    }
  }
});
sourceEditor.on("focus", () => {
  if (sourceDocumentSupportsEditableTargets()) {
    scheduleSourceCursorPreviewSync();
    void refreshSourceLineAdd();
  }
});
sourceEditor.on("blur", () => {
  hideSourceLineAdd();
});
sourceEditor.on("sourceselectionchange", () => {
  if (sourceDocumentSupportsEditableTargets()) {
    void refreshSourceLineAdd();
  }
});
document.addEventListener("selectionchange", () => {
  if (!sourceEditor.hasFocus()) {
    return;
  }
  if (sourceDocumentSupportsEditableTargets()) {
    scheduleSourceCursorPreviewSync();
    syncSourceOutlineActiveItem();
  }
  syncSourceFindIndexFromSelection();
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
  if ((event.target instanceof Element ? event.target : null)?.closest("button")) {
    event.preventDefault();
  }
  event.stopPropagation();
});
sourceFindInput?.addEventListener("input", () => syncSourceFindMatches({ anchor: sourceEditor.selection().from }));
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
  syncSourceFindMatches({ anchor: sourceEditor.selection().from });
});
sourceFindPanel?.querySelector("[data-source-find-previous]")?.addEventListener("click", () => moveSourceFindSelection(-1));
sourceFindPanel?.querySelector("[data-source-find-next]")?.addEventListener("click", () => moveSourceFindSelection(1));
sourceFindPanel?.querySelector("[data-source-find-close]")?.addEventListener("click", () => closeSourceFindPanel());
sourceFindPanel?.querySelector("[data-source-replace-current]")?.addEventListener("click", replaceCurrentSourceFindMatch);
sourceFindPanel?.querySelector("[data-source-replace-all]")?.addEventListener("click", replaceAllSourceFindMatches);
}

function sourceLineStart(position) {
  const source = sourceEditor.documentText();
  const clamped = Math.max(0, Math.min(source.length, position));
  return source.lastIndexOf("\n", clamped - 1) + 1;
}

function sourceEditorLineHeight() {
  return sourceEditor.lineHeight();
}

function handleSourceIndentBackspace(event) {
  if (event.key !== "Backspace" || event.altKey || event.ctrlKey || event.metaKey || event.isComposing) {
    return false;
  }
  const start = sourceEditor.selection().from;
  const end = sourceEditor.selection().to;
  if (start !== end) {
    return false;
  }
  const source = sourceEditor.documentText();
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
  sourceEditor.replaceRange("", removeStart, start, "start");
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
  const start = sourceEditor.selection().from;
  const end = sourceEditor.selection().to;
  const source = sourceEditor.documentText() || "";
  if (source.slice(start, end).includes("\n")) {
    return false;
  }
  const lineStart = source.lastIndexOf("\n", start - 1) + 1;
  const lineEnd = source.indexOf("\n", end);
  const safeLineEnd = lineEnd < 0 ? source.length : lineEnd;
  const lineBeforeSelection = source.slice(lineStart, start);
  const lineAfterSelection = source.slice(end, safeLineEnd);
  const codeBeforeSelection = stripSourceStructureLineComment(lineBeforeSelection);
  const codeAfterSelection = stripSourceStructureLineComment(lineAfterSelection);
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
  const selection = sourceEditor.documentText().slice(start, end);
  const replacement = `[ ${selection} ]`;
  sourceEditor.replaceRange(replacement, start, end, "end");
  const innerStart = start + 2;
  const innerEnd = innerStart + selection.length;
  sourceEditor.setSelection(innerStart, innerEnd, sourceEditor.selection().direction || "none");
  sourceEditorContentChanged();
}

function insertSourceBracePair() {
  const start = sourceEditor.selection().from;
  const end = sourceEditor.selection().to;
  const selection = sourceEditor.documentText().slice(start, end);
  sourceEditor.replaceRange(`{${selection}}`, start, end, "end");
  const innerStart = start + 1;
  const innerEnd = innerStart + selection.length;
  sourceEditor.setSelection(innerStart, innerEnd, sourceEditor.selection().direction || "none");
  sourceEditorContentChanged();
}

function handleSourceClosingBrace(event) {
  const start = sourceEditor.selection().from;
  const end = sourceEditor.selection().to;
  const source = sourceEditor.documentText();
  if (start === end && source[start] === "}") {
    event.preventDefault();
    event.stopPropagation();
    sourceEditor.setSelection(start + 1, start + 1);
    return true;
  }

  const lineStart = source.lastIndexOf("\n", start - 1) + 1;
  const linePrefix = source.slice(lineStart, start);
  if (start === end && /^[\t ]+$/.test(linePrefix)) {
    const indentStart = sourceClosingBraceIndentStart(lineStart, start, linePrefix);
    event.preventDefault();
    event.stopPropagation();
    sourceEditor.replaceRange("}", indentStart, end, "end");
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
  const start = sourceEditor.selection().from;
  const end = sourceEditor.selection().to;
  const source = sourceEditor.documentText();
  if (start !== end || source[start - 1] !== "{" || source[start] !== "}") {
    return false;
  }
  event.preventDefault();
  event.stopPropagation();
  sourceEditor.replaceRange("", start - 1, start + 1, "start");
  sourceEditorContentChanged();
  return true;
}

function handleSourceRewriteRhsPatternAssist(event) {
  if (event.altKey || event.ctrlKey || event.metaKey || event.isComposing || event.key !== "[") {
    return false;
  }
  const cursor = sourceEditor.selection().from;
  if (cursor !== sourceEditor.selection().to) {
    return false;
  }
  const source = sourceEditor.documentText() || "";
  const lineStart = source.lastIndexOf("\n", cursor - 1) + 1;
  const lineEnd = source.indexOf("\n", cursor);
  const safeLineEnd = lineEnd < 0 ? source.length : lineEnd;
  const line = source.slice(lineStart, safeLineEnd);
  const cursorColumn = cursor - lineStart;
  const code = stripSourceStructureLineComment(line);
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
  const rhsPattern = sourceEmptyRewritePattern(pattern);
  const firstSlot = sourceRewritePatternSlotOffsets(rhsPattern)[0];
  sourceEditor.replaceRange(rhsPattern, cursor, cursor, "end");
  if (Number.isInteger(firstSlot)) {
    const slot = cursor + firstSlot;
    sourceEditor.setSelection(slot, slot);
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
  if (stripSourceStructureLineComment(lineBeforeCursor).length !== lineBeforeCursor.length) {
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
    || sourceEditor.selection().from !== sourceEditor.selection().to
  ) {
    return false;
  }
  const source = sourceEditor.documentText() || "";
  const cursor = sourceEditor.selection().from;
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
  sourceEditor.setSelection(target, target);
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
    || sourceEditor.selection().from !== sourceEditor.selection().to
  ) {
    return false;
  }
  const source = sourceEditor.documentText() || "";
  const cursor = sourceEditor.selection().from;
  const lineStart = source.lastIndexOf("\n", cursor - 1) + 1;
  const lineEnd = source.indexOf("\n", cursor);
  const safeLineEnd = lineEnd < 0 ? source.length : lineEnd;
  const lineBeforeCursor = source.slice(lineStart, cursor);
  if (stripSourceStructureLineComment(lineBeforeCursor).length !== lineBeforeCursor.length) {
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
    if (!hasTrailingHorizontalSpace) {
      sourceEditor.replaceRange(" ", close + 1, close + 1, "end");
    }
    const cursorAfterCell = close + 2;
    sourceEditor.setSelection(cursorAfterCell, cursorAfterCell);
    sourceEditorContentChanged();
    return true;
  }
  const replacement = hasTrailingHorizontalSpace ? "[  ]" : "[  ] ";
  event.preventDefault();
  event.stopPropagation();
  sourceEditor.replaceRange(replacement, open, close + 1, "end");
  const cursorAfterCell = open + replacement.length + (hasTrailingHorizontalSpace ? 1 : 0);
  sourceEditor.setSelection(cursorAfterCell, cursorAfterCell);
  sourceEditorContentChanged();
  return true;
}

function handleSourceRewritePatternTab(event) {
  if (
    event.key !== "Tab"
    || event.altKey
    || event.ctrlKey
    || event.metaKey
    || sourceEditor.selection().from !== sourceEditor.selection().to
  ) {
    return false;
  }
  const source = sourceEditor.documentText() || "";
  const cursor = sourceEditor.selection().from;
  const lineStart = source.lastIndexOf("\n", cursor - 1) + 1;
  const lineEnd = source.indexOf("\n", cursor);
  const safeLineEnd = lineEnd < 0 ? source.length : lineEnd;
  const line = source.slice(lineStart, safeLineEnd);
  const code = stripSourceStructureLineComment(line);
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
      sourceEditor.replaceRange(lhsPattern, cursor, cursor, "end");
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
  sourceEditor.setSelection(fallbackTarget, fallbackTarget);
  updateSourceMeta();
  hideSourceCompletions();
  return true;
}

function sourceEditorContentChanged(options = {}) {
  const preserveCompletions = Boolean(options.preserveSourceCompletions && keepSourceCompletionsVisibleDuringEdit());
  const puzzleSource = sourceDocumentSupportsEditableTargets();
  if (puzzleSource) {
    scheduleSourceHighlight();
    scheduleSourceOutlineRefresh();
  } else {
    resetSourcePuzzleAnalysisState();
  }
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

function sourceLineIndexAtOffset(lines, offset) {
  const position = Math.max(0, offset);
  for (let index = 0; index < lines.length; index += 1) {
    if (position <= lines[index].end || index === lines.length - 1) {
      return index;
    }
  }
  return Math.max(0, lines.length - 1);
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
  const source = sourceEditor.documentText() || "";
  const safeStart = Math.max(0, Math.min(source.length, start));
  const safeEnd = Math.max(safeStart, Math.min(source.length, end));
  const startRect = sourceEditor.coordsAtOffset(safeStart);
  const endRect = sourceEditor.coordsAtOffset(safeEnd);
  if (!startRect || !endRect) {
    return null;
  }
  const wrapRect = sourceEditorWrap.getBoundingClientRect();
  const lineHeight = sourceEditorLineHeight();
  return {
    left: startRect.left - wrapRect.left - 2,
    top: startRect.top - wrapRect.top,
    width: Math.max(8, endRect.right - startRect.left + 4),
    height: Math.max(lineHeight, endRect.bottom - startRect.top),
  };
}

function loadSourceEditableTargetFromPosition(position, options = {}) {
  if (!sourceDocumentSupportsEditableTargets()) {
    return "";
  }
  const source = sourceEditor.documentText() || "";
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

function sourceEditorPositionFromPoint(clientX, clientY, options = {}) {
  const lines = editorSourceLinesWithOffsets(sourceEditor.documentText());
  if (!lines.length) {
    return null;
  }
  const visualOffset = Number.isInteger(options.visualOffset)
    ? options.visualOffset
    : sourceViewOffsetFromVisualPoint(clientX, clientY);
  if (Number.isInteger(visualOffset)) {
    return sourceLineColumnForOffset(lines, visualOffset);
  }
  return null;
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
  return {
    viewOffset,
    documentOffset: Math.max(0, Math.min(String(source || "").length, viewOffset)),
    position,
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

sourceEditor.on("sourcecompletioncommand", (event) => {
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

sourceEditor.on("sourcelineaddrequest", (event) => {
  if (!sourceDocumentSupportsEditableTargets()) {
    return;
  }
  const cursor = Math.max(0, Math.min(
    sourceEditor.documentText().length,
    Number(event.detail?.cursorOffset) || 0,
  ));
  event.preventDefault();
  sourceEditor.setSelection(cursor, cursor);
  sourceEditor.focus({ preventScroll: true });
  void showSourceCompletions({ manual: true });
});

function sourceEditingCommandKeyEvent(event) {
  const command = event.detail?.command || "";
  const key = command === "open-brace"
    ? "{"
    : command === "close-brace"
      ? "}"
      : command === "open-bracket"
        ? "["
        : command === "backspace"
          ? "Backspace"
          : command === "enter"
            ? "Enter"
            : "Tab";
  return {
    key,
    shiftKey: command === "shift-tab",
    altKey: false,
    ctrlKey: false,
    metaKey: false,
    isComposing: false,
    preventDefault() {
      event.preventDefault();
    },
    stopPropagation() {},
  };
}

sourceEditor.on("sourceeditingcommand", (event) => {
  if (!isTextDocument(documents[currentDocumentIndex])) {
    return;
  }
  const command = event.detail?.command || "";
  const keyEvent = sourceEditingCommandKeyEvent(event);
  if (command === "open-brace" || command === "close-brace" || command === "backspace") {
    if (handleSourceBraceAssist(keyEvent)) {
      return;
    }
    if (command === "backspace") {
      handleSourceIndentBackspace(keyEvent);
    }
    return;
  }
  if (command === "open-bracket") {
    if (handleSourceRewriteLhsBracketAssist(keyEvent)) {
      return;
    }
    handleSourceRewriteRhsPatternAssist(keyEvent);
    return;
  }
  if (command === "tab" || command === "shift-tab") {
    if (handleSourceRuleBracketCellSlotTab(keyEvent)) {
      return;
    }
    if (handleSourceRuleBracketCellTabExit(keyEvent)) {
      return;
    }
    handleSourceRewritePatternTab(keyEvent);
    return;
  }
  if (command === "enter") {
    const insert = nextLineIndent();
    if (sourceNewlineCursorOffset(insert) !== null) {
      event.preventDefault();
      insertSourceNewlineAtSelection();
    }
  }
});

sourceEditor.on("keydown", (event) => {
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
  }
});
function insertAtSelection(value) {
  sourceEditor.replaceRange(
    value,
    sourceEditor.selection().from,
    sourceEditor.selection().to,
    "end",
  );
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
  sourceEditor.replaceRange(value || "", 0, sourceEditor.documentText().length, "start");
  hideSourceColorEditor();
  hideSourceCompletions();
  if (selectionStart !== null) {
    sourceEditor.setSelection(selectionStart, selectionEnd ?? selectionStart);
  }
  updateSourceMeta();
  if (documents[currentDocumentIndex]) {
    documents[currentDocumentIndex].source = sourceEditorDocumentValue();
  }
  if (sourceDocumentSupportsEditableTargets()) {
    scheduleSourceHighlight();
    scheduleSourceOutlineRefresh(true);
    resetLevelBuilderFromSource();
  } else {
    resetSourcePuzzleAnalysisState();
  }
}

function bindSourceEditorPopoverEvents() {
sourceEditorWrap?.addEventListener("scroll", hideSourceColorEditor);
sourceEditorWrap?.addEventListener("scroll", hideSourceCompletions);
sourceEditor.on("click", syncSourceOutlineActiveItem);
sourceEditor.on("keyup", syncSourceOutlineActiveItem);
sourceOutlineList?.addEventListener("click", (event) => {
  const row = /** @type {HTMLElement | null} */ ((event.target instanceof Element ? event.target : null)?.closest("[data-source-outline-id]"));
  if (!row || !sourceOutlineList.contains(row)) {
    return;
  }
  if ((event.target instanceof Element ? event.target : null)?.closest("[data-source-outline-toggle]")) {
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
  const row = /** @type {HTMLElement | null} */ ((event.target instanceof Element ? event.target : null)?.closest("[data-source-outline-id]"));
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
window.addEventListener("resize", hideSourceColorEditor);
}

function nextLineIndent() {
  const source = sourceEditor.documentText();
  const start = sourceEditor.selection().from;
  const lineStart = source.lastIndexOf("\n", start - 1) + 1;
  const lineBeforeCursor = source.slice(lineStart, start);
  const currentIndent = lineBeforeCursor.match(/^[\t ]*/)?.[0] || "";
  const extraIndent = "";
  const afterCursor = source.slice(sourceEditor.selection().to);
  const nextNonWhitespace = afterCursor.match(/^\s*(.)/)?.[1] || "";

  if (lineBeforeCursor.trimEnd().endsWith("{") && nextNonWhitespace === "}") {
    return `\n${currentIndent}${extraIndent}\n${currentIndent}`;
  }
  return `\n${currentIndent}${extraIndent}`;
}

function insertSourceNewlineAtSelection() {
  const insert = nextLineIndent();
  const start = sourceEditor.selection().from;
  const end = sourceEditor.selection().to;
  const cursorOffset = sourceNewlineCursorOffset(insert);
  sourceEditor.replaceRange(insert, start, end, "end");
  if (cursorOffset !== null) {
    const cursor = start + cursorOffset;
    sourceEditor.setSelection(cursor, cursor);
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
    button.className = "option-button source-level-name-option";
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
