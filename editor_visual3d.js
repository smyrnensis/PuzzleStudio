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
const VISUAL3D_SLICE_SCRUB_STEP_PX = 18;
const VISUAL3D_CAMERA_MIN_PITCH_DEGREES = -90;
const VISUAL3D_CAMERA_MAX_PITCH_DEGREES = 90;
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
  return normalizedVisualEditorAnimationDuration(value);
}

function normalizeVisual3dFrameCells(cells) {
  return normalizeVisualEditorFrameCells("visual3d", cells);
}

function ensureVisual3dAnimationState() {
  ensureVisualEditorAnimationState("visual3d");
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

function setVisual3dAnimationDuration(value) {
  return updateVisualEditorAnimationDuration("visual3d", value);
}

function resetVisual3dBuilder(
  width = visual3d.width,
  height = visual3d.height,
  depth = visual3d.depth,
) {
  resetVisualEditorBuilder("visual3d", { width, height, depth }, {
    beforeReset: () => {
      resetVisual3dClipState({ clipboard: true });
      ensureVisual3dPalette();
    },
    configureState: (state) => {
      state.slice = Math.max(0, Math.min(visual3dAxisSize() - 1, Number(state.slice) || 0));
      state.hoverSlice = null;
    },
  });
}

function clampVisual3dSize(value) {
  return clampVisualEditorSize(value);
}

function withVisual3dPaneScrollPreserved(render) {
  return withVisualPaneScrollPreserved(visual3dBuilder, render);
}

function renderVisual3dBuilder() {
  if (!visual3dBuilder || !visual3dSliceBoard || !visual3dPalette || !visual3dPreviewHost) {
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

function visual3dAnimationFramePreview(frame, index) {
  const host = document.createElement("div");
  host.className = "visual-animation-3d-preview visual-animation-3d-runtime-preview";
  host.setAttribute("aria-hidden", "true");
  queueVisual3dRuntimeThumbnail(host, frame, index);
  return [host];
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
    renderCells: (index) => visual3dAnimationFramePreview(visual3d.frames[index], index),
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
      maxSize: VISUAL_EDITOR_MAX_SIZE,
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
    if (visual3dAnimationFrameInput) {
      visual3dAnimationFrameInput.value = String((visual3d.animationFrameIndex || 0) + 1);
      visual3dAnimationFrameInput.max = String(visual3d.animationFrameCount || 1);
    }
    if (visual3dAnimationFrameTotal) {
      visual3dAnimationFrameTotal.textContent = String(visual3d.animationFrameCount || 1);
    }
    syncSharedVisualAnimationToolbarState(visual3d.animationFrameCount || 1, VISUAL_ANIMATION_MAX_FRAMES);
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
  syncVisualEditorGridButton("visual3d");
}

function toggleVisual3dGrid() {
  toggleVisualEditorGrid("visual3d");
}

function visual3dEditScope() {
  if (visual3d.editScope !== "all") {
    visual3d.editScope = "slice";
  }
  return visual3d.editScope;
}

function renderVisual3dScopeControl() {
  const scope = visual3dEditScope();
  if (visual3dEditScopeButton) {
    const label = scope === "all" ? "Edit whole volume" : "Edit current slice";
    visual3dEditScopeButton.setAttribute("aria-label", label);
    visual3dEditScopeButton.title = label;
    for (const icon of visual3dEditScopeButton.querySelectorAll("[data-visual3d-edit-scope-icon]")) {
      icon.hidden = icon.dataset.visual3dEditScopeIcon !== scope;
    }
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
  toggleVisualEditorClipMode("visual3d");
}

function deactivateVisual3dClipMode(options = {}) {
  deactivateVisualEditorClipMode("visual3d", options);
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
  return pasteVisualEditorCell("visual3d", index, clipboardValue);
}

function visual3dClipForCurrentPalette(clipboard) {
  return remapVisualEditorClipboardPalette("visual3d", clipboard);
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
  return commitVisualEditorCellMutation(
    "visual3d",
    before,
    changed,
    message,
    "Clip did not change 3D visual",
  );
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
  toggleVisualEditorTranslateMode("visual3d");
}

function deactivateVisual3dTranslateMode(options = {}) {
  deactivateVisualEditorTranslateMode("visual3d", options);
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
          visualPositiveModulo(u + du, plane.width),
          visualPositiveModulo(v + dv, plane.height),
        );
        next[visual3dCellIndex(target.x, target.y, target.z)] = originCells[visual3dCellIndex(source.x, source.y, source.z)];
      }
    }
  }
  return next;
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
  if (!visualCellsEqual(visual3d.cells, drag.originCells)) {
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
  toggleVisualEditorBucketMode("visual3d");
}

function deactivateVisual3dBucketModeAfterUse() {
  deactivateVisualEditorBucketModeAfterUse("visual3d");
}

function renderVisual3dPalette() {
  withVisual3dPaneScrollPreserved(() => renderVisual3dPaletteContent());
}

function setVisual3dCurrentColorTag(index, rawName, linked = true) {
  return applyVisualPaletteColorName("visual3d", index, rawName, {
    linked: Boolean(linked),
    reportError: true,
  });
}

function renderVisual3dPaletteContent() {
  ensureVisual3dPalette();
  renderVisualPaletteContentForDimension({
    dimension: "visual3d",
    target: visual3dPalette,
    shapeField: visual3dShapeField,
    leadingControl: visualMarkerTool,
    bucketActive: visual3dBucketActive,
    emptyTitle: "Paint empty voxel",
    emptyAriaLabel: "Paint empty voxel",
    colorAriaLabel: (index, name) => name
      ? `Paint 3D visual color ${index + 1}: ${name}`
      : `Paint 3D visual color ${index + 1}`,
    onSelect: selectVisual3dColor,
    onRemove: removeVisual3dColor,
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
  queueVisual3dRuntimePreview();
  renderVisual3dCameraControls();
}

function renderVisual3dPresentationSurfaces() {
  renderVisual3dPreview();
  renderVisual3dAnimationFrameStrip();
  if (visual3d.animationMode) {
    const context = sharedVisualAnimationController("visual3d");
    renderSharedVisualAnimationPlaybackView(
      context,
      context.frames[context.state.animationPlaybackIndex] || context.state.cells,
    );
  }
}

function visual3dRuntimeCells(frames = [visual3d.cells]) {
  return frames.map((cells) => Array.from(
    { length: visual3d.width * visual3d.height * visual3d.depth },
    (_, index) => validVisual3dColorIndex(cells?.[index]) ? cells[index] : null,
  ));
}

function visual3dRuntimeSlice(slice = visual3d.slice) {
  return {
    axis: visual3d.axis,
    index: visual3dPlaneWorldSlice(visual3d.axis, slice),
  };
}

function visual3dRuntimeState({ playing = false } = {}) {
  ensureVisual3dAnimationState();
  const frames = visual3d.animationMode ? visual3d.frames : [visual3d.cells];
  const frameIndex = visual3d.animationMode ? visual3d.animationFrameIndex : 0;
  return {
    width: visual3d.width,
    depth: visual3d.height,
    height: visual3d.depth,
    palette: visual3dPaletteColors(),
    frames: visual3dRuntimeCells(frames),
    frameIndex,
    frameDurationMs: Math.max(1, Math.round(
      normalizedVisual3dAnimationDuration() / Math.max(1, frames.length),
    )),
    playing: playing && frames.length > 1,
  };
}

function visual3dRuntimePresentation(surfaceId, interaction, options = {}) {
  const camera = visual3dCamera();
  const selection = options.selectionVisible === false
    ? null
    : normalizeVisual3dClipBox(visual3dClipSelection);
  return {
    surface: {
      surfaceId,
      interaction,
    },
    renderer: {
      kind: "visual3d",
      activeSlice: options.slicesVisible === false ? null : visual3dRuntimeSlice(),
      hoverSlice: options.slicesVisible !== false && Number.isInteger(visual3d.hoverSlice)
        ? visual3dRuntimeSlice(visual3d.hoverSlice)
        : null,
      selectionBox: selection ? {
        min: { x: selection.minX, y: selection.minY, z: selection.minZ },
        max: { x: selection.maxX, y: selection.maxY, z: selection.maxZ },
      } : null,
      camera: {
        projection: "orthographic",
        yawDegrees: camera.yawDegrees,
        pitchDegrees: camera.pitchDegrees,
        rollDegrees: 0,
        zoom: camera.zoom,
      },
      view: {
        target: {
          x: (visual3d.width - 1) / 2,
          y: (visual3d.height - 1) / 2,
          z: (visual3d.depth - 1) / 2,
        },
        insets: visual3dRuntimeViewportInsets(options.reserveOverlaySpace !== false),
      },
      settings: {
        voxelGridVisible: options.gridVisible ?? visual3dGridVisible,
        boundsVisible: true,
      },
      style: visual3dRuntimeStyle(),
    },
  };
}

function visual3dRuntimeViewportInsets(reserveOverlaySpace) {
  if (!reserveOverlaySpace) {
    return { top: 0, right: 0, bottom: 0, left: 0 };
  }
  const controlHeight = Number.parseFloat(
    visual3dCssVar("--visual3d-overlay-control-height", "22"),
  );
  if (!Number.isFinite(controlHeight) || controlHeight < 0) {
    throw new Error("3D visual overlay control height must be a non-negative CSS length.");
  }
  const safeInset = 8 + controlHeight + 4;
  return { top: safeInset, right: 0, bottom: safeInset, left: 0 };
}

function visual3dRuntimeStyle() {
  return {
    boundsFill: visual3dRuntimeLinearColor("--visual3d-frame-fill", "rgba(137, 148, 158, 0.10)"),
    boundsStroke: visual3dRuntimeLinearColor("--visual3d-frame-stroke", "rgba(137, 148, 158, 0.38)"),
    voxelGridStroke: visual3dRuntimeLinearColor("--visual3d-voxel-grid-stroke", "rgba(20, 24, 28, 0.38)"),
    activeSliceFill: visual3dRuntimeLinearColor("--visual3d-slice-active-fill", "rgba(125, 208, 160, 0.022)"),
    activeSliceStroke: visual3dRuntimeLinearColor("--visual3d-slice-active-stroke", "rgba(125, 208, 160, 0.12)"),
    hoverSliceFill: visual3dRuntimeLinearColor("--visual3d-slice-hover-fill", "rgba(137, 148, 158, 0.025)"),
    hoverSliceStroke: visual3dRuntimeLinearColor("--visual3d-slice-hover-stroke", "rgba(137, 148, 158, 0.15)"),
    selectionFill: visual3dRuntimeLinearColor("--visual3d-clip-fill", "rgba(125, 208, 160, 0.08)"),
    selectionStroke: visual3dRuntimeLinearColor("--visual3d-clip-stroke", "rgba(125, 208, 160, 0.9)"),
  };
}

function visual3dRuntimeLinearColor(name, fallback) {
  const canvas = document.createElement("canvas");
  canvas.width = 1;
  canvas.height = 1;
  const context = canvas.getContext("2d", { willReadFrequently: true });
  if (!context) {
    throw new Error("3D visual presentation colors require a canvas color parser.");
  }
  context.clearRect(0, 0, 1, 1);
  context.fillStyle = visual3dCssVar(name, fallback);
  context.fillRect(0, 0, 1, 1);
  const [red, green, blue, alpha] = context.getImageData(0, 0, 1, 1).data;
  const linear = (channel) => {
    const srgb = channel / 255;
    return srgb <= 0.04045
      ? srgb / 12.92
      : ((srgb + 0.055) / 1.055) ** 2.4;
  };
  return {
    red: linear(red),
    green: linear(green),
    blue: linear(blue),
    alpha: alpha / 255,
  };
}

function queueVisual3dRuntimePreview() {
  if (!visual3dPreviewHost || visual3dBuilder?.hidden) {
    return;
  }
  const surfaceId = "visual3d-authoring";
  const state = visual3dRuntimeState();
  const presentation = visual3dRuntimePresentation(surfaceId, {
      kind: "selectSlice",
      axis: visual3d.axis,
    });
  window.PuzzleStudioSpriteProjection.project({
    host: visual3dPreviewHost,
    surfaceId,
    draft: state,
    presentation,
    interactive: true,
    onPointer: handleVisual3dRuntimePointer,
    onHit: applyVisual3dProjectionHit,
    onReady: clearVisual3dActionError,
    onError: (error) => setVisual3dActionStatus(
      `3D preview failed: ${userFacingRuntimeError(error)}`,
      "is-error",
    ),
  });
}

function queueVisual3dRuntimePlayback() {
  if (!visualAnimationPlaybackView || !visual3d.animationMode) {
    return;
  }
  const surfaceId = "visual3d-playback";
  const state = visual3dRuntimeState({ playing: true });
  const presentation = visual3dRuntimePresentation(
      surfaceId,
      { kind: "observe" },
      {
        gridVisible: false,
        slicesVisible: false,
        selectionVisible: false,
        reserveOverlaySpace: false,
      },
    );
  window.PuzzleStudioSpriteProjection.project({
    host: visualAnimationPlaybackView,
    surfaceId,
    draft: state,
    presentation,
    onError: (error) => setVisual3dActionStatus(
      `3D playback failed: ${userFacingRuntimeError(error)}`,
      "is-error",
    ),
  });
}

function queueVisual3dRuntimeThumbnail(host, cells, index) {
  const surfaceId = `visual3d-thumbnail-${index}`;
  const state = visual3dRuntimeState();
  state.frames = visual3dRuntimeCells([cells]);
  state.frameIndex = 0;
  state.playing = false;
  const presentation = visual3dRuntimePresentation(
    surfaceId,
    { kind: "observe" },
    {
      gridVisible: false,
      slicesVisible: false,
      selectionVisible: false,
      reserveOverlaySpace: false,
    },
  );
  window.PuzzleStudioSpriteProjection.project({
    host,
    surfaceId,
    draft: state,
    presentation,
    onError: (error) => setVisual3dActionStatus(
      `3D animation thumbnail failed: ${userFacingRuntimeError(error)}`,
      "is-error",
    ),
  });
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

function visual3dCssVar(name, fallback) {
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return value || fallback;
}

function visual3dClamp(value, min, max) {
  return Math.min(max, Math.max(min, value));
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

function toggleVisual3dAddPalette() {
  toggleVisualPaletteAdd("visual3d");
}

function previewNewVisual3dColor(color, options = {}) {
  previewNewVisualPaletteColor("visual3d", color, options);
}

function updateSelectedVisual3dColor(value, options = {}) {
  updateSelectedVisualPaletteColor("visual3d", value, options);
}

function closeVisual3dColorEditor() {
  closeVisualPaletteEditor("visual3d");
}

function cancelVisual3dColorAdd() {
  cancelVisualPaletteColorAdd("visual3d");
}

function removeVisual3dColor() {
  removeSelectedVisualPaletteColor("visual3d");
}

function removeVisual3dPaletteColor(deletedIndex) {
  return removeVisualPaletteColorAt("visual3d", deletedIndex);
}

function renderVisual3dColorSurfaces() {
  syncVisual3dPaletteSwatches();
  syncVisual3dColorAdjusters();
  renderVisual3dSliceBoard();
  renderVisual3dPreview();
}

function syncVisual3dPaletteSwatches() {
  syncVisualPaletteSwatchesForDimension("visual3d", visual3dPalette);
}

function syncVisual3dColorAdjusters() {
  syncVisualColorAdjustersForDimension("visual3d", visual3dPalette);
}

function validVisual3dColorIndex(index) {
  return validVisualPaletteColorIndex("visual3d", index);
}

function normalizedVisual3dCellColorIndex(index) {
  return normalizedVisualPaletteCellColorIndex("visual3d", index);
}

function visual3dColorForColorIndex(index) {
  return visualPaletteColorForIndex("visual3d", index);
}

function visual3dInkForColorIndex(index) {
  return visualPaletteInkForIndex("visual3d", index);
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
  const erase = editorPointerEraseIntent(event);
  if (event.button !== 0 && !erase) {
    return;
  }
  if (visual3dBucketActive && !erase) {
    const index = visual3dSliceCellIndexFromElement(document.elementFromPoint(event.clientX, event.clientY));
    if (!Number.isInteger(index) || index < 0) {
      return;
    }
    event.preventDefault();
    const before = visualEditSnapshot("visual3d");
    if (bucketFillVisual3dFromSliceIndex(index)) {
      pushVisualEditUndoSnapshot("visual3d", before);
    }
    return;
  }
  if (visual3dClipActive && !erase) {
    startVisual3dClip(event);
    return;
  }
  if (visual3dTranslateActive && !erase) {
    startVisual3dTranslate(event);
    return;
  }
  const index = visual3dSliceCellIndexFromElement(document.elementFromPoint(event.clientX, event.clientY));
  if (!Number.isInteger(index) || index < 0) {
    return;
  }
  event.preventDefault();
  visual3dPaintDrag = {
    pointerId: event.pointerId,
    colorIndex: erase ? null : visual3d.selectedColorIndex,
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

function updateVisual3dDimension(axis, value, options = {}) {
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
    if (!options.preserveInput) renderVisual3dControls();
    return;
  }
  remapVisual3dFrames(next, (x, y, z) => ({ x, y, z }));
  resetVisual3dClipState();
  visual3d.slice = Math.min(visual3d.slice, visual3dAxisSize() - 1);
  if (options.preserveInput) {
    visual3dWidthInput.value = String(visual3d.width);
    visual3dHeightInput.value = String(visual3d.height);
    visual3dDepthInput.value = String(visual3d.depth);
    renderVisual3dSliceBoard();
    renderVisual3dPresentationSurfaces();
    syncVisual3dSourceActionButtons();
  } else {
    renderVisual3dBuilder();
  }
  pushVisualEditUndoSnapshot("visual3d", before);
}

function remapVisual3dFrames(nextExtent, sourceCoordinates) {
  remapVisualEditorFrames("visual3d", nextExtent, sourceCoordinates);
}

function visual3dScaleFactor() {
  return visualEditorScaleFactorForDimension("visual3d");
}

function canScaleDownVisual3d(factor = visual3dScaleFactor()) {
  return canScaleDownVisualEditor("visual3d", factor);
}

function scaleUpVisual3d() {
  return scaleVisualEditor("visual3d", "up");
}

function scaleDownVisual3d() {
  return scaleVisualEditor("visual3d", "down");
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
  const fixed = visual3dPlaneWorldSlice(axis, stack);
  const maxY = visual3d.height - 1;
  const maxZ = visual3d.depth - 1;
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
  return visualEditorObjectName(visual3dNameInput, "VoxelVisual");
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
  return frames.map((frame) =>
    Array.from({ length: visual3d.depth }, (_, sourceZ) => {
      const worldZ = visual3d.depth - 1 - sourceZ;
      return Array.from({ length: visual3d.height }, (_, y) =>
        Array.from({ length: visual3d.width }, (_, x) => {
          const cell = frame[visual3dCellIndex(x, y, worldZ)];
          return Number.isInteger(cell) ? cell : null;
        }));
    }));
}

function visual3dEditMutationRequest(operation, options = {}) {
  return visualEditorMutationRequest({
    operation,
    dimension: "3d",
    state: visual3d,
    name: options.name ?? visual3dObjectName(),
    originalName: options.originalName ?? visual3d.editSourceName ?? visual3dObjectName(),
    cursor: options.cursor,
    palette: visual3dPaletteSourceTokens(),
    frames: visual3dEditFrames(),
    durationMs: visual3d.animationMode ? normalizedVisual3dAnimationDuration() : null,
    frameDurationMs: visual3d.animationMode ? visual3d.frameDurationMs : null,
    includeFrameDurationMs: true,
  });
}

async function updateVisual3dInSource() {
  return updateVisualEditorSourceDefinition({
    dimension: "visual3d",
    state: visual3d,
    label: "3D visual",
    request: () => visual3dEditMutationRequest("update"),
  });
}

async function addVisual3dToSource() {
  return addVisualEditorSourceDefinition({
    dimension: "visual3d",
    state: visual3d,
    nameInput: visual3dNameInput,
    label: "3D visual",
    request: (source, document) => visual3dEditMutationRequest(
        canReplaceCurrentVisual3dDefinition(source) ? "duplicate" : "insert",
        { cursor: visualSourceCursorPosition(source, document) },
      ),
  });
}

function newVisual3dDraft() {
  newVisualEditorDraft({
    dimension: "visual3d",
    nameInput: visual3dNameInput,
    name: "VoxelVisual",
    reset: () => resetVisual3dBuilder(5, 5, 5),
  });
}

function activeVisual3dEditDocument() {
  return activeVisualEditorDocument("visual3d");
}

function activeVisual3dEditSource() {
  return activeVisualEditorSource("visual3d");
}

const VISUAL3D_SOURCE_INDENT = "";

function visual3dSourceIndent(indent = "") {
  return String(indent || "").replace(/\t/g, VISUAL3D_SOURCE_INDENT);
}

async function loadVisual3dFromSourcePosition(position, options = {}) {
  if (!isPuzzleDocument(activeDocument()) || !isTextDocument(activeDocument())) {
    return null;
  }
  const source = sourceEditor.documentText() || "";
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
        const message = visualSourceContractError(target.sourceVisual)
          || (target.sourceVisual.status === "invalid"
            ? `Cannot edit invalid 3D visual ${visual3dNameInput.value || ""}`.trim()
            : `Loaded unfinished 3D visual ${visual3dNameInput.value || ""}`.trim());
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
  setVisualEditorDimensionSourceTarget("visual3d", target, document);
  syncVisual3dSourceActionButtons();
}

function clearVisual3dEditSource() {
  clearVisualEditorDimensionSourceTarget("visual3d");
}

function invalidateVisual3dEditSourceForDocument(document = activeDocument()) {
  return invalidateVisualEditorDimensionSourceTarget("visual3d", document);
}

function canReplaceCurrentVisual3dDefinition(source) {
  return canReplaceVisualEditorDefinition("visual3d", source, visual3dSourceIndent);
}

function syncVisual3dSourceActionButtons() {
  syncVisualEditorSourceActionButtons("visual3d", { indentForSource: visual3dSourceIndent });
}

function currentVisual3dEditSourceRange(source) {
  return currentVisualEditorSourceRange("visual3d", source, visual3dSourceIndent);
}

function applyIncompleteVisual3dSourceTarget(name, target) {
  applyIncompleteVisualEditorSourceTarget({
    dimension: "visual3d",
    name,
    target,
    nameInput: visual3dNameInput,
    beforeReset: () => resetVisual3dClipState({ clipboard: true }),
  });
}

function applyLoadedVisual3d(name, loaded) {
  applyLoadedVisualEditorSource({
    dimension: "visual3d",
    name: name || "VoxelVisual",
    nameInput: visual3dNameInput,
    loaded,
    frames: loaded.frames,
    animationMode: loaded.frames.length > 1 || Number.isFinite(loaded.animationDurationMs),
    beforeApply: () => resetVisual3dClipState({ clipboard: true }),
    configureState: (state) => {
      state.axis = "z";
      state.slice = 0;
      state.hoverSlice = null;
    },
  });
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
  setVisualDimensionActionStatus("visual3d", text, className);
}

function clearVisual3dActionError() {
  clearVisualDimensionActionError("visual3d");
}

function handleVisual3dRuntimePointer(eventData) {
  const gesture = String(eventData?.gesture || "");
  const x = Number(eventData?.xCss);
  const y = Number(eventData?.yCss);
  if (gesture === "press") {
    if (Number(eventData?.button) !== 0) {
      return { forward: false };
    }
    visual3dPreviewDrag = { x, y, startX: x, startY: y, moved: false };
    visual3dPreviewHost?.classList.add("is-dragging");
    return { forward: false };
  }
  if (gesture === "move" && visual3dPreviewDrag) {
    const deltaX = x - visual3dPreviewDrag.x;
    const deltaY = y - visual3dPreviewDrag.y;
    visual3dPreviewDrag.x = x;
    visual3dPreviewDrag.y = y;
    if (Math.hypot(x - visual3dPreviewDrag.startX, y - visual3dPreviewDrag.startY) > 4) {
      visual3dPreviewDrag.moved = true;
    }
    const camera = visual3dCamera();
    camera.yawDegrees = visual3dNormalizeDegrees(camera.yawDegrees + deltaX * 0.35);
    camera.pitchDegrees = visual3dClampNumber(
      camera.pitchDegrees - deltaY * 0.25,
      VISUAL3D_CAMERA_MIN_PITCH_DEGREES,
      VISUAL3D_CAMERA_MAX_PITCH_DEGREES,
    );
    renderVisual3dCameraControls();
    renderVisual3dPreview();
    return { forward: false };
  }
  if ((gesture === "release" || gesture === "leave") && visual3dPreviewDrag) {
    const select = gesture === "release" && !visual3dPreviewDrag.moved;
    visual3dPreviewDrag = null;
    visual3dPreviewHost?.classList.remove("is-dragging");
    return select
      ? { forward: true, gesture: "press", mutate: true }
      : { forward: gesture === "leave", gesture: "leave", mutate: false };
  }
  return {
    forward: gesture === "move" || gesture === "leave",
    gesture,
    mutate: false,
  };
}

function visual3dNormalizeDegrees(value) {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) {
    return 0;
  }
  return ((parsed % 360) + 360) % 360;
}

function visual3dSliceIndexForWorldIndex(index, axis = visual3d.axis) {
  const max = visual3dAxisSize(axis) - 1;
  const world = visual3dClamp(Math.trunc(Number(index) || 0), 0, max);
  return axis === "x" ? world : max - world;
}

async function applyVisual3dProjectionHit(hit, options = {}) {
  if (!hit) {
    if (!options.mutate) {
      clearVisual3dHoverSlice({ render: false });
    }
    return false;
  }
  if (hit.kind !== "slice") {
    return false;
  }
  const axis = ["x", "y", "z"].includes(hit.axis) ? hit.axis : visual3d.axis;
  if (axis !== visual3d.axis) {
    return false;
  }
  const slice = visual3dSliceIndexForWorldIndex(hit.index, axis);
  if (options.mutate === true) {
    return setVisual3dSlice(slice);
  }
  if (visual3d.hoverSlice !== slice) {
    visual3d.hoverSlice = slice;
    renderVisual3dPreview();
  }
  return false;
}

function clearVisual3dHoverSlice(options = {}) {
  if (visual3d.hoverSlice === null) {
    return;
  }
  visual3d.hoverSlice = null;
  if (options.render !== false) {
    renderVisual3dPreview();
  }
}

for (const input of [
  visual3dNameInput,
  visual3dWidthInput,
  visual3dHeightInput,
  visual3dDepthInput,
  visual3dScaleInput,
  visual3dSliceValue,
  visual3dAnimationDurationInput,
  visual3dAnimationFrameInput,
]) {
  installSelectAllOnFocus(input);
}
visual3dNameInput?.addEventListener("input", () => {
  renderVisual3dPreview();
  syncVisual3dSourceActionButtons();
});
sourceEditor.on("change", () => {
  invalidateVisual3dEditSourceForDocument(activeDocument());
  syncVisual3dSourceActionButtons();
});
function bindVisual3dDimensionInput(input, axis) {
  bindVisualEditorDimensionInput(input, axis, updateVisual3dDimension);
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
visual3dAnimationDurationInput?.addEventListener("keydown", (event) => {
  if (event.key !== "Enter") {
    return;
  }
  event.preventDefault();
  setVisual3dAnimationDuration(visual3dAnimationDurationInput.value);
});
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
visual3dSliceBoard?.addEventListener("contextmenu", (event) => event.preventDefault());
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
