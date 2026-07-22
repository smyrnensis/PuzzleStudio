let visual3dActionClearTimer = 0;
let visual3dPreviewDrag = null;
let visual3dCameraScrubDrag = null;
let visual3dSliceScrubDrag = null;
let visual3dBucketActive = false;
let visual3dTranslateActive = false;
let visual3dTranslateDrag = null;
let visual3dClipActive = false;
let visual3dClipSelection = null;
let visual3dClipDrag = null;
let visual3dClipClipboard = null;
let visual3dClipFloating = null;
let visual3dGridVisible = true;
const VISUAL3D_EDITOR_MAX_SIZE = 64;
const VISUAL3D_ANIMATION_MAX_FRAMES = 24;
const VISUAL3D_SLICE_SCRUB_STEP_PX = 18;
const VISUAL3D_CAMERA_MIN_PITCH_DEGREES = -90;
const VISUAL3D_CAMERA_MAX_PITCH_DEGREES = 90;
const VISUAL3D_PREVIEW_BASE_ZOOM = 1;
const VISUAL3D_CAMERA_DEFAULT = {
  yawDegrees: 15,
  pitchDegrees: 30,
  zoom: 1,
};

function visual3dFrameCellCount() {
  return visual3d.width * visual3d.height * visual3d.depth;
}

function visual3dAxisSize(axis = visual3d.axis) {
  if (axis === "x") return visual3d.width;
  if (axis === "y") return visual3d.height;
  return visual3d.depth;
}

function visual3dPlaneSize(axis = visual3d.axis) {
  if (axis === "x") return { width: visual3d.height, height: visual3d.depth };
  if (axis === "y") return { width: visual3d.width, height: visual3d.depth };
  return { width: visual3d.width, height: visual3d.height };
}

function normalizedVisual3dAnimationDuration(value = visual3d.animationDurationMs) {
  return Math.max(20, Math.min(5000, Math.trunc(Number(value) || 120)));
}

function normalizeVisual3dFrameCells(cells) {
  const count = visual3dFrameCellCount();
  return Array.from({ length: count }, (_, index) => (
    validVisual3dColorIndex(cells?.[index]) ? cells[index] : null
  ));
}

function ensureVisual3dAnimationState() {
  if (!Array.isArray(visual3d.frames) || !visual3d.frames.length) {
    visual3d.frames = [normalizeVisual3dFrameCells(visual3d.cells)];
  }
  visual3d.animationFrameCount = Math.max(
    1,
    Math.min(VISUAL3D_ANIMATION_MAX_FRAMES, Math.trunc(Number(visual3d.animationFrameCount) || visual3d.frames.length || 1)),
  );
  while (visual3d.frames.length < visual3d.animationFrameCount) {
    visual3d.frames.push(normalizeVisual3dFrameCells(visual3d.frames[visual3d.frames.length - 1]));
  }
  visual3d.frames.length = visual3d.animationFrameCount;
  visual3d.frames = visual3d.frames.map(normalizeVisual3dFrameCells);
  visual3d.animationFrameIndex = Math.max(0, Math.min(
    visual3d.animationFrameCount - 1,
    Math.trunc(Number(visual3d.animationFrameIndex) || 0),
  ));
  visual3d.animationPlaybackIndex = Math.max(0, Math.min(
    visual3d.animationFrameCount - 1,
    Math.trunc(Number(visual3d.animationPlaybackIndex) || 0),
  ));
  visual3d.animationDurationMs = normalizedVisual3dAnimationDuration();
  if (visual3d.animationMode) {
    visual3d.cells = visual3d.frames[visual3d.animationFrameIndex];
  }
}

function commitVisual3dActiveFrame() {
  if (!visual3d.animationMode) {
    return;
  }
  const cells = visual3d.cells;
  const frameIndex = Math.max(0, Math.trunc(Number(visual3d.animationFrameIndex) || 0));
  ensureVisual3dAnimationState();
  visual3d.animationFrameIndex = Math.min(frameIndex, visual3d.animationFrameCount - 1);
  visual3d.frames[visual3d.animationFrameIndex] = normalizeVisual3dFrameCells(cells);
  visual3d.cells = visual3d.frames[visual3d.animationFrameIndex];
}

function setVisual3dAnimationMode(enabled, options = {}) {
  visual3d.animationMode = Boolean(enabled);
  if (visual3d.animationMode) {
    ensureVisual3dAnimationState();
  } else {
    visual3d.animationPlaying = false;
  }
  if (options.render !== false) {
    renderVisual3dBuilder();
  }
  syncPreviewModeButtonState();
}

function setVisual3dAnimationFrame(index) {
  selectSharedVisualAnimationFrame("visual3d", index);
}

function setVisual3dAnimationFrameCount(value) {
  const before = visualEditSnapshot("visual3d");
  commitVisual3dActiveFrame();
  visual3d.animationFrameCount = Math.max(1, Math.min(
    VISUAL3D_ANIMATION_MAX_FRAMES,
    Math.trunc(Number(value) || 1),
  ));
  ensureVisual3dAnimationState();
  visual3d.animationFrameIndex = Math.min(visual3d.animationFrameIndex, visual3d.animationFrameCount - 1);
  visual3d.cells = visual3d.frames[visual3d.animationFrameIndex];
  renderVisual3dBuilder();
  pushVisualEditUndoSnapshot("visual3d", before);
}

function moveVisual3dAnimationFrame(delta) {
  moveSharedVisualAnimationFrame("visual3d", delta);
}

function insertVisual3dAnimationFrameAt(index) {
  return insertSharedVisualAnimationFrameAt("visual3d", index);
}

function removeVisual3dAnimationFrameAt(index) {
  return removeSharedVisualAnimationFrameAt("visual3d", index);
}

function setVisual3dAnimationDuration(value) {
  const before = visualEditSnapshot("visual3d");
  visual3d.animationDurationMs = normalizedVisual3dAnimationDuration(value);
  renderVisual3dControls();
  pushVisualEditUndoSnapshot("visual3d", before);
}

function resetVisual3dBuilder(
  width = visual3d.width,
  height = visual3d.height,
  depth = visual3d.depth,
) {
  resetVisual3dClipState({ clipboard: true });
  ensureVisual3dPalette();
  visual3d.width = clampVisual3dSize(width);
  visual3d.height = clampVisual3dSize(height);
  visual3d.depth = clampVisual3dSize(depth);
  visual3d.slice = Math.max(0, Math.min(visual3dAxisSize() - 1, Number(visual3d.slice) || 0));
  visual3d.hoverSlice = null;
  visual3d.cells = Array.from({ length: visual3dFrameCellCount() }, () => null);
  visual3d.frames = [visual3d.cells];
  visual3d.animationFrameIndex = 0;
  visual3d.animationFrameCount = 1;
  visual3d.animationPlaybackIndex = 0;
  visual3d.sourcePreludeRows = [];
  visual3d.sourceSpatialOps = [];
  if (!validVisual3dColorIndex(visual3d.selectedColorIndex)) {
    visual3d.selectedColorIndex = 0;
  }
  renderVisual3dBuilder();
}

function clampVisual3dSize(value) {
  const size = Math.trunc(Number(value) || 5);
  return Math.max(1, Math.min(VISUAL3D_EDITOR_MAX_SIZE, size));
}

function withVisual3dPaneScrollPreserved(render) {
  return withVisualPaneScrollPreserved(visual3dBuilder, render);
}

function renderVisual3dBuilder() {
  if (!visual3dBuilder || !visual3dSliceBoard || !visual3dPalette || !visual3dPreviewCanvas) {
    return;
  }
  withVisual3dPaneScrollPreserved(() => {
    mountSharedVisualAnimationUi("3d");
    commitVisual3dActiveFrame();
    visual3dBuilder.classList.toggle("is-animation-mode", Boolean(visual3d.animationMode));
    renderVisual3dControls();
    renderVisual3dPalette();
    renderVisual3dSliceBoard();
    renderVisual3dPreview();
    renderVisual3dAnimationFrameStrip();
    syncVisualAnimationPlayback();
    syncVisual3dSourceActionButtons();
  });
}

function visual3dAnimationFramePreview(frame) {
  const canvas = document.createElement("canvas");
  canvas.className = "visual-animation-3d-preview";
  canvas.width = 52;
  canvas.height = 52;
  canvas.setAttribute("aria-hidden", "true");
  renderVisual3dPreviewCanvas(canvas, frame, { overlays: false });
  return [canvas];
}

function renderVisual3dAnimationFrameStrip() {
  if (!visual3dAnimationFrameStrip || !visual3d.animationMode) {
    return;
  }
  ensureVisual3dAnimationState();
  const plane = visual3dPlaneSize();
  renderVisualAnimationFrameStripView({
    target: visual3dAnimationFrameStrip,
    frameCount: visual3d.animationFrameCount,
    activeIndex: visual3d.animationFrameIndex,
    playingIndex: visual3d.animationPlaybackIndex,
    size: Math.max(plane.width, plane.height),
    renderCells: (index) => visual3dAnimationFramePreview(visual3d.frames[index]),
    onSelect: setVisual3dAnimationFrame,
    noun: "3D visual animation",
  });
}

function renderVisual3dControls() {
  withVisual3dPaneScrollPreserved(() => {
    renderVisualEditorUpperControls(
      visual3dBuilder.querySelector(".visual-controls"),
      visualEditorUpperControls3d(),
    );
    visual3dNameInput.value = visual3dNameInput.value || "VoxelVisual";
    renderVisualShapeBindControl(visual3dShapeField, {
      state: visual3d,
      render: renderVisual3dControls,
      onChange: () => {
        syncVisual3dSourceActionButtons();
        renderVisual3dBuilder();
      },
    });
    if (visual3d.animationMode) {
      ensureVisual3dAnimationState();
    }
    visual3dWidthInput.value = String(visual3d.width);
    visual3dHeightInput.value = String(visual3d.height);
    visual3dDepthInput.value = String(visual3d.depth);
    syncVisual3dBucketButton();
    syncVisual3dTranslateButton();
    syncVisualMarkerControl();
    syncVisual3dGridButton();
    renderVisual3dClipActions();
    renderVisual3dScopeControl();
    renderVisual3dEditorToolbar();
    renderVisual3dCameraControls();
    renderVisualScaleControl({
      size: Math.max(visual3d.width, visual3d.height, visual3d.depth),
      maxSize: VISUAL3D_EDITOR_MAX_SIZE,
      scaleInput: visual3dScaleInput,
      scaleUpButton: visual3dScaleUpButton,
      scaleDownButton: visual3dScaleDownButton,
      canScaleDown: canScaleDownVisual3d,
      noun: "3D visual",
    });
    if (visual3dSliceValue instanceof HTMLInputElement) {
      visual3dSliceValue.min = "1";
      visual3dSliceValue.max = String(visual3dAxisSize());
      visual3dSliceValue.value = String(visual3d.slice + 1);
    } else if (visual3dSliceValue) {
      visual3dSliceValue.textContent = `${visual3d.slice + 1} / ${visual3dAxisSize()}`;
    }
    if (visual3dAnimationDurationInput) {
      visual3dAnimationDurationInput.value = String(normalizedVisual3dAnimationDuration());
    }
    if (visual3dAnimationFrameCountInput) {
      visual3dAnimationFrameCountInput.value = String(visual3d.animationFrameCount || 1);
    }
    if (visual3dAnimationFrameInput) {
      visual3dAnimationFrameInput.value = String((visual3d.animationFrameIndex || 0) + 1);
      visual3dAnimationFrameInput.max = String(visual3d.animationFrameCount || 1);
    }
    if (visual3dAnimationFrameTotal) {
      visual3dAnimationFrameTotal.textContent = String(visual3d.animationFrameCount || 1);
    }
    syncSharedVisualAnimationToolbarState(visual3d.animationFrameCount || 1, VISUAL3D_ANIMATION_MAX_FRAMES);
    const sliceTotal = document.querySelector("#visual3dSliceTotal");
    if (sliceTotal) {
      sliceTotal.textContent = String(visual3dAxisSize());
    }
    if (visual3dPreviousSliceButton) {
      visual3dPreviousSliceButton.disabled = visual3d.slice <= 0;
      visual3dPreviousSliceButton.dataset.tooltip = "Previous slice";
    }
    if (visual3dNextSliceButton) {
      visual3dNextSliceButton.disabled = visual3d.slice >= visual3dAxisSize() - 1;
      visual3dNextSliceButton.dataset.tooltip = "Next slice";
    }
    for (const button of visual3dAxisButtons) {
      const active = button.dataset.visual3dAxis === visual3d.axis;
      button.classList.toggle("is-active", active);
      button.setAttribute("aria-pressed", String(active));
      button.dataset.tooltip = `${button.dataset.visual3dAxis.toUpperCase()} axis`;
    }
  });
}

function renderVisual3dEditorToolbar() {
  renderVisualEditorToolbar({ dimension: "3d", target: visual3dToolbarHost });
}

function selectVisual3dBrushSize(size) {
  visualBrushSizePx = normalizeVisualBrushSize(size);
  visual3dBucketActive = false;
  visual3dTranslateActive = false;
  deactivateVisual3dClipMode({ render: false });
  syncVisualMarkerControl();
  renderVisual3dBuilder();
  setVisual3dActionStatus(`Brush: ${visualBrushSizePx}px`, "is-ok");
}

function syncVisual3dGridButton() {
  if (!visualGridButton) {
    return;
  }
  visualGridButton.classList.toggle("is-active", visual3dGridVisible);
  visualGridButton.setAttribute("aria-pressed", String(visual3dGridVisible));
  visualGridButton.title = "Toggle grid";
  visualGridButton.setAttribute("aria-label", "Toggle 3D visual slice grid");
}

function toggleVisual3dGrid() {
  visual3dGridVisible = !visual3dGridVisible;
  syncVisual3dGridButton();
  renderVisual3dSliceBoard();
  renderVisual3dPresentationSurfaces();
  setVisual3dActionStatus(visual3dGridVisible ? "3D visual grid visible" : "3D visual grid hidden", "is-ok");
}

function visual3dEditScope() {
  if (visual3d.editScope !== "all") {
    visual3d.editScope = "slice";
  }
  return visual3d.editScope;
}

function renderVisual3dScopeControl() {
  const scope = visual3dEditScope();
  const buttons = [
    {
      button: visual3dScopeSliceButton,
      scope: "slice",
      label: "Scope 2D",
      title: "Scope 2D slice",
    },
    {
      button: visual3dScopeAllButton,
      scope: "all",
      label: "Scope 3D",
      title: "Scope 3D volume",
    },
  ];
  for (const item of buttons) {
    if (!item.button) {
      continue;
    }
    const active = item.scope === scope;
    item.button.classList.toggle("is-active", active);
    item.button.setAttribute("aria-label", item.label);
    item.button.setAttribute("aria-pressed", String(active));
    item.button.title = item.title;
  }
  updateVisual3dScopedActionLabels();
}

function updateVisual3dScopedActionLabels() {
  const isAll = visual3dEditScope() === "all";
  const target = isAll ? "whole visual" : "current slice";
  setVisual3dButtonLabel(visual3dRotatePlaneLeftButton, `Rotate ${target} CCW`);
  setVisual3dButtonLabel(visual3dRotatePlaneRightButton, `Rotate ${target} CW`);
  setVisual3dButtonLabel(visual3dFlipPlaneHorizontalButton, `Flip ${target} horizontally`);
  setVisual3dButtonLabel(visual3dFlipPlaneVerticalButton, `Flip ${target} vertically`);
  setVisual3dButtonLabel(visual3dFillButton, "Fill");
  visual3dFillButton.dataset.tooltip = "Fill";
  syncVisualEditCommandLabels("3d");
  renderVisual3dClipActions();
  syncVisual3dTranslateButton();
}

function syncVisual3dTranslateButton() {
  if (!visual3dTranslateButton) {
    return;
  }
  visual3dTranslateButton.classList.toggle("is-active", visual3dTranslateActive);
  visual3dTranslateButton.setAttribute("aria-pressed", String(visual3dTranslateActive));
  visual3dTranslateButton.setAttribute("aria-label", "Move");
  visual3dTranslateButton.title = "Move";
  visual3dTranslateButton.dataset.tooltip = "Move";
}

function renderVisual3dClipActions() {
  if (!visual3dClipActions) {
    return;
  }
  const actions = document.createElement("span");
  actions.className = "visual-clip-actions";
  const button = renderVisualClipButton({
    title: "Clip",
    ariaLabel: "Clip",
    active: visual3dClipActive,
    icon: visualLucideIconSvg("mouse-pointer-2"),
  });
  button.dataset.tooltip = "Clip";
  actions.append(button);
  visual3dClipActions.replaceChildren(actions);
}

function toggleVisual3dClipMode() {
  if (visual3dClipActive) {
    deactivateVisual3dClipMode();
    setVisual3dActionStatus("Brush: paint individual voxels", "is-ok");
    return;
  }
  visual3dBucketActive = false;
  visual3dTranslateActive = false;
  visual3dClipActive = true;
  visual3dClipDrag = null;
  renderVisual3dBuilder();
  setVisual3dActionStatus(
    visual3dClipSelection ? "Clip: drag selection to move it" : "Clip: drag to select an area",
    "is-ok",
  );
}

function deactivateVisual3dClipMode(options = {}) {
  const wasActive = visual3dClipActive || visual3dClipSelection || visual3dClipDrag || visual3dClipFloating;
  visual3dClipActive = false;
  visual3dClipDrag = null;
  visual3dClipFloating = null;
  if (options.clearSelection !== false) {
    visual3dClipSelection = null;
  }
  if (options.render === false || !wasActive) {
    return;
  }
  renderVisual3dBuilder();
}

function resetVisual3dClipState(options = {}) {
  visual3dClipActive = false;
  visual3dClipSelection = null;
  visual3dClipDrag = null;
  visual3dClipFloating = null;
  if (options.clipboard) {
    visual3dClipClipboard = null;
  }
}

function normalizeVisual3dClipBox(box) {
  if (!box) {
    return null;
  }
  const next = {};
  for (const axis of ["x", "y", "z"]) {
    const min = Math.trunc(Number(box[`min${axis.toUpperCase()}`]));
    const max = Math.trunc(Number(box[`max${axis.toUpperCase()}`]));
    const limit = visual3dAxisSize(axis);
    if (!Number.isInteger(min) || !Number.isInteger(max) || min < 0 || max < min || max >= limit) {
      return null;
    }
    next[`min${axis.toUpperCase()}`] = min;
    next[`max${axis.toUpperCase()}`] = max;
  }
  return next;
}

function visual3dClipBoxDimensions(box = visual3dClipSelection) {
  const normalized = normalizeVisual3dClipBox(box);
  return normalized ? {
    width: normalized.maxX - normalized.minX + 1,
    height: normalized.maxY - normalized.minY + 1,
    depth: normalized.maxZ - normalized.minZ + 1,
  } : null;
}

function visual3dClipBoxContainsCoords(box, coords) {
  const normalized = normalizeVisual3dClipBox(box);
  return Boolean(normalized && coords
    && coords.x >= normalized.minX && coords.x <= normalized.maxX
    && coords.y >= normalized.minY && coords.y <= normalized.maxY
    && coords.z >= normalized.minZ && coords.z <= normalized.maxZ);
}

function visual3dClipPlaneRect(box = visual3dClipSelection, axis = visual3d.axis) {
  const normalized = normalizeVisual3dClipBox(box);
  if (!normalized) {
    return null;
  }
  const corners = [];
  for (const x of [normalized.minX, normalized.maxX]) {
    for (const y of [normalized.minY, normalized.maxY]) {
      for (const z of [normalized.minZ, normalized.maxZ]) {
        corners.push(visual3dPlaneCoordinates(axis, x, y, z));
      }
    }
  }
  const us = corners.map((point) => point.u);
  const vs = corners.map((point) => point.v);
  const minU = Math.min(...us);
  const maxU = Math.max(...us);
  const minV = Math.min(...vs);
  const maxV = Math.max(...vs);
  return { x: minU, y: minV, width: maxU - minU + 1, height: maxV - minV + 1 };
}

function visual3dClipRectFromCells(start, end) {
  return {
    x: Math.min(start.x, end.x),
    y: Math.min(start.y, end.y),
    width: Math.abs(end.x - start.x) + 1,
    height: Math.abs(end.y - start.y) + 1,
  };
}

function visual3dClipBoxFromPlaneRect(rect, options = {}) {
  if (!rect) {
    return null;
  }
  const existing = normalizeVisual3dClipBox(options.base);
  const fullDepth = options.fullDepth === true;
  const fixedStack = visual3dPlaneWorldSlice(visual3d.axis, visual3d.slice);
  const points = [
    visual3dCoordsFromPlane(visual3d.axis, visual3d.slice, rect.x, rect.y),
    visual3dCoordsFromPlane(visual3d.axis, visual3d.slice, rect.x + rect.width - 1, rect.y + rect.height - 1),
  ];
  const box = existing || {
    minX: 0, maxX: visual3d.width - 1,
    minY: 0, maxY: visual3d.height - 1,
    minZ: 0, maxZ: visual3d.depth - 1,
  };
  for (const worldAxis of ["x", "y", "z"]) {
    if (worldAxis === visual3d.axis) {
      if (!existing) {
        box[`min${worldAxis.toUpperCase()}`] = fullDepth ? 0 : fixedStack;
        box[`max${worldAxis.toUpperCase()}`] = fullDepth ? visual3dAxisSize(worldAxis) - 1 : fixedStack;
      }
      continue;
    }
    const values = points.map((point) => point[worldAxis]);
    box[`min${worldAxis.toUpperCase()}`] = Math.min(...values);
    box[`max${worldAxis.toUpperCase()}`] = Math.max(...values);
  }
  return normalizeVisual3dClipBox(box);
}

function visual3dClipSelectionContainsSliceCell(cell) {
  const rect = visual3dClipPlaneRect();
  if (!rect || !cell) {
    return false;
  }
  if (visual3dEditScope() === "slice") {
    const fixed = visual3dPlaneWorldSlice(visual3d.axis, visual3d.slice);
    if (fixed < visual3dClipSelection[`min${visual3d.axis.toUpperCase()}`]
      || fixed > visual3dClipSelection[`max${visual3d.axis.toUpperCase()}`]) {
      return false;
    }
  }
  return cell.x >= rect.x && cell.x < rect.x + rect.width && cell.y >= rect.y && cell.y < rect.y + rect.height;
}

function visual3dClipCellFromClient(clientX, clientY, geometry) {
  if (!geometry || geometry.width <= 0 || geometry.height <= 0) {
    return null;
  }
  const plane = visual3dPlaneSize();
  return {
    x: Math.max(0, Math.min(plane.width - 1, Math.floor(((clientX - geometry.left) / geometry.width) * plane.width))),
    y: Math.max(0, Math.min(plane.height - 1, Math.floor(((clientY - geometry.top) / geometry.height) * plane.height))),
  };
}

function visual3dClipCells(box) {
  const normalized = normalizeVisual3dClipBox(box);
  if (!normalized) {
    return [];
  }
  const cells = [];
  for (let z = normalized.minZ; z <= normalized.maxZ; z += 1) {
    for (let y = normalized.minY; y <= normalized.maxY; y += 1) {
      for (let x = normalized.minX; x <= normalized.maxX; x += 1) {
        const value = visual3d.cells[visual3dCellIndex(x, y, z)];
        cells.push(validVisual3dColorIndex(value) ? value : null);
      }
    }
  }
  return cells;
}

function visual3dSliceClipCells(rect = visual3dClipPlaneRect()) {
  if (!rect) {
    return [];
  }
  const cells = [];
  for (let v = rect.y; v < rect.y + rect.height; v += 1) {
    for (let u = rect.x; u < rect.x + rect.width; u += 1) {
      const coords = visual3dCoordsFromPlane(visual3d.axis, visual3d.slice, u, v);
      const value = visual3d.cells[visual3dCellIndex(coords.x, coords.y, coords.z)];
      cells.push(validVisual3dColorIndex(value) ? value : null);
    }
  }
  return cells;
}

function visual3dClipClipboardFromSelection(box, dimensions) {
  if (visual3dEditScope() === "slice") {
    const rect = visual3dClipPlaneRect(box);
    return { dimension: "3d", scope: "slice", width: rect.width, height: rect.height, depth: 1,
      cells: visual3dSliceClipCells(rect), colors: visual3dPaletteColors() };
  }
  return { dimension: "3d", scope: "all", ...dimensions, cells: visual3dClipCells(box), colors: visual3dPaletteColors() };
}

function pasteVisual3dClipCell(index, clipboardValue) {
  if (clipboardValue === null) {
    return false;
  }
  if (!validVisual3dColorIndex(clipboardValue)) {
    throw new Error(`Invalid 3D clip palette index ${clipboardValue}`);
  }
  if (visual3d.cells[index] === clipboardValue) {
    return false;
  }
  visual3d.cells[index] = clipboardValue;
  return true;
}

function visual3dClipForCurrentPalette(clipboard) {
  if (!Array.isArray(clipboard?.colors)) return clipboard;
  const palette = visual3dPaletteEntries();
  const colorToIndex = new Map(palette.map((entry, index) => [normalizeVisualColor(entry.color), index]));
  const sourceToTarget = clipboard.colors.map((rawColor) => {
    const color = normalizeVisualColor(rawColor);
    if (color === "#00000000") return null;
    if (!colorToIndex.has(color)) {
      if (palette.length >= VISUAL_COLOR_TOKENS.length) {
        throw new Error("Paste needs more colors than the 3D visual palette can hold");
      }
      colorToIndex.set(color, palette.length);
      palette.push({ color });
    }
    return colorToIndex.get(color);
  });
  return { ...clipboard, cells: clipboard.cells.map((value) => value === null ? null : sourceToTarget[value]) };
}

function setVisual3dClipCells(box, clipboard) {
  const normalized = normalizeVisual3dClipBox(box);
  const dimensions = visual3dClipBoxDimensions(normalized);
  if (!normalized || !dimensions || !clipboard || dimensions.width !== clipboard.width
    || dimensions.height !== clipboard.height || dimensions.depth !== clipboard.depth
    || clipboard.cells.length !== dimensions.width * dimensions.height * dimensions.depth) {
    return false;
  }
  let changed = false;
  let offset = 0;
  for (let z = normalized.minZ; z <= normalized.maxZ; z += 1) {
    for (let y = normalized.minY; y <= normalized.maxY; y += 1) {
      for (let x = normalized.minX; x <= normalized.maxX; x += 1) {
        const index = visual3dCellIndex(x, y, z);
        if (pasteVisual3dClipCell(index, clipboard.cells[offset])) {
          changed = true;
        }
        offset += 1;
      }
    }
  }
  return changed;
}

function setVisual3dSliceClipCells(rect, clipboard) {
  if (!rect || !clipboard || clipboard.scope !== "slice"
    || rect.width !== clipboard.width || rect.height !== clipboard.height
    || clipboard.cells.length !== rect.width * rect.height) {
    return false;
  }
  let changed = false;
  let offset = 0;
  for (let v = rect.y; v < rect.y + rect.height; v += 1) {
    for (let u = rect.x; u < rect.x + rect.width; u += 1) {
      const coords = visual3dCoordsFromPlane(visual3d.axis, visual3d.slice, u, v);
      const index = visual3dCellIndex(coords.x, coords.y, coords.z);
      if (pasteVisual3dClipCell(index, clipboard.cells[offset])) {
        changed = true;
      }
      offset += 1;
    }
  }
  return changed;
}

function clearVisual3dClipBox(box) {
  const normalized = normalizeVisual3dClipBox(box);
  if (!normalized) {
    return false;
  }
  let changed = false;
  for (let z = normalized.minZ; z <= normalized.maxZ; z += 1) {
    for (let y = normalized.minY; y <= normalized.maxY; y += 1) {
      for (let x = normalized.minX; x <= normalized.maxX; x += 1) {
        const index = visual3dCellIndex(x, y, z);
        if (visual3d.cells[index] !== null) {
          visual3d.cells[index] = null;
          changed = true;
        }
      }
    }
  }
  return changed;
}

function commitVisual3dClipMutation(before, changed, message) {
  renderVisual3dBuilder();
  if (!changed) {
    setVisual3dActionStatus("Clip did not change 3D visual", "is-ok");
    return false;
  }
  syncVisual3dSourceActionButtons();
  setVisual3dActionStatus(message, "is-ok");
  setStatus(message, "is-ok");
  pushVisualEditUndoSnapshot("visual3d", before);
  return true;
}

function deleteVisual3dClipSelection() {
  if (visual3dClipFloating) {
    visual3dClipFloating = null;
    visual3dClipSelection = null;
    visual3dClipDrag = null;
    renderVisual3dBuilder();
    setVisual3dActionStatus("Clip preview discarded", "is-ok");
    return true;
  }
  const box = normalizeVisual3dClipBox(visual3dClipSelection);
  if (!box) {
    setVisual3dActionStatus("No clip selection", "is-error");
    return false;
  }
  const before = visualEditSnapshot("visual3d");
  return commitVisual3dClipMutation(before, clearVisual3dClipBox(box), "Deleted selected 3D area");
}

function pasteVisual3dClipClipboard() {
  if (!visual3dClipClipboard) {
    setVisual3dActionStatus("No copied clip", "is-error");
    return false;
  }
  const before = visualEditSnapshot("visual3d");
  let clipboard;
  try {
    clipboard = visual3dClipForCurrentPalette(visual3dClipClipboard);
  } catch (error) {
    setVisual3dActionStatus(error?.message || String(error), "is-error");
    return false;
  }
  if (clipboard.scope === "slice") {
    const baseRect = visual3dClipPlaneRect() || { x: 0, y: 0, width: 1, height: 1 };
    const rect = {
      x: baseRect.x,
      y: baseRect.y,
      width: clipboard.width,
      height: clipboard.height,
    };
    const plane = visual3dPlaneSize();
    if (rect.x + rect.width > plane.width || rect.y + rect.height > plane.height) {
      setVisual3dActionStatus("Copied slice clip does not fit at selection", "is-error");
      return false;
    }
    const target = visual3dClipBoxFromPlaneRect(rect, { fullDepth: false });
    const changed = setVisual3dSliceClipCells(rect, clipboard);
    visual3dClipSelection = target;
    visual3dClipFloating = null;
    commitVisual3dClipMutation(before, changed, `Pasted ${rect.width}x${rect.height} slice clip`);
    return true;
  }
  const base = normalizeVisual3dClipBox(visual3dClipSelection) || {
    minX: 0, maxX: 0, minY: 0, maxY: 0, minZ: 0, maxZ: 0,
  };
  const target = normalizeVisual3dClipBox({
    minX: base.minX,
    maxX: base.minX + clipboard.width - 1,
    minY: base.minY,
    maxY: base.minY + clipboard.height - 1,
    minZ: base.minZ,
    maxZ: base.minZ + clipboard.depth - 1,
  });
  if (!target) {
    setVisual3dActionStatus("Copied clip does not fit at selection", "is-error");
    return false;
  }
  const changed = setVisual3dClipCells(target, clipboard);
  visual3dClipSelection = target;
  visual3dClipFloating = null;
  const dimensions = visual3dClipBoxDimensions(target);
  commitVisual3dClipMutation(before, changed, `Pasted ${dimensions.width}x${dimensions.height}x${dimensions.depth} clip`);
  return true;
}

function visual3dWholeEditBox() {
  if (visual3dEditScope() === "slice") {
    const plane = visual3dPlaneSize();
    return visual3dClipBoxFromPlaneRect({ x: 0, y: 0, width: plane.width, height: plane.height }, { fullDepth: false });
  }
  return { minX: 0, maxX: visual3d.width - 1, minY: 0, maxY: visual3d.height - 1,
    minZ: 0, maxZ: visual3d.depth - 1 };
}

function visual3dEditBox() {
  return visual3dClipActive ? normalizeVisual3dClipBox(visual3dClipSelection) : visual3dWholeEditBox();
}

function visual3dClipboardSourceText(clipboard) {
  const rows = [];
  for (let z = 0; z < clipboard.depth; z += 1) {
    if (z > 0) rows.push("-");
    for (let y = 0; y < clipboard.height; y += 1) {
      const offset = (z * clipboard.height + y) * clipboard.width;
      rows.push(clipboard.cells.slice(offset, offset + clipboard.width)
        .map((value) => validVisual3dColorIndex(value) ? VISUAL_COLOR_TOKENS[value] : ".").join(""));
    }
  }
  return [`colors = ${visual3dPaletteSourceTokens().join(" ")}`, "shape = {", ...rows, "}"].join("\n");
}

async function copyVisual3dEditRegion() {
  const box = visual3dEditBox();
  const dimensions = visual3dClipBoxDimensions(box);
  if (!box || !dimensions) return false;
  visual3dClipClipboard = visual3dClipClipboardFromSelection(box, dimensions);
  try {
    await copyTextToClipboard(visual3dClipboardSourceText(visual3dClipClipboard));
  } catch (error) {
    setVisual3dActionStatus(`Copy failed: ${error?.message || error}`, "is-error");
    return false;
  }
  renderVisual3dBuilder();
  setVisual3dActionStatus(`Copied ${dimensions.width}x${dimensions.height}x${dimensions.depth} edit region`, "is-ok");
  return true;
}

async function cutVisual3dEditRegion() {
  const box = visual3dEditBox();
  if (!box) return false;
  try {
    if (!await copyVisual3dEditRegion()) return false;
  } catch (error) {
    setVisual3dActionStatus(`Copy failed; 3D visual was not cut: ${error?.message || error}`, "is-error");
    return false;
  }
  const before = visualEditSnapshot("visual3d");
  return commitVisual3dClipMutation(before, clearVisual3dClipBox(box), "Cut 3D edit region");
}

function pasteVisual3dEditRegion() {
  if (!visual3dClipClipboard) {
    setVisual3dActionStatus("No copied 3D visual region", "is-error");
    return false;
  }
  const previousSelection = visual3dClipSelection;
  if (!visual3dClipActive) visual3dClipSelection = visual3dWholeEditBox();
  const result = pasteVisual3dClipClipboard();
  if (!visual3dClipActive) visual3dClipSelection = previousSelection;
  return result;
}

function deleteVisual3dEditRegion() {
  if (!visual3dClipActive) {
    deleteVisual3dScoped();
    return true;
  }
  return deleteVisual3dClipSelection();
}

function runVisual3dEditCommand(command) {
  if (visual3dClipActive && !normalizeVisual3dClipBox(visual3dClipSelection)) {
    setVisual3dActionStatus("Select a clip region first", "is-error");
    return false;
  }
  if (command === "copy") return copyVisual3dEditRegion();
  if (command === "cut") return cutVisual3dEditRegion();
  if (command === "paste") return pasteVisual3dEditRegion();
  if (command === "delete") return deleteVisual3dEditRegion();
  throw new Error(`Unknown 3D visual edit command ${command}`);
}

function visual3dClipBoxShiftedInPlane(box, du, dv) {
  const rect = visual3dClipPlaneRect(box);
  if (!rect) {
    return null;
  }
  const targetRect = { ...rect, x: rect.x + du, y: rect.y + dv };
  if (targetRect.x < 0 || targetRect.y < 0
    || targetRect.x + targetRect.width > visual3dPlaneSize().width
    || targetRect.y + targetRect.height > visual3dPlaneSize().height) {
    return null;
  }
  return visual3dClipBoxFromPlaneRect(targetRect, { base: box });
}

function visual3dClipResizeRect(origin, edge, cell) {
  if (!origin || !edge || !cell) {
    return null;
  }
  let left = origin.x;
  let right = origin.x + origin.width - 1;
  let top = origin.y;
  let bottom = origin.y + origin.height - 1;
  if (edge.includes("w")) left = Math.max(0, Math.min(cell.x, right));
  const plane = visual3dPlaneSize();
  if (edge.includes("e")) right = Math.min(plane.width - 1, Math.max(cell.x, left));
  if (edge.includes("n")) top = Math.max(0, Math.min(cell.y, bottom));
  if (edge.includes("s")) bottom = Math.min(plane.height - 1, Math.max(cell.y, top));
  return { x: left, y: top, width: right - left + 1, height: bottom - top + 1 };
}

function toggleVisual3dTranslateMode() {
  if (visual3dTranslateActive) {
    deactivateVisual3dTranslateMode();
    return;
  }
  visual3dBucketActive = false;
  deactivateVisual3dClipMode({ render: false });
  visual3dTranslateActive = true;
  visual3dTranslateDrag = null;
  renderVisual3dBuilder();
  setVisual3dActionStatus(
    visual3dEditScope() === "all" ? "Translate: drag the whole visual" : "Translate: drag the current slice",
    "is-ok",
  );
}

function deactivateVisual3dTranslateMode(options = {}) {
  const wasActive = visual3dTranslateActive || visual3dTranslateDrag;
  if (visual3dTranslateDrag && visual3dSliceBoard.hasPointerCapture?.(visual3dTranslateDrag.pointerId)) {
    visual3dSliceBoard.releasePointerCapture(visual3dTranslateDrag.pointerId);
  }
  visual3dTranslateActive = false;
  visual3dTranslateDrag = null;
  if (options.render === false || !wasActive) {
    return;
  }
  renderVisual3dBuilder();
  setVisual3dActionStatus("Brush: paint individual voxels", "is-ok");
}

function visual3dPositiveModulo(value, size) {
  return ((value % size) + size) % size;
}

function translatedVisual3dCells(originCells, du, dv, scope) {
  const plane = visual3dPlaneSize();
  const next = scope === "all"
    ? Array.from({ length: visual3dFrameCellCount() }, () => null)
    : [...originCells];
  const firstStack = scope === "all" ? 0 : visual3d.slice;
  const lastStack = scope === "all" ? visual3dAxisSize() - 1 : visual3d.slice;
  for (let stack = firstStack; stack <= lastStack; stack += 1) {
    for (let v = 0; v < plane.height; v += 1) {
      for (let u = 0; u < plane.width; u += 1) {
        const source = visual3dCoordsFromPlane(visual3d.axis, stack, u, v);
        const target = visual3dCoordsFromPlane(
          visual3d.axis,
          stack,
          visual3dPositiveModulo(u + du, plane.width),
          visual3dPositiveModulo(v + dv, plane.height),
        );
        next[visual3dCellIndex(target.x, target.y, target.z)] = originCells[visual3dCellIndex(source.x, source.y, source.z)];
      }
    }
  }
  return next;
}

function visual3dCellsEqual(left, right) {
  return left.length === right.length && left.every((value, index) => value === right[index]);
}

function startVisual3dTranslate(event) {
  event.preventDefault();
  const rect = visual3dSliceBoard.getBoundingClientRect();
  visual3dTranslateDrag = {
    pointerId: event.pointerId,
    startClientX: event.clientX,
    startClientY: event.clientY,
    width: rect.width,
    height: rect.height,
    scope: visual3dEditScope(),
    originCells: [...visual3d.cells],
    beforeSnapshot: visualEditSnapshot("visual3d"),
  };
  visual3dSliceBoard.setPointerCapture?.(event.pointerId);
  visual3dSliceBoard.classList.add("is-translating");
}

function continueVisual3dTranslate(event) {
  if (!visual3dTranslateDrag || visual3dTranslateDrag.pointerId !== event.pointerId) {
    return false;
  }
  event.preventDefault();
  const plane = visual3dPlaneSize();
  const du = Math.round((event.clientX - visual3dTranslateDrag.startClientX) / (visual3dTranslateDrag.width / plane.width));
  const dv = Math.round((event.clientY - visual3dTranslateDrag.startClientY) / (visual3dTranslateDrag.height / plane.height));
  visual3d.cells = translatedVisual3dCells(
    visual3dTranslateDrag.originCells,
    du,
    dv,
    visual3dTranslateDrag.scope,
  );
  renderVisual3dSliceBoard();
  renderVisual3dPreview();
  visual3dSliceBoard.classList.add("is-translating");
  return true;
}

function stopVisual3dTranslate(event) {
  if (!visual3dTranslateDrag || visual3dTranslateDrag.pointerId !== event.pointerId) {
    return false;
  }
  if (visual3dSliceBoard.hasPointerCapture?.(event.pointerId)) {
    visual3dSliceBoard.releasePointerCapture(event.pointerId);
  }
  const drag = visual3dTranslateDrag;
  visual3dTranslateDrag = null;
  visual3dSliceBoard.classList.remove("is-translating");
  if (!visual3dCellsEqual(visual3d.cells, drag.originCells)) {
    pushVisualEditUndoSnapshot("visual3d", drag.beforeSnapshot);
    syncVisual3dSourceActionButtons();
  }
  return true;
}

function syncVisual3dBucketButton() {
  if (!visual3dFillButton) {
    return;
  }
  visual3dFillButton.classList.toggle("is-active", visual3dBucketActive);
  visual3dFillButton.setAttribute("aria-pressed", String(visual3dBucketActive));
}

function setVisual3dButtonLabel(button, label) {
  if (!button) {
    return;
  }
  button.setAttribute("aria-label", label);
  button.title = label;
}

function visual3dSquareIconSvg() {
  return `
    ${editorIconSvg("square")}
  `;
}

function visual3dCubeIconSvg() {
  return `
    ${editorIconSvg("box")}
  `;
}

function toggleVisual3dEditScope() {
  setVisual3dEditScope(visual3dEditScope() === "all" ? "slice" : "all");
}

function setVisual3dEditScope(scope) {
  const previousScope = visual3dEditScope();
  visual3d.editScope = scope === "all" ? "all" : "slice";
  if (visual3dClipSelection && previousScope !== visual3d.editScope) {
    const rect = visual3dClipPlaneRect();
    visual3dClipSelection = visual3dClipBoxFromPlaneRect(rect, {
      fullDepth: visual3d.editScope === "all",
    });
    visual3dClipFloating = null;
    visual3dClipDrag = null;
  }
  renderVisual3dScopeControl();
  renderVisual3dSliceBoard();
  setVisual3dActionStatus(
    visual3d.editScope === "all" ? "3D edits affect the whole visual" : "2D edits affect the current slice",
    "is-ok",
  );
}

function toggleVisual3dBucketMode() {
  if (!visual3dBucketActive) {
    deactivateVisual3dClipMode({ render: false });
    visual3dTranslateActive = false;
  }
  visual3dBucketActive = !visual3dBucketActive;
  syncVisual3dBucketButton();
  const scope = visual3dEditScope();
  setVisual3dActionStatus(
    visual3dBucketActive
      ? scope === "all" ? "Bucket: click a voxel to fill its 3D component" : "Bucket: click a slice area to fill its component"
      : "Brush: paint individual voxels",
    "is-ok",
  );
}

function deactivateVisual3dBucketModeAfterUse() {
  if (!visual3dBucketActive) {
    return;
  }
  visual3dBucketActive = false;
  syncVisual3dBucketButton();
}

function renderVisual3dPalette() {
  withVisual3dPaneScrollPreserved(() => renderVisual3dPaletteContent());
}

function setVisual3dCurrentColorTag(index, rawName, linked = true) {
  if (!validVisual3dColorIndex(index)) {
    throw new Error(`Invalid 3D visual palette index ${index}`);
  }
  const name = sanitizeVisualColorAssetRef(rawName);
  if (!name) {
    setVisual3dActionStatus("Enter a color tag name", "is-error");
    return false;
  }
  visual3d.palette[index].bind = { type: "color", name, linked: Boolean(linked) };
  visual3d.colorTagPickerOpen = false;
  syncVisual3dSourceActionButtons();
  renderVisual3dPalette();
  return true;
}

function renderVisual3dPaletteContent() {
  ensureVisual3dPalette();
  visual3dPalette.replaceChildren();
  const selectedIsTransparent = visual3d.selectedColorIndex === null;
  if (selectedIsTransparent || validVisual3dColorIndex(visual3d.selectedColorIndex)) {
    const selected = selectedIsTransparent ? { color: "#00000000" } : visual3dPaletteEntries()[visual3d.selectedColorIndex];
    const selectedBind = selectedIsTransparent ? { available: false, linked: false, name: "" } : visualPaletteEntryBindInfo(selected);
    const selectedDisplayName = selectedBind.linked && selectedBind.name ? selectedBind.name : "";
    const currentWrap = document.createElement("span");
    currentWrap.className = "visual-current-color-wrap";
    const currentButton = document.createElement("button");
    currentButton.type = "button";
    currentButton.className = "visual-current-color-button";
    currentButton.classList.toggle("is-transparent", selectedIsTransparent);
    currentButton.classList.toggle("is-bound", selectedBind.available && selectedBind.linked);
    currentButton.classList.toggle("is-unlinked", selectedBind.available && !selectedBind.linked);
    currentButton.style.setProperty("--visual-current-color", normalizeVisualColor(selected.color));
    currentButton.title = selectedIsTransparent
      ? "Transparent eraser cannot be edited"
      : "Edit selected color";
    currentButton.setAttribute(
      "aria-label",
      selectedIsTransparent
        ? "Selected transparent eraser color #00000000, not editable"
        : selectedDisplayName ? `Edit selected color ${selectedDisplayName}` : `Edit selected color ${selected.color}`,
    );
    currentButton.setAttribute("aria-disabled", String(selectedIsTransparent));
    currentButton.setAttribute("aria-expanded", String(!selectedIsTransparent && visual3d.editPaletteOpen));
    currentButton.innerHTML = `<span class="visual-current-color-swatch" aria-hidden="true"></span>`;
    if (selectedIsTransparent) {
      currentButton.insertAdjacentHTML("beforeend", `
        <span class="visual-current-transparent-icon" aria-hidden="true">
          ${editorIconSvg("eraser")}
        </span>
      `);
    } else {
      currentButton.insertAdjacentHTML("beforeend", `
        <span class="visual-current-edit-icon" aria-hidden="true">
          ${editorIconSvg("pencil")}
        </span>
      `);
    }
    const currentHexInput = document.createElement("input");
    currentHexInput.type = "text";
    currentHexInput.className = "visual-current-value-input visual-current-hex-input";
    currentHexInput.value = selectedDisplayName || (selectedIsTransparent ? "#00000000" : normalizeVisualColor(selected.color));
    currentHexInput.classList.toggle("is-name-mode", Boolean(selectedDisplayName));
    currentHexInput.placeholder = selectedDisplayName ? "color_name" : "#rrggbbaa";
    currentHexInput.spellcheck = false;
    currentHexInput.autocomplete = "off";
    currentHexInput.readOnly = selectedIsTransparent;
    currentHexInput.setAttribute(
      "aria-label",
      selectedIsTransparent
        ? "Transparent color code"
        : selectedDisplayName ? "Selected color tag" : "Selected color code",
    );
    const applyCurrentHex = (options = {}) => {
      if (currentHexInput.classList.contains("is-name-mode")) {
        setVisual3dCurrentColorTag(visual3d.selectedColorIndex, currentHexInput.value, true);
        return;
      }
      const parsed = parseVisualHexColor(currentHexInput.value);
      if (!parsed) {
        if (options.reportError) {
          setVisual3dActionStatus("Use #rrggbb or #rrggbbaa", "is-error");
        }
        return;
      }
      updateSelectedVisual3dColor(parsed, {
        deferHistory: !options.commitHistory,
        commitHistory: Boolean(options.commitHistory),
      });
    };
    let pendingEditMenu = null;
    if (!selectedIsTransparent) {
      currentButton.addEventListener("click", () => {
        const opening = !visual3d.editPaletteOpen;
        if (!opening) {
          commitVisualColorEditHistory("visual3d");
        }
        visual3d.editPaletteOpen = opening;
        visual3d.addPaletteOpen = false;
        visual3d.addDraftColorIndex = null;
        visual3d.customColorOpen = opening;
        renderVisual3dPalette();
      });
      currentHexInput.addEventListener("input", () => applyCurrentHex());
      currentHexInput.addEventListener("change", () => applyCurrentHex({ reportError: true, commitHistory: true }));
      currentHexInput.addEventListener("keydown", (event) => {
        event.stopPropagation();
        if (event.key !== "Enter") {
          return;
        }
        event.preventDefault();
        applyCurrentHex({ reportError: true, commitHistory: true });
      });
    }
    currentWrap.append(currentButton, currentHexInput);
    if (!selectedIsTransparent) {
      const bind = visualPaletteEntryBindInfo(selected);
      currentWrap.append(renderVisualCurrentColorTagButton({
        state: visual3d,
        entry: selected,
        onToggle: () => {
          visual3d.editPaletteOpen = false;
          renderVisual3dPalette();
        },
      }));
      if (bind.linked && bind.name) {
        const unlink = document.createElement("button");
        unlink.type = "button";
        unlink.className = "icon-button is-danger visual-current-tag-unlink-button visual-icon-button";
        unlink.title = `Unlink color tag ${bind.name}`;
        unlink.setAttribute("aria-label", unlink.title);
        unlink.innerHTML = visualUnlinkIconSvg();
        unlink.addEventListener("click", () => setVisual3dCurrentColorTag(visual3d.selectedColorIndex, bind.name, false));
        currentWrap.append(unlink);
      }
      if (visual3d.colorTagPickerOpen) {
        const colorAssets = visualSourceColorAssets();
        const picker = renderVisualAssetNamePicker({
          className: "visual-color-tag-picker",
          names: visualColorAssetNames(),
          value: bind.name || defaultVisualAssetName("color", visual3d.selectedColorIndex),
          placeholder: "color_name",
          ariaLabel: "Color tag name",
          emptyText: "No named colors yet",
          optionMeta: (name) => ({ color: colorAssets.get(name) }),
          onCommit: (name) => setVisual3dCurrentColorTag(visual3d.selectedColorIndex, name, true),
          onCancel: () => {
            visual3d.colorTagPickerOpen = false;
            renderVisual3dPalette();
          },
        });
        currentWrap.append(picker);
      }
    }
    if (!selectedIsTransparent && visual3d.editPaletteOpen) {
      const editorPanel = document.createElement("span");
      editorPanel.className = "visual-current-editor-panel";
      const editMenu = renderVisualColorMenu({
        mode: "edit",
        customValue: selected.color,
        onChange: updateSelectedVisual3dColor,
        onPreset: updateSelectedVisual3dColor,
        renderPalette: renderVisual3dPalette,
      });
      editorPanel.append(editMenu);
      currentWrap.append(editorPanel);
      pendingEditMenu = editMenu;
    }
    currentWrap.append(visual3dShapeField);
    visual3dPalette.append(currentWrap);
    if (pendingEditMenu) {
      positionVisualColorMenu(pendingEditMenu, currentButton, { side: "left" });
    }
  }

  renderVisualPaletteGrid({
    target: visual3dPalette,
    leadingControl: visualMarkerTool,
    entries: visual3dPaletteEntries(),
    selectedIndex: visual3d.selectedColorIndex,
    bucketActive: visual3dBucketActive,
    emptyTitle: "Paint empty voxel",
    emptyAriaLabel: "Paint empty voxel",
    colorAriaLabel: (index, name) => name
      ? `Paint 3D visual color ${index + 1}: ${name}`
      : `Paint 3D visual color ${index + 1}`,
    onSelect: selectVisual3dColor,
    onAdd: toggleVisual3dAddPalette,
    onRemove: removeVisual3dColor,
    addOpen: visual3d.addPaletteOpen,
    renderAddMenu: () => renderVisualColorMenu({
      mode: "add",
      customValue: validVisual3dColorIndex(visual3d.addDraftColorIndex)
        ? visual3dPaletteEntries()[visual3d.addDraftColorIndex].color
        : nextVisualPresetColor(visual3dPaletteEntries()),
      onDiscard: cancelVisual3dColorAdd,
      onChange: previewNewVisual3dColor,
      onPreset: previewNewVisual3dColor,
      renderPalette: renderVisual3dPalette,
    }),
  });
}

function renderVisual3dSliceBoard() {
  withVisual3dPaneScrollPreserved(() => {
    visual3dSliceBoard.replaceChildren();
    visual3dSliceBoard.classList.toggle("is-grid-hidden", !visual3dGridVisible);
    visual3dSliceBoard.classList.toggle("is-translate-active", visual3dTranslateActive);
    visual3dSliceBoard.classList.toggle("is-clip-active", visual3dClipActive);
    visual3dSliceBoard.classList.toggle("is-clip-floating", Boolean(visual3dClipFloating));
    const planeSize = visual3dPlaneSize();
    visual3dSliceBoard.style.setProperty("--visual-size", Math.max(planeSize.width, planeSize.height));
    visual3dSliceBoard.style.setProperty("--visual-cols", planeSize.width);
    visual3dSliceBoard.style.setProperty("--visual-rows", planeSize.height);
    const selectionRect = visual3dClipPlaneRect();
    const fixed = visual3dPlaneWorldSlice(visual3d.axis, visual3d.slice);
    const normalKey = `${visual3d.axis.toUpperCase()}`;
    const selectionIntersectsSlice = Boolean(
      visual3dClipSelection
      && fixed >= visual3dClipSelection[`min${normalKey}`]
      && fixed <= visual3dClipSelection[`max${normalKey}`],
    );
    const cellCount = planeSize.width * planeSize.height;
    for (let index = 0; index < cellCount; index += 1) {
      const coords = visual3dCoordsFromSliceCell(index);
      const voxelIndex = visual3dCellIndex(coords.x, coords.y, coords.z);
      const colorIndex = validVisual3dColorIndex(visual3d.cells[voxelIndex]) ? visual3d.cells[voxelIndex] : null;
      const button = document.createElement("button");
      button.type = "button";
      button.className = "visual-cell visual-color-swatch";
      button.dataset.index = String(index);
      button.dataset.voxelIndex = String(voxelIndex);
      button.dataset.colorIndex = colorIndex === null ? "erase" : String(colorIndex);
      const u = index % planeSize.width;
      const v = Math.floor(index / planeSize.width);
      button.classList.toggle("is-clip-selected", Boolean(
        selectionIntersectsSlice
        && selectionRect
        && u >= selectionRect.x
        && u < selectionRect.x + selectionRect.width
        && v >= selectionRect.y
        && v < selectionRect.y + selectionRect.height,
      ));
      button.style.setProperty("--visual-swatch-color", visual3dColorForColorIndex(colorIndex));
      button.style.setProperty("--visual-cell-ink", visual3dInkForColorIndex(colorIndex));
      button.setAttribute("aria-label", `Voxel ${coords.x + 1}, ${coords.y + 1}, ${coords.z + 1}`);
      visual3dSliceBoard.append(button);
    }
    renderVisual3dClipSelectionFrame();
  });
}

function renderVisual3dClipSelectionFrame() {
  const rect = visual3dClipPlaneRect();
  if (!rect) {
    return;
  }
  renderVisual3dClipFloatingPreview(rect);
  const frame = document.createElement("div");
  frame.className = "visual-clip-selection-frame";
  frame.style.setProperty("--visual-clip-x", String(rect.x));
  frame.style.setProperty("--visual-clip-y", String(rect.y));
  frame.style.setProperty("--visual-clip-width", String(rect.width));
  frame.style.setProperty("--visual-clip-height", String(rect.height));
  frame.setAttribute("aria-hidden", "true");
  if (!visual3dClipFloating) {
    for (const edge of ["n", "e", "s", "w"]) {
      const node = document.createElement("span");
      node.className = `visual-clip-selection-edge visual-clip-selection-edge-${edge}`;
      node.dataset.visual3dClipResize = edge;
      frame.append(node);
    }
  }
  for (const handle of ["nw", "ne", "sw", "se"]) {
    const node = document.createElement("span");
    node.className = `visual-clip-selection-handle visual-clip-selection-handle-${handle}`;
    if (!visual3dClipFloating) {
      node.dataset.visual3dClipResize = handle;
    }
    frame.append(node);
  }
  visual3dSliceBoard.append(frame);
}

function visual3dClipFloatingPlaneCells(rect) {
  const box = normalizeVisual3dClipBox(visual3dClipSelection);
  const clipboard = visual3dClipClipboard;
  if (!box || !clipboard || !visual3dClipFloating) {
    return null;
  }
  const fixed = visual3dPlaneWorldSlice(visual3d.axis, visual3d.slice);
  const normalKey = visual3d.axis.toUpperCase();
  if (fixed < box[`min${normalKey}`] || fixed > box[`max${normalKey}`]) {
    return null;
  }
  if (clipboard.scope === "slice") {
    return clipboard.width === rect.width && clipboard.height === rect.height ? clipboard.cells : null;
  }
  const cells = [];
  for (let v = rect.y; v < rect.y + rect.height; v += 1) {
    for (let u = rect.x; u < rect.x + rect.width; u += 1) {
      const coords = visual3dCoordsFromPlane(visual3d.axis, visual3d.slice, u, v);
      const x = coords.x - box.minX;
      const y = coords.y - box.minY;
      const z = coords.z - box.minZ;
      const index = ((z * clipboard.height + y) * clipboard.width) + x;
      cells.push(clipboard.cells[index] ?? null);
    }
  }
  return cells;
}

function renderVisual3dClipFloatingPreview(rect) {
  const cells = visual3dClipFloatingPlaneCells(rect);
  if (!cells) {
    return;
  }
  const preview = document.createElement("div");
  preview.className = `visual-clip-floating-preview is-${visual3dClipFloating.kind || "copy"}`;
  preview.style.setProperty("--visual-clip-x", String(rect.x));
  preview.style.setProperty("--visual-clip-y", String(rect.y));
  preview.style.setProperty("--visual-clip-width", String(rect.width));
  preview.style.setProperty("--visual-clip-height", String(rect.height));
  preview.style.setProperty("--visual-clip-preview-cols", String(rect.width));
  preview.setAttribute("aria-hidden", "true");
  for (const colorIndex of cells) {
    const validIndex = validVisual3dColorIndex(colorIndex) ? colorIndex : null;
    const cell = document.createElement("span");
    cell.className = "visual-clip-preview-cell visual-color-swatch";
    cell.style.setProperty("--visual-swatch-color", visual3dColorForColorIndex(validIndex));
    cell.style.setProperty("--visual-cell-ink", visual3dInkForColorIndex(validIndex));
    preview.append(cell);
  }
  visual3dSliceBoard.append(preview);
}

function renderVisual3dPreview() {
  renderVisual3dPreviewCanvas(visual3dPreviewCanvas, visual3d.cells, { overlays: true });
  renderVisual3dCameraControls();
}

function renderVisual3dPresentationSurfaces() {
  renderVisual3dPreview();
  renderVisual3dAnimationFrameStrip();
  if (visual3d.animationMode) {
    const context = sharedVisualAnimationController("visual3d");
    const frame = context.frames[context.state.animationPlaybackIndex] || context.state.cells;
    renderSharedVisualAnimationPlaybackView(context, frame);
  }
}

function renderVisual3dPreviewCanvas(canvas, cells, options = {}) {
  const rect = canvas.getBoundingClientRect();
  const width = Math.max(1, Math.round(rect.width || canvas.width || 420));
  const height = Math.max(1, Math.round(rect.height || canvas.height || 320));
  const ratio = window.devicePixelRatio || 1;
  if (canvas.width !== Math.round(width * ratio) || canvas.height !== Math.round(height * ratio)) {
    canvas.width = Math.round(width * ratio);
    canvas.height = Math.round(height * ratio);
  }
  const ctx = canvas.getContext("2d");
  if (!ctx) {
    return;
  }
  ctx.setTransform(ratio, 0, 0, ratio, 0, 0);
  ctx.clearRect(0, 0, width, height);
  ctx.fillStyle = visual3dCssVar("--visual3d-preview-bg", "#1d2023");
  ctx.fillRect(0, 0, width, height);

  const view = visual3dPreviewView(width, height, {
    reserveOverlaySpace: options.overlays !== false,
  });
  drawVisual3dBounds(ctx, view);

  const occupied = visual3dOccupancyMap(cells);
  const faces = visual3dMergedVoxelFaces(occupied, view);
  const previewOwner = visual3dPreviewRenderOwner();
  const sceneFaces = [
    ...faces.map((face) => ({ ...face, kind: "voxel", ownerCell: previewOwner, renderPriority: 0 })),
    ...(options.overlays === false ? [] : visual3dSliceSurfaceFaces(visual3d.hoverSlice, view, "hover", occupied, 1)
      .map((face) => ({ ...face, ownerCell: previewOwner }))),
    ...(options.overlays === false ? [] : visual3dSliceSurfaceFaces(visual3d.slice, view, "active", occupied, 2)
      .map((face) => ({ ...face, ownerCell: previewOwner }))),
  ];
  assignVisual3dPrimitiveOrder(sceneFaces);
  sceneFaces.sort(Puzzle3VisualCore.comparePrimitiveOrder);
  for (const face of sceneFaces) {
    if (face.kind === "slice") {
      drawVisual3dSliceFace(ctx, face, face.mode);
    } else {
      drawVisual3dFace(ctx, face);
    }
  }
  if (options.overlays !== false) {
    drawVisual3dClipBounds(ctx, view);
    canvas._visual3dPreviewView = view;
  }
}

function visual3dPreviewView(width, height, options = {}) {
  const reserveOverlaySpace = options.reserveOverlaySpace !== false;
  const padding = 0;
  const contentPadding = reserveOverlaySpace ? padding : 3;
  const overlayControlHeight = Number.parseFloat(
    visual3dCssVar("--visual3d-overlay-control-height", "22"),
  );
  const overlaySafeInset = 8 + overlayControlHeight + 4;
  const safeTop = reserveOverlaySpace ? overlaySafeInset : 0;
  const safeBottom = reserveOverlaySpace ? overlaySafeInset : 0;
  const boundsView = {
    cellScale: 1,
    originX: 0,
    originY: 0,
  };
  const points = visual3dBoundsCorners().map((corner) => visual3dProject(corner, boundsView));
  const minX = Math.min(...points.map((point) => point.x));
  const maxX = Math.max(...points.map((point) => point.x));
  const minY = Math.min(...points.map((point) => point.y));
  const maxY = Math.max(...points.map((point) => point.y));
  const projectedWidth = Math.max(1, maxX - minX);
  const projectedHeight = Math.max(1, maxY - minY);
  const availableWidth = Math.max(1, width - contentPadding * 2);
  const safeHeight = Math.max(1, height - safeTop - safeBottom);
  const availableHeight = Math.max(1, safeHeight - contentPadding * 2);
  const scale = Math.max(4, Math.min(availableWidth / projectedWidth, availableHeight / projectedHeight) * VISUAL3D_PREVIEW_BASE_ZOOM)
    * visual3dCamera().zoom;
  return {
    cellScale: scale,
    originX: width / 2 - ((minX + maxX) / 2) * scale,
    originY: safeTop + safeHeight / 2 - ((minY + maxY) / 2) * scale,
  };
}

function visual3dBoundsCorners() {
  const min = -0.5;
  const maxX = visual3d.width - 0.5;
  const maxY = visual3d.height - 0.5;
  const maxDepth = visual3d.depth - 0.5;
  return [
    { x: min, y: min, z: min },
    { x: maxX, y: min, z: min },
    { x: maxX, y: maxY, z: min },
    { x: min, y: maxY, z: min },
    { x: min, y: min, z: maxDepth },
    { x: maxX, y: min, z: maxDepth },
    { x: maxX, y: maxY, z: maxDepth },
    { x: min, y: maxY, z: maxDepth },
  ];
}

function visual3dCamera() {
  if (!visual3d.camera) {
    visual3d.camera = { ...VISUAL3D_CAMERA_DEFAULT };
  }
  visual3d.camera.yawDegrees = visual3dNormalizeDegrees(visual3d.camera.yawDegrees ?? VISUAL3D_CAMERA_DEFAULT.yawDegrees);
  visual3d.camera.pitchDegrees = visual3dClampNumber(
    visual3d.camera.pitchDegrees ?? VISUAL3D_CAMERA_DEFAULT.pitchDegrees,
    VISUAL3D_CAMERA_MIN_PITCH_DEGREES,
    VISUAL3D_CAMERA_MAX_PITCH_DEGREES,
  );
  visual3d.camera.zoom = visual3dClampNumber(visual3d.camera.zoom ?? VISUAL3D_CAMERA_DEFAULT.zoom, 0.25, 4);
  return visual3d.camera;
}

function visual3dFaceGridOrder(corners) {
  return Puzzle3VisualCore.faceGridOrder(corners, visual3dVisualView());
}

function visual3dVisualView() {
  return { camera: visual3dCamera() };
}

function visual3dPreviewRenderOwner() {
  return {
    key: "visual3d-preview",
    order: { x: 0, y: 0, z: 0 },
    depth: 0,
  };
}

function assignVisual3dPrimitiveOrder(primitives) {
  const keyCounts = new Map();
  for (const [index, primitive] of primitives.entries()) {
    const baseKey = primitive.key
      ? String(primitive.key)
      : `${primitive.kind || "primitive"}:${index}`;
    const occurrence = keyCounts.get(baseKey) || 0;
    keyCounts.set(baseKey, occurrence + 1);
    primitive.frameIndex = index;
    primitive.stableKey = occurrence === 0 ? baseKey : `${baseKey}#${occurrence}`;
  }
}

function renderVisual3dCameraControls() {
  const camera = visual3dCamera();
  renderVisual3dCameraScrub(visual3dCameraYawScrub, "yaw", Math.round(camera.yawDegrees));
  renderVisual3dCameraScrub(visual3dCameraPitchScrub, "pitch", Math.round(camera.pitchDegrees));
  renderVisual3dCameraScrub(visual3dCameraZoomScrub, "zoom", Number(camera.zoom.toFixed(2)));
}

function renderVisual3dCameraScrub(element, kind, value) {
  if (!(element instanceof HTMLElement)) {
    return;
  }
  const text = String(value);
  element.textContent = text;
  element.setAttribute("aria-label", `Drag vertically to adjust ${kind}, current ${text}`);
}

function visual3dClampNumber(value, min, max) {
  const parsed = Number(value);
  const fallback = min <= 0 && max >= 0 ? 0 : min;
  return Math.min(max, Math.max(min, Number.isFinite(parsed) ? parsed : fallback));
}

function visual3dOccupancyMap(cells = visual3d.cells) {
  const occupied = new Map();
  for (let z = 0; z < visual3d.depth; z += 1) {
    for (let y = 0; y < visual3d.height; y += 1) {
      for (let x = 0; x < visual3d.width; x += 1) {
        const colorIndex = cells?.[visual3dCellIndex(x, y, z)];
        if (validVisual3dColorIndex(colorIndex)) {
          occupied.set(visual3dVoxelKey(x, y, z), {
            x,
            y,
            z,
            colorIndex,
            opaque: visual3dColorIsOpaque(visual3dColorForColorIndex(colorIndex)),
          });
        }
      }
    }
  }
  return occupied;
}

function visual3dCssVar(name, fallback) {
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return value || fallback;
}

function drawVisual3dBounds(ctx, view) {
  const z = -0.5;
  const corners = [
    visual3dProject({ x: -0.5, y: -0.5, z }, view),
    visual3dProject({ x: visual3d.width - 0.5, y: -0.5, z }, view),
    visual3dProject({ x: visual3d.width - 0.5, y: visual3d.height - 0.5, z }, view),
    visual3dProject({ x: -0.5, y: visual3d.height - 0.5, z }, view),
  ];
  ctx.fillStyle = visual3dCssVar("--visual3d-frame-fill", "rgba(137, 148, 158, 0.10)");
  ctx.strokeStyle = visual3dCssVar("--visual3d-frame-stroke", "rgba(137, 148, 158, 0.38)");
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(corners[0].x, corners[0].y);
  for (const point of corners.slice(1)) {
    ctx.lineTo(point.x, point.y);
  }
  ctx.closePath();
  ctx.fill();
  ctx.stroke();
}

function drawVisual3dClipBounds(ctx, view) {
  const box = normalizeVisual3dClipBox(visual3dClipSelection);
  if (!box) {
    return;
  }
  const min = { x: box.minX - 0.5, y: box.minY - 0.5, z: box.minZ - 0.5 };
  const max = { x: box.maxX + 0.5, y: box.maxY + 0.5, z: box.maxZ + 0.5 };
  const corners = [
    { x: min.x, y: min.y, z: min.z },
    { x: max.x, y: min.y, z: min.z },
    { x: max.x, y: max.y, z: min.z },
    { x: min.x, y: max.y, z: min.z },
    { x: min.x, y: min.y, z: max.z },
    { x: max.x, y: min.y, z: max.z },
    { x: max.x, y: max.y, z: max.z },
    { x: min.x, y: max.y, z: max.z },
  ].map((point) => visual3dProject(point, view));
  const faces = [
    [0, 1, 2, 3],
    [4, 5, 6, 7],
    [0, 1, 5, 4],
    [1, 2, 6, 5],
    [2, 3, 7, 6],
    [3, 0, 4, 7],
  ];
  ctx.fillStyle = visual3dCssVar("--visual3d-clip-fill", "rgba(125, 208, 160, 0.08)");
  for (const face of faces) {
    ctx.beginPath();
    ctx.moveTo(corners[face[0]].x, corners[face[0]].y);
    for (const index of face.slice(1)) {
      ctx.lineTo(corners[index].x, corners[index].y);
    }
    ctx.closePath();
    ctx.fill();
  }
  const edges = [
    [0, 1], [1, 2], [2, 3], [3, 0],
    [4, 5], [5, 6], [6, 7], [7, 4],
    [0, 4], [1, 5], [2, 6], [3, 7],
  ];
  ctx.strokeStyle = visual3dCssVar("--visual3d-clip-stroke", "rgba(125, 208, 160, 0.9)");
  ctx.lineWidth = 1.5;
  ctx.beginPath();
  for (const [from, to] of edges) {
    ctx.moveTo(corners[from].x, corners[from].y);
    ctx.lineTo(corners[to].x, corners[to].y);
  }
  ctx.stroke();
}

function visual3dSliceHitPlanes(view) {
  const hitPlanes = [];
  for (let index = 0; index < visual3dAxisSize(); index += 1) {
    hitPlanes.push({ index, points: visual3dSliceHitPlaneCorners(index, view) });
  }
  return hitPlanes;
}

function visual3dSliceHitEdges(view) {
  if (visual3d.axis !== "z") {
    return [];
  }
  const min = -0.5;
  const maxX = visual3d.width - 0.5;
  const maxY = visual3d.height - 0.5;
  const maxDepth = visual3d.depth - 0.5;
  return [
    { x: min, y: min },
    { x: maxX, y: min },
    { x: maxX, y: maxY },
    { x: min, y: maxY },
  ].map((edge) => {
    const from = visual3dProject({ x: edge.x, y: edge.y, z: min }, view);
    const to = visual3dProject({ x: edge.x, y: edge.y, z: maxDepth }, view);
    return {
      axis: "z",
      from,
      to,
      min,
      max: maxDepth,
      hitRadius: visual3dClamp(view.cellScale * 0.34, 8, 18),
    };
  });
}

function visual3dSliceHitPlaneCorners(slice, view) {
  const min = -0.5;
  const maxX = visual3d.width - 0.5;
  const maxY = visual3d.height - 0.5;
  const maxDepth = visual3d.depth - 0.5;
  const fixed = visual3dPlaneWorldSlice(visual3d.axis, slice);
  let corners = [];
  if (visual3d.axis === "x") {
    corners = [
      { x: fixed, y: min, z: min },
      { x: fixed, y: min, z: maxDepth },
      { x: fixed, y: maxY, z: maxDepth },
      { x: fixed, y: maxY, z: min },
    ];
  } else if (visual3d.axis === "y") {
    corners = [
      { x: min, y: fixed, z: min },
      { x: maxX, y: fixed, z: min },
      { x: maxX, y: fixed, z: maxDepth },
      { x: min, y: fixed, z: maxDepth },
    ];
  } else {
    corners = [
      { x: min, y: min, z: fixed },
      { x: maxX, y: min, z: fixed },
      { x: maxX, y: maxY, z: fixed },
      { x: min, y: maxY, z: fixed },
    ];
  }
  return corners.map((corner) => visual3dProject(corner, view));
}

function visual3dSliceSurfaceFaces(slice, view, mode, occupied, order = 0) {
  if (!Number.isInteger(slice)) {
    return [];
  }
  const groups = new Map();
  const fill = visual3dSliceOverlayFill(mode);
  const stroke = visual3dSliceOverlayStroke(mode);
  const plane = visual3dPlaneSize();
  for (let row = 0; row < plane.height; row += 1) {
    for (let col = 0; col < plane.width; col += 1) {
      const grid = visual3dCoordsFromPlane(visual3d.axis, slice, col, row);
      if (occupied.has(visual3dVoxelKey(grid.x, grid.y, grid.z))) {
        continue;
      }
      visual3dAddSliceSurfaceFace(groups, "zNeg", grid, { x: grid.x, y: grid.y, z: grid.z - 1 }, slice, occupied);
      visual3dAddSliceSurfaceFace(groups, "zPos", grid, { x: grid.x, y: grid.y, z: grid.z + 1 }, slice, occupied);
      visual3dAddSliceSurfaceFace(groups, "xNeg", grid, { x: grid.x - 1, y: grid.y, z: grid.z }, slice, occupied);
      visual3dAddSliceSurfaceFace(groups, "xPos", grid, { x: grid.x + 1, y: grid.y, z: grid.z }, slice, occupied);
      visual3dAddSliceSurfaceFace(groups, "yPos", grid, { x: grid.x, y: grid.y + 1, z: grid.z }, slice, occupied);
      visual3dAddSliceSurfaceFace(groups, "yNeg", grid, { x: grid.x, y: grid.y - 1, z: grid.z }, slice, occupied);
    }
  }
  return visual3dMergedSliceFaces(groups, view, fill, stroke, mode, order);
}

function visual3dAddSliceSurfaceFace(groups, side, grid, neighbor, slice, occupied) {
  if (visual3dGridInSliceVolume(neighbor, slice) || occupied.has(visual3dVoxelKey(neighbor.x, neighbor.y, neighbor.z))) {
    return;
  }
  const info = visual3dSliceFaceGroupInfo(side, grid);
  let group = groups.get(info.key);
  if (!group) {
    group = {
      side,
      planeIndex: info.planeIndex,
      cells: new Set(),
    };
    groups.set(info.key, group);
  }
  group.cells.add(`${info.u},${info.v}`);
}

function visual3dSliceFaceGroupInfo(side, grid) {
  if (side === "zNeg") {
    return { key: `${side}:${grid.z}`, planeIndex: grid.z, u: grid.x, v: grid.y };
  }
  if (side === "zPos") {
    return { key: `${side}:${grid.z + 1}`, planeIndex: grid.z + 1, u: grid.x, v: grid.y };
  }
  if (side === "xNeg") {
    return { key: `${side}:${grid.x}`, planeIndex: grid.x, u: grid.y, v: grid.z };
  }
  if (side === "xPos") {
    return { key: `${side}:${grid.x + 1}`, planeIndex: grid.x + 1, u: grid.y, v: grid.z };
  }
  if (side === "yPos") {
    return { key: `${side}:${grid.y + 1}`, planeIndex: grid.y + 1, u: grid.x, v: grid.z };
  }
  return { key: `${side}:${grid.y}`, planeIndex: grid.y, u: grid.x, v: grid.z };
}

function visual3dMergedSliceFaces(groups, view, fill, stroke, mode, order) {
  const faces = [];
  for (const group of groups.values()) {
    const polygons = [...group.cells].map((key) => {
      const [u, v] = key.split(",").map(Number);
      return visual3dMergedSliceFaceCorners(group.side, group.planeIndex, { u0: u, u1: u, v0: v, v1: v });
    });
    const projectedPolygons = polygons.map((polygon) => polygon.map((corner) => visual3dProject(corner, view)));
    const projectedPoints = projectedPolygons.flat();
    faces.push({
      kind: "slice",
      key: `slice:${mode}:${order}:${group.side}:${group.planeIndex}:${[...group.cells].sort().join(";")}`,
      mode,
      order,
      renderPriority: order,
      polygons: projectedPolygons.map((polygon) => polygon.map(({ x, y }) => ({ x, y }))),
      edges: visual3dSliceGroupBoundaryEdges(group).map((edge) => edge.map((point) => visual3dProject(
        visual3dSliceGroupPoint(group.side, group.planeIndex, point.u, point.v),
        view,
      ))),
      depth: projectedPoints.reduce((total, point) => total + point.depth, 0) / Math.max(1, projectedPoints.length),
      gridOrder: visual3dFaceGridOrder(polygons.flat()),
      fill,
      stroke,
    });
  }
  return faces;
}

function visual3dSliceGroupBoundaryEdges(group) {
  const edgeMap = new Map();
  const addEdge = (a, b) => {
    const forward = `${a.u},${a.v}:${b.u},${b.v}`;
    const reverse = `${b.u},${b.v}:${a.u},${a.v}`;
    if (edgeMap.has(reverse)) {
      edgeMap.delete(reverse);
      return;
    }
    edgeMap.set(forward, [a, b]);
  };
  for (const key of group.cells) {
    const [u, v] = key.split(",").map(Number);
    addEdge({ u, v }, { u: u + 1, v });
    addEdge({ u: u + 1, v }, { u: u + 1, v: v + 1 });
    addEdge({ u: u + 1, v: v + 1 }, { u, v: v + 1 });
    addEdge({ u, v: v + 1 }, { u, v });
  }
  return [...edgeMap.values()];
}

function visual3dSliceGroupPoint(side, planeIndex, u, v) {
  const a = u - 0.5;
  const b = v - 0.5;
  const plane = planeIndex - 0.5;
  if (side === "zNeg" || side === "zPos") {
    return { x: a, y: b, z: plane };
  }
  if (side === "xNeg" || side === "xPos") {
    return { x: plane, y: a, z: b };
  }
  return { x: a, y: plane, z: b };
}

function visual3dMergedSliceFaceCorners(side, planeIndex, rect) {
  const a0 = rect.u0 - 0.5;
  const a1 = rect.u1 + 0.5;
  const b0 = rect.v0 - 0.5;
  const b1 = rect.v1 + 0.5;
  const plane = planeIndex - 0.5;
  if (side === "zNeg") {
    return [
      { x: a1, y: b0, z: plane },
      { x: a0, y: b0, z: plane },
      { x: a0, y: b1, z: plane },
      { x: a1, y: b1, z: plane },
    ];
  }
  if (side === "zPos") {
    return [
      { x: a0, y: b0, z: plane },
      { x: a1, y: b0, z: plane },
      { x: a1, y: b1, z: plane },
      { x: a0, y: b1, z: plane },
    ];
  }
  if (side === "xNeg") {
    return [
      { x: plane, y: a0, z: b0 },
      { x: plane, y: a0, z: b1 },
      { x: plane, y: a1, z: b1 },
      { x: plane, y: a1, z: b0 },
    ];
  }
  if (side === "xPos") {
    return [
      { x: plane, y: a0, z: b1 },
      { x: plane, y: a0, z: b0 },
      { x: plane, y: a1, z: b0 },
      { x: plane, y: a1, z: b1 },
    ];
  }
  if (side === "yPos") {
    return [
      { x: a0, y: plane, z: b1 },
      { x: a1, y: plane, z: b1 },
      { x: a1, y: plane, z: b0 },
      { x: a0, y: plane, z: b0 },
    ];
  }
  return [
    { x: a0, y: plane, z: b0 },
    { x: a1, y: plane, z: b0 },
    { x: a1, y: plane, z: b1 },
    { x: a0, y: plane, z: b1 },
  ];
}

function visual3dGridInSlice(grid, slice) {
  if (!Number.isInteger(slice)) {
    return false;
  }
  const worldSlice = visual3dPlaneWorldSlice(visual3d.axis, slice);
  if (visual3d.axis === "x") {
    return grid.x === worldSlice;
  }
  if (visual3d.axis === "y") {
    return grid.y === worldSlice;
  }
  return grid.z === worldSlice;
}

function visual3dGridInSliceVolume(grid, slice) {
  return grid.x >= 0
    && grid.y >= 0
    && grid.z >= 0
    && grid.x < visual3d.width
    && grid.y < visual3d.height
    && grid.z < visual3d.depth
    && visual3dGridInSlice(grid, slice);
}

function visual3dSliceOverlayFill(mode) {
  return mode === "active"
    ? visual3dCssVar("--visual3d-slice-active-fill", "rgba(125, 208, 160, 0.022)")
    : visual3dCssVar("--visual3d-slice-hover-fill", "rgba(137, 148, 158, 0.025)");
}

function visual3dSliceOverlayStroke(mode) {
  return mode === "active"
    ? visual3dCssVar("--visual3d-slice-active-stroke", "rgba(125, 208, 160, 0.12)")
    : visual3dCssVar("--visual3d-slice-hover-stroke", "rgba(137, 148, 158, 0.15)");
}

function visual3dSliceVoxelCoord(slice, col, row) {
  return visual3dCoordsFromPlane(visual3d.axis, slice, col, row);
}

function visual3dProjectedSliceFace(fill, stroke, view, corners) {
  const projected = corners.map((corner) => visual3dProject(corner, view));
  return {
    points: projected.map(({ x, y }) => ({ x, y })),
    depth: projected.reduce((total, point) => total + point.depth, 0) / projected.length,
    fill,
    stroke,
  };
}

function drawVisual3dSliceFace(ctx, face, mode) {
  if (face.polygons) {
    ctx.beginPath();
    for (const polygon of face.polygons) {
      ctx.moveTo(polygon[0].x, polygon[0].y);
      for (const point of polygon.slice(1)) {
        ctx.lineTo(point.x, point.y);
      }
      ctx.closePath();
    }
    ctx.fillStyle = face.fill;
    ctx.fill();
    ctx.strokeStyle = face.stroke;
    ctx.lineWidth = mode === "active" ? 0.72 : 0.58;
    ctx.beginPath();
    for (const edge of face.edges || []) {
      ctx.moveTo(edge[0].x, edge[0].y);
      ctx.lineTo(edge[1].x, edge[1].y);
    }
    ctx.stroke();
    return;
  }
  const expanded = visual3dExpandPolygon(face.points, 0.18);
  ctx.beginPath();
  ctx.moveTo(expanded[0].x, expanded[0].y);
  for (const point of expanded.slice(1)) {
    ctx.lineTo(point.x, point.y);
  }
  ctx.closePath();
  ctx.fillStyle = face.fill;
  ctx.strokeStyle = face.stroke;
  ctx.lineWidth = mode === "active" ? 0.72 : 0.58;
  ctx.fill();
  ctx.stroke();
}

function visual3dProject(position, view) {
  const camera = visual3dCamera();
  return Puzzle3VisualCore.projectOrthographic(position, {
    camera,
    center: {
      x: (visual3d.width - 1) / 2,
      y: (visual3d.height - 1) / 2,
      z: (visual3d.depth - 1) / 2,
    },
    origin: { x: view.originX, y: view.originY },
    scale: view.cellScale,
  });
}

function visual3dMergedVoxelFaces(occupied, view) {
  const voxels = [...occupied.values()].map(({ x, y, z, colorIndex }) => ({
    x,
    y,
    z,
    colorIndex,
  }));
  return Puzzle3VisualCore.mergeVoxelFaces(voxels, {
    faces: visual3dVoxelFaceSpecs,
    isFaceVisible: (voxel, face) => visual3dFaceIsOpen(occupied, face.neighborKey, visual3dColorForColorIndex(voxel.colorIndex)),
    group: (voxel, face) => {
      const fill = visual3dShadeColor(visual3dColorForColorIndex(voxel.colorIndex), face.light);
      const info = visual3dSliceFaceGroupInfo(face.side, voxel);
      const key = `${info.key}:${fill}`;
      return {
        key,
        u: info.u,
        v: info.v,
        group: {
          key,
          side: face.side,
          planeIndex: info.planeIndex,
          fill,
        },
      };
    },
    rectsFromCells: visual3dUnitFaceRects,
    face: (group, rect) => {
      const corners = visual3dMergedSliceFaceCorners(group.side, group.planeIndex, rect);
      const projected = corners.map((corner) => visual3dProject(corner, view));
      const key = `${group.key}:${rect.u0},${rect.u1},${rect.v0},${rect.v1}`;
      return {
        key,
        points: projected.map(({ x, y }) => ({ x, y })),
        depth: projected.reduce((total, point) => total + point.depth, 0) / projected.length,
        gridOrder: visual3dFaceGridOrder(corners),
        renderPriority: 0,
        fill: group.fill,
        overlays: visual3dVoxelFaceOverlays(group.side, group.planeIndex, rect, view),
      };
    },
  });
}

function visual3dUnitFaceRects(cells) {
  return [...cells]
    .map((key) => {
      const [u, v] = key.split(",").map(Number);
      return { u0: u, u1: u, v0: v, v1: v };
    })
    .sort((left, right) => left.v0 - right.v0 || left.u0 - right.u0);
}

function visual3dVoxelFaceOverlays(side, planeIndex, rect, view) {
  const overlaysByMode = new Map();
  for (let v = rect.v0; v <= rect.v1; v += 1) {
    for (let u = rect.u0; u <= rect.u1; u += 1) {
      const grid = visual3dVoxelGridFromFaceCell(side, planeIndex, u, v);
      for (const mode of visual3dVoxelOverlayModesForGrid(grid)) {
        const corners = visual3dMergedSliceFaceCorners(side, planeIndex, { u0: u, u1: u, v0: v, v1: v });
        const polygon = corners.map((corner) => {
          const projected = visual3dProject(corner, view);
          return { x: projected.x, y: projected.y };
        });
        if (!overlaysByMode.has(mode)) {
          overlaysByMode.set(mode, []);
        }
        overlaysByMode.get(mode).push(polygon);
      }
    }
  }
  return [...overlaysByMode.entries()].map(([mode, polygons]) => ({ mode, polygons }));
}

function visual3dVoxelOverlayModesForGrid(grid) {
  const modes = [];
  if (
    Number.isInteger(visual3d.hoverSlice)
    && visual3d.hoverSlice !== visual3d.slice
    && visual3dGridInSlice(grid, visual3d.hoverSlice)
  ) {
    modes.push("hover");
  }
  if (Number.isInteger(visual3d.slice) && visual3dGridInSlice(grid, visual3d.slice)) {
    modes.push("active");
  }
  return modes;
}

function visual3dVoxelGridFromFaceCell(side, planeIndex, u, v) {
  if (side === "zNeg") {
    return { x: u, y: v, z: planeIndex };
  }
  if (side === "zPos") {
    return { x: u, y: v, z: planeIndex - 1 };
  }
  if (side === "xNeg") {
    return { x: planeIndex, y: u, z: v };
  }
  if (side === "xPos") {
    return { x: planeIndex - 1, y: u, z: v };
  }
  if (side === "yPos") {
    return { x: u, y: planeIndex - 1, z: v };
  }
  return { x: u, y: planeIndex, z: v };
}

function visual3dVoxelFaceSpecs(voxel) {
  return [
    { side: "zNeg", neighborKey: visual3dVoxelKey(voxel.x, voxel.y, voxel.z - 1), light: -0.22 },
    { side: "zPos", neighborKey: visual3dVoxelKey(voxel.x, voxel.y, voxel.z + 1), light: 0.10 },
    { side: "xNeg", neighborKey: visual3dVoxelKey(voxel.x - 1, voxel.y, voxel.z), light: -0.08 },
    { side: "xPos", neighborKey: visual3dVoxelKey(voxel.x + 1, voxel.y, voxel.z), light: 0.02 },
    { side: "yPos", neighborKey: visual3dVoxelKey(voxel.x, voxel.y + 1, voxel.z), light: -0.04 },
    { side: "yNeg", neighborKey: visual3dVoxelKey(voxel.x, voxel.y - 1, voxel.z), light: -0.16 },
  ];
}

function visual3dFaceIsOpen(occupied, neighborKey, fill) {
  const neighbor = occupied.get(neighborKey);
  return !(neighbor?.opaque || visual3dColorForColorIndex(neighbor?.colorIndex) === fill);
}

function drawVisual3dFace(ctx, face) {
  const expanded = visual3dExpandPolygon(face.points, 0.35);
  ctx.beginPath();
  ctx.moveTo(expanded[0].x, expanded[0].y);
  for (const point of expanded.slice(1)) {
    ctx.lineTo(point.x, point.y);
  }
  ctx.closePath();
  ctx.fillStyle = face.fill;
  ctx.fill();
  if (visual3dGridVisible) {
    ctx.strokeStyle = visual3dCssVar("--visual3d-voxel-grid-stroke", "rgba(20, 24, 28, 0.38)");
    ctx.lineWidth = 0.72;
    ctx.stroke();
  }
  for (const overlay of face.overlays || []) {
    drawVisual3dVoxelOverlayFace(ctx, overlay);
  }
}

function drawVisual3dVoxelOverlayFace(ctx, face) {
  const style = visual3dSliceVoxelOverlayStyle(face.mode, face.polygons);
  if (style.kind === "tint") {
    visual3dTintVoxelFace(ctx, face.polygons, style);
    return;
  }
  visual3dStripeVoxelFace(ctx, face.polygons, style);
}

function visual3dStripeVoxelFace(ctx, polygons, style) {
  const rotated = polygons.flat().map((point) => visual3dRotatePoint(point, -style.angle));
  const minX = Math.min(...rotated.map((point) => point.x)) - style.gap * 2;
  const maxX = Math.max(...rotated.map((point) => point.x)) + style.gap * 2;
  const minY = Math.min(...rotated.map((point) => point.y)) - style.gap * 2;
  const maxY = Math.max(...rotated.map((point) => point.y)) + style.gap * 2;
  const width = Math.max(1, maxX - minX);
  const band = Math.max(1, style.gap / 2);
  const overlap = 0.25;
  const startY = Math.floor(minY / style.gap) * style.gap;
  ctx.save();
  visual3dClipPolygons(ctx, polygons);
  ctx.rotate(style.angle);
  for (let y = startY; y <= maxY; y += style.gap) {
    ctx.fillStyle = `rgba(255, 255, 255, ${style.lightAlpha})`;
    ctx.fillRect(minX, y, width, band + overlap);
    ctx.fillStyle = `rgba(0, 0, 0, ${style.darkAlpha})`;
    ctx.fillRect(minX, y + band, width, band + overlap);
  }
  ctx.restore();
}

function visual3dTintVoxelFace(ctx, polygons, style) {
  ctx.save();
  visual3dClipPolygons(ctx, polygons);
  ctx.fillStyle = `rgba(255, 255, 255, ${style.lightAlpha})`;
  ctx.fill();
  ctx.fillStyle = `rgba(0, 0, 0, ${style.darkAlpha})`;
  ctx.fill();
  ctx.restore();
}

function visual3dSliceVoxelOverlayStyle(mode, polygons) {
  const edge = visual3dProjectedVoxelEdgeLength(polygons);
  return mode === "active"
    ? {
        kind: "stripe",
        angle: 0.98,
        gap: visual3dClamp(edge * 0.42, 10, 22),
        lightAlpha: 0.105,
        darkAlpha: 0.06,
      }
    : { kind: "tint", lightAlpha: 0.055, darkAlpha: 0.025 };
}

function visual3dProjectedVoxelEdgeLength(polygons) {
  const lengths = [];
  for (const polygon of polygons) {
    for (let index = 0; index < polygon.length; index += 1) {
      const next = polygon[(index + 1) % polygon.length];
      lengths.push(Math.hypot(next.x - polygon[index].x, next.y - polygon[index].y));
    }
  }
  lengths.sort((a, b) => a - b);
  return lengths[Math.floor(lengths.length / 2)] || 16;
}

function visual3dClamp(value, min, max) {
  return Math.min(max, Math.max(min, value));
}

function visual3dClipPolygons(ctx, polygons) {
  ctx.beginPath();
  for (const polygon of polygons) {
    ctx.moveTo(polygon[0].x, polygon[0].y);
    for (const point of polygon.slice(1)) {
      ctx.lineTo(point.x, point.y);
    }
    ctx.closePath();
  }
  ctx.clip();
}

function visual3dRotatePoint(point, angle) {
  const cos = Math.cos(angle);
  const sin = Math.sin(angle);
  return {
    x: point.x * cos - point.y * sin,
    y: point.x * sin + point.y * cos,
  };
}

function visual3dExpandPolygon(points, amount) {
  const center = points.reduce(
    (acc, point) => ({ x: acc.x + point.x / points.length, y: acc.y + point.y / points.length }),
    { x: 0, y: 0 },
  );
  return points.map((point) => {
    const dx = point.x - center.x;
    const dy = point.y - center.y;
    const length = Math.hypot(dx, dy) || 1;
    return {
      x: point.x + (dx / length) * amount,
      y: point.y + (dy / length) * amount,
    };
  });
}

function visual3dShadeColor(color, amount) {
  const rgba = visual3dParseColor(color);
  if (!rgba) {
    return color;
  }
  return visual3dFormatColor({
    r: visual3dLightenChannel(rgba.r, amount),
    g: visual3dLightenChannel(rgba.g, amount),
    b: visual3dLightenChannel(rgba.b, amount),
    a: rgba.a,
  });
}

function visual3dParseColor(color) {
  const normalized = parseVisualHexColor(color);
  if (!normalized) {
    return null;
  }
  return {
    r: Number.parseInt(normalized.slice(1, 3), 16),
    g: Number.parseInt(normalized.slice(3, 5), 16),
    b: Number.parseInt(normalized.slice(5, 7), 16),
    a: normalized.length === 9 ? Number.parseInt(normalized.slice(7, 9), 16) / 255 : 1,
  };
}

function visual3dColorIsOpaque(color) {
  const rgba = visual3dParseColor(color);
  return !rgba || rgba.a >= 0.999;
}

function visual3dFormatColor(color) {
  const r = visual3dClampColorChannel(color.r);
  const g = visual3dClampColorChannel(color.g);
  const b = visual3dClampColorChannel(color.b);
  const a = Math.max(0, Math.min(1, color.a));
  if (a >= 0.999) {
    return `rgb(${r}, ${g}, ${b})`;
  }
  return `rgba(${r}, ${g}, ${b}, ${Number(a.toFixed(3))})`;
}

function visual3dClampColorChannel(value) {
  return Math.max(0, Math.min(255, Math.round(value)));
}

function visual3dLightenChannel(value, light) {
  if (light < 0) {
    return visual3dClampColorChannel(value + value * light);
  }
  return visual3dClampColorChannel(value + (255 - value) * light);
}

function visual3dVoxelKey(x, y, z) {
  return `${x},${y},${z}`;
}

function selectVisual3dColor(index) {
  commitVisualColorEditHistory("visual3d");
  visual3d.selectedColorIndex = validVisual3dColorIndex(index) ? index : null;
  visual3d.addPaletteOpen = false;
  visual3d.editPaletteOpen = false;
  visual3d.customColorOpen = false;
  visual3d.addDraftColorIndex = null;
  renderVisual3dPalette();
}

function addVisual3dColor() {
  commitVisualColorEditHistory("visual3d");
  const before = visualEditSnapshot("visual3d");
  const draftIndex = validVisual3dColorIndex(visual3d.addDraftColorIndex) ? visual3d.addDraftColorIndex : null;
  if (draftIndex === null && visual3dPaletteEntries().length >= VISUAL_COLOR_TOKENS.length) {
    setVisual3dActionStatus(`Palette limit is ${VISUAL_COLOR_TOKENS.length} colors`, "is-error");
    return;
  }
  if (draftIndex === null) {
    visual3dPaletteEntries().push({ color: normalizeVisualColor(nextVisualPresetColor(visual3dPaletteEntries())) });
    visual3d.selectedColorIndex = visual3dPaletteEntries().length - 1;
  } else {
    visual3d.selectedColorIndex = draftIndex;
  }
  visual3d.addPaletteOpen = false;
  visual3d.editPaletteOpen = false;
  visual3d.customColorOpen = false;
  visual3d.addDraftColorIndex = null;
  renderVisual3dBuilder();
  pushVisualEditUndoSnapshot("visual3d", before);
}

function toggleVisual3dAddPalette() {
  commitVisualColorEditHistory("visual3d");
  const before = visualEditSnapshot("visual3d");
  const opening = !visual3d.addPaletteOpen;
  if (opening && visual3dPaletteEntries().length >= VISUAL_COLOR_TOKENS.length) {
    setVisual3dActionStatus(`Palette limit is ${VISUAL_COLOR_TOKENS.length} colors`, "is-error");
    return;
  }
  visual3d.addPaletteOpen = opening;
  visual3d.editPaletteOpen = false;
  visual3d.customColorOpen = opening;
  if (opening) {
    if (!validVisual3dColorIndex(visual3d.addDraftColorIndex)) {
      visual3dPaletteEntries().push({ color: normalizeVisualColor(nextVisualPresetColor(visual3dPaletteEntries())) });
      visual3d.addDraftColorIndex = visual3dPaletteEntries().length - 1;
    }
    visual3d.selectedColorIndex = visual3d.addDraftColorIndex;
    renderVisual3dBuilder();
    pushVisualEditUndoSnapshot("visual3d", before);
    return;
  }
  visual3d.addDraftColorIndex = null;
  renderVisual3dBuilder();
  pushVisualEditUndoSnapshot("visual3d", before);
}

function previewNewVisual3dColor(color, options = {}) {
  const before = options.deferHistory ? null : visualEditSnapshot("visual3d");
  if (options.deferHistory) {
    beginVisualColorEditHistory("visual3d");
  }
  if (!validVisual3dColorIndex(visual3d.addDraftColorIndex) && visual3dPaletteEntries().length >= VISUAL_COLOR_TOKENS.length) {
    return;
  }
  if (!validVisual3dColorIndex(visual3d.addDraftColorIndex)) {
    visual3dPaletteEntries().push({ color: normalizeVisualColor(color) });
    visual3d.addDraftColorIndex = visual3dPaletteEntries().length - 1;
    visual3d.selectedColorIndex = visual3d.addDraftColorIndex;
    renderVisual3dBuilder();
  } else {
    visual3dPaletteEntries()[visual3d.addDraftColorIndex].color = normalizeVisualColor(color);
    visual3d.selectedColorIndex = visual3d.addDraftColorIndex;
    renderVisual3dColorSurfaces();
  }
  if (options.closeMenu) {
    visual3d.addPaletteOpen = false;
    visual3d.editPaletteOpen = false;
    visual3d.customColorOpen = false;
    visual3d.addDraftColorIndex = null;
    renderVisual3dBuilder();
  }
  if (options.deferHistory) {
    return;
  }
  pushVisualEditUndoSnapshot("visual3d", before);
}

function updateSelectedVisual3dColor(value, options = {}) {
  const before = options.deferHistory || options.commitHistory ? null : visualEditSnapshot("visual3d");
  if (options.deferHistory || options.commitHistory) {
    beginVisualColorEditHistory("visual3d");
  }
  if (!validVisual3dColorIndex(visual3d.selectedColorIndex)) {
    visual3d.selectedColorIndex = 0;
  }
  const selected = visual3dPaletteEntries()[visual3d.selectedColorIndex];
  if (!selected) {
    return;
  }
  selected.color = normalizeVisualColor(value);
  if (options.closeMenu) {
    visual3d.editPaletteOpen = false;
    visual3d.customColorOpen = false;
    visual3d.addDraftColorIndex = null;
    renderVisual3dBuilder();
    if (options.deferHistory || options.commitHistory) {
      commitVisualColorEditHistory("visual3d");
    } else {
      pushVisualEditUndoSnapshot("visual3d", before);
    }
    return;
  }
  renderVisual3dColorSurfaces();
  if (options.deferHistory) {
    return;
  }
  if (options.commitHistory) {
    commitVisualColorEditHistory("visual3d");
    return;
  }
  pushVisualEditUndoSnapshot("visual3d", before);
}

function closeVisual3dColorEditor() {
  commitVisualColorEditHistory("visual3d");
  visual3d.addPaletteOpen = false;
  visual3d.editPaletteOpen = false;
  visual3d.customColorOpen = false;
  visual3d.addDraftColorIndex = null;
  renderVisual3dPalette();
}

function cancelVisual3dColorAdd() {
  discardVisualColorEditHistory("visual3d");
  const before = visualEditSnapshot("visual3d");
  if (validVisual3dColorIndex(visual3d.addDraftColorIndex)) {
    removeVisual3dPaletteColor(visual3d.addDraftColorIndex);
  }
  visual3d.addPaletteOpen = false;
  visual3d.editPaletteOpen = false;
  visual3d.customColorOpen = false;
  visual3d.addDraftColorIndex = null;
  renderVisual3dBuilder();
  pushVisualEditUndoSnapshot("visual3d", before);
}

function removeVisual3dColor() {
  commitVisualColorEditHistory("visual3d");
  const before = visualEditSnapshot("visual3d");
  const deletedIndex = visual3d.selectedColorIndex;
  const palette = visual3dPaletteEntries();
  if (!validVisual3dColorIndex(deletedIndex) || palette.length <= 1) {
    return;
  }
  visual3d.addPaletteOpen = false;
  visual3d.editPaletteOpen = false;
  visual3d.customColorOpen = false;
  visual3d.addDraftColorIndex = null;
  removeVisual3dPaletteColor(deletedIndex);
  renderVisual3dBuilder();
  pushVisualEditUndoSnapshot("visual3d", before);
}

function removeVisual3dPaletteColor(deletedIndex) {
  const palette = visual3dPaletteEntries();
  if (!validVisual3dColorIndex(deletedIndex) || palette.length <= 1) {
    return;
  }
  const oldPaletteLength = palette.length;
  palette.splice(deletedIndex, 1);
  visual3d.cells = visual3d.cells.map((colorIndex) => {
    if (!Number.isInteger(colorIndex) || colorIndex < 0 || colorIndex >= oldPaletteLength) {
      return null;
    }
    if (colorIndex === deletedIndex) {
      return null;
    }
    return colorIndex > deletedIndex ? colorIndex - 1 : colorIndex;
  });
  visual3d.selectedColorIndex = Math.min(deletedIndex, palette.length - 1);
}

function renderVisual3dColorSurfaces() {
  syncVisual3dPaletteSwatches();
  syncVisual3dColorAdjusters();
  renderVisual3dSliceBoard();
  renderVisual3dPreview();
}

function syncVisual3dPaletteSwatches() {
  for (const [index, entry] of visual3dPaletteEntries().entries()) {
    const color = normalizeVisualColor(entry.color);
    for (const token of visual3dPalette.querySelectorAll(`[data-color-index="${index}"]`)) {
      token.style.setProperty("--visual-swatch-color", color);
      token.style.setProperty("--visual-token-ink", readableInkForColor(color));
      token.title = `Paint ${color}`;
    }
  }
  const selected = visual3dPaletteEntries()[visual3d.selectedColorIndex];
  const currentButton = visual3dPalette.querySelector(".visual-current-color-button");
  if (currentButton && selected) {
    const normalized = normalizeVisualColor(selected.color);
    currentButton.style.setProperty("--visual-current-color", normalized);
    currentButton.setAttribute("aria-label", `Edit selected color ${normalized}`);
    const currentHexInput = visual3dPalette.querySelector(".visual-current-hex-input");
    if (currentHexInput && document.activeElement !== currentHexInput) {
      currentHexInput.value = normalized;
    }
  }
}

function syncVisual3dColorAdjusters() {
  const selected = validVisual3dColorIndex(visual3d.selectedColorIndex)
    ? visual3dPaletteEntries()[visual3d.selectedColorIndex]
    : null;
  if (!selected) {
    return;
  }
  const normalized = normalizeVisualColor(selected.color);
  for (const adjuster of visual3dPalette.querySelectorAll(".visual-color-adjuster")) {
    if (adjuster.contains(document.activeElement)) {
      continue;
    }
    adjuster.syncColor?.(normalized);
  }
}

function validVisual3dColorIndex(index) {
  return Number.isInteger(index) && index >= 0 && index < visual3dPaletteEntries().length;
}

function normalizedVisual3dCellColorIndex(index) {
  const colorIndex = visual3d.cells[index];
  return validVisual3dColorIndex(colorIndex) ? colorIndex : null;
}

function visual3dColorForColorIndex(index) {
  return validVisual3dColorIndex(index) ? normalizeVisualColor(visual3dPaletteEntries()[index].color) : "#00000000";
}

function visual3dInkForColorIndex(index) {
  return validVisual3dColorIndex(index) ? readableInkForColor(visual3dPaletteEntries()[index].color) : "#8d969f";
}

function visual3dPaletteEntries() {
  ensureVisual3dPalette();
  return visual3d.palette;
}

function ensureVisual3dPalette() {
  if (!Array.isArray(visual3d.palette)) {
    visual3d.palette = [];
  }
}

function visual3dCellIndex(x, y, z) {
  return ((z * visual3d.height + y) * visual3d.width) + x;
}

function visual3dCoordsFromSliceCell(index) {
  const { width } = visual3dPlaneSize();
  const u = index % width;
  const v = Math.floor(index / width);
  return visual3dCoordsFromSliceUv(u, v);
}

function visual3dCoordsFromSliceUv(u, v) {
  return visual3dCoordsFromPlane(visual3d.axis, visual3d.slice, u, v);
}

function visual3dSliceCellIndexFromElement(element) {
  const cell = element?.closest?.(".visual-cell");
  if (!cell || !visual3dSliceBoard.contains(cell)) {
    return -1;
  }
  const index = Number(cell.dataset.index);
  const plane = visual3dPlaneSize();
  return Number.isInteger(index) && index >= 0 && index < plane.width * plane.height ? index : -1;
}

function paintVisual3dCellAtSliceIndex(index, colorIndex) {
  const plane = visual3dPlaneSize();
  if (!Number.isInteger(index) || index < 0 || index >= plane.width * plane.height) {
    return false;
  }
  const coords = visual3dCoordsFromSliceCell(index);
  const voxelIndex = visual3dCellIndex(coords.x, coords.y, coords.z);
  const nextColorIndex = validVisual3dColorIndex(colorIndex) ? colorIndex : null;
  if (visual3d.cells[voxelIndex] === nextColorIndex) {
    return false;
  }
  visual3d.cells[voxelIndex] = nextColorIndex;
  renderVisual3dSliceBoard();
  renderVisual3dPreview();
  return true;
}

function floodFillVisual3dSliceComponentAtIndex(index, colorIndex) {
  const plane = visual3dPlaneSize();
  if (!Number.isInteger(index) || index < 0 || index >= plane.width * plane.height) {
    return 0;
  }
  const startCoords = visual3dCoordsFromSliceCell(index);
  const startVoxelIndex = visual3dCellIndex(startCoords.x, startCoords.y, startCoords.z);
  const nextColorIndex = validVisual3dColorIndex(colorIndex) ? colorIndex : null;
  const targetColorIndex = normalizedVisual3dCellColorIndex(startVoxelIndex);
  if (targetColorIndex === nextColorIndex) {
    return 0;
  }
  const { width, height } = plane;
  const visited = new Uint8Array(width * height);
  const region = visual3dClipActive ? normalizeVisual3dClipBox(visual3dClipSelection) : null;
  const stack = [index];
  let changed = 0;
  while (stack.length) {
    const current = stack.pop();
    if (visited[current]) {
      continue;
    }
    const coords = visual3dCoordsFromSliceCell(current);
    if (region && !visual3dClipBoxContainsCoords(region, coords)) {
      continue;
    }
    const voxelIndex = visual3dCellIndex(coords.x, coords.y, coords.z);
    if (normalizedVisual3dCellColorIndex(voxelIndex) !== targetColorIndex) {
      continue;
    }
    visited[current] = 1;
    visual3d.cells[voxelIndex] = nextColorIndex;
    changed += 1;
    const u = current % width;
    const v = Math.floor(current / width);
    if (u > 0) {
      stack.push(current - 1);
    }
    if (u < width - 1) {
      stack.push(current + 1);
    }
    if (v > 0) {
      stack.push(current - width);
    }
    if (v < height - 1) {
      stack.push(current + width);
    }
  }
  return changed;
}

function floodFillVisual3dComponentAtSliceIndex(index, colorIndex) {
  const plane = visual3dPlaneSize();
  if (!Number.isInteger(index) || index < 0 || index >= plane.width * plane.height) {
    return 0;
  }
  const startCoords = visual3dCoordsFromSliceCell(index);
  const startVoxelIndex = visual3dCellIndex(startCoords.x, startCoords.y, startCoords.z);
  const nextColorIndex = validVisual3dColorIndex(colorIndex) ? colorIndex : null;
  const targetColorIndex = normalizedVisual3dCellColorIndex(startVoxelIndex);
  if (targetColorIndex === nextColorIndex) {
    return 0;
  }
  const visited = new Uint8Array(visual3d.cells.length);
  const region = visual3dClipActive ? normalizeVisual3dClipBox(visual3dClipSelection) : null;
  const stack = [startCoords];
  let changed = 0;
  while (stack.length) {
    const current = stack.pop();
    if (
      current.x < 0 || current.y < 0 || current.z < 0
      || current.x >= visual3d.width || current.y >= visual3d.height || current.z >= visual3d.depth
    ) {
      continue;
    }
    if (region && !visual3dClipBoxContainsCoords(region, current)) {
      continue;
    }
    const voxelIndex = visual3dCellIndex(current.x, current.y, current.z);
    if (visited[voxelIndex] || normalizedVisual3dCellColorIndex(voxelIndex) !== targetColorIndex) {
      continue;
    }
    visited[voxelIndex] = 1;
    visual3d.cells[voxelIndex] = nextColorIndex;
    changed += 1;
    stack.push(
      { x: current.x - 1, y: current.y, z: current.z },
      { x: current.x + 1, y: current.y, z: current.z },
      { x: current.x, y: current.y - 1, z: current.z },
      { x: current.x, y: current.y + 1, z: current.z },
      { x: current.x, y: current.y, z: current.z - 1 },
      { x: current.x, y: current.y, z: current.z + 1 },
    );
  }
  return changed;
}

function bucketFillVisual3dFromSliceIndex(index) {
  const plane = visual3dPlaneSize();
  if (!Number.isInteger(index) || index < 0 || index >= plane.width * plane.height) {
    return false;
  }
  if (visual3dClipActive && !normalizeVisual3dClipBox(visual3dClipSelection)) {
    setVisual3dActionStatus("Select a clip region before bucket fill", "is-error");
    return false;
  }
  const startCoords = visual3dCoordsFromSliceCell(index);
  if (visual3dClipActive && !visual3dClipBoxContainsCoords(visual3dClipSelection, startCoords)) {
    setVisual3dActionStatus("Bucket fill start must be inside the clip region", "is-error");
    return false;
  }
  const colorIndex = visual3d.selectedColorIndex;
  const allScope = visual3dEditScope() === "all";
  const count = allScope
    ? floodFillVisual3dComponentAtSliceIndex(index, colorIndex)
    : floodFillVisual3dSliceComponentAtIndex(index, colorIndex);
  if (!count) {
    setVisual3dActionStatus("Connected component already has that color", "is-ok");
    deactivateVisual3dBucketModeAfterUse();
    return true;
  }
  visual3d.addPaletteOpen = false;
  visual3d.editPaletteOpen = false;
  visual3d.customColorOpen = false;
  visual3d.addDraftColorIndex = null;
  visual3d.hoverSlice = null;
  deactivateVisual3dBucketModeAfterUse();
  renderVisual3dBuilder();
  const nextColorIndex = validVisual3dColorIndex(colorIndex) ? colorIndex : null;
  const message = nextColorIndex === null
    ? allScope ? "Filled 3D component with empty voxels" : "Filled slice component with empty voxels"
    : allScope ? "Filled 3D component" : "Filled slice component";
  setVisual3dActionStatus(message, "is-ok");
  setStatus(message, "is-ok");
  return true;
}

function bucketFillVisual3dFromElement(element) {
  return bucketFillVisual3dFromSliceIndex(visual3dSliceCellIndexFromElement(element));
}

function paintVisual3dCellFromElement(element) {
  return paintVisual3dCellAtSliceIndex(visual3dSliceCellIndexFromElement(element), visual3d.selectedColorIndex);
}

function startVisual3dClip(event) {
  event.preventDefault();
  const geometry = visual3dSliceBoard.getBoundingClientRect();
  const cell = visual3dClipCellFromClient(event.clientX, event.clientY, geometry);
  if (!cell) {
    return;
  }
  const resizeHandle = !visual3dClipFloating && visual3dClipSelection
    ? event.target.closest?.("[data-visual3d-clip-resize]")
    : null;
  if (resizeHandle) {
    visual3dClipDrag = {
      mode: "resize",
      pointerId: event.pointerId,
      geometry,
      startCell: cell,
      originBox: visual3dClipSelection,
      originRect: visual3dClipPlaneRect(),
      edge: resizeHandle.dataset.visual3dClipResize,
    };
  } else if (visual3dClipSelectionContainsSliceCell(cell)) {
    visual3dClipDrag = {
      mode: "move",
      pointerId: event.pointerId,
      geometry,
      startCell: cell,
      originBox: visual3dClipSelection,
    };
  } else {
    const rect = visual3dClipRectFromCells(cell, cell);
    visual3dClipSelection = visual3dClipBoxFromPlaneRect(rect, {
      fullDepth: visual3dEditScope() === "all",
    });
    visual3dClipFloating = null;
    visual3dClipDrag = {
      mode: "select",
      pointerId: event.pointerId,
      geometry,
      startCell: cell,
      originBox: visual3dClipSelection,
    };
  }
  visual3dSliceBoard.setPointerCapture?.(event.pointerId);
  renderVisual3dSliceBoard();
}

function continueVisual3dClip(event) {
  if (!visual3dClipDrag || visual3dClipDrag.pointerId !== event.pointerId) {
    return false;
  }
  event.preventDefault();
  const cell = visual3dClipCellFromClient(event.clientX, event.clientY, visual3dClipDrag.geometry);
  if (!cell) {
    return true;
  }
  if (visual3dClipDrag.mode === "select") {
    const rect = visual3dClipRectFromCells(visual3dClipDrag.startCell, cell);
    visual3dClipSelection = visual3dClipBoxFromPlaneRect(rect, {
      base: visual3dClipDrag.originBox,
      fullDepth: visual3dEditScope() === "all",
    });
  } else if (visual3dClipDrag.mode === "move") {
    const du = cell.x - visual3dClipDrag.startCell.x;
    const dv = cell.y - visual3dClipDrag.startCell.y;
    const next = visual3dClipBoxShiftedInPlane(visual3dClipDrag.originBox, du, dv);
    if (next) {
      visual3dClipSelection = next;
    }
  } else if (visual3dClipDrag.mode === "resize") {
    const rect = visual3dClipResizeRect(visual3dClipDrag.originRect, visual3dClipDrag.edge, cell);
    const next = visual3dClipBoxFromPlaneRect(rect, { base: visual3dClipDrag.originBox });
    if (next) {
      visual3dClipSelection = next;
    }
  }
  renderVisual3dSliceBoard();
  renderVisual3dPreview();
  return true;
}

function stopVisual3dClip(event) {
  if (!visual3dClipDrag || visual3dClipDrag.pointerId !== event.pointerId) {
    return false;
  }
  if (visual3dSliceBoard.hasPointerCapture?.(event.pointerId)) {
    visual3dSliceBoard.releasePointerCapture(event.pointerId);
  }
  event.preventDefault();
  const mode = visual3dClipDrag.mode;
  visual3dClipDrag = null;
  visual3dClipSelection = normalizeVisual3dClipBox(visual3dClipSelection);
  renderVisual3dBuilder();
  const dimensions = visual3dClipBoxDimensions();
  if (dimensions) {
    const verb = mode === "move" ? "Clip range moved" : mode === "resize" ? "Clip range resized" : "Clip range selected";
    setVisual3dActionStatus(`${verb} ${dimensions.width}x${dimensions.height}x${dimensions.depth}`, "is-ok");
  }
  return true;
}

function handleVisual3dClipKeyboard(event) {
  if (currentPreviewMode !== "visual3d" || visual3dBuilder.hidden || !visual3dClipActive
    || visualClipShortcutTargetIsText(event.target)) {
    return false;
  }
  const key = event.key.length === 1 ? event.key.toLowerCase() : event.key;
  const modifier = (event.metaKey && !event.ctrlKey) || (event.ctrlKey && !event.metaKey);
  let handled = false;
  if (!modifier && !event.altKey && ["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(key)) {
    const du = key === "ArrowLeft" ? -1 : key === "ArrowRight" ? 1 : 0;
    const dv = key === "ArrowUp" ? -1 : key === "ArrowDown" ? 1 : 0;
    const next = visual3dClipBoxShiftedInPlane(visual3dClipSelection, du, dv);
    if (!next) {
      setVisual3dActionStatus("Clip must stay inside 3D visual", "is-error");
      handled = true;
    } else {
      visual3dClipSelection = next;
      renderVisual3dBuilder();
      setVisual3dActionStatus("Clip range moved", "is-ok");
      handled = true;
    }
  }
  if (!handled) {
    return false;
  }
  event.preventDefault();
  event.stopPropagation();
  return true;
}

function startVisual3dPaint(event) {
  if (event.button !== 0) {
    return;
  }
  if (visual3dClipActive) {
    startVisual3dClip(event);
    return;
  }
  if (visual3dTranslateActive) {
    startVisual3dTranslate(event);
    return;
  }
  const index = visual3dSliceCellIndexFromElement(document.elementFromPoint(event.clientX, event.clientY));
  if (!Number.isInteger(index) || index < 0) {
    return;
  }
  event.preventDefault();
  if (visual3dBucketActive) {
    const before = visualEditSnapshot("visual3d");
    if (bucketFillVisual3dFromSliceIndex(index)) {
      pushVisualEditUndoSnapshot("visual3d", before);
    }
    return;
  }
  visual3dPaintDrag = {
    pointerId: event.pointerId,
    colorIndex: visual3d.selectedColorIndex,
    lastIndex: -1,
    beforeSnapshot: visualEditSnapshot("visual3d"),
    changed: false,
  };
  visual3dSliceBoard.setPointerCapture?.(event.pointerId);
  paintVisual3dDragIndex(index);
}

function continueVisual3dPaint(event) {
  if (continueVisual3dClip(event)) {
    return;
  }
  if (continueVisual3dTranslate(event)) {
    return;
  }
  if (!visual3dPaintDrag || visual3dPaintDrag.pointerId !== event.pointerId) {
    return;
  }
  event.preventDefault();
  paintVisual3dDragIndex(visual3dSliceCellIndexFromElement(document.elementFromPoint(event.clientX, event.clientY)));
}

function stopVisual3dPaint(event) {
  if (stopVisual3dClip(event)) {
    return;
  }
  if (stopVisual3dTranslate(event)) {
    return;
  }
  if (!visual3dPaintDrag || visual3dPaintDrag.pointerId !== event.pointerId) {
    return;
  }
  if (visual3dSliceBoard.hasPointerCapture?.(event.pointerId)) {
    visual3dSliceBoard.releasePointerCapture(event.pointerId);
  }
  if (visual3dPaintDrag.changed) {
    pushVisualEditUndoSnapshot("visual3d", visual3dPaintDrag.beforeSnapshot);
  }
  visual3dPaintDrag = null;
}

function paintVisual3dDragIndex(index) {
  if (!visual3dPaintDrag || !Number.isInteger(index) || index < 0 || index === visual3dPaintDrag.lastIndex) {
    return;
  }
  const plane = visual3dPlaneSize();
  const centerU = (index % plane.width) + 0.5;
  const centerV = Math.floor(index / plane.width) + 0.5;
  const diameter = visualBrushDiameterForSize(Math.min(plane.width, plane.height));
  const radius = diameter / 2;
  const minU = visualBrushSizePx === 1 ? Math.floor(centerU) : Math.max(0, Math.floor(centerU - radius - 0.5));
  const maxU = visualBrushSizePx === 1 ? minU : Math.min(plane.width - 1, Math.ceil(centerU + radius - 0.5));
  const minV = visualBrushSizePx === 1 ? Math.floor(centerV) : Math.max(0, Math.floor(centerV - radius - 0.5));
  const maxV = visualBrushSizePx === 1 ? minV : Math.min(plane.height - 1, Math.ceil(centerV + radius - 0.5));
  visual3dPaintDrag.lastIndex = index;
  for (let v = minV; v <= maxV; v += 1) {
    for (let u = minU; u <= maxU; u += 1) {
      const dx = u + 0.5 - centerU;
      const dy = v + 0.5 - centerV;
      if (visualBrushSizePx !== 1 && (dx * dx) + (dy * dy) > radius * radius) {
        continue;
      }
      if (paintVisual3dCellAtSliceIndex((v * plane.width) + u, visual3dPaintDrag.colorIndex)) {
        visual3dPaintDrag.changed = true;
      }
    }
  }
}

function updateVisual3dDimension(axis, value) {
  const before = visualEditSnapshot("visual3d");
  const nextValue = clampVisual3dSize(value);
  const next = visual3d.sizeBound
    ? { width: nextValue, height: nextValue, depth: nextValue }
    : {
        width: axis === "width" ? nextValue : visual3d.width,
        height: axis === "height" ? nextValue : visual3d.height,
        depth: axis === "depth" ? nextValue : visual3d.depth,
      };
  if (next.width === visual3d.width
    && next.height === visual3d.height
    && next.depth === visual3d.depth) {
    renderVisual3dControls();
    return;
  }
  remapVisual3dFrames(next, (x, y, z) => ({ x, y, z }));
  resetVisual3dClipState();
  visual3d.slice = Math.min(visual3d.slice, visual3dAxisSize() - 1);
  renderVisual3dBuilder();
  pushVisualEditUndoSnapshot("visual3d", before);
}

function remapVisual3dFrames(nextExtent, sourceCoordinates) {
  commitVisual3dActiveFrame();
  const previous = {
    width: visual3d.width,
    height: visual3d.height,
    depth: visual3d.depth,
  };
  const remap = (frame) => {
    const next = Array.from({ length: nextExtent.width * nextExtent.height * nextExtent.depth }, () => null);
    for (let z = 0; z < nextExtent.depth; z += 1) {
      for (let y = 0; y < nextExtent.height; y += 1) {
        for (let x = 0; x < nextExtent.width; x += 1) {
          const source = sourceCoordinates(x, y, z);
          if (!source || source.x < 0 || source.x >= previous.width
            || source.y < 0 || source.y >= previous.height
            || source.z < 0 || source.z >= previous.depth) {
            continue;
          }
          const sourceIndex = ((source.z * previous.height + source.y) * previous.width) + source.x;
          const colorIndex = frame[sourceIndex];
          next[((z * nextExtent.height + y) * nextExtent.width) + x] = validVisual3dColorIndex(colorIndex)
            ? colorIndex
            : null;
        }
      }
    }
    return next;
  };
  const frames = visual3d.animationMode && visual3d.frames.length
    ? visual3d.frames
    : [visual3d.cells];
  visual3d.frames = frames.map(remap);
  visual3d.width = nextExtent.width;
  visual3d.height = nextExtent.height;
  visual3d.depth = nextExtent.depth;
  visual3d.animationFrameCount = visual3d.frames.length;
  visual3d.animationFrameIndex = Math.min(visual3d.animationFrameIndex, visual3d.frames.length - 1);
  visual3d.animationPlaybackIndex = Math.min(visual3d.animationPlaybackIndex, visual3d.frames.length - 1);
  visual3d.cells = visual3d.frames[visual3d.animationFrameIndex];
}

function visual3dScaleFactor() {
  return visualEditorScaleFactor(visual3dScaleInput, VISUAL3D_EDITOR_MAX_SIZE);
}

function canScaleDownVisual3d(factor = visual3dScaleFactor()) {
  return factor > 1
    && visual3d.width >= factor
    && visual3d.height >= factor
    && visual3d.depth >= factor
    && visual3d.width % factor === 0
    && visual3d.height % factor === 0
    && visual3d.depth % factor === 0;
}

function scaleUpVisual3d() {
  const before = visualEditSnapshot("visual3d");
  const factor = visual3dScaleFactor();
  const next = {
    width: visual3d.width * factor,
    height: visual3d.height * factor,
    depth: visual3d.depth * factor,
  };
  if (Math.max(next.width, next.height, next.depth) > VISUAL3D_EDITOR_MAX_SIZE) {
    setVisual3dActionStatus(`3D visual size limit is ${VISUAL3D_EDITOR_MAX_SIZE}`, "is-error");
    renderVisual3dControls();
    return;
  }

  remapVisual3dFrames(next, (x, y, z) => ({
    x: Math.floor(x / factor),
    y: Math.floor(y / factor),
    z: Math.floor(z / factor),
  }));
  resetVisual3dClipState();
  visual3d.slice = Math.min(visual3d.slice * factor, visual3dAxisSize() - 1);
  visual3d.hoverSlice = null;
  renderVisual3dBuilder();
  const message = `Scaled ${factor}x to ${next.width}x${next.height}x${next.depth}`;
  setVisual3dActionStatus(message, "is-ok");
  setStatus(`Scaled 3D visual ${factor}x to ${next.width}x${next.height}x${next.depth}`, "is-ok");
  pushVisualEditUndoSnapshot("visual3d", before);
}

function scaleDownVisual3d() {
  const before = visualEditSnapshot("visual3d");
  const factor = visual3dScaleFactor();
  if (!canScaleDownVisual3d(factor)) {
    setVisual3dActionStatus(`Dimensions ${visual3d.width}x${visual3d.height}x${visual3d.depth} are not divisible by ${factor}`, "is-error");
    renderVisual3dControls();
    return;
  }

  const next = {
    width: visual3d.width / factor,
    height: visual3d.height / factor,
    depth: visual3d.depth / factor,
  };
  remapVisual3dFrames(next, (x, y, z) => ({
    x: x * factor,
    y: y * factor,
    z: z * factor,
  }));
  resetVisual3dClipState();
  visual3d.slice = Math.min(Math.floor(visual3d.slice / factor), visual3dAxisSize() - 1);
  visual3d.hoverSlice = null;
  renderVisual3dBuilder();
  const message = `Scaled down ${factor}x to ${next.width}x${next.height}x${next.depth}`;
  setVisual3dActionStatus(message, "is-ok");
  setStatus(`Scaled 3D visual down ${factor}x to ${next.width}x${next.height}x${next.depth}`, "is-ok");
  pushVisualEditUndoSnapshot("visual3d", before);
}

function setVisual3dAxis(axis) {
  const nextAxis = ["x", "y", "z"].includes(axis) ? axis : "z";
  if (visual3dClipSelection && visual3dEditScope() === "slice" && nextAxis !== visual3d.axis) {
    visual3dClipSelection = null;
    visual3dClipFloating = null;
    visual3dClipDrag = null;
  }
  visual3d.axis = nextAxis;
  visual3d.slice = Math.min(visual3d.slice, visual3dAxisSize(nextAxis) - 1);
  visual3d.hoverSlice = null;
  renderVisual3dBuilder();
}

function setVisual3dSlice(value) {
  const nextSlice = Math.max(0, Math.min(visual3dAxisSize() - 1, Math.trunc(Number(value) || 0)));
  if (visual3dClipSelection && visual3dEditScope() === "slice" && nextSlice !== visual3d.slice) {
    visual3dClipSelection = null;
    visual3dClipFloating = null;
    visual3dClipDrag = null;
  }
  visual3d.slice = nextSlice;
  renderVisual3dControls();
  renderVisual3dSliceBoard();
  renderVisual3dPreview();
}

function moveVisual3dSlice(delta) {
  setVisual3dSlice(visual3d.slice + delta);
}

function applyVisual3dSliceInput() {
  if (!(visual3dSliceValue instanceof HTMLInputElement)) {
    return;
  }
  setVisual3dSlice(Math.trunc(Number(visual3dSliceValue.value) || 1) - 1);
}

function visual3dSliceScrubTarget(event) {
  return event.target?.closest?.("[data-visual3d-slice-scrub]") || null;
}

function startVisual3dSliceScrub(event) {
  const target = visual3dSliceScrubTarget(event);
  if (!target || event.button !== 0) {
    return;
  }
  event.preventDefault();
  event.stopPropagation();
  visual3dSliceScrubDrag = {
    pointerId: event.pointerId,
    target,
    inputTarget: event.target === visual3dSliceValue,
    startX: event.clientX,
    moved: false,
    slice: visual3d.slice,
  };
  target.setPointerCapture?.(event.pointerId);
  target.classList.add("is-dragging");
  document.documentElement.classList.add("is-visual3d-slice-scrubbing");
}

function continueVisual3dSliceScrub(event) {
  if (!visual3dSliceScrubDrag || visual3dSliceScrubDrag.pointerId !== event.pointerId) {
    return;
  }
  event.preventDefault();
  event.stopPropagation();
  const deltaX = event.clientX - visual3dSliceScrubDrag.startX;
  if (Math.abs(deltaX) > 2) {
    visual3dSliceScrubDrag.moved = true;
  }
  setVisual3dSlice(visual3dSliceScrubDrag.slice + Math.round(deltaX / VISUAL3D_SLICE_SCRUB_STEP_PX));
}

function stopVisual3dSliceScrub(event) {
  if (!visual3dSliceScrubDrag || visual3dSliceScrubDrag.pointerId !== event.pointerId) {
    return;
  }
  event.preventDefault();
  event.stopPropagation();
  finishVisual3dSliceScrub(event.pointerId);
}

function finishVisual3dSliceScrub(pointerId = null) {
  if (!visual3dSliceScrubDrag) {
    return;
  }
  const { target, inputTarget, moved } = visual3dSliceScrubDrag;
  if (pointerId !== null && target.hasPointerCapture?.(pointerId)) {
    target.releasePointerCapture(pointerId);
  }
  target.classList.remove("is-dragging");
  document.documentElement.classList.remove("is-visual3d-slice-scrubbing");
  visual3dSliceScrubDrag = null;
  if (!moved && inputTarget && visual3dSliceValue instanceof HTMLInputElement) {
    visual3dSliceValue.focus();
    visual3dSliceValue.select();
  }
}

function deleteVisual3dSlice() {
  const before = visualEditSnapshot("visual3d");
  const plane = visual3dPlaneSize();
  for (let index = 0; index < plane.width * plane.height; index += 1) {
    const coords = visual3dCoordsFromSliceCell(index);
    visual3d.cells[visual3dCellIndex(coords.x, coords.y, coords.z)] = null;
  }
  renderVisual3dBuilder();
  setVisual3dActionStatus("Deleted current slice contents", "is-ok");
  pushVisualEditUndoSnapshot("visual3d", before);
}

function deleteVisual3dBuilder() {
  const before = visualEditSnapshot("visual3d");
  resetVisual3dBuilder();
  setVisual3dActionStatus("Deleted whole 3D visual contents", "is-ok");
  pushVisualEditUndoSnapshot("visual3d", before);
}

function deleteVisual3dScoped() {
  if (visual3dEditScope() === "all") {
    deleteVisual3dBuilder();
  } else {
    deleteVisual3dSlice();
  }
}

function transformVisual3dCells(mapper, message) {
  const before = visualEditSnapshot("visual3d");
  const previousCells = visual3d.cells;
  const nextCells = Array.from({ length: visual3dFrameCellCount() }, () => null);
  for (let z = 0; z < visual3d.depth; z += 1) {
    for (let y = 0; y < visual3d.height; y += 1) {
      for (let x = 0; x < visual3d.width; x += 1) {
        const sourceIndex = visual3dCellIndex(x, y, z);
        const colorIndex = previousCells[sourceIndex];
        if (!validVisual3dColorIndex(colorIndex)) {
          continue;
        }
        const target = mapper(x, y, z);
        nextCells[visual3dCellIndex(target.x, target.y, target.z)] = colorIndex;
      }
    }
  }
  visual3d.cells = nextCells;
  visual3d.hoverSlice = null;
  renderVisual3dSliceBoard();
  renderVisual3dPreview();
  setVisual3dActionStatus(message, "is-ok");
  pushVisualEditUndoSnapshot("visual3d", before);
}

function visual3dPlaneCoordinates(axis, x, y, z) {
  const maxY = visual3d.height - 1;
  const maxZ = visual3d.depth - 1;
  if (axis === "x") {
    return { stack: x, u: maxY - y, v: maxZ - z };
  }
  if (axis === "y") {
    return { stack: maxY - y, u: x, v: maxZ - z };
  }
  return { stack: maxZ - z, u: x, v: maxY - y };
}

function visual3dCoordsFromPlane(axis, stack, u, v) {
  const maxY = visual3d.height - 1;
  const maxZ = visual3d.depth - 1;
  const fixed = visual3dPlaneWorldSlice(axis, stack);
  if (axis === "x") {
    return { x: fixed, y: maxY - u, z: maxZ - v };
  }
  if (axis === "y") {
    return { x: u, y: fixed, z: maxZ - v };
  }
  return { x: u, y: maxY - v, z: fixed };
}

function visual3dPlaneWorldSlice(axis, stack) {
  const axisSize = visual3dAxisSize(axis);
  const normalized = Math.max(0, Math.min(axisSize - 1, Math.trunc(Number(stack) || 0)));
  return axis === "x" ? normalized : axisSize - 1 - normalized;
}

function visual3dCurrentSliceDescriptor() {
  return {
    axis: ["x", "y", "z"].includes(visual3d.axis) ? visual3d.axis : "z",
    slice: Math.max(0, Math.min(visual3dAxisSize() - 1, Math.trunc(Number(visual3d.slice) || 0))),
  };
}

function readVisual3dSliceCells(axis, slice) {
  const cells = [];
  const plane = visual3dPlaneSize(axis);
  for (let v = 0; v < plane.height; v += 1) {
    for (let u = 0; u < plane.width; u += 1) {
      const source = visual3dCoordsFromPlane(axis, slice, u, v);
      cells.push(visual3d.cells[visual3dCellIndex(source.x, source.y, source.z)] ?? null);
    }
  }
  return cells;
}

function visual3dPaletteColors() {
  return visual3dPaletteEntries().map((entry) => normalizeVisualColor(entry.color));
}

function visual3dSliceCellColors(cells) {
  const paletteColors = visual3dPaletteColors();
  return cells.map((colorIndex) => (
    Number.isInteger(colorIndex) && colorIndex >= 0 && colorIndex < paletteColors.length
      ? paletteColors[colorIndex]
      : null
  ));
}

function visual3dClipboardPaletteColor(entry) {
  return parseVisualHexColor(typeof entry === "string" ? entry : entry?.color);
}

function visual3dClipboardCellColors(copied) {
  if (Array.isArray(copied.colors)) {
    return copied.colors.map((color) => parseVisualHexColor(color) || null);
  }
  if (!Array.isArray(copied.palette) || !Array.isArray(copied.cells)) {
    return null;
  }
  const paletteColors = copied.palette.map(visual3dClipboardPaletteColor);
  return copied.cells.map((colorIndex) => (
    Number.isInteger(colorIndex) && colorIndex >= 0 && colorIndex < paletteColors.length
      ? paletteColors[colorIndex]
      : null
  ));
}

function visual3dPastedSliceCells(copied, targetSize) {
  const cellCount = targetSize * targetSize;
  const colors = visual3dClipboardCellColors(copied);
  if (!colors) {
    return {
      cells: copied.cells.map((colorIndex) => (validVisual3dColorIndex(colorIndex) ? colorIndex : null)),
      addedColors: 0,
    };
  }
  if (colors.length !== cellCount) {
    return { error: "Copied slice color data is incomplete" };
  }
  const palette = visual3dPaletteEntries();
  const colorToIndex = new Map(palette.map((entry, index) => [normalizeVisualColor(entry.color), index]));
  const missingColors = [];
  for (const color of colors) {
    if (!color || color === "#00000000" || colorToIndex.has(color) || missingColors.includes(color)) {
      continue;
    }
    missingColors.push(color);
  }
  if (palette.length + missingColors.length > VISUAL_COLOR_TOKENS.length) {
    return {
      error: `Paste needs ${missingColors.length} more colors, but the 3D visual palette has ${VISUAL_COLOR_TOKENS.length - palette.length} slots`,
    };
  }
  for (const color of missingColors) {
    colorToIndex.set(color, palette.length);
    palette.push({ color });
  }
  return {
    cells: colors.map((color) => (color && color !== "#00000000" ? colorToIndex.get(color) : null)),
    addedColors: missingColors.length,
  };
}

function writeVisual3dSliceCells(axis, slice, cells) {
  const plane = visual3dPlaneSize(axis);
  for (let v = 0; v < plane.height; v += 1) {
    for (let u = 0; u < plane.width; u += 1) {
      const colorIndex = cells[(v * plane.width) + u];
      const target = visual3dCoordsFromPlane(axis, slice, u, v);
      visual3d.cells[visual3dCellIndex(target.x, target.y, target.z)] = validVisual3dColorIndex(colorIndex)
        ? colorIndex
        : null;
    }
  }
}

function transformVisual3dCurrentPlane(mapper, message) {
  const axis = ["x", "y", "z"].includes(visual3d.axis) ? visual3d.axis : "z";
  const plane = visual3dPlaneSize(axis);
  transformVisual3dCells((x, y, z) => {
    const plane = visual3dPlaneCoordinates(axis, x, y, z);
    const next = mapper(plane.u, plane.v, visual3dPlaneSize(axis).width, visual3dPlaneSize(axis).height);
    return visual3dCoordsFromPlane(axis, plane.stack, next.u, next.v);
  }, `${message} all ${axis.toUpperCase()} slices`);
}

function transformVisual3dCurrentSlice(mapper, message) {
  const before = visualEditSnapshot("visual3d");
  const source = visual3dCurrentSliceDescriptor();
  const previousCells = readVisual3dSliceCells(source.axis, source.slice);
  const plane = visual3dPlaneSize(source.axis);
  const nextCells = Array.from({ length: plane.width * plane.height }, () => null);
  for (let v = 0; v < plane.height; v += 1) {
    for (let u = 0; u < plane.width; u += 1) {
      const colorIndex = previousCells[(v * plane.width) + u];
      if (!validVisual3dColorIndex(colorIndex)) {
        continue;
      }
      const next = mapper(u, v, plane.width, plane.height);
      nextCells[(next.v * plane.width) + next.u] = colorIndex;
    }
  }
  writeVisual3dSliceCells(source.axis, source.slice, nextCells);
  visual3d.hoverSlice = null;
  renderVisual3dSliceBoard();
  renderVisual3dPreview();
  setVisual3dActionStatus(`${message} ${source.axis.toUpperCase()} slice ${source.slice + 1}`, "is-ok");
  pushVisualEditUndoSnapshot("visual3d", before);
}

function transformVisual3dScoped(mapper, message) {
  if (visual3dEditScope() === "all") {
    transformVisual3dCurrentPlane(mapper, message);
  } else {
    transformVisual3dCurrentSlice(mapper, message);
  }
}

function rotateVisual3dPlaneLeft() {
  const plane = visual3dPlaneSize();
  if (plane.width !== plane.height) {
    setVisual3dActionStatus("Rotate requires a square edit plane", "is-error");
    return;
  }
  transformVisual3dScoped((u, v, width) => ({ u: v, v: width - 1 - u }), "Rotated left");
}

function rotateVisual3dPlaneRight() {
  const plane = visual3dPlaneSize();
  if (plane.width !== plane.height) {
    setVisual3dActionStatus("Rotate requires a square edit plane", "is-error");
    return;
  }
  transformVisual3dScoped((u, v, width) => ({ u: width - 1 - v, v: u }), "Rotated right");
}

function flipVisual3dPlaneHorizontal() {
  transformVisual3dScoped((u, v, width) => ({ u: width - 1 - u, v }), "Flipped horizontal");
}

function flipVisual3dPlaneVertical() {
  transformVisual3dScoped((u, v, width, height) => ({ u, v: height - 1 - v }), "Flipped vertical");
}

function visual3dObjectName() {
  const raw = String(visual3dNameInput?.value || "").trim();
  const explicitAnimation = raw.startsWith("!");
  const cleaned = raw
    .replace(/^!+/, "")
    .replace(/[^\w:@]+/g, "_")
    .replace(/(?!^)@/g, "_")
    .replace(/^_+|_+$/g, "");
  const name = cleaned || "VoxelVisual";
  return explicitAnimation ? `!${name}` : name;
}

function visual3dClipboardText() {
  return visual3dObjectDefinitionText("");
}

function visual3dObjectDefinitionText(indent, name = visual3dObjectName()) {
  const normalizedIndent = visual3dSourceIndent(indent);
  const lines = [
    `${normalizedIndent}${name} {`,
    `${normalizedIndent}colors = ${visual3dPaletteSourceTokens().join(" ")}`,
    `${normalizedIndent}shape = {`,
    ...visual3dVoxelRows().map((row) => `${normalizedIndent}${row}`),
    `${normalizedIndent}}`,
    `${normalizedIndent}}`,
  ];
  return lines.join("\n");
}

function visual3dPaletteSourceTokens() {
  return visual3dPaletteEntries().map((entry) => visual3dPaletteSourceToken(entry));
}

function visual3dPaletteSourceToken(entry) {
  const bind = visualPaletteEntryBindInfo(entry);
  if (bind.linked && bind.name) {
    return bind.name;
  }
  const color = normalizeVisualColor(entry?.color || "#00000000");
  return color === "#00000000" ? "transparent" : color;
}

function visual3dVoxelRows() {
  const rows = [];
  for (let z = 0; z < visual3d.depth; z += 1) {
    if (z > 0) {
      rows.push("-");
    }
    for (let y = 0; y < visual3d.height; y += 1) {
      const row = [];
      for (let x = 0; x < visual3d.width; x += 1) {
        const coords = visual3dCoordsFromPlane("z", z, x, y);
        const colorIndex = visual3d.cells[visual3dCellIndex(coords.x, coords.y, coords.z)];
        row.push(validVisual3dColorIndex(colorIndex) ? VISUAL_COLOR_TOKENS[colorIndex] : ".");
      }
      rows.push(row.join(""));
    }
  }
  return rows;
}

function visual3dEditFrames() {
  commitVisual3dActiveFrame();
  const frames = Array.isArray(visual3d.frames) && visual3d.frames.length
    ? visual3d.frames.map((frame) => Array.isArray(frame) ? frame.slice() : [])
    : [[]];
  frames[visual3d.animationMode ? visual3d.animationFrameIndex : 0] = visual3d.cells.slice();
  return frames.map((frame) => Array.from({ length: visual3d.depth }, (_, sourceZ) =>
    Array.from({ length: visual3d.height }, (_, y) =>
      Array.from({ length: visual3d.width }, (_, x) => {
        const worldZ = visual3d.depth - 1 - sourceZ;
        const cell = frame[visual3dCellIndex(x, y, worldZ)];
        return Number.isInteger(cell) ? cell : null;
      }))));
}

function visual3dEditMutationRequest(operation, options = {}) {
  const shape = visualAssetBindInfo(visual3d.shapeBind, "shape");
  const colorBindings = visual3d.palette
    .map((entry) => ({ entry, bind: visualPaletteEntryBindInfo(entry) }))
    .filter(({ bind }) => bind.linked && bind.name)
    .map(({ entry, bind }) => ({ name: bind.name, color: normalizeVisualColor(entry.color) }));
  return {
    operation,
    dimension: "3d",
    name: options.name ?? visual3dObjectName(),
    originalName: options.originalName ?? visual3d.editSourceName ?? visual3dObjectName(),
    cursor: options.cursor,
    palette: visual3dPaletteSourceTokens(),
    frames: visual3dEditFrames(),
    durationMs: visual3d.animationMode ? normalizedVisual3dAnimationDuration() : null,
    frameDurationMs: visual3d.animationMode ? visual3d.frameDurationMs : null,
    shapeRef: shape.linked ? shape.name : null,
    preludeRows: visual3d.sourcePreludeRows || [],
    spatialOps: visual3d.sourceSpatialOps || [],
    colorBindings,
  };
}

async function updateVisual3dInSource() {
  try {
    await commitVisualEditorMutation({
      state: visual3d,
      request: () => visual3dEditMutationRequest("update"),
    });
  } catch (error) {
    setVisual3dActionStatus("No selected 3D visual source range", "is-error");
    setStatus("No selected 3D visual source range", "is-error");
    setVisual3dActionStatus(userFacingRuntimeError(error), "is-error");
    return;
  }
  setVisual3dActionStatus("Updated 3D visual", "is-ok");
  setStatus("Updated 3D visual", "is-ok");
  syncVisual3dSourceActionButtons();
}

async function addVisual3dToSource() {
  let result;
  try {
    ({ result } = await commitVisualEditorMutation({
      state: visual3d,
      allowActiveDocument: true,
      request: (source, document) => visual3dEditMutationRequest(
        canReplaceCurrentVisual3dDefinition(source) ? "duplicate" : "insert",
        { cursor: visualSourceCursorPosition(source, document) },
      ),
    }));
  } catch (error) {
    setVisual3dActionStatus(userFacingRuntimeError(error), "is-error");
    return;
  }
  visual3dNameInput.value = result.name;
  setVisual3dActionStatus("Added 3D visual", "is-ok");
  setStatus("Added 3D visual", "is-ok");
  syncVisual3dSourceActionButtons();
}

function newVisual3dDraft() {
  const before = visualEditSnapshot("visual3d");
  clearVisual3dEditSource();
  visual3dNameInput.value = "VoxelVisual";
  visual3d.palette = [{ color: "#ff004d" }];
  visual3d.selectedColorIndex = 0;
  visual3d.animationMode = false;
  resetVisual3dBuilder(5, 5, 5);
  setVisual3dActionStatus("Started new 3D visual", "is-ok");
  pushVisualEditUndoSnapshot("visual3d", before);
}

function activeVisual3dEditDocument() {
  return visualEditorOwnedDocument(visual3d);
}

function activeVisual3dEditSource() {
  return visualEditorSourceSnapshot(visual3d).source;
}

const VISUAL3D_SOURCE_INDENT = "";

function visual3dSourceIndent(indent = "") {
  return String(indent || "").replace(/\t/g, VISUAL3D_SOURCE_INDENT);
}

async function loadVisual3dFromSourcePosition(position, options = {}) {
  if (!isPuzzleDocument(activeDocument()) || !isTextDocument(activeDocument())) {
    return null;
  }
  const source = sourceEditor.value || "";
  if (typeof resolveSourceTargetFromWasm !== "function") {
    return null;
  }
  const target = await resolveSourceTargetFromWasm(source, position);
  if (!sourceTargetMatches(target, "visual", "3d")) {
    return null;
  }
  return loadVisual3dSourceTarget(target, options);
}

function loadVisual3dSourceTarget(target, options = {}) {
  if (!isPuzzleDocument(activeDocument()) || !isTextDocument(activeDocument())) {
    return null;
  }
  if (!Number.isInteger(target?.bodyStart) || !Number.isInteger(target?.bodyEnd)) {
    return null;
  }
  if (options.recordHistory && typeof pushSourceNavigationHistory === "function") {
    pushSourceNavigationHistory();
  }
  if (options.switchMode && currentPreviewMode !== "visual3d") {
    setPreviewMode("visual3d");
  }
  const loaded = visual3dTargetPayload(target);
  if (!loaded) {
    if (target?.sourceVisual?.dimension === "3d") {
      applyIncompleteVisual3dSourceTarget(target.name || "", target);
      if (!options.silent) {
        const message = target.sourceVisual.status === "invalid"
          ? `Cannot edit invalid 3D visual ${visual3dNameInput.value || ""}`.trim()
          : `Loaded unfinished 3D visual ${visual3dNameInput.value || ""}`.trim();
        const status = target.sourceVisual.status === "invalid" ? "is-error" : "is-ok";
        setVisual3dActionStatus(message, status);
        setStatus(message, status);
      }
      return `visual3d:${target.name}:${target.start ?? target.bodyStart}`;
    } else if (!options.silent) {
      setVisual3dActionStatus("No editable 3D visual here", "is-error");
    }
    return null;
  }
  setVisual3dEditSource(target, activeDocument());
  applyLoadedVisual3d(target.name || "VoxelVisual", loaded);
  if (!options.silent) {
    setVisual3dActionStatus(`Loaded ${visual3dNameInput.value}`, "is-ok");
    setStatus(`Loaded 3D visual ${visual3dNameInput.value}`, "is-ok");
  }
  return `visual3d:${target.name}:${target.start ?? target.bodyStart}`;
}

function visual3dTargetPayload(target) {
  const payload = target?.sourceVisual?.dimension === "3d" ? target.sourceVisual : null;
  const documentContract = projectVisualDocumentContract(payload);
  if (!documentContract || documentContract.dimension !== "3d") {
    return null;
  }
  const { width, height, depth } = documentContract.extent;
  const palette = documentContract.resolvedPalette
    .map((entry) => {
      const paletteEntry = { color: normalizeVisualColor(entry?.color) };
      if (entry?.linked && typeof entry.source === "string" && entry.source.trim()) {
        paletteEntry.bind = { type: "color", name: entry.source.trim(), linked: true };
      }
      return paletteEntry;
    });
  const frames = documentContract.cellsByFrame.map((layers) => layers.flat());
  const frameCellCount = width * height * depth;
  if (width < 1 || height < 1 || depth < 1 || !palette.length || !frames.length || frames.some((frame) => frame.length !== frameCellCount)) {
    return null;
  }
  return {
    width,
    height,
    depth,
    palette,
    cells: frames[0].slice(),
    frames,
    animationDurationMs: documentContract.durationMs,
    frameDurationMs: documentContract.frameDurationMs,
    shapeBind: documentContract.shapeRef
      ? { type: "shape", name: documentContract.shapeRef, linked: true }
      : null,
    sourcePreludeRows: documentContract.preludeRows,
    sourceSpatialOps: documentContract.spatialOps,
  };
}

function setVisual3dEditSource(target, document = activeDocument()) {
  setVisualEditorSourceTarget(visual3d, target, document);
  syncVisual3dSourceActionButtons();
}

function clearVisual3dEditSource() {
  clearVisualEditorSourceTarget(visual3d);
}

function invalidateVisual3dEditSourceForDocument(document = activeDocument()) {
  if (!document || !visual3d.editDocumentId || document.id !== visual3d.editDocumentId) {
    return false;
  }
  return invalidateVisualEditorSourceTarget(visual3d, document);
}

function canReplaceCurrentVisual3dDefinition(source) {
  return Boolean(currentVisual3dEditSourceRange(source));
}

function syncVisual3dSourceActionButtons() {
  const hasEditableSource = canReplaceCurrentVisual3dDefinition(activeVisual3dEditSource());
  if (visual3dUpdateButton) {
    visual3dUpdateButton.disabled = !hasEditableSource;
  }
  if (visual3dInsertButton) {
    visual3dInsertButton.disabled = false;
  }
}

function currentVisual3dEditSourceRange(source) {
  return visualEditorSourceRange(visual3d, source, visual3dSourceIndent);
}

function applyIncompleteVisual3dSourceTarget(name, target) {
  resetVisual3dClipState({ clipboard: true });
  if (target && typeof target === "object") {
    setVisual3dEditSource(target, activeDocument());
  }
  visual3dNameInput.value = name || "";
  visual3d.width = clampVisual3dSize(visual3d.width);
  visual3d.height = clampVisual3dSize(visual3d.height);
  visual3d.depth = clampVisual3dSize(visual3d.depth);
  visual3d.axis = "z";
  visual3d.slice = 0;
  visual3d.hoverSlice = null;
  visual3d.palette = [];
  visual3d.cells = Array.from({ length: visual3dFrameCellCount() }, () => null);
  visual3d.frames = [visual3d.cells.slice()];
  visual3d.animationMode = false;
  visual3d.animationFrameIndex = 0;
  visual3d.animationFrameCount = 1;
  visual3d.animationPlaybackIndex = 0;
  visual3d.animationDurationMs = null;
  visual3d.frameDurationMs = null;
  visual3d.shapeBind = null;
  visual3d.sourcePreludeRows = Array.isArray(target?.sourceVisual?.preludeRows)
    ? target.sourceVisual.preludeRows.slice()
    : [];
  visual3d.sourceSpatialOps = Array.isArray(target?.sourceVisual?.spatialOps)
    ? target.sourceVisual.spatialOps.slice()
    : [];
  visual3d.selectedColorIndex = null;
  visual3d.addPaletteOpen = false;
  visual3d.editPaletteOpen = false;
  visual3d.customColorOpen = false;
  visual3d.addDraftColorIndex = null;
  renderVisual3dBuilder();
  syncPreviewModeButtonState();
}

function applyLoadedVisual3d(name, loaded) {
  resetVisual3dClipState({ clipboard: true });
  visual3dNameInput.value = name || "VoxelVisual";
  visual3d.width = loaded.width;
  visual3d.height = loaded.height;
  visual3d.depth = loaded.depth;
  visual3d.axis = "z";
  visual3d.slice = 0;
  visual3d.hoverSlice = null;
  visual3d.palette = loaded.palette;
  visual3d.cells = loaded.cells;
  visual3d.frames = loaded.frames;
  visual3d.animationMode = loaded.frames.length > 1 || Number.isFinite(loaded.animationDurationMs);
  visual3d.animationFrameIndex = 0;
  visual3d.animationFrameCount = Math.max(1, loaded.frames.length);
  visual3d.animationPlaybackIndex = 0;
  visual3d.animationDurationMs = loaded.animationDurationMs;
  visual3d.frameDurationMs = loaded.frameDurationMs;
  visual3d.shapeBind = loaded.shapeBind;
  visual3d.sourcePreludeRows = loaded.sourcePreludeRows;
  visual3d.sourceSpatialOps = loaded.sourceSpatialOps;
  visual3d.selectedColorIndex = visual3d.palette.length ? 0 : null;
  visual3d.addPaletteOpen = false;
  visual3d.editPaletteOpen = false;
  visual3d.customColorOpen = false;
  visual3d.addDraftColorIndex = null;
  renderVisual3dBuilder();
  syncPreviewModeButtonState();
}

function resetVisual3dCamera() {
  visual3d.camera = { ...VISUAL3D_CAMERA_DEFAULT };
  visual3d.hoverSlice = null;
  renderVisual3dCameraControls();
  renderVisual3dPresentationSurfaces();
  setVisual3dActionStatus("Reset camera", "is-ok");
}

function visual3dCameraScrubTarget(event) {
  return event.target?.closest?.("[data-visual3d-camera]") || null;
}

function startVisual3dCameraScrub(event) {
  const target = visual3dCameraScrubTarget(event);
  if (!target || event.button !== 0) {
    return;
  }
  event.preventDefault();
  event.stopPropagation();
  const kind = target.dataset.visual3dCamera;
  visual3dCameraScrubDrag = {
    pointerId: event.pointerId,
    target,
    kind,
    startX: event.clientX,
    startY: event.clientY,
    moved: false,
    value: visual3dCameraValue(kind),
  };
  target.setPointerCapture?.(event.pointerId);
  target.classList.add("is-dragging");
  document.documentElement.classList.add("is-visual3d-camera-scrubbing");
  document.documentElement.classList.add("is-vertical-scrubbing");
}

function continueVisual3dCameraScrub(event) {
  if (!visual3dCameraScrubDrag || visual3dCameraScrubDrag.pointerId !== event.pointerId) {
    return;
  }
  event.preventDefault();
  event.stopPropagation();
  const deltaY = visual3dCameraScrubDrag.startY - event.clientY;
  if (Math.abs(deltaY) > 2) {
    visual3dCameraScrubDrag.moved = true;
  }
  setVisual3dCameraValue(
    visual3dCameraScrubDrag.kind,
    visual3dCameraScrubDrag.value + deltaY * visual3dCameraScrubScale(visual3dCameraScrubDrag.kind),
  );
}

function stopVisual3dCameraScrub(event) {
  if (!visual3dCameraScrubDrag || visual3dCameraScrubDrag.pointerId !== event.pointerId) {
    return;
  }
  event.preventDefault();
  event.stopPropagation();
  finishVisual3dCameraScrub(event.pointerId);
}

function finishVisual3dCameraScrub(pointerId = null) {
  if (!visual3dCameraScrubDrag) {
    return;
  }
  const { target } = visual3dCameraScrubDrag;
  if (pointerId !== null && target.hasPointerCapture?.(pointerId)) {
    target.releasePointerCapture(pointerId);
  }
  target.classList.remove("is-dragging");
  document.documentElement.classList.remove("is-visual3d-camera-scrubbing");
  document.documentElement.classList.remove("is-vertical-scrubbing");
  visual3dCameraScrubDrag = null;
}

function adjustVisual3dCameraScrubWithKey(event) {
  const target = visual3dCameraScrubTarget(event);
  if (!target || !["ArrowLeft", "ArrowDown", "ArrowRight", "ArrowUp"].includes(event.key)) {
    return;
  }
  event.preventDefault();
  const direction = event.key === "ArrowLeft" || event.key === "ArrowDown" ? -1 : 1;
  const kind = target.dataset.visual3dCamera;
  const multiplier = event.shiftKey ? 10 : 1;
  setVisual3dCameraValue(kind, visual3dCameraValue(kind) + direction * visual3dCameraKeyStep(kind) * multiplier);
}

function visual3dCameraValue(kind) {
  const camera = visual3dCamera();
  if (kind === "yaw") {
    return camera.yawDegrees;
  }
  if (kind === "pitch") {
    return camera.pitchDegrees;
  }
  return camera.zoom;
}

function setVisual3dCameraValue(kind, value) {
  const camera = visual3dCamera();
  if (kind === "yaw") {
    camera.yawDegrees = visual3dNormalizeDegrees(value);
  } else if (kind === "pitch") {
    camera.pitchDegrees = visual3dClampNumber(
      value,
      VISUAL3D_CAMERA_MIN_PITCH_DEGREES,
      VISUAL3D_CAMERA_MAX_PITCH_DEGREES,
    );
  } else if (kind === "zoom") {
    camera.zoom = visual3dClampNumber(value, 0.25, 4);
  }
  renderVisual3dCameraControls();
  renderVisual3dPresentationSurfaces();
}

function visual3dCameraScrubScale(kind) {
  return kind === "zoom" ? 0.01 : 0.5;
}

function visual3dCameraKeyStep(kind) {
  return kind === "zoom" ? 0.05 : 1;
}

function setVisual3dActionStatus(text, className = "") {
  if (!visual3dActionStatus) {
    return;
  }
  window.clearTimeout(visual3dActionClearTimer);
  visual3dActionStatus.className = `visual-action-status tool-feedback-bar ${className}`.trim();
  visual3dActionStatus.textContent = text;
  setPaneStatus("visual", text, className);
  if (text && className === "is-ok") {
    visual3dActionClearTimer = window.setTimeout(() => {
      if (visual3dActionStatus.textContent === text && visual3dActionStatus.classList.contains("is-ok")) {
        visual3dActionStatus.className = "visual-action-status tool-feedback-bar";
        visual3dActionStatus.textContent = "";
      }
    }, 1800);
  }
}

function clearVisual3dActionError() {
  if (!visual3dActionStatus?.classList.contains("is-error")) {
    return;
  }
  setVisual3dActionStatus("");
}

function startVisual3dPreviewDrag(event) {
  if (event.button !== 0) {
    return;
  }
  event.preventDefault();
  visual3dPreviewDrag = {
    pointerId: event.pointerId,
    x: event.clientX,
    y: event.clientY,
    startX: event.clientX,
    startY: event.clientY,
    moved: false,
  };
  visual3dPreviewCanvas.setPointerCapture?.(event.pointerId);
  visual3dPreviewCanvas.classList.add("is-dragging");
}

function continueVisual3dPreviewDrag(event) {
  if (!visual3dPreviewDrag || visual3dPreviewDrag.pointerId !== event.pointerId) {
    setVisual3dHoverSliceFromEvent(event);
    return;
  }
  event.preventDefault();
  const camera = visual3dCamera();
  const deltaX = event.clientX - visual3dPreviewDrag.x;
  const deltaY = event.clientY - visual3dPreviewDrag.y;
  visual3dPreviewDrag.x = event.clientX;
  visual3dPreviewDrag.y = event.clientY;
  if (Math.hypot(event.clientX - visual3dPreviewDrag.startX, event.clientY - visual3dPreviewDrag.startY) > 4) {
    visual3dPreviewDrag.moved = true;
  }
  camera.yawDegrees = visual3dNormalizeDegrees(camera.yawDegrees + deltaX * 0.35);
  camera.pitchDegrees = visual3dClampNumber(
    camera.pitchDegrees - deltaY * 0.25,
    VISUAL3D_CAMERA_MIN_PITCH_DEGREES,
    VISUAL3D_CAMERA_MAX_PITCH_DEGREES,
  );
  renderVisual3dCameraControls();
  renderVisual3dPreview();
}

function stopVisual3dPreviewDrag(event) {
  if (!visual3dPreviewDrag || visual3dPreviewDrag.pointerId !== event.pointerId) {
    return;
  }
  if (visual3dPreviewCanvas.hasPointerCapture?.(event.pointerId)) {
    visual3dPreviewCanvas.releasePointerCapture(event.pointerId);
  }
  const wasClick = !visual3dPreviewDrag.moved;
  visual3dPreviewDrag = null;
  visual3dPreviewCanvas.classList.remove("is-dragging");
  const hitSlice = setVisual3dHoverSliceFromEvent(event);
  if (wasClick && Number.isInteger(hitSlice)) {
    setVisual3dSlice(hitSlice);
  }
}

function visual3dNormalizeDegrees(value) {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) {
    return 0;
  }
  return ((parsed % 360) + 360) % 360;
}

function setVisual3dHoverSliceFromEvent(event) {
  const next = visual3dSliceFromPreviewEvent(event);
  if (visual3d.hoverSlice !== next) {
    visual3d.hoverSlice = next;
    renderVisual3dPreview();
  }
  return next;
}

function visual3dSliceFromPreviewEvent(event) {
  const rect = visual3dPreviewCanvas.getBoundingClientRect();
  const x = event.clientX - rect.left;
  const y = event.clientY - rect.top;
  const point = { x, y };
  const view = visual3dPreviewCanvas._visual3dPreviewView
    || visual3dPreviewView(Math.max(1, Math.round(rect.width)), Math.max(1, Math.round(rect.height)));
  const ray = visual3dPreviewRay(point, view);
  const voxelHit = visual3dRaycastOccupiedVoxel(ray);
  if (voxelHit) {
    return visual3dSliceIndexForVoxel(voxelHit.grid);
  }
  return visual3dApproximateSliceFromRay(ray);
}

function visual3dPreviewRay(point, view) {
  const camera = visual3dCamera();
  const yaw = visual3dDegreesToRadians(camera.yawDegrees ?? 0);
  const pitch = visual3dDegreesToRadians(camera.pitchDegrees ?? 0);
  const scale = Math.max(0.000001, view.cellScale * (camera.zoom ?? 1));
  const screenU = (point.x - view.originX) / scale;
  const screenV = (point.y - view.originY) / scale;
  const sinYaw = Math.sin(yaw);
  const cosYaw = Math.cos(yaw);
  const sinPitch = Math.sin(pitch);
  const cosPitch = Math.cos(pitch);
  const yawYAtDepthZero = -sinPitch * screenV;
  const centerX = (visual3d.width - 1) / 2;
  const centerY = (visual3d.height - 1) / 2;
  const centerDepth = (visual3d.depth - 1) / 2;
  return {
    origin: {
      x: centerX + screenU * cosYaw + yawYAtDepthZero * sinYaw,
      y: centerY - screenU * sinYaw + yawYAtDepthZero * cosYaw,
      z: centerDepth - cosPitch * screenV,
    },
    direction: {
      x: -sinYaw * cosPitch,
      y: -cosYaw * cosPitch,
      z: sinPitch,
    },
  };
}

function visual3dRaycastOccupiedVoxel(ray) {
  let best = null;
  for (let z = 0; z < visual3d.depth; z += 1) {
    for (let y = 0; y < visual3d.height; y += 1) {
      for (let x = 0; x < visual3d.width; x += 1) {
        if (!validVisual3dColorIndex(visual3d.cells[visual3dCellIndex(x, y, z)])) {
          continue;
        }
        const hit = visual3dRayAabbInterval(ray, {
          min: { x: x - 0.5, y: y - 0.5, z: z - 0.5 },
          max: { x: x + 0.5, y: y + 0.5, z: z + 0.5 },
        });
        if (!hit) {
          continue;
        }
        if (!best || hit.tMax > best.tMax + 0.000001) {
          best = { grid: { x, y, z }, tMax: hit.tMax };
        }
      }
    }
  }
  return best;
}

function visual3dApproximateSliceFromRay(ray) {
  const bounds = {
    min: { x: -0.5, y: -0.5, z: -0.5 },
    max: { x: visual3d.width - 0.5, y: visual3d.height - 0.5, z: visual3d.depth - 0.5 },
  };
  const hit = visual3dRayAabbInterval(ray, bounds);
  if (!hit) {
    return null;
  }
  const point = {
    x: ray.origin.x + ray.direction.x * hit.tMax,
    y: ray.origin.y + ray.direction.y * hit.tMax,
    z: ray.origin.z + ray.direction.z * hit.tMax,
  };
  return visual3dSliceIndexForWorldPoint(point);
}

function visual3dRayAabbInterval(ray, bounds) {
  let tMin = -Infinity;
  let tMax = Infinity;
  for (const axis of ["x", "y", "z"]) {
    const origin = ray.origin[axis];
    const direction = ray.direction[axis];
    if (Math.abs(direction) < 0.000001) {
      if (origin < bounds.min[axis] || origin > bounds.max[axis]) {
        return null;
      }
      continue;
    }
    const t1 = (bounds.min[axis] - origin) / direction;
    const t2 = (bounds.max[axis] - origin) / direction;
    tMin = Math.max(tMin, Math.min(t1, t2));
    tMax = Math.min(tMax, Math.max(t1, t2));
    if (tMin > tMax) {
      return null;
    }
  }
  return { tMin, tMax };
}

function visual3dSliceIndexForVoxel(grid) {
  return visual3dSliceIndexForWorldPoint(grid);
}

function visual3dSliceIndexForWorldPoint(point) {
  const max = visual3dAxisSize() - 1;
  if (visual3d.axis === "x") {
    return visual3dClamp(Math.round(point.x), 0, max);
  }
  if (visual3d.axis === "y") {
    return visual3dClamp(max - Math.round(point.y), 0, max);
  }
  return visual3dClamp(max - Math.round(point.z), 0, max);
}

function visual3dDegreesToRadians(value) {
  return (Number(value) * Math.PI) / 180;
}

function clearVisual3dHoverSlice() {
  if (visual3d.hoverSlice === null) {
    return;
  }
  visual3d.hoverSlice = null;
  renderVisual3dPreview();
}

function visual3dNearestSliceEdgeHit(point, edges) {
  let best = null;
  for (const edge of edges) {
    const candidate = visual3dSliceEdgeHit(point, edge);
    if (!candidate) {
      continue;
    }
    if (
      !best
      || candidate.distance < best.distance
      || (Math.abs(candidate.distance - best.distance) < 0.001 && candidate.index < best.index)
    ) {
      best = candidate;
    }
  }
  return best;
}

function visual3dSliceEdgeHit(point, edge) {
  const dx = edge.to.x - edge.from.x;
  const dy = edge.to.y - edge.from.y;
  const lengthSquared = dx * dx + dy * dy;
  if (lengthSquared <= 0.0001) {
    return null;
  }
  const rawT = ((point.x - edge.from.x) * dx + (point.y - edge.from.y) * dy) / lengthSquared;
  const t = visual3dClamp(rawT, 0, 1);
  const nearest = {
    x: edge.from.x + dx * t,
    y: edge.from.y + dy * t,
  };
  const distance = Math.hypot(point.x - nearest.x, point.y - nearest.y);
  if (distance > edge.hitRadius) {
    return null;
  }
  const axisSize = visual3dAxisSize();
  const world = Math.max(0, Math.min(axisSize - 1, Math.round(edge.min + (edge.max - edge.min) * t)));
  return {
    index: Math.max(0, Math.min(axisSize - 1, axisSize - 1 - world)),
    distance,
  };
}

function visual3dPointInPolygon(point, polygon) {
  let inside = false;
  for (let i = 0, j = polygon.length - 1; i < polygon.length; j = i, i += 1) {
    const xi = polygon[i].x;
    const yi = polygon[i].y;
    const xj = polygon[j].x;
    const yj = polygon[j].y;
    const intersects = ((yi > point.y) !== (yj > point.y))
      && (point.x < ((xj - xi) * (point.y - yi)) / ((yj - yi) || 1) + xi);
    if (intersects) {
      inside = !inside;
    }
  }
  return inside;
}

for (const input of [
  visual3dNameInput,
  visual3dWidthInput,
  visual3dHeightInput,
  visual3dDepthInput,
  visual3dScaleInput,
  visual3dSliceValue,
  visual3dAnimationDurationInput,
  visual3dAnimationFrameCountInput,
  visual3dAnimationFrameInput,
]) {
  installSelectAllOnFocus(input);
}
visual3dNameInput?.addEventListener("input", () => {
  renderVisual3dPreview();
  syncVisual3dSourceActionButtons();
});
sourceEditor.addEventListener("input", () => {
  invalidateVisual3dEditSourceForDocument(activeDocument());
  syncVisual3dSourceActionButtons();
});
function bindVisual3dDimensionInput(input, axis) {
  input?.addEventListener("input", () => {
    if (input.validity.valid && input.value !== "") {
      updateVisual3dDimension(axis, input.value);
    }
  });
  input?.addEventListener("change", () => updateVisual3dDimension(axis, input.value));
  input?.addEventListener("keydown", (event) => {
    if (event.key !== "Enter") return;
    event.preventDefault();
    updateVisual3dDimension(axis, input.value);
  });
}
bindVisual3dDimensionInput(visual3dWidthInput, "width");
bindVisual3dDimensionInput(visual3dHeightInput, "height");
bindVisual3dDimensionInput(visual3dDepthInput, "depth");
visual3dScaleInput?.addEventListener("input", () => {
  clearVisual3dActionError();
  renderVisual3dControls();
});
visual3dScaleInput?.addEventListener("keydown", (event) => {
  if (event.key !== "Enter") {
    return;
  }
  event.preventDefault();
});
visual3dSliceValue?.addEventListener("change", applyVisual3dSliceInput);
visual3dSliceValue?.addEventListener("keydown", (event) => {
  if (event.key !== "Enter") {
    return;
  }
  event.preventDefault();
  applyVisual3dSliceInput();
});
visual3dAnimationDurationInput?.addEventListener("change", () => setVisual3dAnimationDuration(visual3dAnimationDurationInput.value));
visual3dAnimationFrameCountInput?.addEventListener("change", () => setVisual3dAnimationFrameCount(visual3dAnimationFrameCountInput.value));
const visual3dSliceScrub = document.querySelector("[data-visual3d-slice-scrub]");
visual3dSliceScrub?.addEventListener("pointerdown", startVisual3dSliceScrub);
visual3dSliceScrub?.addEventListener("pointermove", continueVisual3dSliceScrub);
visual3dSliceScrub?.addEventListener("pointerup", stopVisual3dSliceScrub);
visual3dSliceScrub?.addEventListener("pointercancel", stopVisual3dSliceScrub);
for (const scrub of [visual3dCameraYawScrub, visual3dCameraPitchScrub, visual3dCameraZoomScrub]) {
  scrub?.addEventListener("pointerdown", startVisual3dCameraScrub);
  scrub?.addEventListener("pointermove", continueVisual3dCameraScrub);
  scrub?.addEventListener("pointerup", stopVisual3dCameraScrub);
  scrub?.addEventListener("pointercancel", stopVisual3dCameraScrub);
  scrub?.addEventListener("keydown", adjustVisual3dCameraScrubWithKey);
}
window.addEventListener("pointerup", stopVisual3dCameraScrub, true);
window.addEventListener("pointercancel", stopVisual3dCameraScrub, true);
window.addEventListener("pointerup", stopVisual3dSliceScrub, true);
window.addEventListener("pointercancel", stopVisual3dSliceScrub, true);
window.addEventListener("blur", () => {
  finishVisual3dCameraScrub();
  finishVisual3dSliceScrub();
});
visual3dPalette?.addEventListener("keydown", (event) => {
  const token = event.target.closest(".visual-token");
  if (!token || (event.key !== "Enter" && event.key !== " ")) {
    return;
  }
  const rawIndex = token.dataset.colorIndex;
  if (rawIndex === undefined) {
    return;
  }
  event.preventDefault();
  selectVisual3dColor(rawIndex === "erase" ? null : Number(rawIndex));
});
visual3dSliceBoard?.addEventListener("pointerdown", startVisual3dPaint);
visual3dSliceBoard?.addEventListener("pointermove", continueVisual3dPaint);
visual3dSliceBoard?.addEventListener("pointerup", stopVisual3dPaint);
visual3dSliceBoard?.addEventListener("pointercancel", stopVisual3dPaint);
visual3dSliceBoard?.addEventListener("keydown", (event) => {
  if (visual3dTranslateActive) {
    event.preventDefault();
    return;
  }
  if (event.key === "Enter" || event.key === " ") {
    const mutate = visual3dBucketActive ? bucketFillVisual3dFromElement : paintVisual3dCellFromElement;
    if (withVisualEditHistory("visual3d", () => mutate(event.target))) {
      event.preventDefault();
      event.stopPropagation();
    }
  }
});
visual3dScaleDownButton?.addEventListener("click", scaleDownVisual3d);
visual3dScaleUpButton?.addEventListener("click", scaleUpVisual3d);
visual3dRotatePlaneLeftButton?.addEventListener("click", rotateVisual3dPlaneLeft);
visual3dRotatePlaneRightButton?.addEventListener("click", rotateVisual3dPlaneRight);
visual3dFlipPlaneHorizontalButton?.addEventListener("click", flipVisual3dPlaneHorizontal);
visual3dFlipPlaneVerticalButton?.addEventListener("click", flipVisual3dPlaneVertical);
visual3dResetCameraButton?.addEventListener("click", resetVisual3dCamera);
visual3dPreviewCanvas?.addEventListener("pointerdown", startVisual3dPreviewDrag);
visual3dPreviewCanvas?.addEventListener("pointermove", continueVisual3dPreviewDrag);
visual3dPreviewCanvas?.addEventListener("pointerup", stopVisual3dPreviewDrag);
visual3dPreviewCanvas?.addEventListener("pointercancel", stopVisual3dPreviewDrag);
visual3dPreviewCanvas?.addEventListener("pointerleave", clearVisual3dHoverSlice);
document.addEventListener("click", (event) => {
  if (!visual3dTranslateActive || visual3dSliceBoard?.contains(event.target)) {
    return;
  }
  if (event.target.closest?.("#visual3dTranslateButton")) {
    return;
  }
  deactivateVisual3dTranslateMode();
});
document.addEventListener("keydown", (event) => {
  handleVisual3dClipKeyboard(event);
});
window.addEventListener("resize", () => {
  if (!visual3dBuilder?.hidden) {
    renderVisual3dPreview();
  }
});
registerSourceEditableTarget?.("visual3d", {
  load: loadVisual3dFromSourcePosition,
});

function syncVisual3dBuilderAfterScriptLoad() {
  if (currentPreviewMode === "visual3d" && typeof loadFirstFocusedPuzzleEntry === "function") {
    loadFirstFocusedPuzzleEntry("visual", "visual3d");
  }
}

resetVisual3dBuilder();
syncVisual3dBuilderAfterScriptLoad();
