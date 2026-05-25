// Workbench layout boundary.
//
// Owns pane identity, pane visibility, pane ordering, pane resize/drag state,
// and shared editor frame geometry helpers. This block may move DOM panes and
// update workbench CSS variables. It must not own documents, source text,
// preview compilation, runtime fixture data, or tool-specific editing state.
const EXPLORER_PANE_ID = "explorer";
const SOURCE_WORK_PANE_ID = "source";
const PREVIEW_WORK_PANE_ID = "preview";
const WORK_PANE_IDS = [
  SOURCE_WORK_PANE_ID,
  PREVIEW_WORK_PANE_ID,
  "level",
  "solver",
  "sprite",
  "sounds",
  "psimport",
  "docs",
];

const PANE_ID_ALIASES = {
  code: SOURCE_WORK_PANE_ID,
  level3d: "level",
  sprite3d: "sprite",
};
const PREVIEW_MODE_TO_WORK_PANE_ID = {
  play: PREVIEW_WORK_PANE_ID,
  edit: "level",
  level3d: "level",
  solver: "solver",
  sprite: "sprite",
  sprite3d: "sprite",
  sounds: "sounds",
  psimport: "psimport",
  docs: "docs",
};
const PREVIEW_MODE_IDS = Object.keys(PREVIEW_MODE_TO_WORK_PANE_ID);
const PREVIEW_HOST_WORK_PANE_IDS = WORK_PANE_IDS.filter((paneId) => paneId !== SOURCE_WORK_PANE_ID);
const MAX_VISIBLE_WORK_PANES = 2;
const WORK_PANE_DEFAULT_WIDTHS = {
  [SOURCE_WORK_PANE_ID]: "42%",
  [PREVIEW_WORK_PANE_ID]: "420px",
  level: "420px",
  solver: "420px",
  sprite: "520px",
  sounds: "420px",
  psimport: "520px",
  docs: "480px",
};
const WORK_PANE_MIN_WIDTHS = {
  [SOURCE_WORK_PANE_ID]: 240,
  [PREVIEW_WORK_PANE_ID]: 300,
  level: 300,
  solver: 300,
  sprite: 320,
  sounds: 300,
  psimport: 320,
  docs: 300,
};
const WORKBENCH_SPLITTER_COLUMN = "var(--workbench-splitter-width)";
let explorerPaneVisible = true;
let visibleWorkPanes = [SOURCE_WORK_PANE_ID, PREVIEW_WORK_PANE_ID];
let focusedWorkPaneId = PREVIEW_WORK_PANE_ID;
let draggingSplitter = false;
let draggingExplorerSplitter = false;
let draggingPreviewLogSplitter = false;
let draggingPaneSplitterElement = null;
let draggingWorkPaneId = "";
let paneDropTargetId = "";
let paneDropSide = "";
let draggingSplitterPointerId = null;
let draggingExplorerSplitterPointerId = null;
let draggingPreviewLogSplitterPointerId = null;
let previewLogHeightPinned = false;
let pendingExplorerCollapse = false;
let pendingPaneCollapse = "";
let explorerWidthBeforeResize = "";
let paneWidthBeforeResize = "";
let lastExplorerPaneWidth = "";
let lastSplitCodePaneWidth = "";
let resizingPaneEdge = null;
let workPaneWidths = { ...WORK_PANE_DEFAULT_WIDTHS };

function normalizePaneId(paneId) {
  return PANE_ID_ALIASES[paneId] || paneId || "";
}

function isWorkPaneId(paneId) {
  return WORK_PANE_IDS.includes(normalizePaneId(paneId));
}

function isPreviewHostPaneId(paneId) {
  return PREVIEW_HOST_WORK_PANE_IDS.includes(normalizePaneId(paneId));
}

function isPreviewHostVisible() {
  return visibleWorkPanes.some((paneId) => isPreviewHostPaneId(paneId));
}

function isPaneVisible(paneId) {
  const normalized = normalizePaneId(paneId);
  if (normalized === EXPLORER_PANE_ID) {
    return explorerPaneVisible;
  }
  return visibleWorkPanes.includes(normalized);
}

function normalizePreviewMode(mode) {
  return PREVIEW_MODE_IDS.includes(mode) ? mode : "play";
}

function workPaneIdForPreviewMode(mode) {
  return PREVIEW_MODE_TO_WORK_PANE_ID[normalizePreviewMode(mode)] || PREVIEW_WORK_PANE_ID;
}

function previewModeForWorkPaneId(paneId) {
  const normalized = normalizePaneId(paneId);
  return Object.entries(PREVIEW_MODE_TO_WORK_PANE_ID)
    .find(([, workPaneId]) => workPaneId === normalized)?.[0] || "play";
}

function workPaneElementForPaneId(paneId) {
  const normalized = normalizePaneId(paneId);
  if (normalized === SOURCE_WORK_PANE_ID) {
    return workbench.querySelector(".code-pane");
  }
  if (isPreviewHostPaneId(normalized)) {
    return workbench.querySelector(`.preview-pane[data-work-pane="${normalized}"]`);
  }
  return null;
}

function toolPaneTitle(paneId) {
  return {
    [PREVIEW_WORK_PANE_ID]: "Preview",
    level: "Level",
    level3d: "3D Level",
    solver: "Solve",
    sprite: "Sprite",
    sounds: "Sound",
    psimport: "PuzzleScript import",
    docs: "Docs",
  }[paneId] || paneId;
}

function toolPanePanelForPaneId(paneId) {
  return {
    [PREVIEW_WORK_PANE_ID]: playPreview,
    level: levelBuilder,
    level3d: level3dBuilder,
    solver: solverPanel,
    sprite: spriteBuilder,
    sounds: soundsBuilder,
    psimport: psImportPanel,
    docs: docsPanel,
  }[paneId] || null;
}

function createPaneCloseButton(paneId) {
  const button = document.createElement("button");
  button.className = "pane-close-button";
  button.type = "button";
  button.dataset.paneClose = paneId;
  button.setAttribute("aria-label", `Hide ${toolPaneTitle(paneId)} pane`);
  button.title = `Hide ${toolPaneTitle(paneId)} pane`;
  button.innerHTML = `
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M18 6 6 18"></path>
      <path d="m6 6 12 12"></path>
    </svg>
  `;
  return button;
}

function createToolPane(paneId, panel) {
  const pane = document.createElement("section");
  pane.className = "preview-pane";
  pane.dataset.workPane = paneId;
  pane.setAttribute("aria-label", toolPaneTitle(paneId));

  const header = document.createElement("div");
  header.className = "pane-header";
  header.dataset.paneDragHandle = paneId;
  header.draggable = true;
  header.title = `Drag header to move ${toolPaneTitle(paneId)} pane`;
  const title = document.createElement("div");
  title.className = "pane-title";
  const label = document.createElement("span");
  label.className = "tool-pane-title";
  label.textContent = toolPaneTitle(paneId);
  const actions = document.createElement("div");
  actions.className = "pane-actions";
  title.append(label);
  if (paneId === "level" && levelPaneModeSwitch) {
    title.append(levelPaneModeSwitch);
  }
  if (paneId === "sprite" && spritePaneModeSwitch) {
    title.append(spritePaneModeSwitch);
  }
  actions.append(createPaneCloseButton(paneId));
  header.append(title, actions);
  pane.append(header, panel);
  if (paneId === "level" && level3dBuilder) {
    pane.append(level3dBuilder);
  }
  if (paneId === "sprite" && sprite3dBuilder) {
    pane.append(sprite3dBuilder);
  }
  return pane;
}

function initializePhysicalWorkPanes() {
  const previewPane = workbench.querySelector(".preview-pane");
  if (!previewPane || previewPane.dataset.physicalPanesInitialized === "true") {
    return;
  }
  previewPane.dataset.physicalPanesInitialized = "true";
  previewPane.dataset.workPane = PREVIEW_WORK_PANE_ID;
  previewPane.setAttribute("aria-label", toolPaneTitle(PREVIEW_WORK_PANE_ID));

  const previewDragHandle = previewPane.querySelector("[data-pane-drag-handle]");
  if (previewDragHandle) {
    previewDragHandle.dataset.paneDragHandle = PREVIEW_WORK_PANE_ID;
    previewDragHandle.draggable = true;
    previewDragHandle.title = "Drag header to move Preview pane";
  }
  const previewClose = previewPane.querySelector("[data-pane-close]");
  if (previewClose) {
    previewClose.className = "pane-close-button";
    previewClose.dataset.paneClose = PREVIEW_WORK_PANE_ID;
    previewClose.setAttribute("aria-label", "Hide Preview pane");
    previewClose.title = "Hide Preview pane";
    previewClose.innerHTML = `
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M18 6 6 18"></path>
        <path d="m6 6 12 12"></path>
      </svg>
    `;
  }
  if (gamePaneTitle) {
    gamePaneTitle.textContent = toolPaneTitle(PREVIEW_WORK_PANE_ID);
  }
  if (runButton) {
    runButton.hidden = false;
  }
  if (levelPaneModeSwitch) {
    levelPaneModeSwitch.hidden = true;
  }
  if (spritePaneModeSwitch) {
    spritePaneModeSwitch.hidden = true;
  }

  for (const paneId of PREVIEW_HOST_WORK_PANE_IDS) {
    if (paneId === PREVIEW_WORK_PANE_ID) {
      continue;
    }
    const panel = toolPanePanelForPaneId(paneId);
    if (!panel) {
      continue;
    }
    workbench.append(createToolPane(paneId, panel));
  }
}

function layoutPaneIdsFor(paneIds, previewMode = currentPreviewMode || "play", options = {}) {
  const result = [];
  for (const paneId of paneIds) {
    const normalized = normalizePaneId(paneId);
    if (!isWorkPaneId(normalized) || result.includes(normalized)) {
      continue;
    }
    const element = workPaneElementForPaneId(normalized);
    if (!element) {
      continue;
    }
    result.push(normalized);
  }
  if (result.length || options.allowEmpty) {
    return result;
  }
  return [SOURCE_WORK_PANE_ID];
}

function layoutPaneIds() {
  return layoutPaneIdsFor(visibleWorkPanes);
}

function normalizeVisibleWorkPaneList(paneIds, options = {}) {
  const limit = Number.isFinite(options.limit) ? options.limit : MAX_VISIBLE_WORK_PANES;
  const next = [];
  for (const paneId of paneIds || []) {
    const normalized = normalizePaneId(paneId);
    if (isWorkPaneId(normalized) && !next.includes(normalized) && workPaneElementForPaneId(normalized)) {
      next.push(normalized);
    }
    if (next.length >= limit) {
      break;
    }
  }
  if (!next.length && !options.allowEmpty) {
    next.push(SOURCE_WORK_PANE_ID);
  }
  return next;
}

function workPaneWidth(paneId) {
  return workPaneWidths[paneId] || WORK_PANE_DEFAULT_WIDTHS[paneId] || "420px";
}

function workPaneMinimumWidth(paneId) {
  return WORK_PANE_MIN_WIDTHS[normalizePaneId(paneId)] || 300;
}

function setWorkPaneWidth(paneId, width) {
  if (!isWorkPaneId(paneId) || !width) {
    return;
  }
  workPaneWidths[paneId] = width;
  if (paneId === SOURCE_WORK_PANE_ID) {
    lastSplitCodePaneWidth = width;
    workbench.style.setProperty("--code-pane-width", width);
  }
}

function renderedWorkPaneWidth(paneId) {
  const element = workPaneElementForPaneId(paneId);
  const width = element?.getBoundingClientRect?.().width || 0;
  return width > 0 ? `${Math.round(width)}px` : "";
}

function inheritReplacedPaneSlotWidth(replacedPaneId, nextPaneId, replaceIndex, paneCount) {
  if (!isWorkPaneId(replacedPaneId) || !isWorkPaneId(nextPaneId) || replaceIndex >= paneCount - 1) {
    return;
  }
  const width = renderedWorkPaneWidth(replacedPaneId) || workPaneWidth(replacedPaneId);
  setWorkPaneWidth(nextPaneId, width);
}

function ensurePaneSplitterCount(count) {
  const splitters = Array.from(workbench.querySelectorAll(".pane-splitter"));
  while (splitters.length < count) {
    const splitter = document.createElement("div");
    splitter.className = "pane-splitter";
    splitter.setAttribute("role", "separator");
    splitter.setAttribute("aria-label", "Resize panes");
    splitter.setAttribute("aria-orientation", "vertical");
    splitter.addEventListener("pointerdown", startPaneResize);
    splitter.addEventListener("lostpointercapture", stopPaneResize);
    workbench.append(splitter);
    splitters.push(splitter);
  }
  splitters.forEach((splitter, index) => {
    splitter.hidden = index >= count;
  });
  return splitters.slice(0, count);
}

function syncWorkbenchGridLayout() {
  const panes = layoutPaneIds();
  const columns = [];
  let columnIndex = 1;

  if (explorerPaneVisible) {
    columns.push("var(--explorer-pane-width)");
    columns.push(WORKBENCH_SPLITTER_COLUMN);
    const explorer = workbench.querySelector(".explorer-pane");
    if (explorer) {
      explorer.style.gridColumn = String(columnIndex);
    }
    if (explorerSplitter) {
      explorerSplitter.style.gridColumn = String(columnIndex + 1);
      explorerSplitter.hidden = false;
    }
    columnIndex += 2;
  } else if (explorerSplitter) {
    explorerSplitter.hidden = true;
  }

  const splitters = ensurePaneSplitterCount(Math.max(0, panes.length - 1));
  panes.forEach((paneId, index) => {
    const isLast = index === panes.length - 1;
    columns.push(isLast ? "minmax(0, 1fr)" : `minmax(0, ${workPaneWidth(paneId)})`);
    const element = workPaneElementForPaneId(paneId);
    if (element) {
      element.style.gridColumn = String(columnIndex);
    }
    columnIndex += 1;
    const splitter = splitters[index];
    if (splitter) {
      columns.push(WORKBENCH_SPLITTER_COLUMN);
      splitter.style.gridColumn = String(columnIndex);
      splitter.dataset.leftPane = paneId;
      splitter.dataset.rightPane = panes[index + 1];
      columnIndex += 1;
    }
  });

  workbench.style.setProperty("--workbench-grid-columns", columns.join(" "));
}

function ensureLevel3dPaneFrameWidth() {
  if (currentPreviewMode === "level3d" && typeof scheduleLevel3dSurfaceResize === "function") {
    scheduleLevel3dSurfaceResize();
  }
}

function focusWorkPane(paneId) {
  const normalized = normalizePaneId(paneId);
  if (!isWorkPaneId(normalized)) {
    return false;
  }
  focusedWorkPaneId = normalized;
  workbench.dataset.focusedWorkPane = focusedWorkPaneId;
  return true;
}

function setVisibleWorkPanes(nextPaneIds) {
  const next = normalizeVisibleWorkPaneList(nextPaneIds);
  if (!next.length) {
    return false;
  }
  visibleWorkPanes = next;
  return true;
}

function workPaneIdToReplaceForOpen(options = {}) {
  const requested = normalizePaneId(options.replacePaneId || "");
  if (visibleWorkPanes.includes(requested)) {
    return requested;
  }
  if (visibleWorkPanes.includes(focusedWorkPaneId)) {
    const unfocused = visibleWorkPanes.find((paneId) => paneId !== focusedWorkPaneId);
    if (unfocused) {
      return unfocused;
    }
  }
  return visibleWorkPanes[visibleWorkPanes.length - 1] || "";
}

function activePreviewWorkPaneId() {
  const active = workPaneIdForPreviewMode(currentPreviewMode || "play");
  if (visibleWorkPanes.includes(active)) {
    return active;
  }
  return visibleWorkPanes.find((paneId) => isPreviewHostPaneId(paneId)) || active;
}

function normalizePaneDragId(paneId) {
  return paneId === "active-preview" ? activePreviewWorkPaneId() : normalizePaneId(paneId);
}

function workPaneElementForDragEvent(event) {
  const element = event.target?.closest?.(".code-pane, .preview-pane[data-work-pane]");
  return element && workbench.contains(element) ? element : null;
}

function workPaneIdForElement(element) {
  return normalizePaneId(element?.dataset?.workPane || (element?.classList.contains("code-pane") ? SOURCE_WORK_PANE_ID : ""));
}

function paneDropSideForEvent(event, element) {
  const rect = element.getBoundingClientRect();
  return event.clientX < rect.left + rect.width / 2 ? "before" : "after";
}

function shouldIgnorePaneDragStart(event) {
  return Boolean(event.target?.closest?.(
    "button, input, textarea, select, a, [role='button'], [role='group'], .document-tab, .preview-log-panel"
  ));
}

function clearWorkPaneDropState(options = {}) {
  paneDropTargetId = "";
  paneDropSide = "";
  for (const element of workbench.querySelectorAll(".code-pane, .preview-pane[data-work-pane]")) {
    element.classList.remove("is-pane-drop-before", "is-pane-drop-after");
    if (!options.keepDragSource) {
      element.classList.remove("is-pane-drag-source");
    }
  }
  if (!options.keepDragSource) {
    workbench.classList.remove("is-dragging-work-pane");
  }
}

function markWorkPaneDropTarget(targetPaneId, side) {
  clearWorkPaneDropState({ keepDragSource: true });
  paneDropTargetId = targetPaneId;
  paneDropSide = side;
  const element = workPaneElementForPaneId(targetPaneId);
  element?.classList.add(side === "before" ? "is-pane-drop-before" : "is-pane-drop-after");
}

function moveWorkPane(draggedPaneId, targetPaneId, side) {
  const dragged = normalizePaneDragId(draggedPaneId);
  const target = normalizePaneDragId(targetPaneId);
  if (!isWorkPaneId(dragged) || !isWorkPaneId(target) || dragged === target) {
    return false;
  }
  const next = visibleWorkPanes.filter((paneId) => paneId !== dragged);
  const targetIndex = next.indexOf(target);
  if (targetIndex < 0) {
    return false;
  }
  next.splice(side === "after" ? targetIndex + 1 : targetIndex, 0, dragged);
  visibleWorkPanes = next;
  focusWorkPane(dragged);
  applyPaneVisibility();
  return true;
}

function startWorkPaneDrag(event) {
  if (shouldIgnorePaneDragStart(event)) {
    event.preventDefault();
    return;
  }
  const handle = event.target?.closest?.("[data-pane-drag-handle]");
  const paneId = normalizePaneDragId(handle?.dataset.paneDragHandle);
  if (!isWorkPaneId(paneId) || !isPaneVisible(paneId)) {
    event.preventDefault();
    return;
  }
  draggingWorkPaneId = paneId;
  paneDropTargetId = "";
  paneDropSide = "";
  workbench.classList.add("is-dragging-work-pane");
  workPaneElementForPaneId(paneId)?.classList.add("is-pane-drag-source");
  event.dataTransfer.effectAllowed = "move";
  event.dataTransfer.setData("text/x-puzzle-work-pane", paneId);
  event.dataTransfer.setData("text/plain", paneId);
}

function handleWorkPaneDragOver(event) {
  if (!draggingWorkPaneId) {
    return;
  }
  const targetElement = workPaneElementForDragEvent(event);
  const targetPaneId = workPaneIdForElement(targetElement);
  if (!targetElement || !isWorkPaneId(targetPaneId)) {
    return;
  }
  event.preventDefault();
  event.dataTransfer.dropEffect = "move";
  const side = paneDropSideForEvent(event, targetElement);
  if (targetPaneId === draggingWorkPaneId) {
    clearWorkPaneDropState({ keepDragSource: true });
    return;
  }
  if (paneDropTargetId !== targetPaneId || paneDropSide !== side) {
    markWorkPaneDropTarget(targetPaneId, side);
  }
}

function handleWorkPaneDrop(event) {
  if (!draggingWorkPaneId) {
    return;
  }
  event.preventDefault();
  const targetElement = workPaneElementForDragEvent(event);
  const targetPaneId = workPaneIdForElement(targetElement);
  const side = targetElement ? paneDropSideForEvent(event, targetElement) : paneDropSide;
  moveWorkPane(draggingWorkPaneId, targetPaneId || paneDropTargetId, side || paneDropSide || "after");
  draggingWorkPaneId = "";
  clearWorkPaneDropState();
}

function handleWorkPaneFocus(event) {
  const element = workPaneElementForDragEvent(event);
  const paneId = workPaneIdForElement(element);
  if (element && isPaneVisible(paneId)) {
    focusWorkPane(paneId);
  }
}

function stopWorkPaneDrag() {
  draggingWorkPaneId = "";
  clearWorkPaneDropState();
}

function selectFallbackPreviewPane(closedPaneId) {
  if (!isPreviewHostPaneId(closedPaneId) || workPaneIdForPreviewMode(currentPreviewMode) !== closedPaneId) {
    return;
  }
  const nextPreviewPane = visibleWorkPanes.find((paneId) => isPreviewHostPaneId(paneId));
  if (nextPreviewPane) {
    setPreviewMode(previewModeForWorkPaneId(nextPreviewPane), { skipPaneSync: true });
  }
}

function closeWorkPane(paneId) {
  const normalized = paneId === "active-preview" ? activePreviewWorkPaneId() : normalizePaneId(paneId);
  const next = visibleWorkPanes.filter((candidate) => candidate !== normalized);
  if (!isWorkPaneId(normalized) || !visibleWorkPanes.includes(normalized) || !layoutPaneIdsFor(next, currentPreviewMode, { allowEmpty: true }).length) {
    return false;
  }
  visibleWorkPanes = next;
  selectFallbackPreviewPane(normalized);
  applyPaneVisibility();
  return true;
}

function showWorkPane(paneId, options = {}) {
  const normalized = normalizePaneId(paneId);
  if (!isWorkPaneId(normalized)) {
    return false;
  }
  visibleWorkPanes = normalizeVisibleWorkPaneList(visibleWorkPanes);
  if (visibleWorkPanes.includes(normalized)) {
    if (options.focus !== false) {
      focusWorkPane(normalized);
    }
    applyPaneVisibility();
    return true;
  }
  if (visibleWorkPanes.length < MAX_VISIBLE_WORK_PANES) {
    visibleWorkPanes.push(normalized);
  } else {
    const replacePaneId = workPaneIdToReplaceForOpen(options);
    const replaceIndex = visibleWorkPanes.indexOf(replacePaneId);
    if (replaceIndex < 0) {
      inheritReplacedPaneSlotWidth(visibleWorkPanes[visibleWorkPanes.length - 1], normalized, visibleWorkPanes.length - 1, visibleWorkPanes.length);
      visibleWorkPanes[visibleWorkPanes.length - 1] = normalized;
    } else {
      inheritReplacedPaneSlotWidth(replacePaneId, normalized, replaceIndex, visibleWorkPanes.length);
      visibleWorkPanes[replaceIndex] = normalized;
    }
  }
  visibleWorkPanes = normalizeVisibleWorkPaneList(visibleWorkPanes);
  if (options.focus !== false) {
    focusWorkPane(normalized);
  }
  applyPaneVisibility();
  return true;
}

function showPreviewModePane(mode, options = {}) {
  const paneId = workPaneIdForPreviewMode(mode);
  if (options.replaceWorkPanes) {
    if (!setVisibleWorkPanes([paneId])) {
      return false;
    }
    if (options.focus !== false) {
      focusWorkPane(paneId);
    }
    applyPaneVisibility();
    return true;
  }
  return showWorkPane(paneId, options);
}

function openPreviewModePane(mode, options = {}) {
  const previewMode = normalizePreviewMode(mode);
  showPreviewModePane(previewMode, {
    focus: true,
    replaceWorkPanes: options.replaceWorkPanes === true,
  });
  setPreviewMode(previewMode, { skipPaneSync: true });
}

function setVisiblePanes(nextPanes) {
  const requested = Array.from(nextPanes || []).map((paneId) => normalizePaneId(paneId));
  const nextExplorerVisible = requested.includes(EXPLORER_PANE_ID);
  const nextWorkPanes = requested.filter((paneId) => isWorkPaneId(paneId));
  if (!setVisibleWorkPanes(nextWorkPanes)) {
    return;
  }
  explorerPaneVisible = nextExplorerVisible;
  applyPaneVisibility();
}

function applyPaneVisibility() {
  visibleWorkPanes = normalizeVisibleWorkPaneList(visibleWorkPanes);
  if (explorerPaneVisible && lastExplorerPaneWidth) {
    workbench.style.setProperty("--explorer-pane-width", lastExplorerPaneWidth);
  }
  if (lastSplitCodePaneWidth) {
    setWorkPaneWidth(SOURCE_WORK_PANE_ID, lastSplitCodePaneWidth);
  }
  pendingExplorerCollapse = false;
  pendingPaneCollapse = "";
  if (!visibleWorkPanes.includes(focusedWorkPaneId)) {
    focusWorkPane(visibleWorkPanes[visibleWorkPanes.length - 1] || SOURCE_WORK_PANE_ID);
  } else {
    workbench.dataset.focusedWorkPane = focusedWorkPaneId;
  }
  workbench.dataset.activePreviewMode = currentPreviewMode || "play";
  workbench.dataset.activePreviewPane = workPaneIdForPreviewMode(currentPreviewMode || "play");
  workbench.classList.toggle("is-explorer-hidden", !explorerPaneVisible);
  workbench.classList.toggle("is-code-hidden", !isPaneVisible(SOURCE_WORK_PANE_ID));
  workbench.classList.toggle("is-preview-hidden", !isPreviewHostVisible());
  workbench.dataset.collapsingPane = "";
  workbench.dataset.collapsingPreview = "false";
  for (const paneId of WORK_PANE_IDS) {
    const element = workPaneElementForPaneId(paneId);
    const visible = isPaneVisible(paneId);
    if (element) {
      element.hidden = !visible;
    }
    if (paneId === "level") {
      if (levelBuilder) {
        levelBuilder.hidden = !visible || currentLevelPaneMode !== "edit";
      }
      if (level3dBuilder) {
        level3dBuilder.hidden = !visible || currentLevelPaneMode !== "level3d";
      }
      continue;
    }
    if (paneId === "sprite") {
      if (spriteBuilder) {
        spriteBuilder.hidden = !visible || currentSpritePaneMode !== "sprite";
      }
      if (sprite3dBuilder) {
        sprite3dBuilder.hidden = !visible || currentSpritePaneMode !== "sprite3d";
      }
      continue;
    }
    const panel = toolPanePanelForPaneId(paneId);
    if (panel) {
      panel.hidden = !visible;
    }
  }
  if (levelPaneModeSwitch) {
    levelPaneModeSwitch.hidden = !isPaneVisible("level");
  }
  if (spritePaneModeSwitch) {
    spritePaneModeSwitch.hidden = !isPaneVisible("sprite");
  }
  syncWorkbenchGridLayout();
  paneToggleButtons.forEach((button) => {
    const normalized = normalizePaneId(button.dataset.paneToggle);
    const active = normalized === PREVIEW_WORK_PANE_ID
      ? isPreviewHostVisible()
      : isPaneVisible(normalized);
    button.classList.toggle("is-active", active);
    button.setAttribute("aria-pressed", active ? "true" : "false");
  });
  document.querySelectorAll("[data-pane-close]").forEach((button) => {
    const paneId = button.dataset.paneClose === "active-preview"
      ? activePreviewWorkPaneId()
      : normalizePaneId(button.dataset.paneClose);
    const next = visibleWorkPanes.filter((candidate) => candidate !== paneId);
    const disabled = !isWorkPaneId(paneId) || !isPaneVisible(paneId) || !layoutPaneIdsFor(next, currentPreviewMode, { allowEmpty: true }).length;
    button.disabled = disabled;
    button.setAttribute("aria-disabled", String(disabled));
  });
  syncPreviewModeButtonState();
  scheduleBoardScaleSync();
  requestAnimationFrame(syncPreviewViewportScale);
}

function togglePaneVisibility(pane) {
  const normalized = normalizePaneId(pane);
  if (normalized === EXPLORER_PANE_ID) {
    explorerPaneVisible = !explorerPaneVisible;
    applyPaneVisibility();
    return;
  }
  if (!isWorkPaneId(normalized)) {
    return;
  }
  if (normalized === PREVIEW_WORK_PANE_ID) {
    showPreviewModePane(currentPreviewMode);
    return;
  }
  if (isPaneVisible(normalized)) {
    focusWorkPane(normalized);
  } else {
    showWorkPane(normalized);
  }
  applyPaneVisibility();
}

function revealPreviewPane() {
  showWorkPane(PREVIEW_WORK_PANE_ID);
}

function revealCodePane() {
  showWorkPane(SOURCE_WORK_PANE_ID);
}

function activeResizePointerMatches(event, pointerId) {
  return !event || pointerId === null || event.pointerId === pointerId;
}

function releasePointerCaptureIfHeld(element, pointerId) {
  if (!element || pointerId === null) {
    return;
  }
  try {
    if (element.hasPointerCapture(pointerId)) {
      element.releasePointerCapture(pointerId);
    }
  } catch {
    // The pointer can already be gone after blur/cancel or browser edge release.
  }
}

function stopActiveResize(event) {
  stopPaneResize(event);
  stopExplorerResize(event);
  stopPreviewLogResize(event);
}

function handleExplorerToggleShortcut(event) {
  if (!(event.metaKey && !event.ctrlKey && !event.altKey && !event.shiftKey && event.key.toLowerCase() === "b")) {
    return false;
  }
  event.preventDefault();
  event.stopImmediatePropagation();
  togglePaneVisibility("explorer");
  return true;
}

function startPaneResize(event) {
  const splitter = event.currentTarget;
  const leftPaneId = normalizePaneId(splitter?.dataset.leftPane);
  const rightPaneId = normalizePaneId(splitter?.dataset.rightPane);
  if (!isWorkPaneId(leftPaneId) || !isWorkPaneId(rightPaneId)) {
    return;
  }
  stopActiveResize();
  draggingSplitter = true;
  draggingPaneSplitterElement = splitter;
  draggingSplitterPointerId = event.pointerId;
  const leftPane = workPaneElementForPaneId(leftPaneId);
  const rightPane = workPaneElementForPaneId(rightPaneId);
  if (!leftPane || !rightPane) {
    draggingSplitter = false;
    draggingPaneSplitterElement = null;
    draggingSplitterPointerId = null;
    return;
  }
  resizingPaneEdge = { leftPaneId, rightPaneId };
  paneWidthBeforeResize = `${leftPane.getBoundingClientRect().width}px`;
  workbench.classList.add("is-resizing-panes");
  splitter.classList.add("is-active-splitter");
  splitter.setPointerCapture(event.pointerId);
  event.preventDefault();
}

function resizePanes(event) {
  if (!draggingSplitter || !activeResizePointerMatches(event, draggingSplitterPointerId)) {
    return;
  }
  const leftPane = workPaneElementForPaneId(resizingPaneEdge?.leftPaneId);
  const rightPane = workPaneElementForPaneId(resizingPaneEdge?.rightPaneId);
  if (!leftPane || !rightPane || !draggingPaneSplitterElement) {
    return;
  }
  const leftRect = leftPane.getBoundingClientRect();
  const rightRect = rightPane.getBoundingClientRect();
  const availableWidth = rightRect.right - leftRect.left;
  const minLeft = workPaneMinimumWidth(resizingPaneEdge.leftPaneId);
  const minRight = workPaneMinimumWidth(resizingPaneEdge.rightPaneId);
  const snapWidth = 72;
  const splitHandleWidth = draggingPaneSplitterElement.getBoundingClientRect().width || 6;
  const maxLeft = Math.max(minLeft, availableWidth - splitHandleWidth - minRight);
  const pointerX = event.clientX - leftRect.left;
  pendingPaneCollapse = "";
  let next = Math.max(minLeft, Math.min(maxLeft, pointerX));
  if (pointerX >= availableWidth - snapWidth) {
    pendingPaneCollapse = resizingPaneEdge.rightPaneId;
    next = Math.max(0, availableWidth - splitHandleWidth);
  }
  setWorkPaneWidth(resizingPaneEdge.leftPaneId, `${next}px`);
  syncWorkbenchGridLayout();
  syncPreviewViewportScale();
  workbench.dataset.collapsingPane = pendingPaneCollapse || "";
  workbench.dataset.collapsingPreview = pendingPaneCollapse && isPreviewHostPaneId(pendingPaneCollapse) ? "true" : "false";
}

function stopPaneResize(event) {
  if (!draggingSplitter || !activeResizePointerMatches(event, draggingSplitterPointerId)) {
    return;
  }
  const pointerId = draggingSplitterPointerId;
  const splitter = draggingPaneSplitterElement;
  draggingSplitter = false;
  draggingPaneSplitterElement = null;
  draggingSplitterPointerId = null;
  workbench.classList.remove("is-resizing-panes");
  splitter?.classList.remove("is-active-splitter");
  workbench.dataset.collapsingPane = "";
  workbench.dataset.collapsingPreview = "false";
  releasePointerCaptureIfHeld(splitter, pointerId);
  if (pendingPaneCollapse) {
    setWorkPaneWidth(resizingPaneEdge?.leftPaneId, paneWidthBeforeResize || workPaneWidth(resizingPaneEdge?.leftPaneId));
    if (visibleWorkPanes.length > 1) {
      visibleWorkPanes = visibleWorkPanes.filter((paneId) => paneId !== pendingPaneCollapse);
    }
    if (isPreviewHostPaneId(pendingPaneCollapse) && workPaneIdForPreviewMode(currentPreviewMode) === pendingPaneCollapse) {
      const nextPreviewPane = visibleWorkPanes.find((paneId) => isPreviewHostPaneId(paneId));
      if (nextPreviewPane) {
        setPreviewMode(previewModeForWorkPaneId(nextPreviewPane), { skipPaneSync: true });
      }
    }
    applyPaneVisibility();
  } else {
    const leftPaneId = resizingPaneEdge?.leftPaneId;
    if (leftPaneId) {
      setWorkPaneWidth(leftPaneId, workPaneWidth(leftPaneId));
    }
  }
  pendingPaneCollapse = "";
  paneWidthBeforeResize = "";
  resizingPaneEdge = null;
}

function startExplorerResize(event) {
  if (!explorerPaneVisible) {
    return;
  }
  stopActiveResize();
  draggingExplorerSplitter = true;
  draggingExplorerSplitterPointerId = event.pointerId;
  const explorer = workbench.querySelector(".explorer-pane");
  explorerWidthBeforeResize = `${explorer.getBoundingClientRect().width}px`;
  workbench.classList.add("is-resizing-explorer");
  explorerSplitter.setPointerCapture(event.pointerId);
  event.preventDefault();
}

function resizeExplorer(event) {
  if (!draggingExplorerSplitter || !activeResizePointerMatches(event, draggingExplorerSplitterPointerId)) {
    return;
  }
  const rect = workbench.getBoundingClientRect();
  const minWidth = 150;
  const snapWidth = 54;
  const maxWidth = Math.max(minWidth, Math.min(420, rect.width - 360));
  const pointerX = event.clientX - rect.left;
  pendingExplorerCollapse = pointerX <= snapWidth;
  const next = pendingExplorerCollapse ? 0 : Math.max(minWidth, Math.min(maxWidth, pointerX));
  workbench.style.setProperty("--explorer-pane-width", `${next}px`);
  syncPreviewViewportScale();
  workbench.classList.toggle("is-explorer-collapse-pending", pendingExplorerCollapse);
}

function stopExplorerResize(event) {
  if (!draggingExplorerSplitter || !activeResizePointerMatches(event, draggingExplorerSplitterPointerId)) {
    return;
  }
  const pointerId = draggingExplorerSplitterPointerId;
  draggingExplorerSplitter = false;
  draggingExplorerSplitterPointerId = null;
  workbench.classList.remove("is-resizing-explorer", "is-explorer-collapse-pending");
  releasePointerCaptureIfHeld(explorerSplitter, pointerId);
  if (pendingExplorerCollapse) {
    lastExplorerPaneWidth = explorerWidthBeforeResize || lastExplorerPaneWidth;
    explorerPaneVisible = false;
    applyPaneVisibility();
  } else {
    lastExplorerPaneWidth = workbench.style.getPropertyValue("--explorer-pane-width");
  }
  pendingExplorerCollapse = false;
  explorerWidthBeforeResize = "";
}

function startPreviewLogResize(event) {
  if (!playPreview || playPreview.hidden) {
    return;
  }
  stopActiveResize();
  draggingPreviewLogSplitter = true;
  draggingPreviewLogSplitterPointerId = event.pointerId;
  previewLogHeightPinned = true;
  playPreview.classList.add("is-resizing-log");
  previewLogSplitter.setPointerCapture(event.pointerId);
  event.preventDefault();
}

function resizePreviewLog(event) {
  if (!draggingPreviewLogSplitter || !playPreview || !activeResizePointerMatches(event, draggingPreviewLogSplitterPointerId)) {
    return;
  }
  const rect = playPreview.getBoundingClientRect();
  const splitterHeight = previewLogSplitter?.getBoundingClientRect().height || 6;
  const minPreviewHeight = 180;
  const minLogHeight = 72;
  const maxLogHeight = Math.max(minLogHeight, rect.height - splitterHeight - minPreviewHeight);
  const next = Math.max(minLogHeight, Math.min(maxLogHeight, rect.bottom - event.clientY - 12));
  playPreview.style.setProperty("--preview-log-height", `${Math.round(next)}px`);
  syncPreviewViewportScale();
  schedulePreviewViewportSync(3);
  event.preventDefault();
}

function stopPreviewLogResize(event) {
  if (!draggingPreviewLogSplitter || !activeResizePointerMatches(event, draggingPreviewLogSplitterPointerId)) {
    return;
  }
  const pointerId = draggingPreviewLogSplitterPointerId;
  draggingPreviewLogSplitter = false;
  draggingPreviewLogSplitterPointerId = null;
  playPreview?.classList.remove("is-resizing-log");
  releasePointerCaptureIfHeld(previewLogSplitter, pointerId);
  syncPreviewViewportScale();
  schedulePreviewViewportSync(3);
}
function fitEditorAspectFrame(available, aspect, virtualHeight) {
  const safeAspect = Number.isFinite(aspect) && aspect > 0
    ? aspect
    : previewDefaultLogicalWidth / previewDefaultLogicalHeight;
  const safeVirtualHeight = Math.max(1, Number(virtualHeight) || previewMinimumHeight);
  const virtualWidth = Math.max(1, Math.round(safeVirtualHeight * safeAspect));
  const scale = Math.max(
    0.0001,
    Math.min(
      Math.max(1, Number(available?.width) || 1) / virtualWidth,
      Math.max(1, Number(available?.height) || 1) / safeVirtualHeight,
    ),
  );
  return {
    width: Math.max(1, Math.floor(virtualWidth * scale)),
    height: Math.max(1, Math.floor(safeVirtualHeight * scale)),
    virtualWidth,
    virtualHeight: safeVirtualHeight,
    scale,
  };
}
function editorFrameAvailableSize(frame, options = {}) {
  return {
    width: Math.max(1, Math.floor(editorFrameContentInlineSize(frame, options))),
    height: Math.max(1, Math.floor(editorFrameContentBlockSize(frame, options))),
  };
}

function editorFrameContentInlineSize(frame, options = {}) {
  if (!frame) {
    return 0;
  }
  const container = options.container || frame.parentElement;
  const containerWidth = elementContentWidth(container);
  if (containerWidth > 0) {
    return Math.max(0, containerWidth - elementInlineOuterSpacing(frame));
  }
  return elementContentWidth(frame);
}

function editorFrameContentBlockSize(frame, options = {}) {
  if (!frame) {
    return 0;
  }
  const container = options.container || frame.parentElement;
  const reservedBlock = Math.max(0, Number(options.reservedBlock) || 0);
  const containerHeight = elementContentHeight(container);
  if (containerHeight > 0) {
    return Math.max(0, containerHeight - elementBlockOuterSpacing(frame) - reservedBlock);
  }
  return Math.max(0, elementContentHeight(frame) - reservedBlock);
}

function elementOuterBlockSize(element) {
  if (!element) {
    return 0;
  }
  return element.getBoundingClientRect().height +
    elementBlockMargins(element);
}

function elementInlineOuterSpacing(element) {
  return elementBoxSpacing(element, "inline", ["margin", "border", "padding"]);
}

function elementBlockOuterSpacing(element) {
  return elementBoxSpacing(element, "block", ["margin", "border", "padding"]);
}

function elementBlockMargins(element) {
  return elementBoxSpacing(element, "block", ["margin"]);
}

function elementBoxSpacing(element, axis, parts = ["margin", "border", "padding"]) {
  if (!element) {
    return 0;
  }
  const style = window.getComputedStyle(element);
  const sides = axis === "block"
    ? ["Top", "Bottom"]
    : ["Left", "Right"];
  let total = 0;
  for (const part of parts) {
    for (const side of sides) {
      total += cssPixelNumber(style[`${part}${side}Width`] ?? style[`${part}${side}`]);
    }
  }
  return total;
}

function cssPixelNumber(value) {
  const number = Number.parseFloat(value || "0");
  return Number.isFinite(number) ? number : 0;
}

function elementContentWidth(element) {
  if (!element) {
    return 0;
  }
  const style = window.getComputedStyle(element);
  const padding = parseFloat(style.paddingLeft || "0") + parseFloat(style.paddingRight || "0");
  return Math.max(0, element.clientWidth - padding);
}

function elementContentHeight(element) {
  if (!element) {
    return 0;
  }
  const style = window.getComputedStyle(element);
  const padding = parseFloat(style.paddingTop || "0") + parseFloat(style.paddingBottom || "0");
  return Math.max(0, element.clientHeight - padding);
}
