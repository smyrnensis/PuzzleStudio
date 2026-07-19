let sprite3dActionClearTimer = 0;
let sprite3dPreviewDrag = null;
let sprite3dCameraScrubDrag = null;
let sprite3dSliceScrubDrag = null;
let sprite3dBucketActive = false;
let sprite3dTranslateActive = false;
let sprite3dTranslateDrag = null;
let sprite3dClipActive = false;
let sprite3dClipSelection = null;
let sprite3dClipDrag = null;
let sprite3dClipClipboard = null;
let sprite3dClipFloating = null;
let sprite3dGridVisible = true;
const SPRITE3D_EDITOR_MAX_SIZE = 64;
const SPRITE3D_ANIMATION_MAX_FRAMES = 24;
const SPRITE3D_SLICE_SCRUB_STEP_PX = 18;
const SPRITE3D_CAMERA_MIN_PITCH_DEGREES = -90;
const SPRITE3D_CAMERA_MAX_PITCH_DEGREES = 90;
const SPRITE3D_PREVIEW_BASE_ZOOM = 1;
const SPRITE3D_CAMERA_DEFAULT = {
  yawDegrees: 15,
  pitchDegrees: 30,
  zoom: 1,
};

function sprite3dFrameCellCount() {
  return sprite3d.width * sprite3d.height * sprite3d.depth;
}

function sprite3dAxisSize(axis = sprite3d.axis) {
  if (axis === "x") return sprite3d.width;
  if (axis === "y") return sprite3d.height;
  return sprite3d.depth;
}

function sprite3dPlaneSize(axis = sprite3d.axis) {
  if (axis === "x") return { width: sprite3d.height, height: sprite3d.depth };
  if (axis === "y") return { width: sprite3d.width, height: sprite3d.depth };
  return { width: sprite3d.width, height: sprite3d.height };
}

function normalizedSprite3dAnimationDuration(value = sprite3d.animationDurationMs) {
  return Math.max(20, Math.min(5000, Math.trunc(Number(value) || 120)));
}

function normalizeSprite3dFrameCells(cells) {
  const count = sprite3dFrameCellCount();
  return Array.from({ length: count }, (_, index) => (
    validSprite3dColorIndex(cells?.[index]) ? cells[index] : null
  ));
}

function ensureSprite3dAnimationState() {
  if (!Array.isArray(sprite3d.frames) || !sprite3d.frames.length) {
    sprite3d.frames = [normalizeSprite3dFrameCells(sprite3d.cells)];
  }
  sprite3d.animationFrameCount = Math.max(
    1,
    Math.min(SPRITE3D_ANIMATION_MAX_FRAMES, Math.trunc(Number(sprite3d.animationFrameCount) || sprite3d.frames.length || 1)),
  );
  while (sprite3d.frames.length < sprite3d.animationFrameCount) {
    sprite3d.frames.push(normalizeSprite3dFrameCells(sprite3d.frames[sprite3d.frames.length - 1]));
  }
  sprite3d.frames.length = sprite3d.animationFrameCount;
  sprite3d.frames = sprite3d.frames.map(normalizeSprite3dFrameCells);
  sprite3d.animationFrameIndex = Math.max(0, Math.min(
    sprite3d.animationFrameCount - 1,
    Math.trunc(Number(sprite3d.animationFrameIndex) || 0),
  ));
  sprite3d.animationPlaybackIndex = Math.max(0, Math.min(
    sprite3d.animationFrameCount - 1,
    Math.trunc(Number(sprite3d.animationPlaybackIndex) || 0),
  ));
  sprite3d.animationDurationMs = normalizedSprite3dAnimationDuration();
  if (sprite3d.animationMode) {
    sprite3d.cells = sprite3d.frames[sprite3d.animationFrameIndex];
  }
}

function commitSprite3dActiveFrame() {
  if (!sprite3d.animationMode) {
    return;
  }
  const cells = sprite3d.cells;
  const frameIndex = Math.max(0, Math.trunc(Number(sprite3d.animationFrameIndex) || 0));
  ensureSprite3dAnimationState();
  sprite3d.animationFrameIndex = Math.min(frameIndex, sprite3d.animationFrameCount - 1);
  sprite3d.frames[sprite3d.animationFrameIndex] = normalizeSprite3dFrameCells(cells);
  sprite3d.cells = sprite3d.frames[sprite3d.animationFrameIndex];
}

function setSprite3dAnimationMode(enabled, options = {}) {
  sprite3d.animationMode = Boolean(enabled);
  if (sprite3d.animationMode) {
    ensureSprite3dAnimationState();
  } else {
    sprite3d.animationPlaying = false;
  }
  if (options.render !== false) {
    renderSprite3dBuilder();
  }
  syncPreviewModeButtonState();
}

function setSprite3dAnimationFrame(index) {
  selectSharedSpriteAnimationFrame("sprite3d", index);
}

function setSprite3dAnimationFrameCount(value) {
  const before = visualEditSnapshot("sprite3d");
  commitSprite3dActiveFrame();
  sprite3d.animationFrameCount = Math.max(1, Math.min(
    SPRITE3D_ANIMATION_MAX_FRAMES,
    Math.trunc(Number(value) || 1),
  ));
  ensureSprite3dAnimationState();
  sprite3d.animationFrameIndex = Math.min(sprite3d.animationFrameIndex, sprite3d.animationFrameCount - 1);
  sprite3d.cells = sprite3d.frames[sprite3d.animationFrameIndex];
  renderSprite3dBuilder();
  pushVisualEditUndoSnapshot("sprite3d", before);
}

function moveSprite3dAnimationFrame(delta) {
  moveSharedSpriteAnimationFrame("sprite3d", delta);
}

function insertSprite3dAnimationFrameAt(index) {
  return insertSharedSpriteAnimationFrameAt("sprite3d", index);
}

function removeSprite3dAnimationFrameAt(index) {
  return removeSharedSpriteAnimationFrameAt("sprite3d", index);
}

function setSprite3dAnimationDuration(value) {
  const before = visualEditSnapshot("sprite3d");
  sprite3d.animationDurationMs = normalizedSprite3dAnimationDuration(value);
  renderSprite3dControls();
  pushVisualEditUndoSnapshot("sprite3d", before);
}

function resetSprite3dBuilder(
  width = sprite3d.width,
  height = sprite3d.height,
  depth = sprite3d.depth,
) {
  resetSprite3dClipState({ clipboard: true });
  ensureSprite3dPalette();
  sprite3d.width = clampSprite3dSize(width);
  sprite3d.height = clampSprite3dSize(height);
  sprite3d.depth = clampSprite3dSize(depth);
  sprite3d.slice = Math.max(0, Math.min(sprite3dAxisSize() - 1, Number(sprite3d.slice) || 0));
  sprite3d.hoverSlice = null;
  sprite3d.cells = Array.from({ length: sprite3dFrameCellCount() }, () => null);
  sprite3d.frames = [sprite3d.cells];
  sprite3d.animationFrameIndex = 0;
  sprite3d.animationFrameCount = 1;
  sprite3d.animationPlaybackIndex = 0;
  sprite3d.sourceSpatialOps = [];
  if (!validSprite3dColorIndex(sprite3d.selectedColorIndex)) {
    sprite3d.selectedColorIndex = 0;
  }
  renderSprite3dBuilder();
}

function clampSprite3dSize(value) {
  const size = Math.trunc(Number(value) || 5);
  return Math.max(1, Math.min(SPRITE3D_EDITOR_MAX_SIZE, size));
}

function withSprite3dPaneScrollPreserved(render) {
  return withSpritePaneScrollPreserved(sprite3dBuilder, render);
}

function renderSprite3dBuilder() {
  if (!sprite3dBuilder || !sprite3dSliceBoard || !sprite3dPalette || !sprite3dPreviewCanvas) {
    return;
  }
  withSprite3dPaneScrollPreserved(() => {
    mountSharedSpriteAnimationUi("3d");
    commitSprite3dActiveFrame();
    sprite3dBuilder.classList.toggle("is-animation-mode", Boolean(sprite3d.animationMode));
    renderSprite3dControls();
    renderSprite3dPalette();
    renderSprite3dSliceBoard();
    renderSprite3dPreview();
    renderSprite3dAnimationFrameStrip();
    syncSpriteAnimationPlayback();
    syncSprite3dSourceActionButtons();
  });
}

function sprite3dAnimationFramePreview(frame) {
  const canvas = document.createElement("canvas");
  canvas.className = "sprite-animation-3d-preview";
  canvas.width = 52;
  canvas.height = 52;
  canvas.setAttribute("aria-hidden", "true");
  renderSprite3dPreviewCanvas(canvas, frame, { overlays: false });
  return [canvas];
}

function renderSprite3dAnimationFrameStrip() {
  if (!sprite3dAnimationFrameStrip || !sprite3d.animationMode) {
    return;
  }
  ensureSprite3dAnimationState();
  const plane = sprite3dPlaneSize();
  const showInsertTargets = spriteAnimationInsertMode && sprite3d.animationFrameCount < SPRITE3D_ANIMATION_MAX_FRAMES;
  const showRemoveTargets = spriteAnimationRemoveMode && sprite3d.animationFrameCount > 1;
  renderSpriteAnimationFrameStripView({
    target: sprite3dAnimationFrameStrip,
    frameCount: sprite3d.animationFrameCount,
    activeIndex: sprite3d.animationFrameIndex,
    playingIndex: sprite3d.animationPlaybackIndex,
    size: Math.max(plane.width, plane.height),
    showInsertTargets,
    showRemoveTargets,
    renderCells: (index) => sprite3dAnimationFramePreview(sprite3d.frames[index]),
    onSelect: setSprite3dAnimationFrame,
    onRemove: removeSprite3dAnimationFrameAt,
    renderInsertTarget: (index) => spriteAnimationInsertTargetButton(index, insertSprite3dAnimationFrameAt, "3D sprite animation"),
    noun: "3D sprite animation",
  });
}

function renderSprite3dControls() {
  withSprite3dPaneScrollPreserved(() => {
    renderSpriteEditorUpperControls(
      sprite3dBuilder.querySelector(".sprite-controls"),
      spriteEditorUpperControls3d(),
    );
    sprite3dNameInput.value = sprite3dNameInput.value || "VoxelSprite";
    renderSpriteShapeBindControl(sprite3dShapeField, {
      state: sprite3d,
      render: renderSprite3dControls,
      onChange: () => {
        syncSprite3dSourceActionButtons();
        renderSprite3dBuilder();
      },
    });
    if (sprite3d.animationMode) {
      ensureSprite3dAnimationState();
    }
    sprite3dWidthInput.value = String(sprite3d.width);
    sprite3dHeightInput.value = String(sprite3d.height);
    sprite3dDepthInput.value = String(sprite3d.depth);
    syncSprite3dBucketButton();
    syncSprite3dTranslateButton();
    syncSpriteMarkerControl();
    syncSprite3dGridButton();
    renderSprite3dClipActions();
    renderSprite3dScopeControl();
    renderSprite3dEditorToolbar();
    renderSprite3dCameraControls();
    renderSpriteScaleControl({
      size: Math.max(sprite3d.width, sprite3d.height, sprite3d.depth),
      maxSize: SPRITE3D_EDITOR_MAX_SIZE,
      scaleInput: sprite3dScaleInput,
      scaleUpButton: sprite3dScaleUpButton,
      scaleDownButton: sprite3dScaleDownButton,
      canScaleDown: canScaleDownSprite3d,
      noun: "3D sprite",
    });
    if (sprite3dSliceValue instanceof HTMLInputElement) {
      sprite3dSliceValue.min = "1";
      sprite3dSliceValue.max = String(sprite3dAxisSize());
      sprite3dSliceValue.value = String(sprite3d.slice + 1);
    } else if (sprite3dSliceValue) {
      sprite3dSliceValue.textContent = `${sprite3d.slice + 1} / ${sprite3dAxisSize()}`;
    }
    if (sprite3dAnimationDurationInput) {
      sprite3dAnimationDurationInput.value = String(normalizedSprite3dAnimationDuration());
    }
    if (sprite3dAnimationFrameCountInput) {
      sprite3dAnimationFrameCountInput.value = String(sprite3d.animationFrameCount || 1);
    }
    if (sprite3dAnimationFrameInput) {
      sprite3dAnimationFrameInput.value = String((sprite3d.animationFrameIndex || 0) + 1);
      sprite3dAnimationFrameInput.max = String(sprite3d.animationFrameCount || 1);
    }
    if (sprite3dAnimationFrameTotal) {
      sprite3dAnimationFrameTotal.textContent = String(sprite3d.animationFrameCount || 1);
    }
    syncSharedSpriteAnimationToolbarState(sprite3d.animationFrameCount || 1, SPRITE3D_ANIMATION_MAX_FRAMES);
    const sliceTotal = document.querySelector("#sprite3dSliceTotal");
    if (sliceTotal) {
      sliceTotal.textContent = String(sprite3dAxisSize());
    }
    if (sprite3dPreviousSliceButton) {
      sprite3dPreviousSliceButton.disabled = sprite3d.slice <= 0;
      sprite3dPreviousSliceButton.dataset.tooltip = "Previous slice";
      setEditorShortcutHint(sprite3dPreviousSliceButton, { key: "[" });
    }
    if (sprite3dNextSliceButton) {
      sprite3dNextSliceButton.disabled = sprite3d.slice >= sprite3dAxisSize() - 1;
      sprite3dNextSliceButton.dataset.tooltip = "Next slice";
      setEditorShortcutHint(sprite3dNextSliceButton, { key: "]" });
    }
    for (const button of sprite3dAxisButtons) {
      const active = button.dataset.sprite3dAxis === sprite3d.axis;
      button.classList.toggle("is-active", active);
      button.setAttribute("aria-pressed", String(active));
      button.dataset.tooltip = `${button.dataset.sprite3dAxis.toUpperCase()} axis`;
      setEditorShortcutHint(button, { key: button.dataset.sprite3dAxis });
    }
  });
}

function renderSprite3dEditorToolbar() {
  renderSpriteEditorToolbar({ dimension: "3d", target: sprite3dToolbarHost });
}

function selectSprite3dBrushSize(size) {
  spriteBrushSizePx = normalizeSpriteBrushSize(size);
  sprite3dBucketActive = false;
  sprite3dTranslateActive = false;
  deactivateSprite3dClipMode({ render: false });
  syncSpriteMarkerControl();
  renderSprite3dBuilder();
  setSprite3dActionStatus(`Brush: ${spriteBrushSizePx}px`, "is-ok");
}

function syncSprite3dGridButton() {
  if (!spriteGridButton) {
    return;
  }
  spriteGridButton.classList.toggle("is-active", sprite3dGridVisible);
  spriteGridButton.setAttribute("aria-pressed", String(sprite3dGridVisible));
  spriteGridButton.title = "Toggle grid";
  spriteGridButton.setAttribute("aria-label", "Toggle 3D sprite slice grid");
}

function toggleSprite3dGrid() {
  sprite3dGridVisible = !sprite3dGridVisible;
  syncSprite3dGridButton();
  renderSprite3dSliceBoard();
  renderSprite3dPresentationSurfaces();
  setSprite3dActionStatus(sprite3dGridVisible ? "3D sprite grid visible" : "3D sprite grid hidden", "is-ok");
}

function sprite3dEditScope() {
  if (sprite3d.editScope !== "all") {
    sprite3d.editScope = "slice";
  }
  return sprite3d.editScope;
}

function renderSprite3dScopeControl() {
  const scope = sprite3dEditScope();
  const buttons = [
    {
      button: sprite3dScopeSliceButton,
      scope: "slice",
      label: "Scope 2D",
      title: "Scope 2D slice",
    },
    {
      button: sprite3dScopeAllButton,
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
  updateSprite3dScopedActionLabels();
}

function updateSprite3dScopedActionLabels() {
  const isAll = sprite3dEditScope() === "all";
  const target = isAll ? "whole sprite" : "current slice";
  setSprite3dButtonLabel(sprite3dRotatePlaneLeftButton, `Rotate ${target} CCW`);
  setSprite3dButtonLabel(sprite3dRotatePlaneRightButton, `Rotate ${target} CW`);
  setSprite3dButtonLabel(sprite3dFlipPlaneHorizontalButton, `Flip ${target} horizontally`);
  setSprite3dButtonLabel(sprite3dFlipPlaneVerticalButton, `Flip ${target} vertically`);
  setSprite3dButtonLabel(sprite3dFillButton, "Fill");
  sprite3dFillButton.dataset.tooltip = "Fill";
  setEditorShortcutHint(sprite3dFillButton, { key: "f" });
  syncSpriteEditCommandLabels("3d");
  renderSprite3dClipActions();
  syncSprite3dTranslateButton();
}

function syncSprite3dTranslateButton() {
  if (!sprite3dTranslateButton) {
    return;
  }
  sprite3dTranslateButton.classList.toggle("is-active", sprite3dTranslateActive);
  sprite3dTranslateButton.setAttribute("aria-pressed", String(sprite3dTranslateActive));
  sprite3dTranslateButton.setAttribute("aria-label", "Move");
  sprite3dTranslateButton.title = "Move";
  sprite3dTranslateButton.dataset.tooltip = "Move";
  setEditorShortcutHint(sprite3dTranslateButton, { key: "m" });
}

function renderSprite3dClipActions() {
  if (!sprite3dClipActions) {
    return;
  }
  const actions = document.createElement("span");
  actions.className = "sprite-clip-actions";
  const button = renderSpriteClipButton({
    title: "Clip",
    ariaLabel: "Clip",
    active: sprite3dClipActive,
    onClick: toggleSprite3dClipMode,
    icon: spriteLucideIconSvg("mouse-pointer-2"),
  });
  button.dataset.tooltip = "Clip";
  setEditorShortcutHint(button, { key: "c" });
  actions.append(button);
  sprite3dClipActions.replaceChildren(actions);
}

function toggleSprite3dClipMode() {
  if (sprite3dClipActive) {
    deactivateSprite3dClipMode();
    setSprite3dActionStatus("Brush: paint individual voxels", "is-ok");
    return;
  }
  sprite3dBucketActive = false;
  sprite3dTranslateActive = false;
  sprite3dClipActive = true;
  sprite3dClipDrag = null;
  renderSprite3dBuilder();
  setSprite3dActionStatus(
    sprite3dClipSelection ? "Clip: drag selection to move it" : "Clip: drag to select an area",
    "is-ok",
  );
}

function deactivateSprite3dClipMode(options = {}) {
  const wasActive = sprite3dClipActive || sprite3dClipSelection || sprite3dClipDrag || sprite3dClipFloating;
  sprite3dClipActive = false;
  sprite3dClipDrag = null;
  sprite3dClipFloating = null;
  if (options.clearSelection !== false) {
    sprite3dClipSelection = null;
  }
  if (options.render === false || !wasActive) {
    return;
  }
  renderSprite3dBuilder();
}

function resetSprite3dClipState(options = {}) {
  sprite3dClipActive = false;
  sprite3dClipSelection = null;
  sprite3dClipDrag = null;
  sprite3dClipFloating = null;
  if (options.clipboard) {
    sprite3dClipClipboard = null;
  }
}

function normalizeSprite3dClipBox(box) {
  if (!box) {
    return null;
  }
  const next = {};
  for (const axis of ["x", "y", "z"]) {
    const min = Math.trunc(Number(box[`min${axis.toUpperCase()}`]));
    const max = Math.trunc(Number(box[`max${axis.toUpperCase()}`]));
    const limit = sprite3dAxisSize(axis);
    if (!Number.isInteger(min) || !Number.isInteger(max) || min < 0 || max < min || max >= limit) {
      return null;
    }
    next[`min${axis.toUpperCase()}`] = min;
    next[`max${axis.toUpperCase()}`] = max;
  }
  return next;
}

function sprite3dClipBoxDimensions(box = sprite3dClipSelection) {
  const normalized = normalizeSprite3dClipBox(box);
  return normalized ? {
    width: normalized.maxX - normalized.minX + 1,
    height: normalized.maxY - normalized.minY + 1,
    depth: normalized.maxZ - normalized.minZ + 1,
  } : null;
}

function sprite3dClipBoxContainsCoords(box, coords) {
  const normalized = normalizeSprite3dClipBox(box);
  return Boolean(normalized && coords
    && coords.x >= normalized.minX && coords.x <= normalized.maxX
    && coords.y >= normalized.minY && coords.y <= normalized.maxY
    && coords.z >= normalized.minZ && coords.z <= normalized.maxZ);
}

function sprite3dClipPlaneRect(box = sprite3dClipSelection, axis = sprite3d.axis) {
  const normalized = normalizeSprite3dClipBox(box);
  if (!normalized) {
    return null;
  }
  const corners = [];
  for (const x of [normalized.minX, normalized.maxX]) {
    for (const y of [normalized.minY, normalized.maxY]) {
      for (const z of [normalized.minZ, normalized.maxZ]) {
        corners.push(sprite3dPlaneCoordinates(axis, x, y, z));
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

function sprite3dClipRectFromCells(start, end) {
  return {
    x: Math.min(start.x, end.x),
    y: Math.min(start.y, end.y),
    width: Math.abs(end.x - start.x) + 1,
    height: Math.abs(end.y - start.y) + 1,
  };
}

function sprite3dClipBoxFromPlaneRect(rect, options = {}) {
  if (!rect) {
    return null;
  }
  const existing = normalizeSprite3dClipBox(options.base);
  const fullDepth = options.fullDepth === true;
  const fixedStack = sprite3dPlaneWorldSlice(sprite3d.axis, sprite3d.slice);
  const points = [
    sprite3dCoordsFromPlane(sprite3d.axis, sprite3d.slice, rect.x, rect.y),
    sprite3dCoordsFromPlane(sprite3d.axis, sprite3d.slice, rect.x + rect.width - 1, rect.y + rect.height - 1),
  ];
  const box = existing || {
    minX: 0, maxX: sprite3d.width - 1,
    minY: 0, maxY: sprite3d.height - 1,
    minZ: 0, maxZ: sprite3d.depth - 1,
  };
  for (const worldAxis of ["x", "y", "z"]) {
    if (worldAxis === sprite3d.axis) {
      if (!existing) {
        box[`min${worldAxis.toUpperCase()}`] = fullDepth ? 0 : fixedStack;
        box[`max${worldAxis.toUpperCase()}`] = fullDepth ? sprite3dAxisSize(worldAxis) - 1 : fixedStack;
      }
      continue;
    }
    const values = points.map((point) => point[worldAxis]);
    box[`min${worldAxis.toUpperCase()}`] = Math.min(...values);
    box[`max${worldAxis.toUpperCase()}`] = Math.max(...values);
  }
  return normalizeSprite3dClipBox(box);
}

function sprite3dClipSelectionContainsSliceCell(cell) {
  const rect = sprite3dClipPlaneRect();
  if (!rect || !cell) {
    return false;
  }
  if (sprite3dEditScope() === "slice") {
    const fixed = sprite3dPlaneWorldSlice(sprite3d.axis, sprite3d.slice);
    if (fixed < sprite3dClipSelection[`min${sprite3d.axis.toUpperCase()}`]
      || fixed > sprite3dClipSelection[`max${sprite3d.axis.toUpperCase()}`]) {
      return false;
    }
  }
  return cell.x >= rect.x && cell.x < rect.x + rect.width && cell.y >= rect.y && cell.y < rect.y + rect.height;
}

function sprite3dClipCellFromClient(clientX, clientY, geometry) {
  if (!geometry || geometry.width <= 0 || geometry.height <= 0) {
    return null;
  }
  const plane = sprite3dPlaneSize();
  return {
    x: Math.max(0, Math.min(plane.width - 1, Math.floor(((clientX - geometry.left) / geometry.width) * plane.width))),
    y: Math.max(0, Math.min(plane.height - 1, Math.floor(((clientY - geometry.top) / geometry.height) * plane.height))),
  };
}

function sprite3dClipCells(box) {
  const normalized = normalizeSprite3dClipBox(box);
  if (!normalized) {
    return [];
  }
  const cells = [];
  for (let z = normalized.minZ; z <= normalized.maxZ; z += 1) {
    for (let y = normalized.minY; y <= normalized.maxY; y += 1) {
      for (let x = normalized.minX; x <= normalized.maxX; x += 1) {
        const value = sprite3d.cells[sprite3dCellIndex(x, y, z)];
        cells.push(validSprite3dColorIndex(value) ? value : null);
      }
    }
  }
  return cells;
}

function sprite3dSliceClipCells(rect = sprite3dClipPlaneRect()) {
  if (!rect) {
    return [];
  }
  const cells = [];
  for (let v = rect.y; v < rect.y + rect.height; v += 1) {
    for (let u = rect.x; u < rect.x + rect.width; u += 1) {
      const coords = sprite3dCoordsFromPlane(sprite3d.axis, sprite3d.slice, u, v);
      const value = sprite3d.cells[sprite3dCellIndex(coords.x, coords.y, coords.z)];
      cells.push(validSprite3dColorIndex(value) ? value : null);
    }
  }
  return cells;
}

function sprite3dClipClipboardFromSelection(box, dimensions) {
  if (sprite3dEditScope() === "slice") {
    const rect = sprite3dClipPlaneRect(box);
    return { dimension: "3d", scope: "slice", width: rect.width, height: rect.height, depth: 1,
      cells: sprite3dSliceClipCells(rect), colors: sprite3dPaletteColors() };
  }
  return { dimension: "3d", scope: "all", ...dimensions, cells: sprite3dClipCells(box), colors: sprite3dPaletteColors() };
}

function pasteSprite3dClipCell(index, clipboardValue) {
  if (clipboardValue === null) {
    return false;
  }
  if (!validSprite3dColorIndex(clipboardValue)) {
    throw new Error(`Invalid 3D clip palette index ${clipboardValue}`);
  }
  if (sprite3d.cells[index] === clipboardValue) {
    return false;
  }
  sprite3d.cells[index] = clipboardValue;
  return true;
}

function sprite3dClipForCurrentPalette(clipboard) {
  if (!Array.isArray(clipboard?.colors)) return clipboard;
  const palette = sprite3dPaletteEntries();
  const colorToIndex = new Map(palette.map((entry, index) => [normalizeSpriteColor(entry.color), index]));
  const sourceToTarget = clipboard.colors.map((rawColor) => {
    const color = normalizeSpriteColor(rawColor);
    if (color === "#00000000") return null;
    if (!colorToIndex.has(color)) {
      if (palette.length >= SPRITE_COLOR_TOKENS.length) {
        throw new Error("Paste needs more colors than the 3D sprite palette can hold");
      }
      colorToIndex.set(color, palette.length);
      palette.push({ color });
    }
    return colorToIndex.get(color);
  });
  return { ...clipboard, cells: clipboard.cells.map((value) => value === null ? null : sourceToTarget[value]) };
}

function setSprite3dClipCells(box, clipboard) {
  const normalized = normalizeSprite3dClipBox(box);
  const dimensions = sprite3dClipBoxDimensions(normalized);
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
        const index = sprite3dCellIndex(x, y, z);
        if (pasteSprite3dClipCell(index, clipboard.cells[offset])) {
          changed = true;
        }
        offset += 1;
      }
    }
  }
  return changed;
}

function setSprite3dSliceClipCells(rect, clipboard) {
  if (!rect || !clipboard || clipboard.scope !== "slice"
    || rect.width !== clipboard.width || rect.height !== clipboard.height
    || clipboard.cells.length !== rect.width * rect.height) {
    return false;
  }
  let changed = false;
  let offset = 0;
  for (let v = rect.y; v < rect.y + rect.height; v += 1) {
    for (let u = rect.x; u < rect.x + rect.width; u += 1) {
      const coords = sprite3dCoordsFromPlane(sprite3d.axis, sprite3d.slice, u, v);
      const index = sprite3dCellIndex(coords.x, coords.y, coords.z);
      if (pasteSprite3dClipCell(index, clipboard.cells[offset])) {
        changed = true;
      }
      offset += 1;
    }
  }
  return changed;
}

function clearSprite3dClipBox(box) {
  const normalized = normalizeSprite3dClipBox(box);
  if (!normalized) {
    return false;
  }
  let changed = false;
  for (let z = normalized.minZ; z <= normalized.maxZ; z += 1) {
    for (let y = normalized.minY; y <= normalized.maxY; y += 1) {
      for (let x = normalized.minX; x <= normalized.maxX; x += 1) {
        const index = sprite3dCellIndex(x, y, z);
        if (sprite3d.cells[index] !== null) {
          sprite3d.cells[index] = null;
          changed = true;
        }
      }
    }
  }
  return changed;
}

function commitSprite3dClipMutation(before, changed, message) {
  renderSprite3dBuilder();
  if (!changed) {
    setSprite3dActionStatus("Clip did not change 3D sprite", "is-ok");
    return false;
  }
  syncSprite3dSourceActionButtons();
  setSprite3dActionStatus(message, "is-ok");
  setStatus(message, "is-ok");
  pushVisualEditUndoSnapshot("sprite3d", before);
  return true;
}

function deleteSprite3dClipSelection() {
  if (sprite3dClipFloating) {
    sprite3dClipFloating = null;
    sprite3dClipSelection = null;
    sprite3dClipDrag = null;
    renderSprite3dBuilder();
    setSprite3dActionStatus("Clip preview discarded", "is-ok");
    return true;
  }
  const box = normalizeSprite3dClipBox(sprite3dClipSelection);
  if (!box) {
    setSprite3dActionStatus("No clip selection", "is-error");
    return false;
  }
  const before = visualEditSnapshot("sprite3d");
  return commitSprite3dClipMutation(before, clearSprite3dClipBox(box), "Deleted selected 3D area");
}

function pasteSprite3dClipClipboard() {
  if (!sprite3dClipClipboard) {
    setSprite3dActionStatus("No copied clip", "is-error");
    return false;
  }
  const before = visualEditSnapshot("sprite3d");
  let clipboard;
  try {
    clipboard = sprite3dClipForCurrentPalette(sprite3dClipClipboard);
  } catch (error) {
    setSprite3dActionStatus(error?.message || String(error), "is-error");
    return false;
  }
  if (clipboard.scope === "slice") {
    const baseRect = sprite3dClipPlaneRect() || { x: 0, y: 0, width: 1, height: 1 };
    const rect = {
      x: baseRect.x,
      y: baseRect.y,
      width: clipboard.width,
      height: clipboard.height,
    };
    const plane = sprite3dPlaneSize();
    if (rect.x + rect.width > plane.width || rect.y + rect.height > plane.height) {
      setSprite3dActionStatus("Copied slice clip does not fit at selection", "is-error");
      return false;
    }
    const target = sprite3dClipBoxFromPlaneRect(rect, { fullDepth: false });
    const changed = setSprite3dSliceClipCells(rect, clipboard);
    sprite3dClipSelection = target;
    sprite3dClipFloating = null;
    commitSprite3dClipMutation(before, changed, `Pasted ${rect.width}x${rect.height} slice clip`);
    return true;
  }
  const base = normalizeSprite3dClipBox(sprite3dClipSelection) || {
    minX: 0, maxX: 0, minY: 0, maxY: 0, minZ: 0, maxZ: 0,
  };
  const target = normalizeSprite3dClipBox({
    minX: base.minX,
    maxX: base.minX + clipboard.width - 1,
    minY: base.minY,
    maxY: base.minY + clipboard.height - 1,
    minZ: base.minZ,
    maxZ: base.minZ + clipboard.depth - 1,
  });
  if (!target) {
    setSprite3dActionStatus("Copied clip does not fit at selection", "is-error");
    return false;
  }
  const changed = setSprite3dClipCells(target, clipboard);
  sprite3dClipSelection = target;
  sprite3dClipFloating = null;
  const dimensions = sprite3dClipBoxDimensions(target);
  commitSprite3dClipMutation(before, changed, `Pasted ${dimensions.width}x${dimensions.height}x${dimensions.depth} clip`);
  return true;
}

function sprite3dWholeEditBox() {
  if (sprite3dEditScope() === "slice") {
    const plane = sprite3dPlaneSize();
    return sprite3dClipBoxFromPlaneRect({ x: 0, y: 0, width: plane.width, height: plane.height }, { fullDepth: false });
  }
  return { minX: 0, maxX: sprite3d.width - 1, minY: 0, maxY: sprite3d.height - 1,
    minZ: 0, maxZ: sprite3d.depth - 1 };
}

function sprite3dEditBox() {
  return sprite3dClipActive ? normalizeSprite3dClipBox(sprite3dClipSelection) : sprite3dWholeEditBox();
}

function sprite3dClipboardSourceText(clipboard) {
  const rows = [];
  for (let z = 0; z < clipboard.depth; z += 1) {
    if (z > 0) rows.push("-");
    for (let y = 0; y < clipboard.height; y += 1) {
      const offset = (z * clipboard.height + y) * clipboard.width;
      rows.push(clipboard.cells.slice(offset, offset + clipboard.width)
        .map((value) => validSprite3dColorIndex(value) ? SPRITE_COLOR_TOKENS[value] : ".").join(""));
    }
  }
  return [`colors = ${sprite3dPaletteSourceTokens().join(" ")}`, "shape = {", ...rows, "}"].join("\n");
}

async function copySprite3dEditRegion() {
  const box = sprite3dEditBox();
  const dimensions = sprite3dClipBoxDimensions(box);
  if (!box || !dimensions) return false;
  sprite3dClipClipboard = sprite3dClipClipboardFromSelection(box, dimensions);
  try {
    await copyTextToClipboard(sprite3dClipboardSourceText(sprite3dClipClipboard));
  } catch (error) {
    setSprite3dActionStatus(`Copy failed: ${error?.message || error}`, "is-error");
    return false;
  }
  renderSprite3dBuilder();
  setSprite3dActionStatus(`Copied ${dimensions.width}x${dimensions.height}x${dimensions.depth} edit region`, "is-ok");
  return true;
}

async function cutSprite3dEditRegion() {
  const box = sprite3dEditBox();
  if (!box) return false;
  try {
    if (!await copySprite3dEditRegion()) return false;
  } catch (error) {
    setSprite3dActionStatus(`Copy failed; 3D sprite was not cut: ${error?.message || error}`, "is-error");
    return false;
  }
  const before = visualEditSnapshot("sprite3d");
  return commitSprite3dClipMutation(before, clearSprite3dClipBox(box), "Cut 3D edit region");
}

function pasteSprite3dEditRegion() {
  if (!sprite3dClipClipboard) {
    setSprite3dActionStatus("No copied 3D sprite region", "is-error");
    return false;
  }
  const previousSelection = sprite3dClipSelection;
  if (!sprite3dClipActive) sprite3dClipSelection = sprite3dWholeEditBox();
  const result = pasteSprite3dClipClipboard();
  if (!sprite3dClipActive) sprite3dClipSelection = previousSelection;
  return result;
}

function deleteSprite3dEditRegion() {
  if (!sprite3dClipActive) {
    deleteSprite3dScoped();
    return true;
  }
  return deleteSprite3dClipSelection();
}

function runSprite3dEditCommand(command) {
  if (sprite3dClipActive && !normalizeSprite3dClipBox(sprite3dClipSelection)) {
    setSprite3dActionStatus("Select a clip region first", "is-error");
    return false;
  }
  if (command === "copy") return copySprite3dEditRegion();
  if (command === "cut") return cutSprite3dEditRegion();
  if (command === "paste") return pasteSprite3dEditRegion();
  if (command === "delete") return deleteSprite3dEditRegion();
  throw new Error(`Unknown 3D sprite edit command ${command}`);
}

function sprite3dClipBoxShiftedInPlane(box, du, dv) {
  const rect = sprite3dClipPlaneRect(box);
  if (!rect) {
    return null;
  }
  const targetRect = { ...rect, x: rect.x + du, y: rect.y + dv };
  if (targetRect.x < 0 || targetRect.y < 0
    || targetRect.x + targetRect.width > sprite3dPlaneSize().width
    || targetRect.y + targetRect.height > sprite3dPlaneSize().height) {
    return null;
  }
  return sprite3dClipBoxFromPlaneRect(targetRect, { base: box });
}

function sprite3dClipResizeRect(origin, edge, cell) {
  if (!origin || !edge || !cell) {
    return null;
  }
  let left = origin.x;
  let right = origin.x + origin.width - 1;
  let top = origin.y;
  let bottom = origin.y + origin.height - 1;
  if (edge.includes("w")) left = Math.max(0, Math.min(cell.x, right));
  const plane = sprite3dPlaneSize();
  if (edge.includes("e")) right = Math.min(plane.width - 1, Math.max(cell.x, left));
  if (edge.includes("n")) top = Math.max(0, Math.min(cell.y, bottom));
  if (edge.includes("s")) bottom = Math.min(plane.height - 1, Math.max(cell.y, top));
  return { x: left, y: top, width: right - left + 1, height: bottom - top + 1 };
}

function toggleSprite3dTranslateMode() {
  if (sprite3dTranslateActive) {
    deactivateSprite3dTranslateMode();
    return;
  }
  sprite3dBucketActive = false;
  deactivateSprite3dClipMode({ render: false });
  sprite3dTranslateActive = true;
  sprite3dTranslateDrag = null;
  renderSprite3dBuilder();
  setSprite3dActionStatus(
    sprite3dEditScope() === "all" ? "Translate: drag the whole sprite" : "Translate: drag the current slice",
    "is-ok",
  );
}

function deactivateSprite3dTranslateMode(options = {}) {
  const wasActive = sprite3dTranslateActive || sprite3dTranslateDrag;
  if (sprite3dTranslateDrag && sprite3dSliceBoard.hasPointerCapture?.(sprite3dTranslateDrag.pointerId)) {
    sprite3dSliceBoard.releasePointerCapture(sprite3dTranslateDrag.pointerId);
  }
  sprite3dTranslateActive = false;
  sprite3dTranslateDrag = null;
  if (options.render === false || !wasActive) {
    return;
  }
  renderSprite3dBuilder();
  setSprite3dActionStatus("Brush: paint individual voxels", "is-ok");
}

function sprite3dPositiveModulo(value, size) {
  return ((value % size) + size) % size;
}

function translatedSprite3dCells(originCells, du, dv, scope) {
  const plane = sprite3dPlaneSize();
  const next = scope === "all"
    ? Array.from({ length: sprite3dFrameCellCount() }, () => null)
    : [...originCells];
  const firstStack = scope === "all" ? 0 : sprite3d.slice;
  const lastStack = scope === "all" ? sprite3dAxisSize() - 1 : sprite3d.slice;
  for (let stack = firstStack; stack <= lastStack; stack += 1) {
    for (let v = 0; v < plane.height; v += 1) {
      for (let u = 0; u < plane.width; u += 1) {
        const source = sprite3dCoordsFromPlane(sprite3d.axis, stack, u, v);
        const target = sprite3dCoordsFromPlane(
          sprite3d.axis,
          stack,
          sprite3dPositiveModulo(u + du, plane.width),
          sprite3dPositiveModulo(v + dv, plane.height),
        );
        next[sprite3dCellIndex(target.x, target.y, target.z)] = originCells[sprite3dCellIndex(source.x, source.y, source.z)];
      }
    }
  }
  return next;
}

function sprite3dCellsEqual(left, right) {
  return left.length === right.length && left.every((value, index) => value === right[index]);
}

function startSprite3dTranslate(event) {
  event.preventDefault();
  const rect = sprite3dSliceBoard.getBoundingClientRect();
  sprite3dTranslateDrag = {
    pointerId: event.pointerId,
    startClientX: event.clientX,
    startClientY: event.clientY,
    width: rect.width,
    height: rect.height,
    scope: sprite3dEditScope(),
    originCells: [...sprite3d.cells],
    beforeSnapshot: visualEditSnapshot("sprite3d"),
  };
  sprite3dSliceBoard.setPointerCapture?.(event.pointerId);
  sprite3dSliceBoard.classList.add("is-translating");
}

function continueSprite3dTranslate(event) {
  if (!sprite3dTranslateDrag || sprite3dTranslateDrag.pointerId !== event.pointerId) {
    return false;
  }
  event.preventDefault();
  const plane = sprite3dPlaneSize();
  const du = Math.round((event.clientX - sprite3dTranslateDrag.startClientX) / (sprite3dTranslateDrag.width / plane.width));
  const dv = Math.round((event.clientY - sprite3dTranslateDrag.startClientY) / (sprite3dTranslateDrag.height / plane.height));
  sprite3d.cells = translatedSprite3dCells(
    sprite3dTranslateDrag.originCells,
    du,
    dv,
    sprite3dTranslateDrag.scope,
  );
  renderSprite3dSliceBoard();
  renderSprite3dPreview();
  sprite3dSliceBoard.classList.add("is-translating");
  return true;
}

function stopSprite3dTranslate(event) {
  if (!sprite3dTranslateDrag || sprite3dTranslateDrag.pointerId !== event.pointerId) {
    return false;
  }
  if (sprite3dSliceBoard.hasPointerCapture?.(event.pointerId)) {
    sprite3dSliceBoard.releasePointerCapture(event.pointerId);
  }
  const drag = sprite3dTranslateDrag;
  sprite3dTranslateDrag = null;
  sprite3dSliceBoard.classList.remove("is-translating");
  if (!sprite3dCellsEqual(sprite3d.cells, drag.originCells)) {
    pushVisualEditUndoSnapshot("sprite3d", drag.beforeSnapshot);
    syncSprite3dSourceActionButtons();
  }
  return true;
}

function syncSprite3dBucketButton() {
  if (!sprite3dFillButton) {
    return;
  }
  sprite3dFillButton.classList.toggle("is-active", sprite3dBucketActive);
  sprite3dFillButton.setAttribute("aria-pressed", String(sprite3dBucketActive));
}

function setSprite3dButtonLabel(button, label) {
  if (!button) {
    return;
  }
  button.setAttribute("aria-label", label);
  button.title = label;
}

function sprite3dSquareIconSvg() {
  return `
    ${editorIconSvg("square")}
  `;
}

function sprite3dCubeIconSvg() {
  return `
    ${editorIconSvg("box")}
  `;
}

function toggleSprite3dEditScope() {
  setSprite3dEditScope(sprite3dEditScope() === "all" ? "slice" : "all");
}

function setSprite3dEditScope(scope) {
  const previousScope = sprite3dEditScope();
  sprite3d.editScope = scope === "all" ? "all" : "slice";
  if (sprite3dClipSelection && previousScope !== sprite3d.editScope) {
    const rect = sprite3dClipPlaneRect();
    sprite3dClipSelection = sprite3dClipBoxFromPlaneRect(rect, {
      fullDepth: sprite3d.editScope === "all",
    });
    sprite3dClipFloating = null;
    sprite3dClipDrag = null;
  }
  renderSprite3dScopeControl();
  renderSprite3dSliceBoard();
  setSprite3dActionStatus(
    sprite3d.editScope === "all" ? "3D edits affect the whole sprite" : "2D edits affect the current slice",
    "is-ok",
  );
}

function toggleSprite3dBucketMode() {
  if (!sprite3dBucketActive) {
    deactivateSprite3dClipMode({ render: false });
    sprite3dTranslateActive = false;
  }
  sprite3dBucketActive = !sprite3dBucketActive;
  syncSprite3dBucketButton();
  const scope = sprite3dEditScope();
  setSprite3dActionStatus(
    sprite3dBucketActive
      ? scope === "all" ? "Bucket: click a voxel to fill its 3D component" : "Bucket: click a slice area to fill its component"
      : "Brush: paint individual voxels",
    "is-ok",
  );
}

function deactivateSprite3dBucketModeAfterUse() {
  if (!sprite3dBucketActive) {
    return;
  }
  sprite3dBucketActive = false;
  syncSprite3dBucketButton();
}

function renderSprite3dPalette() {
  withSprite3dPaneScrollPreserved(() => renderSprite3dPaletteContent());
}

function setSprite3dCurrentColorTag(index, rawName, linked = true) {
  if (!validSprite3dColorIndex(index)) {
    throw new Error(`Invalid 3D sprite palette index ${index}`);
  }
  const name = sanitizeSpriteColorAssetRef(rawName);
  if (!name) {
    setSprite3dActionStatus("Enter a color tag name", "is-error");
    return false;
  }
  sprite3d.palette[index].bind = { type: "color", name, linked: Boolean(linked) };
  sprite3d.colorTagPickerOpen = false;
  syncSprite3dSourceActionButtons();
  renderSprite3dPalette();
  return true;
}

function renderSprite3dPaletteContent() {
  ensureSprite3dPalette();
  sprite3dPalette.replaceChildren();
  const selectedIsTransparent = sprite3d.selectedColorIndex === null;
  if (selectedIsTransparent || validSprite3dColorIndex(sprite3d.selectedColorIndex)) {
    const selected = selectedIsTransparent ? { color: "#00000000" } : sprite3dPaletteEntries()[sprite3d.selectedColorIndex];
    const selectedBind = selectedIsTransparent ? { available: false, linked: false, name: "" } : spritePaletteEntryBindInfo(selected);
    const selectedDisplayName = selectedBind.linked && selectedBind.name ? selectedBind.name : "";
    const currentWrap = document.createElement("span");
    currentWrap.className = "sprite-current-color-wrap";
    const currentButton = document.createElement("button");
    currentButton.type = "button";
    currentButton.className = "sprite-current-color-button";
    currentButton.classList.toggle("is-transparent", selectedIsTransparent);
    currentButton.classList.toggle("is-bound", selectedBind.available && selectedBind.linked);
    currentButton.classList.toggle("is-unlinked", selectedBind.available && !selectedBind.linked);
    currentButton.style.setProperty("--sprite-current-color", normalizeSpriteColor(selected.color));
    currentButton.title = selectedIsTransparent
      ? "Transparent eraser cannot be edited"
      : selectedDisplayName ? `Pick selected color ${selectedDisplayName}` : "Pick selected color";
    currentButton.setAttribute(
      "aria-label",
      selectedIsTransparent
        ? "Selected transparent eraser color #00000000, not editable"
        : selectedDisplayName ? `Pick selected color ${selectedDisplayName}` : `Pick selected color ${selected.color}`,
    );
    currentButton.setAttribute("aria-disabled", String(selectedIsTransparent));
    currentButton.setAttribute("aria-expanded", String(!selectedIsTransparent && sprite3d.editPaletteOpen));
    currentButton.innerHTML = `<span class="sprite-current-color-swatch" aria-hidden="true"></span>`;
    if (selectedIsTransparent) {
      currentButton.insertAdjacentHTML("beforeend", `
        <span class="sprite-current-transparent-icon" aria-hidden="true">
          ${editorIconSvg("eraser")}
        </span>
      `);
    } else {
      currentButton.insertAdjacentHTML("beforeend", `
        <span class="sprite-current-edit-icon" aria-hidden="true">
          ${editorIconSvg("pencil")}
        </span>
      `);
    }
    const currentHexInput = document.createElement("input");
    currentHexInput.type = "text";
    currentHexInput.className = "sprite-current-value-input sprite-current-hex-input";
    currentHexInput.value = selectedDisplayName || (selectedIsTransparent ? "#00000000" : normalizeSpriteColor(selected.color));
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
        setSprite3dCurrentColorTag(sprite3d.selectedColorIndex, currentHexInput.value, true);
        return;
      }
      const parsed = parseSpriteHexColor(currentHexInput.value);
      if (!parsed) {
        if (options.reportError) {
          setSprite3dActionStatus("Use #rrggbb or #rrggbbaa", "is-error");
        }
        return;
      }
      updateSelectedSprite3dColor(parsed, {
        deferHistory: !options.commitHistory,
        commitHistory: Boolean(options.commitHistory),
      });
    };
    let pendingEditMenu = null;
    if (!selectedIsTransparent) {
      currentButton.addEventListener("click", () => {
        const opening = !sprite3d.editPaletteOpen;
        if (!opening) {
          commitSpriteColorEditHistory("sprite3d");
        }
        sprite3d.editPaletteOpen = opening;
        sprite3d.addPaletteOpen = false;
        sprite3d.addDraftColorIndex = null;
        sprite3d.customColorOpen = opening;
        renderSprite3dPalette();
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
      const bind = spritePaletteEntryBindInfo(selected);
      currentWrap.append(renderSpriteCurrentColorTagButton({
        state: sprite3d,
        entry: selected,
        onToggle: () => {
          sprite3d.editPaletteOpen = false;
          renderSprite3dPalette();
        },
      }));
      if (bind.linked && bind.name) {
        const unlink = document.createElement("button");
        unlink.type = "button";
        unlink.className = "icon-button is-danger sprite-current-tag-unlink-button sprite-icon-button";
        unlink.title = `Unlink color tag ${bind.name}`;
        unlink.setAttribute("aria-label", unlink.title);
        unlink.innerHTML = spriteUnlinkIconSvg();
        unlink.addEventListener("click", () => setSprite3dCurrentColorTag(sprite3d.selectedColorIndex, bind.name, false));
        currentWrap.append(unlink);
      }
      if (sprite3d.colorTagPickerOpen) {
        const colorAssets = spriteSourceColorAssets();
        const picker = renderSpriteAssetNamePicker({
          className: "sprite-color-tag-picker",
          names: spriteColorAssetNames(),
          value: bind.name || defaultSpriteAssetName("color", sprite3d.selectedColorIndex),
          placeholder: "color_name",
          ariaLabel: "Color tag name",
          emptyText: "No named colors yet",
          optionMeta: (name) => ({ color: colorAssets.get(name) }),
          onCommit: (name) => setSprite3dCurrentColorTag(sprite3d.selectedColorIndex, name, true),
          onCancel: () => {
            sprite3d.colorTagPickerOpen = false;
            renderSprite3dPalette();
          },
        });
        currentWrap.append(picker);
      }
    }
    if (!selectedIsTransparent && sprite3d.editPaletteOpen) {
      const editorPanel = document.createElement("span");
      editorPanel.className = "sprite-current-editor-panel";
      const editMenu = renderSpriteColorMenu({
        mode: "edit",
        customValue: selected.color,
        customOnly: true,
        onChange: updateSelectedSprite3dColor,
        onPreset: updateSelectedSprite3dColor,
        renderPalette: renderSprite3dPalette,
      });
      editorPanel.append(editMenu);
      currentWrap.append(editorPanel);
      pendingEditMenu = editMenu;
    }
    currentWrap.append(sprite3dShapeField);
    sprite3dPalette.append(currentWrap);
    if (pendingEditMenu) {
      positionSpriteColorMenu(pendingEditMenu, currentButton, { side: "left" });
    }
  }

  renderSpritePaletteGrid({
    target: sprite3dPalette,
    leadingControl: spriteMarkerTool,
    entries: sprite3dPaletteEntries(),
    selectedIndex: sprite3d.selectedColorIndex,
    bucketActive: sprite3dBucketActive,
    emptyTitle: "Paint empty voxel",
    emptyAriaLabel: "Paint empty voxel",
    colorAriaLabel: (index, name) => name
      ? `Paint 3D sprite color ${index + 1}: ${name}`
      : `Paint 3D sprite color ${index + 1}`,
    onSelect: selectSprite3dColor,
    onAdd: toggleSprite3dAddPalette,
    onRemove: removeSprite3dColor,
    addOpen: sprite3d.addPaletteOpen,
    renderAddMenu: () => renderSpriteColorMenu({
      mode: "add",
      customValue: validSprite3dColorIndex(sprite3d.addDraftColorIndex)
        ? sprite3dPaletteEntries()[sprite3d.addDraftColorIndex].color
        : nextSpritePresetColor(sprite3dPaletteEntries()),
      onDiscard: cancelSprite3dColorAdd,
      onChange: previewNewSprite3dColor,
      onPreset: previewNewSprite3dColor,
      renderPalette: renderSprite3dPalette,
    }),
  });
}

function renderSprite3dSliceBoard() {
  withSprite3dPaneScrollPreserved(() => {
    sprite3dSliceBoard.replaceChildren();
    sprite3dSliceBoard.classList.toggle("is-grid-hidden", !sprite3dGridVisible);
    sprite3dSliceBoard.classList.toggle("is-translate-active", sprite3dTranslateActive);
    sprite3dSliceBoard.classList.toggle("is-clip-active", sprite3dClipActive);
    sprite3dSliceBoard.classList.toggle("is-clip-floating", Boolean(sprite3dClipFloating));
    const planeSize = sprite3dPlaneSize();
    sprite3dSliceBoard.style.setProperty("--sprite-size", Math.max(planeSize.width, planeSize.height));
    sprite3dSliceBoard.style.setProperty("--sprite-cols", planeSize.width);
    sprite3dSliceBoard.style.setProperty("--sprite-rows", planeSize.height);
    const selectionRect = sprite3dClipPlaneRect();
    const fixed = sprite3dPlaneWorldSlice(sprite3d.axis, sprite3d.slice);
    const normalKey = `${sprite3d.axis.toUpperCase()}`;
    const selectionIntersectsSlice = Boolean(
      sprite3dClipSelection
      && fixed >= sprite3dClipSelection[`min${normalKey}`]
      && fixed <= sprite3dClipSelection[`max${normalKey}`],
    );
    const cellCount = planeSize.width * planeSize.height;
    for (let index = 0; index < cellCount; index += 1) {
      const coords = sprite3dCoordsFromSliceCell(index);
      const voxelIndex = sprite3dCellIndex(coords.x, coords.y, coords.z);
      const colorIndex = validSprite3dColorIndex(sprite3d.cells[voxelIndex]) ? sprite3d.cells[voxelIndex] : null;
      const button = document.createElement("button");
      button.type = "button";
      button.className = "sprite-cell sprite-color-swatch";
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
      button.style.setProperty("--sprite-swatch-color", sprite3dColorForColorIndex(colorIndex));
      button.style.setProperty("--sprite-cell-ink", sprite3dInkForColorIndex(colorIndex));
      button.setAttribute("aria-label", `Voxel ${coords.x + 1}, ${coords.y + 1}, ${coords.z + 1}`);
      sprite3dSliceBoard.append(button);
    }
    renderSprite3dClipSelectionFrame();
  });
}

function renderSprite3dClipSelectionFrame() {
  const rect = sprite3dClipPlaneRect();
  if (!rect) {
    return;
  }
  renderSprite3dClipFloatingPreview(rect);
  const frame = document.createElement("div");
  frame.className = "sprite-clip-selection-frame";
  frame.style.setProperty("--sprite-clip-x", String(rect.x));
  frame.style.setProperty("--sprite-clip-y", String(rect.y));
  frame.style.setProperty("--sprite-clip-width", String(rect.width));
  frame.style.setProperty("--sprite-clip-height", String(rect.height));
  frame.setAttribute("aria-hidden", "true");
  if (!sprite3dClipFloating) {
    for (const edge of ["n", "e", "s", "w"]) {
      const node = document.createElement("span");
      node.className = `sprite-clip-selection-edge sprite-clip-selection-edge-${edge}`;
      node.dataset.sprite3dClipResize = edge;
      frame.append(node);
    }
  }
  for (const handle of ["nw", "ne", "sw", "se"]) {
    const node = document.createElement("span");
    node.className = `sprite-clip-selection-handle sprite-clip-selection-handle-${handle}`;
    if (!sprite3dClipFloating) {
      node.dataset.sprite3dClipResize = handle;
    }
    frame.append(node);
  }
  sprite3dSliceBoard.append(frame);
}

function sprite3dClipFloatingPlaneCells(rect) {
  const box = normalizeSprite3dClipBox(sprite3dClipSelection);
  const clipboard = sprite3dClipClipboard;
  if (!box || !clipboard || !sprite3dClipFloating) {
    return null;
  }
  const fixed = sprite3dPlaneWorldSlice(sprite3d.axis, sprite3d.slice);
  const normalKey = sprite3d.axis.toUpperCase();
  if (fixed < box[`min${normalKey}`] || fixed > box[`max${normalKey}`]) {
    return null;
  }
  if (clipboard.scope === "slice") {
    return clipboard.width === rect.width && clipboard.height === rect.height ? clipboard.cells : null;
  }
  const cells = [];
  for (let v = rect.y; v < rect.y + rect.height; v += 1) {
    for (let u = rect.x; u < rect.x + rect.width; u += 1) {
      const coords = sprite3dCoordsFromPlane(sprite3d.axis, sprite3d.slice, u, v);
      const x = coords.x - box.minX;
      const y = coords.y - box.minY;
      const z = coords.z - box.minZ;
      const index = ((z * clipboard.height + y) * clipboard.width) + x;
      cells.push(clipboard.cells[index] ?? null);
    }
  }
  return cells;
}

function renderSprite3dClipFloatingPreview(rect) {
  const cells = sprite3dClipFloatingPlaneCells(rect);
  if (!cells) {
    return;
  }
  const preview = document.createElement("div");
  preview.className = `sprite-clip-floating-preview is-${sprite3dClipFloating.kind || "copy"}`;
  preview.style.setProperty("--sprite-clip-x", String(rect.x));
  preview.style.setProperty("--sprite-clip-y", String(rect.y));
  preview.style.setProperty("--sprite-clip-width", String(rect.width));
  preview.style.setProperty("--sprite-clip-height", String(rect.height));
  preview.style.setProperty("--sprite-clip-preview-cols", String(rect.width));
  preview.setAttribute("aria-hidden", "true");
  for (const colorIndex of cells) {
    const validIndex = validSprite3dColorIndex(colorIndex) ? colorIndex : null;
    const cell = document.createElement("span");
    cell.className = "sprite-clip-preview-cell sprite-color-swatch";
    cell.style.setProperty("--sprite-swatch-color", sprite3dColorForColorIndex(validIndex));
    cell.style.setProperty("--sprite-cell-ink", sprite3dInkForColorIndex(validIndex));
    preview.append(cell);
  }
  sprite3dSliceBoard.append(preview);
}

function renderSprite3dPreview() {
  renderSprite3dPreviewCanvas(sprite3dPreviewCanvas, sprite3d.cells, { overlays: true });
  renderSprite3dCameraControls();
}

function renderSprite3dPresentationSurfaces() {
  renderSprite3dPreview();
  renderSprite3dAnimationFrameStrip();
  if (sprite3d.animationMode) {
    const context = sharedSpriteAnimationController("sprite3d");
    const frame = context.frames[context.state.animationPlaybackIndex] || context.state.cells;
    renderSharedSpriteAnimationPlaybackView(context, frame);
  }
}

function renderSprite3dPreviewCanvas(canvas, cells, options = {}) {
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
  ctx.fillStyle = sprite3dCssVar("--sprite3d-preview-bg", "#1d2023");
  ctx.fillRect(0, 0, width, height);

  const view = sprite3dPreviewView(width, height);
  drawSprite3dBounds(ctx, view);

  const occupied = sprite3dOccupancyMap(cells);
  const faces = sprite3dMergedVoxelFaces(occupied, view);
  const previewOwner = sprite3dPreviewRenderOwner();
  const sceneFaces = [
    ...faces.map((face) => ({ ...face, kind: "voxel", ownerCell: previewOwner, renderPriority: 0 })),
    ...(options.overlays === false ? [] : sprite3dSliceSurfaceFaces(sprite3d.hoverSlice, view, "hover", occupied, 1)
      .map((face) => ({ ...face, ownerCell: previewOwner }))),
    ...(options.overlays === false ? [] : sprite3dSliceSurfaceFaces(sprite3d.slice, view, "active", occupied, 2)
      .map((face) => ({ ...face, ownerCell: previewOwner }))),
  ];
  assignSprite3dPrimitiveOrder(sceneFaces);
  sceneFaces.sort(Puzzle3VisualCore.comparePrimitiveOrder);
  for (const face of sceneFaces) {
    if (face.kind === "slice") {
      drawSprite3dSliceFace(ctx, face, face.mode);
    } else {
      drawSprite3dFace(ctx, face);
    }
  }
  if (options.overlays !== false) {
    drawSprite3dClipBounds(ctx, view);
    canvas._sprite3dPreviewView = view;
  }
}

function sprite3dPreviewView(width, height) {
  const padding = 0;
  const overlayControlHeight = Number.parseFloat(
    sprite3dCssVar("--sprite3d-overlay-control-height", "22"),
  );
  const overlaySafeInset = 8 + overlayControlHeight + 4;
  const safeTop = overlaySafeInset;
  const safeBottom = overlaySafeInset;
  const boundsView = {
    cellScale: 1,
    originX: 0,
    originY: 0,
  };
  const points = sprite3dBoundsCorners().map((corner) => sprite3dProject(corner, boundsView));
  const minX = Math.min(...points.map((point) => point.x));
  const maxX = Math.max(...points.map((point) => point.x));
  const minY = Math.min(...points.map((point) => point.y));
  const maxY = Math.max(...points.map((point) => point.y));
  const projectedWidth = Math.max(1, maxX - minX);
  const projectedHeight = Math.max(1, maxY - minY);
  const availableWidth = Math.max(1, width - padding * 2);
  const safeHeight = Math.max(1, height - safeTop - safeBottom);
  const availableHeight = Math.max(1, safeHeight - padding * 2);
  const scale = Math.max(4, Math.min(availableWidth / projectedWidth, availableHeight / projectedHeight) * SPRITE3D_PREVIEW_BASE_ZOOM)
    * sprite3dCamera().zoom;
  return {
    cellScale: scale,
    originX: width / 2 - ((minX + maxX) / 2) * scale,
    originY: safeTop + safeHeight / 2 - ((minY + maxY) / 2) * scale,
  };
}

function sprite3dBoundsCorners() {
  const min = -0.5;
  const maxX = sprite3d.width - 0.5;
  const maxY = sprite3d.height - 0.5;
  const maxDepth = sprite3d.depth - 0.5;
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

function sprite3dCamera() {
  if (!sprite3d.camera) {
    sprite3d.camera = { ...SPRITE3D_CAMERA_DEFAULT };
  }
  sprite3d.camera.yawDegrees = sprite3dNormalizeDegrees(sprite3d.camera.yawDegrees ?? SPRITE3D_CAMERA_DEFAULT.yawDegrees);
  sprite3d.camera.pitchDegrees = sprite3dClampNumber(
    sprite3d.camera.pitchDegrees ?? SPRITE3D_CAMERA_DEFAULT.pitchDegrees,
    SPRITE3D_CAMERA_MIN_PITCH_DEGREES,
    SPRITE3D_CAMERA_MAX_PITCH_DEGREES,
  );
  sprite3d.camera.zoom = sprite3dClampNumber(sprite3d.camera.zoom ?? SPRITE3D_CAMERA_DEFAULT.zoom, 0.25, 4);
  return sprite3d.camera;
}

function sprite3dFaceGridOrder(corners) {
  return Puzzle3VisualCore.faceGridOrder(corners, sprite3dVisualView());
}

function sprite3dVisualView() {
  return { camera: sprite3dCamera() };
}

function sprite3dPreviewRenderOwner() {
  return {
    key: "sprite3d-preview",
    order: { x: 0, y: 0, z: 0 },
    depth: 0,
  };
}

function assignSprite3dPrimitiveOrder(primitives) {
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

function renderSprite3dCameraControls() {
  const camera = sprite3dCamera();
  renderSprite3dCameraScrub(sprite3dCameraYawScrub, "yaw", Math.round(camera.yawDegrees));
  renderSprite3dCameraScrub(sprite3dCameraPitchScrub, "pitch", Math.round(camera.pitchDegrees));
  renderSprite3dCameraScrub(sprite3dCameraZoomScrub, "zoom", Number(camera.zoom.toFixed(2)));
}

function renderSprite3dCameraScrub(element, kind, value) {
  if (!(element instanceof HTMLElement)) {
    return;
  }
  const text = String(value);
  element.textContent = text;
  element.setAttribute("aria-label", `Drag vertically to adjust ${kind}, current ${text}`);
}

function sprite3dClampNumber(value, min, max) {
  const parsed = Number(value);
  const fallback = min <= 0 && max >= 0 ? 0 : min;
  return Math.min(max, Math.max(min, Number.isFinite(parsed) ? parsed : fallback));
}

function sprite3dOccupancyMap(cells = sprite3d.cells) {
  const occupied = new Map();
  for (let z = 0; z < sprite3d.depth; z += 1) {
    for (let y = 0; y < sprite3d.height; y += 1) {
      for (let x = 0; x < sprite3d.width; x += 1) {
        const colorIndex = cells?.[sprite3dCellIndex(x, y, z)];
        if (validSprite3dColorIndex(colorIndex)) {
          occupied.set(sprite3dVoxelKey(x, y, z), {
            colorIndex,
            opaque: sprite3dColorIsOpaque(sprite3dColorForColorIndex(colorIndex)),
          });
        }
      }
    }
  }
  return occupied;
}

function sprite3dCssVar(name, fallback) {
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return value || fallback;
}

function drawSprite3dBounds(ctx, view) {
  const z = -0.5;
  const corners = [
    sprite3dProject({ x: -0.5, y: -0.5, z }, view),
    sprite3dProject({ x: sprite3d.width - 0.5, y: -0.5, z }, view),
    sprite3dProject({ x: sprite3d.width - 0.5, y: sprite3d.height - 0.5, z }, view),
    sprite3dProject({ x: -0.5, y: sprite3d.height - 0.5, z }, view),
  ];
  ctx.fillStyle = sprite3dCssVar("--sprite3d-frame-fill", "rgba(137, 148, 158, 0.10)");
  ctx.strokeStyle = sprite3dCssVar("--sprite3d-frame-stroke", "rgba(137, 148, 158, 0.38)");
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

function drawSprite3dClipBounds(ctx, view) {
  const box = normalizeSprite3dClipBox(sprite3dClipSelection);
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
  ].map((point) => sprite3dProject(point, view));
  const faces = [
    [0, 1, 2, 3],
    [4, 5, 6, 7],
    [0, 1, 5, 4],
    [1, 2, 6, 5],
    [2, 3, 7, 6],
    [3, 0, 4, 7],
  ];
  ctx.fillStyle = sprite3dCssVar("--sprite3d-clip-fill", "rgba(125, 208, 160, 0.08)");
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
  ctx.strokeStyle = sprite3dCssVar("--sprite3d-clip-stroke", "rgba(125, 208, 160, 0.9)");
  ctx.lineWidth = 1.5;
  ctx.beginPath();
  for (const [from, to] of edges) {
    ctx.moveTo(corners[from].x, corners[from].y);
    ctx.lineTo(corners[to].x, corners[to].y);
  }
  ctx.stroke();
}

function sprite3dSliceHitPlanes(view) {
  const hitPlanes = [];
  for (let index = 0; index < sprite3dAxisSize(); index += 1) {
    hitPlanes.push({ index, points: sprite3dSliceHitPlaneCorners(index, view) });
  }
  return hitPlanes;
}

function sprite3dSliceHitEdges(view) {
  if (sprite3d.axis !== "z") {
    return [];
  }
  const min = -0.5;
  const maxX = sprite3d.width - 0.5;
  const maxY = sprite3d.height - 0.5;
  const maxDepth = sprite3d.depth - 0.5;
  return [
    { x: min, y: min },
    { x: maxX, y: min },
    { x: maxX, y: maxY },
    { x: min, y: maxY },
  ].map((edge) => {
    const from = sprite3dProject({ x: edge.x, y: edge.y, z: min }, view);
    const to = sprite3dProject({ x: edge.x, y: edge.y, z: maxDepth }, view);
    return {
      axis: "z",
      from,
      to,
      min,
      max: maxDepth,
      hitRadius: sprite3dClamp(view.cellScale * 0.34, 8, 18),
    };
  });
}

function sprite3dSliceHitPlaneCorners(slice, view) {
  const min = -0.5;
  const maxX = sprite3d.width - 0.5;
  const maxY = sprite3d.height - 0.5;
  const maxDepth = sprite3d.depth - 0.5;
  const fixed = sprite3dPlaneWorldSlice(sprite3d.axis, slice);
  let corners = [];
  if (sprite3d.axis === "x") {
    corners = [
      { x: fixed, y: min, z: min },
      { x: fixed, y: min, z: maxDepth },
      { x: fixed, y: maxY, z: maxDepth },
      { x: fixed, y: maxY, z: min },
    ];
  } else if (sprite3d.axis === "y") {
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
  return corners.map((corner) => sprite3dProject(corner, view));
}

function sprite3dSliceSurfaceFaces(slice, view, mode, occupied, order = 0) {
  if (!Number.isInteger(slice)) {
    return [];
  }
  const groups = new Map();
  const fill = sprite3dSliceOverlayFill(mode);
  const stroke = sprite3dSliceOverlayStroke(mode);
  const plane = sprite3dPlaneSize();
  for (let row = 0; row < plane.height; row += 1) {
    for (let col = 0; col < plane.width; col += 1) {
      const grid = sprite3dCoordsFromPlane(sprite3d.axis, slice, col, row);
      if (occupied.has(sprite3dVoxelKey(grid.x, grid.y, grid.z))) {
        continue;
      }
      sprite3dAddSliceSurfaceFace(groups, "zNeg", grid, { x: grid.x, y: grid.y, z: grid.z - 1 }, slice, occupied);
      sprite3dAddSliceSurfaceFace(groups, "zPos", grid, { x: grid.x, y: grid.y, z: grid.z + 1 }, slice, occupied);
      sprite3dAddSliceSurfaceFace(groups, "xNeg", grid, { x: grid.x - 1, y: grid.y, z: grid.z }, slice, occupied);
      sprite3dAddSliceSurfaceFace(groups, "xPos", grid, { x: grid.x + 1, y: grid.y, z: grid.z }, slice, occupied);
      sprite3dAddSliceSurfaceFace(groups, "yPos", grid, { x: grid.x, y: grid.y + 1, z: grid.z }, slice, occupied);
      sprite3dAddSliceSurfaceFace(groups, "yNeg", grid, { x: grid.x, y: grid.y - 1, z: grid.z }, slice, occupied);
    }
  }
  return sprite3dMergedSliceFaces(groups, view, fill, stroke, mode, order);
}

function sprite3dAddSliceSurfaceFace(groups, side, grid, neighbor, slice, occupied) {
  if (sprite3dGridInSliceVolume(neighbor, slice) || occupied.has(sprite3dVoxelKey(neighbor.x, neighbor.y, neighbor.z))) {
    return;
  }
  const info = sprite3dSliceFaceGroupInfo(side, grid);
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

function sprite3dSliceFaceGroupInfo(side, grid) {
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

function sprite3dMergedSliceFaces(groups, view, fill, stroke, mode, order) {
  const faces = [];
  for (const group of groups.values()) {
    const polygons = [...group.cells].map((key) => {
      const [u, v] = key.split(",").map(Number);
      return sprite3dMergedSliceFaceCorners(group.side, group.planeIndex, { u0: u, u1: u, v0: v, v1: v });
    });
    const projectedPolygons = polygons.map((polygon) => polygon.map((corner) => sprite3dProject(corner, view)));
    const projectedPoints = projectedPolygons.flat();
    faces.push({
      kind: "slice",
      key: `slice:${mode}:${order}:${group.side}:${group.planeIndex}:${[...group.cells].sort().join(";")}`,
      mode,
      order,
      renderPriority: order,
      polygons: projectedPolygons.map((polygon) => polygon.map(({ x, y }) => ({ x, y }))),
      edges: sprite3dSliceGroupBoundaryEdges(group).map((edge) => edge.map((point) => sprite3dProject(
        sprite3dSliceGroupPoint(group.side, group.planeIndex, point.u, point.v),
        view,
      ))),
      depth: projectedPoints.reduce((total, point) => total + point.depth, 0) / Math.max(1, projectedPoints.length),
      gridOrder: sprite3dFaceGridOrder(polygons.flat()),
      fill,
      stroke,
    });
  }
  return faces;
}

function sprite3dSliceGroupBoundaryEdges(group) {
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

function sprite3dSliceGroupPoint(side, planeIndex, u, v) {
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

function sprite3dMergedSliceFaceCorners(side, planeIndex, rect) {
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

function sprite3dGridInSlice(grid, slice) {
  if (!Number.isInteger(slice)) {
    return false;
  }
  const worldSlice = sprite3dPlaneWorldSlice(sprite3d.axis, slice);
  if (sprite3d.axis === "x") {
    return grid.x === worldSlice;
  }
  if (sprite3d.axis === "y") {
    return grid.y === worldSlice;
  }
  return grid.z === worldSlice;
}

function sprite3dGridInSliceVolume(grid, slice) {
  return grid.x >= 0
    && grid.y >= 0
    && grid.z >= 0
    && grid.x < sprite3d.width
    && grid.y < sprite3d.height
    && grid.z < sprite3d.depth
    && sprite3dGridInSlice(grid, slice);
}

function sprite3dSliceOverlayFill(mode) {
  return mode === "active"
    ? sprite3dCssVar("--sprite3d-slice-active-fill", "rgba(125, 208, 160, 0.022)")
    : sprite3dCssVar("--sprite3d-slice-hover-fill", "rgba(137, 148, 158, 0.025)");
}

function sprite3dSliceOverlayStroke(mode) {
  return mode === "active"
    ? sprite3dCssVar("--sprite3d-slice-active-stroke", "rgba(125, 208, 160, 0.12)")
    : sprite3dCssVar("--sprite3d-slice-hover-stroke", "rgba(137, 148, 158, 0.15)");
}

function sprite3dSliceVoxelCoord(slice, col, row) {
  return sprite3dCoordsFromPlane(sprite3d.axis, slice, col, row);
}

function sprite3dProjectedSliceFace(fill, stroke, view, corners) {
  const projected = corners.map((corner) => sprite3dProject(corner, view));
  return {
    points: projected.map(({ x, y }) => ({ x, y })),
    depth: projected.reduce((total, point) => total + point.depth, 0) / projected.length,
    fill,
    stroke,
  };
}

function drawSprite3dSliceFace(ctx, face, mode) {
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
  const expanded = sprite3dExpandPolygon(face.points, 0.18);
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

function sprite3dProject(position, view) {
  const camera = sprite3dCamera();
  return Puzzle3VisualCore.projectOrthographic(position, {
    camera,
    center: {
      x: (sprite3d.width - 1) / 2,
      y: (sprite3d.height - 1) / 2,
      z: (sprite3d.depth - 1) / 2,
    },
    origin: { x: view.originX, y: view.originY },
    scale: view.cellScale,
  });
}

function sprite3dMergedVoxelFaces(occupied, view) {
  const voxels = [];
  for (let z = 0; z < sprite3d.depth; z += 1) {
    for (let y = 0; y < sprite3d.height; y += 1) {
      for (let x = 0; x < sprite3d.width; x += 1) {
        const colorIndex = sprite3d.cells[sprite3dCellIndex(x, y, z)];
        if (validSprite3dColorIndex(colorIndex)) {
          voxels.push({ x, y, z, colorIndex });
        }
      }
    }
  }
  return Puzzle3VisualCore.mergeVoxelFaces(voxels, {
    faces: sprite3dVoxelFaceSpecs,
    isFaceVisible: (voxel, face) => sprite3dFaceIsOpen(occupied, face.neighborKey, sprite3dColorForColorIndex(voxel.colorIndex)),
    group: (voxel, face) => {
      const fill = sprite3dShadeColor(sprite3dColorForColorIndex(voxel.colorIndex), face.light);
      const info = sprite3dSliceFaceGroupInfo(face.side, voxel);
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
    rectsFromCells: sprite3dUnitFaceRects,
    face: (group, rect) => {
      const corners = sprite3dMergedSliceFaceCorners(group.side, group.planeIndex, rect);
      const projected = corners.map((corner) => sprite3dProject(corner, view));
      const key = `${group.key}:${rect.u0},${rect.u1},${rect.v0},${rect.v1}`;
      return {
        key,
        points: projected.map(({ x, y }) => ({ x, y })),
        depth: projected.reduce((total, point) => total + point.depth, 0) / projected.length,
        gridOrder: sprite3dFaceGridOrder(corners),
        renderPriority: 0,
        fill: group.fill,
        overlays: sprite3dVoxelFaceOverlays(group.side, group.planeIndex, rect, view),
      };
    },
  });
}

function sprite3dUnitFaceRects(cells) {
  return [...cells]
    .map((key) => {
      const [u, v] = key.split(",").map(Number);
      return { u0: u, u1: u, v0: v, v1: v };
    })
    .sort((left, right) => left.v0 - right.v0 || left.u0 - right.u0);
}

function sprite3dVoxelFaceOverlays(side, planeIndex, rect, view) {
  const overlaysByMode = new Map();
  for (let v = rect.v0; v <= rect.v1; v += 1) {
    for (let u = rect.u0; u <= rect.u1; u += 1) {
      const grid = sprite3dVoxelGridFromFaceCell(side, planeIndex, u, v);
      for (const mode of sprite3dVoxelOverlayModesForGrid(grid)) {
        const corners = sprite3dMergedSliceFaceCorners(side, planeIndex, { u0: u, u1: u, v0: v, v1: v });
        const polygon = corners.map((corner) => {
          const projected = sprite3dProject(corner, view);
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

function sprite3dVoxelOverlayModesForGrid(grid) {
  const modes = [];
  if (
    Number.isInteger(sprite3d.hoverSlice)
    && sprite3d.hoverSlice !== sprite3d.slice
    && sprite3dGridInSlice(grid, sprite3d.hoverSlice)
  ) {
    modes.push("hover");
  }
  if (Number.isInteger(sprite3d.slice) && sprite3dGridInSlice(grid, sprite3d.slice)) {
    modes.push("active");
  }
  return modes;
}

function sprite3dVoxelGridFromFaceCell(side, planeIndex, u, v) {
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

function sprite3dVoxelFaceSpecs(voxel) {
  return [
    { side: "zNeg", neighborKey: sprite3dVoxelKey(voxel.x, voxel.y, voxel.z - 1), light: -0.22 },
    { side: "zPos", neighborKey: sprite3dVoxelKey(voxel.x, voxel.y, voxel.z + 1), light: 0.10 },
    { side: "xNeg", neighborKey: sprite3dVoxelKey(voxel.x - 1, voxel.y, voxel.z), light: -0.08 },
    { side: "xPos", neighborKey: sprite3dVoxelKey(voxel.x + 1, voxel.y, voxel.z), light: 0.02 },
    { side: "yPos", neighborKey: sprite3dVoxelKey(voxel.x, voxel.y + 1, voxel.z), light: -0.04 },
    { side: "yNeg", neighborKey: sprite3dVoxelKey(voxel.x, voxel.y - 1, voxel.z), light: -0.16 },
  ];
}

function sprite3dFaceIsOpen(occupied, neighborKey, fill) {
  const neighbor = occupied.get(neighborKey);
  return !(neighbor?.opaque || sprite3dColorForColorIndex(neighbor?.colorIndex) === fill);
}

function drawSprite3dFace(ctx, face) {
  const expanded = sprite3dExpandPolygon(face.points, 0.35);
  ctx.beginPath();
  ctx.moveTo(expanded[0].x, expanded[0].y);
  for (const point of expanded.slice(1)) {
    ctx.lineTo(point.x, point.y);
  }
  ctx.closePath();
  ctx.fillStyle = face.fill;
  ctx.fill();
  if (sprite3dGridVisible) {
    ctx.strokeStyle = sprite3dCssVar("--sprite3d-voxel-grid-stroke", "rgba(20, 24, 28, 0.38)");
    ctx.lineWidth = 0.72;
    ctx.stroke();
  }
  for (const overlay of face.overlays || []) {
    drawSprite3dVoxelOverlayFace(ctx, overlay);
  }
}

function drawSprite3dVoxelOverlayFace(ctx, face) {
  const style = sprite3dSliceVoxelOverlayStyle(face.mode, face.polygons);
  if (style.kind === "tint") {
    sprite3dTintVoxelFace(ctx, face.polygons, style);
    return;
  }
  sprite3dStripeVoxelFace(ctx, face.polygons, style);
}

function sprite3dStripeVoxelFace(ctx, polygons, style) {
  const rotated = polygons.flat().map((point) => sprite3dRotatePoint(point, -style.angle));
  const minX = Math.min(...rotated.map((point) => point.x)) - style.gap * 2;
  const maxX = Math.max(...rotated.map((point) => point.x)) + style.gap * 2;
  const minY = Math.min(...rotated.map((point) => point.y)) - style.gap * 2;
  const maxY = Math.max(...rotated.map((point) => point.y)) + style.gap * 2;
  const width = Math.max(1, maxX - minX);
  const band = Math.max(1, style.gap / 2);
  const overlap = 0.25;
  const startY = Math.floor(minY / style.gap) * style.gap;
  ctx.save();
  sprite3dClipPolygons(ctx, polygons);
  ctx.rotate(style.angle);
  for (let y = startY; y <= maxY; y += style.gap) {
    ctx.fillStyle = `rgba(255, 255, 255, ${style.lightAlpha})`;
    ctx.fillRect(minX, y, width, band + overlap);
    ctx.fillStyle = `rgba(0, 0, 0, ${style.darkAlpha})`;
    ctx.fillRect(minX, y + band, width, band + overlap);
  }
  ctx.restore();
}

function sprite3dTintVoxelFace(ctx, polygons, style) {
  ctx.save();
  sprite3dClipPolygons(ctx, polygons);
  ctx.fillStyle = `rgba(255, 255, 255, ${style.lightAlpha})`;
  ctx.fill();
  ctx.fillStyle = `rgba(0, 0, 0, ${style.darkAlpha})`;
  ctx.fill();
  ctx.restore();
}

function sprite3dSliceVoxelOverlayStyle(mode, polygons) {
  const edge = sprite3dProjectedVoxelEdgeLength(polygons);
  return mode === "active"
    ? {
        kind: "stripe",
        angle: 0.98,
        gap: sprite3dClamp(edge * 0.42, 10, 22),
        lightAlpha: 0.105,
        darkAlpha: 0.06,
      }
    : { kind: "tint", lightAlpha: 0.055, darkAlpha: 0.025 };
}

function sprite3dProjectedVoxelEdgeLength(polygons) {
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

function sprite3dClamp(value, min, max) {
  return Math.min(max, Math.max(min, value));
}

function sprite3dClipPolygons(ctx, polygons) {
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

function sprite3dRotatePoint(point, angle) {
  const cos = Math.cos(angle);
  const sin = Math.sin(angle);
  return {
    x: point.x * cos - point.y * sin,
    y: point.x * sin + point.y * cos,
  };
}

function sprite3dExpandPolygon(points, amount) {
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

function sprite3dShadeColor(color, amount) {
  const rgba = sprite3dParseColor(color);
  if (!rgba) {
    return color;
  }
  return sprite3dFormatColor({
    r: sprite3dLightenChannel(rgba.r, amount),
    g: sprite3dLightenChannel(rgba.g, amount),
    b: sprite3dLightenChannel(rgba.b, amount),
    a: rgba.a,
  });
}

function sprite3dParseColor(color) {
  const normalized = parseSpriteHexColor(color);
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

function sprite3dColorIsOpaque(color) {
  const rgba = sprite3dParseColor(color);
  return !rgba || rgba.a >= 0.999;
}

function sprite3dFormatColor(color) {
  const r = sprite3dClampColorChannel(color.r);
  const g = sprite3dClampColorChannel(color.g);
  const b = sprite3dClampColorChannel(color.b);
  const a = Math.max(0, Math.min(1, color.a));
  if (a >= 0.999) {
    return `rgb(${r}, ${g}, ${b})`;
  }
  return `rgba(${r}, ${g}, ${b}, ${Number(a.toFixed(3))})`;
}

function sprite3dClampColorChannel(value) {
  return Math.max(0, Math.min(255, Math.round(value)));
}

function sprite3dLightenChannel(value, light) {
  if (light < 0) {
    return sprite3dClampColorChannel(value + value * light);
  }
  return sprite3dClampColorChannel(value + (255 - value) * light);
}

function sprite3dVoxelKey(x, y, z) {
  return `${x},${y},${z}`;
}

function selectSprite3dColor(index) {
  commitSpriteColorEditHistory("sprite3d");
  sprite3d.selectedColorIndex = validSprite3dColorIndex(index) ? index : null;
  sprite3d.addPaletteOpen = false;
  sprite3d.editPaletteOpen = false;
  sprite3d.customColorOpen = false;
  sprite3d.addDraftColorIndex = null;
  renderSprite3dPalette();
}

function addSprite3dColor() {
  commitSpriteColorEditHistory("sprite3d");
  const before = visualEditSnapshot("sprite3d");
  const draftIndex = validSprite3dColorIndex(sprite3d.addDraftColorIndex) ? sprite3d.addDraftColorIndex : null;
  if (draftIndex === null && sprite3dPaletteEntries().length >= SPRITE_COLOR_TOKENS.length) {
    setSprite3dActionStatus(`Palette limit is ${SPRITE_COLOR_TOKENS.length} colors`, "is-error");
    return;
  }
  if (draftIndex === null) {
    sprite3dPaletteEntries().push({ color: normalizeSpriteColor(nextSpritePresetColor(sprite3dPaletteEntries())) });
    sprite3d.selectedColorIndex = sprite3dPaletteEntries().length - 1;
  } else {
    sprite3d.selectedColorIndex = draftIndex;
  }
  sprite3d.addPaletteOpen = false;
  sprite3d.editPaletteOpen = false;
  sprite3d.customColorOpen = false;
  sprite3d.addDraftColorIndex = null;
  renderSprite3dBuilder();
  pushVisualEditUndoSnapshot("sprite3d", before);
}

function toggleSprite3dAddPalette() {
  commitSpriteColorEditHistory("sprite3d");
  const before = visualEditSnapshot("sprite3d");
  const opening = !sprite3d.addPaletteOpen;
  if (opening && sprite3dPaletteEntries().length >= SPRITE_COLOR_TOKENS.length) {
    setSprite3dActionStatus(`Palette limit is ${SPRITE_COLOR_TOKENS.length} colors`, "is-error");
    return;
  }
  sprite3d.addPaletteOpen = opening;
  sprite3d.editPaletteOpen = false;
  sprite3d.customColorOpen = opening;
  if (opening) {
    if (!validSprite3dColorIndex(sprite3d.addDraftColorIndex)) {
      sprite3dPaletteEntries().push({ color: normalizeSpriteColor(nextSpritePresetColor(sprite3dPaletteEntries())) });
      sprite3d.addDraftColorIndex = sprite3dPaletteEntries().length - 1;
    }
    sprite3d.selectedColorIndex = sprite3d.addDraftColorIndex;
    renderSprite3dBuilder();
    pushVisualEditUndoSnapshot("sprite3d", before);
    return;
  }
  sprite3d.addDraftColorIndex = null;
  renderSprite3dBuilder();
  pushVisualEditUndoSnapshot("sprite3d", before);
}

function previewNewSprite3dColor(color, options = {}) {
  const before = options.deferHistory ? null : visualEditSnapshot("sprite3d");
  if (options.deferHistory) {
    beginSpriteColorEditHistory("sprite3d");
  }
  if (!validSprite3dColorIndex(sprite3d.addDraftColorIndex) && sprite3dPaletteEntries().length >= SPRITE_COLOR_TOKENS.length) {
    return;
  }
  if (!validSprite3dColorIndex(sprite3d.addDraftColorIndex)) {
    sprite3dPaletteEntries().push({ color: normalizeSpriteColor(color) });
    sprite3d.addDraftColorIndex = sprite3dPaletteEntries().length - 1;
    sprite3d.selectedColorIndex = sprite3d.addDraftColorIndex;
    renderSprite3dBuilder();
  } else {
    sprite3dPaletteEntries()[sprite3d.addDraftColorIndex].color = normalizeSpriteColor(color);
    sprite3d.selectedColorIndex = sprite3d.addDraftColorIndex;
    renderSprite3dColorSurfaces();
  }
  if (options.closeMenu) {
    sprite3d.addPaletteOpen = false;
    sprite3d.editPaletteOpen = false;
    sprite3d.customColorOpen = false;
    sprite3d.addDraftColorIndex = null;
    renderSprite3dBuilder();
  }
  if (options.deferHistory) {
    return;
  }
  pushVisualEditUndoSnapshot("sprite3d", before);
}

function updateSelectedSprite3dColor(value, options = {}) {
  const before = options.deferHistory || options.commitHistory ? null : visualEditSnapshot("sprite3d");
  if (options.deferHistory || options.commitHistory) {
    beginSpriteColorEditHistory("sprite3d");
  }
  if (!validSprite3dColorIndex(sprite3d.selectedColorIndex)) {
    sprite3d.selectedColorIndex = 0;
  }
  const selected = sprite3dPaletteEntries()[sprite3d.selectedColorIndex];
  if (!selected) {
    return;
  }
  selected.color = normalizeSpriteColor(value);
  if (options.closeMenu) {
    sprite3d.editPaletteOpen = false;
    sprite3d.customColorOpen = false;
    sprite3d.addDraftColorIndex = null;
    renderSprite3dBuilder();
    if (options.deferHistory || options.commitHistory) {
      commitSpriteColorEditHistory("sprite3d");
    } else {
      pushVisualEditUndoSnapshot("sprite3d", before);
    }
    return;
  }
  renderSprite3dColorSurfaces();
  if (options.deferHistory) {
    return;
  }
  if (options.commitHistory) {
    commitSpriteColorEditHistory("sprite3d");
    return;
  }
  pushVisualEditUndoSnapshot("sprite3d", before);
}

function closeSprite3dColorEditor() {
  commitSpriteColorEditHistory("sprite3d");
  sprite3d.addPaletteOpen = false;
  sprite3d.editPaletteOpen = false;
  sprite3d.customColorOpen = false;
  sprite3d.addDraftColorIndex = null;
  renderSprite3dPalette();
}

function cancelSprite3dColorAdd() {
  discardSpriteColorEditHistory("sprite3d");
  const before = visualEditSnapshot("sprite3d");
  if (validSprite3dColorIndex(sprite3d.addDraftColorIndex)) {
    removeSprite3dPaletteColor(sprite3d.addDraftColorIndex);
  }
  sprite3d.addPaletteOpen = false;
  sprite3d.editPaletteOpen = false;
  sprite3d.customColorOpen = false;
  sprite3d.addDraftColorIndex = null;
  renderSprite3dBuilder();
  pushVisualEditUndoSnapshot("sprite3d", before);
}

function removeSprite3dColor() {
  commitSpriteColorEditHistory("sprite3d");
  const before = visualEditSnapshot("sprite3d");
  const deletedIndex = sprite3d.selectedColorIndex;
  const palette = sprite3dPaletteEntries();
  if (!validSprite3dColorIndex(deletedIndex) || palette.length <= 1) {
    return;
  }
  sprite3d.addPaletteOpen = false;
  sprite3d.editPaletteOpen = false;
  sprite3d.customColorOpen = false;
  sprite3d.addDraftColorIndex = null;
  removeSprite3dPaletteColor(deletedIndex);
  renderSprite3dBuilder();
  pushVisualEditUndoSnapshot("sprite3d", before);
}

function removeSprite3dPaletteColor(deletedIndex) {
  const palette = sprite3dPaletteEntries();
  if (!validSprite3dColorIndex(deletedIndex) || palette.length <= 1) {
    return;
  }
  const oldPaletteLength = palette.length;
  palette.splice(deletedIndex, 1);
  sprite3d.cells = sprite3d.cells.map((colorIndex) => {
    if (!Number.isInteger(colorIndex) || colorIndex < 0 || colorIndex >= oldPaletteLength) {
      return null;
    }
    if (colorIndex === deletedIndex) {
      return null;
    }
    return colorIndex > deletedIndex ? colorIndex - 1 : colorIndex;
  });
  sprite3d.selectedColorIndex = Math.min(deletedIndex, palette.length - 1);
}

function renderSprite3dColorSurfaces() {
  syncSprite3dPaletteSwatches();
  syncSprite3dColorAdjusters();
  renderSprite3dSliceBoard();
  renderSprite3dPreview();
}

function syncSprite3dPaletteSwatches() {
  for (const [index, entry] of sprite3dPaletteEntries().entries()) {
    const color = normalizeSpriteColor(entry.color);
    for (const token of sprite3dPalette.querySelectorAll(`[data-color-index="${index}"]`)) {
      token.style.setProperty("--sprite-swatch-color", color);
      token.style.setProperty("--sprite-token-ink", readableInkForColor(color));
      token.title = `Paint ${color}`;
    }
  }
  const selected = sprite3dPaletteEntries()[sprite3d.selectedColorIndex];
  const currentButton = sprite3dPalette.querySelector(".sprite-current-color-button");
  if (currentButton && selected) {
    const normalized = normalizeSpriteColor(selected.color);
    currentButton.style.setProperty("--sprite-current-color", normalized);
    currentButton.setAttribute("aria-label", `Pick selected color ${normalized}`);
    const currentHexInput = sprite3dPalette.querySelector(".sprite-current-hex-input");
    if (currentHexInput && document.activeElement !== currentHexInput) {
      currentHexInput.value = normalized;
    }
  }
}

function syncSprite3dColorAdjusters() {
  const selected = validSprite3dColorIndex(sprite3d.selectedColorIndex)
    ? sprite3dPaletteEntries()[sprite3d.selectedColorIndex]
    : null;
  if (!selected) {
    return;
  }
  const normalized = normalizeSpriteColor(selected.color);
  for (const adjuster of sprite3dPalette.querySelectorAll(".sprite-color-adjuster")) {
    if (adjuster.contains(document.activeElement)) {
      continue;
    }
    adjuster.syncColor?.(normalized);
  }
}

function validSprite3dColorIndex(index) {
  return Number.isInteger(index) && index >= 0 && index < sprite3dPaletteEntries().length;
}

function normalizedSprite3dCellColorIndex(index) {
  const colorIndex = sprite3d.cells[index];
  return validSprite3dColorIndex(colorIndex) ? colorIndex : null;
}

function sprite3dColorForColorIndex(index) {
  return validSprite3dColorIndex(index) ? normalizeSpriteColor(sprite3dPaletteEntries()[index].color) : "#00000000";
}

function sprite3dInkForColorIndex(index) {
  return validSprite3dColorIndex(index) ? readableInkForColor(sprite3dPaletteEntries()[index].color) : "#8d969f";
}

function sprite3dPaletteEntries() {
  ensureSprite3dPalette();
  return sprite3d.palette;
}

function ensureSprite3dPalette() {
  if (!Array.isArray(sprite3d.palette)) {
    sprite3d.palette = [];
  }
}

function sprite3dCellIndex(x, y, z) {
  return ((z * sprite3d.height + y) * sprite3d.width) + x;
}

function sprite3dCoordsFromSliceCell(index) {
  const { width } = sprite3dPlaneSize();
  const u = index % width;
  const v = Math.floor(index / width);
  return sprite3dCoordsFromSliceUv(u, v);
}

function sprite3dCoordsFromSliceUv(u, v) {
  return sprite3dCoordsFromPlane(sprite3d.axis, sprite3d.slice, u, v);
}

function sprite3dSliceCellIndexFromElement(element) {
  const cell = element?.closest?.(".sprite-cell");
  if (!cell || !sprite3dSliceBoard.contains(cell)) {
    return -1;
  }
  const index = Number(cell.dataset.index);
  const plane = sprite3dPlaneSize();
  return Number.isInteger(index) && index >= 0 && index < plane.width * plane.height ? index : -1;
}

function paintSprite3dCellAtSliceIndex(index, colorIndex) {
  const plane = sprite3dPlaneSize();
  if (!Number.isInteger(index) || index < 0 || index >= plane.width * plane.height) {
    return false;
  }
  const coords = sprite3dCoordsFromSliceCell(index);
  const voxelIndex = sprite3dCellIndex(coords.x, coords.y, coords.z);
  const nextColorIndex = validSprite3dColorIndex(colorIndex) ? colorIndex : null;
  if (sprite3d.cells[voxelIndex] === nextColorIndex) {
    return false;
  }
  sprite3d.cells[voxelIndex] = nextColorIndex;
  renderSprite3dSliceBoard();
  renderSprite3dPreview();
  return true;
}

function floodFillSprite3dSliceComponentAtIndex(index, colorIndex) {
  const plane = sprite3dPlaneSize();
  if (!Number.isInteger(index) || index < 0 || index >= plane.width * plane.height) {
    return 0;
  }
  const startCoords = sprite3dCoordsFromSliceCell(index);
  const startVoxelIndex = sprite3dCellIndex(startCoords.x, startCoords.y, startCoords.z);
  const nextColorIndex = validSprite3dColorIndex(colorIndex) ? colorIndex : null;
  const targetColorIndex = normalizedSprite3dCellColorIndex(startVoxelIndex);
  if (targetColorIndex === nextColorIndex) {
    return 0;
  }
  const { width, height } = plane;
  const visited = new Uint8Array(width * height);
  const region = sprite3dClipActive ? normalizeSprite3dClipBox(sprite3dClipSelection) : null;
  const stack = [index];
  let changed = 0;
  while (stack.length) {
    const current = stack.pop();
    if (visited[current]) {
      continue;
    }
    const coords = sprite3dCoordsFromSliceCell(current);
    if (region && !sprite3dClipBoxContainsCoords(region, coords)) {
      continue;
    }
    const voxelIndex = sprite3dCellIndex(coords.x, coords.y, coords.z);
    if (normalizedSprite3dCellColorIndex(voxelIndex) !== targetColorIndex) {
      continue;
    }
    visited[current] = 1;
    sprite3d.cells[voxelIndex] = nextColorIndex;
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

function floodFillSprite3dComponentAtSliceIndex(index, colorIndex) {
  const plane = sprite3dPlaneSize();
  if (!Number.isInteger(index) || index < 0 || index >= plane.width * plane.height) {
    return 0;
  }
  const startCoords = sprite3dCoordsFromSliceCell(index);
  const startVoxelIndex = sprite3dCellIndex(startCoords.x, startCoords.y, startCoords.z);
  const nextColorIndex = validSprite3dColorIndex(colorIndex) ? colorIndex : null;
  const targetColorIndex = normalizedSprite3dCellColorIndex(startVoxelIndex);
  if (targetColorIndex === nextColorIndex) {
    return 0;
  }
  const visited = new Uint8Array(sprite3d.cells.length);
  const region = sprite3dClipActive ? normalizeSprite3dClipBox(sprite3dClipSelection) : null;
  const stack = [startCoords];
  let changed = 0;
  while (stack.length) {
    const current = stack.pop();
    if (
      current.x < 0 || current.y < 0 || current.z < 0
      || current.x >= sprite3d.width || current.y >= sprite3d.height || current.z >= sprite3d.depth
    ) {
      continue;
    }
    if (region && !sprite3dClipBoxContainsCoords(region, current)) {
      continue;
    }
    const voxelIndex = sprite3dCellIndex(current.x, current.y, current.z);
    if (visited[voxelIndex] || normalizedSprite3dCellColorIndex(voxelIndex) !== targetColorIndex) {
      continue;
    }
    visited[voxelIndex] = 1;
    sprite3d.cells[voxelIndex] = nextColorIndex;
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

function bucketFillSprite3dFromSliceIndex(index) {
  const plane = sprite3dPlaneSize();
  if (!Number.isInteger(index) || index < 0 || index >= plane.width * plane.height) {
    return false;
  }
  if (sprite3dClipActive && !normalizeSprite3dClipBox(sprite3dClipSelection)) {
    setSprite3dActionStatus("Select a clip region before bucket fill", "is-error");
    return false;
  }
  const startCoords = sprite3dCoordsFromSliceCell(index);
  if (sprite3dClipActive && !sprite3dClipBoxContainsCoords(sprite3dClipSelection, startCoords)) {
    setSprite3dActionStatus("Bucket fill start must be inside the clip region", "is-error");
    return false;
  }
  const colorIndex = sprite3d.selectedColorIndex;
  const allScope = sprite3dEditScope() === "all";
  const count = allScope
    ? floodFillSprite3dComponentAtSliceIndex(index, colorIndex)
    : floodFillSprite3dSliceComponentAtIndex(index, colorIndex);
  if (!count) {
    setSprite3dActionStatus("Connected component already has that color", "is-ok");
    deactivateSprite3dBucketModeAfterUse();
    return true;
  }
  sprite3d.addPaletteOpen = false;
  sprite3d.editPaletteOpen = false;
  sprite3d.customColorOpen = false;
  sprite3d.addDraftColorIndex = null;
  sprite3d.hoverSlice = null;
  deactivateSprite3dBucketModeAfterUse();
  renderSprite3dBuilder();
  const nextColorIndex = validSprite3dColorIndex(colorIndex) ? colorIndex : null;
  const message = nextColorIndex === null
    ? allScope ? "Filled 3D component with empty voxels" : "Filled slice component with empty voxels"
    : allScope ? "Filled 3D component" : "Filled slice component";
  setSprite3dActionStatus(message, "is-ok");
  setStatus(message, "is-ok");
  return true;
}

function bucketFillSprite3dFromElement(element) {
  return bucketFillSprite3dFromSliceIndex(sprite3dSliceCellIndexFromElement(element));
}

function paintSprite3dCellFromElement(element) {
  return paintSprite3dCellAtSliceIndex(sprite3dSliceCellIndexFromElement(element), sprite3d.selectedColorIndex);
}

function startSprite3dClip(event) {
  event.preventDefault();
  const geometry = sprite3dSliceBoard.getBoundingClientRect();
  const cell = sprite3dClipCellFromClient(event.clientX, event.clientY, geometry);
  if (!cell) {
    return;
  }
  const resizeHandle = !sprite3dClipFloating && sprite3dClipSelection
    ? event.target.closest?.("[data-sprite3d-clip-resize]")
    : null;
  if (resizeHandle) {
    sprite3dClipDrag = {
      mode: "resize",
      pointerId: event.pointerId,
      geometry,
      startCell: cell,
      originBox: sprite3dClipSelection,
      originRect: sprite3dClipPlaneRect(),
      edge: resizeHandle.dataset.sprite3dClipResize,
    };
  } else if (sprite3dClipSelectionContainsSliceCell(cell)) {
    sprite3dClipDrag = {
      mode: "move",
      pointerId: event.pointerId,
      geometry,
      startCell: cell,
      originBox: sprite3dClipSelection,
    };
  } else {
    const rect = sprite3dClipRectFromCells(cell, cell);
    sprite3dClipSelection = sprite3dClipBoxFromPlaneRect(rect, {
      fullDepth: sprite3dEditScope() === "all",
    });
    sprite3dClipFloating = null;
    sprite3dClipDrag = {
      mode: "select",
      pointerId: event.pointerId,
      geometry,
      startCell: cell,
      originBox: sprite3dClipSelection,
    };
  }
  sprite3dSliceBoard.setPointerCapture?.(event.pointerId);
  renderSprite3dSliceBoard();
}

function continueSprite3dClip(event) {
  if (!sprite3dClipDrag || sprite3dClipDrag.pointerId !== event.pointerId) {
    return false;
  }
  event.preventDefault();
  const cell = sprite3dClipCellFromClient(event.clientX, event.clientY, sprite3dClipDrag.geometry);
  if (!cell) {
    return true;
  }
  if (sprite3dClipDrag.mode === "select") {
    const rect = sprite3dClipRectFromCells(sprite3dClipDrag.startCell, cell);
    sprite3dClipSelection = sprite3dClipBoxFromPlaneRect(rect, {
      base: sprite3dClipDrag.originBox,
      fullDepth: sprite3dEditScope() === "all",
    });
  } else if (sprite3dClipDrag.mode === "move") {
    const du = cell.x - sprite3dClipDrag.startCell.x;
    const dv = cell.y - sprite3dClipDrag.startCell.y;
    const next = sprite3dClipBoxShiftedInPlane(sprite3dClipDrag.originBox, du, dv);
    if (next) {
      sprite3dClipSelection = next;
    }
  } else if (sprite3dClipDrag.mode === "resize") {
    const rect = sprite3dClipResizeRect(sprite3dClipDrag.originRect, sprite3dClipDrag.edge, cell);
    const next = sprite3dClipBoxFromPlaneRect(rect, { base: sprite3dClipDrag.originBox });
    if (next) {
      sprite3dClipSelection = next;
    }
  }
  renderSprite3dSliceBoard();
  renderSprite3dPreview();
  return true;
}

function stopSprite3dClip(event) {
  if (!sprite3dClipDrag || sprite3dClipDrag.pointerId !== event.pointerId) {
    return false;
  }
  if (sprite3dSliceBoard.hasPointerCapture?.(event.pointerId)) {
    sprite3dSliceBoard.releasePointerCapture(event.pointerId);
  }
  event.preventDefault();
  const mode = sprite3dClipDrag.mode;
  sprite3dClipDrag = null;
  sprite3dClipSelection = normalizeSprite3dClipBox(sprite3dClipSelection);
  renderSprite3dBuilder();
  const dimensions = sprite3dClipBoxDimensions();
  if (dimensions) {
    const verb = mode === "move" ? "Clip range moved" : mode === "resize" ? "Clip range resized" : "Clip range selected";
    setSprite3dActionStatus(`${verb} ${dimensions.width}x${dimensions.height}x${dimensions.depth}`, "is-ok");
  }
  return true;
}

function handleSprite3dClipKeyboard(event) {
  if (currentPreviewMode !== "sprite3d" || sprite3dBuilder.hidden
    || spriteClipShortcutTargetIsText(event.target)) {
    return false;
  }
  const key = event.key.length === 1 ? event.key.toLowerCase() : event.key;
  const modifier = (event.metaKey && !event.ctrlKey) || (event.ctrlKey && !event.metaKey);
  let handled = false;
  if (modifier && !event.altKey && !event.shiftKey && key === "c") {
    handled = runSprite3dEditCommand("copy");
  } else if (modifier && !event.altKey && !event.shiftKey && key === "x") {
    handled = runSprite3dEditCommand("cut");
  } else if (modifier && !event.altKey && !event.shiftKey && key === "v") {
    handled = runSprite3dEditCommand("paste");
  } else if (!modifier && !event.altKey && (key === "Backspace" || key === "Delete")) {
    handled = runSprite3dEditCommand("delete");
  } else if (sprite3dClipActive && !modifier && !event.altKey && key === "Escape") {
    deactivateSprite3dClipMode();
    setSprite3dActionStatus("Brush: paint individual voxels", "is-ok");
    handled = true;
  } else if (sprite3dClipActive && !modifier && !event.altKey && ["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(key)) {
    const du = key === "ArrowLeft" ? -1 : key === "ArrowRight" ? 1 : 0;
    const dv = key === "ArrowUp" ? -1 : key === "ArrowDown" ? 1 : 0;
    const next = sprite3dClipBoxShiftedInPlane(sprite3dClipSelection, du, dv);
    if (!next) {
      setSprite3dActionStatus("Clip must stay inside 3D sprite", "is-error");
      handled = true;
    } else {
      sprite3dClipSelection = next;
      renderSprite3dBuilder();
      setSprite3dActionStatus("Clip range moved", "is-ok");
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

function startSprite3dPaint(event) {
  if (event.button !== 0) {
    return;
  }
  if (sprite3dClipActive) {
    startSprite3dClip(event);
    return;
  }
  if (sprite3dTranslateActive) {
    startSprite3dTranslate(event);
    return;
  }
  const index = sprite3dSliceCellIndexFromElement(document.elementFromPoint(event.clientX, event.clientY));
  if (!Number.isInteger(index) || index < 0) {
    return;
  }
  event.preventDefault();
  if (sprite3dBucketActive) {
    const before = visualEditSnapshot("sprite3d");
    if (bucketFillSprite3dFromSliceIndex(index)) {
      pushVisualEditUndoSnapshot("sprite3d", before);
    }
    return;
  }
  sprite3dPaintDrag = {
    pointerId: event.pointerId,
    colorIndex: sprite3d.selectedColorIndex,
    lastIndex: -1,
    beforeSnapshot: visualEditSnapshot("sprite3d"),
    changed: false,
  };
  sprite3dSliceBoard.setPointerCapture?.(event.pointerId);
  paintSprite3dDragIndex(index);
}

function continueSprite3dPaint(event) {
  if (continueSprite3dClip(event)) {
    return;
  }
  if (continueSprite3dTranslate(event)) {
    return;
  }
  if (!sprite3dPaintDrag || sprite3dPaintDrag.pointerId !== event.pointerId) {
    return;
  }
  event.preventDefault();
  paintSprite3dDragIndex(sprite3dSliceCellIndexFromElement(document.elementFromPoint(event.clientX, event.clientY)));
}

function stopSprite3dPaint(event) {
  if (stopSprite3dClip(event)) {
    return;
  }
  if (stopSprite3dTranslate(event)) {
    return;
  }
  if (!sprite3dPaintDrag || sprite3dPaintDrag.pointerId !== event.pointerId) {
    return;
  }
  if (sprite3dSliceBoard.hasPointerCapture?.(event.pointerId)) {
    sprite3dSliceBoard.releasePointerCapture(event.pointerId);
  }
  if (sprite3dPaintDrag.changed) {
    pushVisualEditUndoSnapshot("sprite3d", sprite3dPaintDrag.beforeSnapshot);
  }
  sprite3dPaintDrag = null;
}

function paintSprite3dDragIndex(index) {
  if (!sprite3dPaintDrag || !Number.isInteger(index) || index < 0 || index === sprite3dPaintDrag.lastIndex) {
    return;
  }
  const plane = sprite3dPlaneSize();
  const centerU = (index % plane.width) + 0.5;
  const centerV = Math.floor(index / plane.width) + 0.5;
  const diameter = spriteBrushDiameterForSize(Math.min(plane.width, plane.height));
  const radius = diameter / 2;
  const minU = spriteBrushSizePx === 1 ? Math.floor(centerU) : Math.max(0, Math.floor(centerU - radius - 0.5));
  const maxU = spriteBrushSizePx === 1 ? minU : Math.min(plane.width - 1, Math.ceil(centerU + radius - 0.5));
  const minV = spriteBrushSizePx === 1 ? Math.floor(centerV) : Math.max(0, Math.floor(centerV - radius - 0.5));
  const maxV = spriteBrushSizePx === 1 ? minV : Math.min(plane.height - 1, Math.ceil(centerV + radius - 0.5));
  sprite3dPaintDrag.lastIndex = index;
  for (let v = minV; v <= maxV; v += 1) {
    for (let u = minU; u <= maxU; u += 1) {
      const dx = u + 0.5 - centerU;
      const dy = v + 0.5 - centerV;
      if (spriteBrushSizePx !== 1 && (dx * dx) + (dy * dy) > radius * radius) {
        continue;
      }
      if (paintSprite3dCellAtSliceIndex((v * plane.width) + u, sprite3dPaintDrag.colorIndex)) {
        sprite3dPaintDrag.changed = true;
      }
    }
  }
}

function updateSprite3dDimension(axis, value) {
  const before = visualEditSnapshot("sprite3d");
  const nextValue = clampSprite3dSize(value);
  const next = sprite3d.sizeBound
    ? { width: nextValue, height: nextValue, depth: nextValue }
    : {
        width: axis === "width" ? nextValue : sprite3d.width,
        height: axis === "height" ? nextValue : sprite3d.height,
        depth: axis === "depth" ? nextValue : sprite3d.depth,
      };
  if (next.width === sprite3d.width
    && next.height === sprite3d.height
    && next.depth === sprite3d.depth) {
    renderSprite3dControls();
    return;
  }
  remapSprite3dFrames(next, (x, y, z) => ({ x, y, z }));
  resetSprite3dClipState();
  sprite3d.slice = Math.min(sprite3d.slice, sprite3dAxisSize() - 1);
  renderSprite3dBuilder();
  pushVisualEditUndoSnapshot("sprite3d", before);
}

function remapSprite3dFrames(nextExtent, sourceCoordinates) {
  commitSprite3dActiveFrame();
  const previous = {
    width: sprite3d.width,
    height: sprite3d.height,
    depth: sprite3d.depth,
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
          next[((z * nextExtent.height + y) * nextExtent.width) + x] = validSprite3dColorIndex(colorIndex)
            ? colorIndex
            : null;
        }
      }
    }
    return next;
  };
  const frames = sprite3d.animationMode && sprite3d.frames.length
    ? sprite3d.frames
    : [sprite3d.cells];
  sprite3d.frames = frames.map(remap);
  sprite3d.width = nextExtent.width;
  sprite3d.height = nextExtent.height;
  sprite3d.depth = nextExtent.depth;
  sprite3d.animationFrameCount = sprite3d.frames.length;
  sprite3d.animationFrameIndex = Math.min(sprite3d.animationFrameIndex, sprite3d.frames.length - 1);
  sprite3d.animationPlaybackIndex = Math.min(sprite3d.animationPlaybackIndex, sprite3d.frames.length - 1);
  sprite3d.cells = sprite3d.frames[sprite3d.animationFrameIndex];
}

function sprite3dScaleFactor() {
  return spriteEditorScaleFactor(sprite3dScaleInput, SPRITE3D_EDITOR_MAX_SIZE);
}

function canScaleDownSprite3d(factor = sprite3dScaleFactor()) {
  return factor > 1
    && sprite3d.width >= factor
    && sprite3d.height >= factor
    && sprite3d.depth >= factor
    && sprite3d.width % factor === 0
    && sprite3d.height % factor === 0
    && sprite3d.depth % factor === 0;
}

function scaleUpSprite3d() {
  const before = visualEditSnapshot("sprite3d");
  const factor = sprite3dScaleFactor();
  const next = {
    width: sprite3d.width * factor,
    height: sprite3d.height * factor,
    depth: sprite3d.depth * factor,
  };
  if (Math.max(next.width, next.height, next.depth) > SPRITE3D_EDITOR_MAX_SIZE) {
    setSprite3dActionStatus(`3D sprite size limit is ${SPRITE3D_EDITOR_MAX_SIZE}`, "is-error");
    renderSprite3dControls();
    return;
  }

  remapSprite3dFrames(next, (x, y, z) => ({
    x: Math.floor(x / factor),
    y: Math.floor(y / factor),
    z: Math.floor(z / factor),
  }));
  resetSprite3dClipState();
  sprite3d.slice = Math.min(sprite3d.slice * factor, sprite3dAxisSize() - 1);
  sprite3d.hoverSlice = null;
  renderSprite3dBuilder();
  const message = `Scaled ${factor}x to ${next.width}x${next.height}x${next.depth}`;
  setSprite3dActionStatus(message, "is-ok");
  setStatus(`Scaled 3D sprite ${factor}x to ${next.width}x${next.height}x${next.depth}`, "is-ok");
  pushVisualEditUndoSnapshot("sprite3d", before);
}

function scaleDownSprite3d() {
  const before = visualEditSnapshot("sprite3d");
  const factor = sprite3dScaleFactor();
  if (!canScaleDownSprite3d(factor)) {
    setSprite3dActionStatus(`Dimensions ${sprite3d.width}x${sprite3d.height}x${sprite3d.depth} are not divisible by ${factor}`, "is-error");
    renderSprite3dControls();
    return;
  }

  const next = {
    width: sprite3d.width / factor,
    height: sprite3d.height / factor,
    depth: sprite3d.depth / factor,
  };
  remapSprite3dFrames(next, (x, y, z) => ({
    x: x * factor,
    y: y * factor,
    z: z * factor,
  }));
  resetSprite3dClipState();
  sprite3d.slice = Math.min(Math.floor(sprite3d.slice / factor), sprite3dAxisSize() - 1);
  sprite3d.hoverSlice = null;
  renderSprite3dBuilder();
  const message = `Scaled down ${factor}x to ${next.width}x${next.height}x${next.depth}`;
  setSprite3dActionStatus(message, "is-ok");
  setStatus(`Scaled 3D sprite down ${factor}x to ${next.width}x${next.height}x${next.depth}`, "is-ok");
  pushVisualEditUndoSnapshot("sprite3d", before);
}

function setSprite3dAxis(axis) {
  const nextAxis = ["x", "y", "z"].includes(axis) ? axis : "z";
  if (sprite3dClipSelection && sprite3dEditScope() === "slice" && nextAxis !== sprite3d.axis) {
    sprite3dClipSelection = null;
    sprite3dClipFloating = null;
    sprite3dClipDrag = null;
  }
  sprite3d.axis = nextAxis;
  sprite3d.slice = Math.min(sprite3d.slice, sprite3dAxisSize(nextAxis) - 1);
  sprite3d.hoverSlice = null;
  renderSprite3dBuilder();
}

function setSprite3dSlice(value) {
  const nextSlice = Math.max(0, Math.min(sprite3dAxisSize() - 1, Math.trunc(Number(value) || 0)));
  if (sprite3dClipSelection && sprite3dEditScope() === "slice" && nextSlice !== sprite3d.slice) {
    sprite3dClipSelection = null;
    sprite3dClipFloating = null;
    sprite3dClipDrag = null;
  }
  sprite3d.slice = nextSlice;
  renderSprite3dControls();
  renderSprite3dSliceBoard();
  renderSprite3dPreview();
}

function moveSprite3dSlice(delta) {
  setSprite3dSlice(sprite3d.slice + delta);
}

function applySprite3dSliceInput() {
  if (!(sprite3dSliceValue instanceof HTMLInputElement)) {
    return;
  }
  setSprite3dSlice(Math.trunc(Number(sprite3dSliceValue.value) || 1) - 1);
}

function sprite3dSliceScrubTarget(event) {
  return event.target?.closest?.("[data-sprite3d-slice-scrub]") || null;
}

function startSprite3dSliceScrub(event) {
  const target = sprite3dSliceScrubTarget(event);
  if (!target || event.button !== 0) {
    return;
  }
  event.preventDefault();
  event.stopPropagation();
  sprite3dSliceScrubDrag = {
    pointerId: event.pointerId,
    target,
    inputTarget: event.target === sprite3dSliceValue,
    startX: event.clientX,
    moved: false,
    slice: sprite3d.slice,
  };
  target.setPointerCapture?.(event.pointerId);
  target.classList.add("is-dragging");
  document.documentElement.classList.add("is-sprite3d-slice-scrubbing");
}

function continueSprite3dSliceScrub(event) {
  if (!sprite3dSliceScrubDrag || sprite3dSliceScrubDrag.pointerId !== event.pointerId) {
    return;
  }
  event.preventDefault();
  event.stopPropagation();
  const deltaX = event.clientX - sprite3dSliceScrubDrag.startX;
  if (Math.abs(deltaX) > 2) {
    sprite3dSliceScrubDrag.moved = true;
  }
  setSprite3dSlice(sprite3dSliceScrubDrag.slice + Math.round(deltaX / SPRITE3D_SLICE_SCRUB_STEP_PX));
}

function stopSprite3dSliceScrub(event) {
  if (!sprite3dSliceScrubDrag || sprite3dSliceScrubDrag.pointerId !== event.pointerId) {
    return;
  }
  event.preventDefault();
  event.stopPropagation();
  finishSprite3dSliceScrub(event.pointerId);
}

function finishSprite3dSliceScrub(pointerId = null) {
  if (!sprite3dSliceScrubDrag) {
    return;
  }
  const { target, inputTarget, moved } = sprite3dSliceScrubDrag;
  if (pointerId !== null && target.hasPointerCapture?.(pointerId)) {
    target.releasePointerCapture(pointerId);
  }
  target.classList.remove("is-dragging");
  document.documentElement.classList.remove("is-sprite3d-slice-scrubbing");
  sprite3dSliceScrubDrag = null;
  if (!moved && inputTarget && sprite3dSliceValue instanceof HTMLInputElement) {
    sprite3dSliceValue.focus();
    sprite3dSliceValue.select();
  }
}

function deleteSprite3dSlice() {
  const before = visualEditSnapshot("sprite3d");
  const plane = sprite3dPlaneSize();
  for (let index = 0; index < plane.width * plane.height; index += 1) {
    const coords = sprite3dCoordsFromSliceCell(index);
    sprite3d.cells[sprite3dCellIndex(coords.x, coords.y, coords.z)] = null;
  }
  renderSprite3dBuilder();
  setSprite3dActionStatus("Deleted current slice contents", "is-ok");
  pushVisualEditUndoSnapshot("sprite3d", before);
}

function deleteSprite3dBuilder() {
  const before = visualEditSnapshot("sprite3d");
  resetSprite3dBuilder();
  setSprite3dActionStatus("Deleted whole 3D sprite contents", "is-ok");
  pushVisualEditUndoSnapshot("sprite3d", before);
}

function deleteSprite3dScoped() {
  if (sprite3dEditScope() === "all") {
    deleteSprite3dBuilder();
  } else {
    deleteSprite3dSlice();
  }
}

function transformSprite3dCells(mapper, message) {
  const before = visualEditSnapshot("sprite3d");
  const previousCells = sprite3d.cells;
  const nextCells = Array.from({ length: sprite3dFrameCellCount() }, () => null);
  for (let z = 0; z < sprite3d.depth; z += 1) {
    for (let y = 0; y < sprite3d.height; y += 1) {
      for (let x = 0; x < sprite3d.width; x += 1) {
        const sourceIndex = sprite3dCellIndex(x, y, z);
        const colorIndex = previousCells[sourceIndex];
        if (!validSprite3dColorIndex(colorIndex)) {
          continue;
        }
        const target = mapper(x, y, z);
        nextCells[sprite3dCellIndex(target.x, target.y, target.z)] = colorIndex;
      }
    }
  }
  sprite3d.cells = nextCells;
  sprite3d.hoverSlice = null;
  renderSprite3dSliceBoard();
  renderSprite3dPreview();
  setSprite3dActionStatus(message, "is-ok");
  pushVisualEditUndoSnapshot("sprite3d", before);
}

function sprite3dPlaneCoordinates(axis, x, y, z) {
  const maxY = sprite3d.height - 1;
  const maxZ = sprite3d.depth - 1;
  if (axis === "x") {
    return { stack: x, u: maxY - y, v: maxZ - z };
  }
  if (axis === "y") {
    return { stack: maxY - y, u: x, v: maxZ - z };
  }
  return { stack: maxZ - z, u: x, v: maxY - y };
}

function sprite3dCoordsFromPlane(axis, stack, u, v) {
  const maxY = sprite3d.height - 1;
  const maxZ = sprite3d.depth - 1;
  const fixed = sprite3dPlaneWorldSlice(axis, stack);
  if (axis === "x") {
    return { x: fixed, y: maxY - u, z: maxZ - v };
  }
  if (axis === "y") {
    return { x: u, y: fixed, z: maxZ - v };
  }
  return { x: u, y: maxY - v, z: fixed };
}

function sprite3dPlaneWorldSlice(axis, stack) {
  const axisSize = sprite3dAxisSize(axis);
  const normalized = Math.max(0, Math.min(axisSize - 1, Math.trunc(Number(stack) || 0)));
  return axis === "x" ? normalized : axisSize - 1 - normalized;
}

function sprite3dCurrentSliceDescriptor() {
  return {
    axis: ["x", "y", "z"].includes(sprite3d.axis) ? sprite3d.axis : "z",
    slice: Math.max(0, Math.min(sprite3dAxisSize() - 1, Math.trunc(Number(sprite3d.slice) || 0))),
  };
}

function readSprite3dSliceCells(axis, slice) {
  const cells = [];
  const plane = sprite3dPlaneSize(axis);
  for (let v = 0; v < plane.height; v += 1) {
    for (let u = 0; u < plane.width; u += 1) {
      const source = sprite3dCoordsFromPlane(axis, slice, u, v);
      cells.push(sprite3d.cells[sprite3dCellIndex(source.x, source.y, source.z)] ?? null);
    }
  }
  return cells;
}

function sprite3dPaletteColors() {
  return sprite3dPaletteEntries().map((entry) => normalizeSpriteColor(entry.color));
}

function sprite3dSliceCellColors(cells) {
  const paletteColors = sprite3dPaletteColors();
  return cells.map((colorIndex) => (
    Number.isInteger(colorIndex) && colorIndex >= 0 && colorIndex < paletteColors.length
      ? paletteColors[colorIndex]
      : null
  ));
}

function sprite3dClipboardPaletteColor(entry) {
  return parseSpriteHexColor(typeof entry === "string" ? entry : entry?.color);
}

function sprite3dClipboardCellColors(copied) {
  if (Array.isArray(copied.colors)) {
    return copied.colors.map((color) => parseSpriteHexColor(color) || null);
  }
  if (!Array.isArray(copied.palette) || !Array.isArray(copied.cells)) {
    return null;
  }
  const paletteColors = copied.palette.map(sprite3dClipboardPaletteColor);
  return copied.cells.map((colorIndex) => (
    Number.isInteger(colorIndex) && colorIndex >= 0 && colorIndex < paletteColors.length
      ? paletteColors[colorIndex]
      : null
  ));
}

function sprite3dPastedSliceCells(copied, targetSize) {
  const cellCount = targetSize * targetSize;
  const colors = sprite3dClipboardCellColors(copied);
  if (!colors) {
    return {
      cells: copied.cells.map((colorIndex) => (validSprite3dColorIndex(colorIndex) ? colorIndex : null)),
      addedColors: 0,
    };
  }
  if (colors.length !== cellCount) {
    return { error: "Copied slice color data is incomplete" };
  }
  const palette = sprite3dPaletteEntries();
  const colorToIndex = new Map(palette.map((entry, index) => [normalizeSpriteColor(entry.color), index]));
  const missingColors = [];
  for (const color of colors) {
    if (!color || color === "#00000000" || colorToIndex.has(color) || missingColors.includes(color)) {
      continue;
    }
    missingColors.push(color);
  }
  if (palette.length + missingColors.length > SPRITE_COLOR_TOKENS.length) {
    return {
      error: `Paste needs ${missingColors.length} more colors, but the 3D sprite palette has ${SPRITE_COLOR_TOKENS.length - palette.length} slots`,
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

function writeSprite3dSliceCells(axis, slice, cells) {
  const plane = sprite3dPlaneSize(axis);
  for (let v = 0; v < plane.height; v += 1) {
    for (let u = 0; u < plane.width; u += 1) {
      const colorIndex = cells[(v * plane.width) + u];
      const target = sprite3dCoordsFromPlane(axis, slice, u, v);
      sprite3d.cells[sprite3dCellIndex(target.x, target.y, target.z)] = validSprite3dColorIndex(colorIndex)
        ? colorIndex
        : null;
    }
  }
}

function transformSprite3dCurrentPlane(mapper, message) {
  const axis = ["x", "y", "z"].includes(sprite3d.axis) ? sprite3d.axis : "z";
  const plane = sprite3dPlaneSize(axis);
  transformSprite3dCells((x, y, z) => {
    const plane = sprite3dPlaneCoordinates(axis, x, y, z);
    const next = mapper(plane.u, plane.v, sprite3dPlaneSize(axis).width, sprite3dPlaneSize(axis).height);
    return sprite3dCoordsFromPlane(axis, plane.stack, next.u, next.v);
  }, `${message} all ${axis.toUpperCase()} slices`);
}

function transformSprite3dCurrentSlice(mapper, message) {
  const before = visualEditSnapshot("sprite3d");
  const source = sprite3dCurrentSliceDescriptor();
  const previousCells = readSprite3dSliceCells(source.axis, source.slice);
  const plane = sprite3dPlaneSize(source.axis);
  const nextCells = Array.from({ length: plane.width * plane.height }, () => null);
  for (let v = 0; v < plane.height; v += 1) {
    for (let u = 0; u < plane.width; u += 1) {
      const colorIndex = previousCells[(v * plane.width) + u];
      if (!validSprite3dColorIndex(colorIndex)) {
        continue;
      }
      const next = mapper(u, v, plane.width, plane.height);
      nextCells[(next.v * plane.width) + next.u] = colorIndex;
    }
  }
  writeSprite3dSliceCells(source.axis, source.slice, nextCells);
  sprite3d.hoverSlice = null;
  renderSprite3dSliceBoard();
  renderSprite3dPreview();
  setSprite3dActionStatus(`${message} ${source.axis.toUpperCase()} slice ${source.slice + 1}`, "is-ok");
  pushVisualEditUndoSnapshot("sprite3d", before);
}

function transformSprite3dScoped(mapper, message) {
  if (sprite3dEditScope() === "all") {
    transformSprite3dCurrentPlane(mapper, message);
  } else {
    transformSprite3dCurrentSlice(mapper, message);
  }
}

function rotateSprite3dPlaneLeft() {
  const plane = sprite3dPlaneSize();
  if (plane.width !== plane.height) {
    setSprite3dActionStatus("Rotate requires a square edit plane", "is-error");
    return;
  }
  transformSprite3dScoped((u, v, width) => ({ u: v, v: width - 1 - u }), "Rotated left");
}

function rotateSprite3dPlaneRight() {
  const plane = sprite3dPlaneSize();
  if (plane.width !== plane.height) {
    setSprite3dActionStatus("Rotate requires a square edit plane", "is-error");
    return;
  }
  transformSprite3dScoped((u, v, width) => ({ u: width - 1 - v, v: u }), "Rotated right");
}

function flipSprite3dPlaneHorizontal() {
  transformSprite3dScoped((u, v, width) => ({ u: width - 1 - u, v }), "Flipped horizontal");
}

function flipSprite3dPlaneVertical() {
  transformSprite3dScoped((u, v, width, height) => ({ u, v: height - 1 - v }), "Flipped vertical");
}

function sprite3dObjectName() {
  const cleaned = String(sprite3dNameInput?.value || "")
    .trim()
    .replace(/[^\w:@]+/g, "_")
    .replace(/(?!^)@/g, "_")
    .replace(/^_+|_+$/g, "");
  return cleaned || "VoxelSprite";
}

function sprite3dClipboardText() {
  return sprite3dObjectDefinitionText("");
}

function sprite3dObjectDefinitionText(indent, name = sprite3dObjectName()) {
  const normalizedIndent = sprite3dSourceIndent(indent);
  const lines = [
    `${normalizedIndent}${name} {`,
    `${normalizedIndent}colors = ${sprite3dPaletteSourceTokens().join(" ")}`,
    `${normalizedIndent}shape = {`,
    ...sprite3dVoxelRows().map((row) => `${normalizedIndent}${row}`),
    `${normalizedIndent}}`,
    `${normalizedIndent}}`,
  ];
  return lines.join("\n");
}

function sprite3dPaletteSourceTokens() {
  return sprite3dPaletteEntries().map((entry) => sprite3dPaletteSourceToken(entry));
}

function sprite3dPaletteSourceToken(entry) {
  const bind = spritePaletteEntryBindInfo(entry);
  if (bind.linked && bind.name) {
    return bind.name;
  }
  const color = normalizeSpriteColor(entry?.color || "#00000000");
  return color === "#00000000" ? "transparent" : color;
}

function sprite3dVoxelRows() {
  const rows = [];
  for (let z = 0; z < sprite3d.depth; z += 1) {
    if (z > 0) {
      rows.push("-");
    }
    for (let y = 0; y < sprite3d.height; y += 1) {
      const row = [];
      for (let x = 0; x < sprite3d.width; x += 1) {
        const coords = sprite3dCoordsFromPlane("z", z, x, y);
        const colorIndex = sprite3d.cells[sprite3dCellIndex(coords.x, coords.y, coords.z)];
        row.push(validSprite3dColorIndex(colorIndex) ? SPRITE_COLOR_TOKENS[colorIndex] : ".");
      }
      rows.push(row.join(""));
    }
  }
  return rows;
}

function sprite3dEditFrames() {
  commitSprite3dActiveFrame();
  const frames = Array.isArray(sprite3d.frames) && sprite3d.frames.length
    ? sprite3d.frames.map((frame) => Array.isArray(frame) ? frame.slice() : [])
    : [[]];
  frames[sprite3d.animationMode ? sprite3d.animationFrameIndex : 0] = sprite3d.cells.slice();
  return frames.map((frame) => Array.from({ length: sprite3d.depth }, (_, sourceZ) =>
    Array.from({ length: sprite3d.height }, (_, y) =>
      Array.from({ length: sprite3d.width }, (_, x) => {
        const worldZ = sprite3d.depth - 1 - sourceZ;
        const cell = frame[sprite3dCellIndex(x, y, worldZ)];
        return Number.isInteger(cell) ? cell : null;
      }))));
}

function sprite3dEditMutationRequest(operation, options = {}) {
  const shape = spriteAssetBindInfo(sprite3d.shapeBind, "shape");
  const colorBindings = sprite3d.palette
    .map((entry) => ({ entry, bind: spritePaletteEntryBindInfo(entry) }))
    .filter(({ bind }) => bind.linked && bind.name)
    .map(({ entry, bind }) => ({ name: bind.name, color: normalizeSpriteColor(entry.color) }));
  return {
    operation,
    dimension: "3d",
    name: options.name ?? sprite3dObjectName(),
    originalName: options.originalName ?? sprite3d.editSourceName ?? sprite3dObjectName(),
    cursor: options.cursor,
    palette: sprite3dPaletteSourceTokens(),
    frames: sprite3dEditFrames(),
    durationMs: sprite3d.animationMode ? normalizedSprite3dAnimationDuration() : null,
    frameDurationMs: sprite3d.animationMode ? sprite3d.frameDurationMs : null,
    shapeRef: shape.linked ? shape.name : null,
    spatialOps: sprite3d.sourceSpatialOps || [],
    colorBindings,
  };
}

async function updateSprite3dInSource() {
  try {
    await commitSpriteEditorMutation({
      state: sprite3d,
      request: () => sprite3dEditMutationRequest("update"),
    });
  } catch (error) {
    setSprite3dActionStatus("No selected 3D sprite source range", "is-error");
    setStatus("No selected 3D sprite source range", "is-error");
    setSprite3dActionStatus(userFacingRuntimeError(error), "is-error");
    return;
  }
  setSprite3dActionStatus("Updated 3D sprite", "is-ok");
  setStatus("Updated 3D sprite", "is-ok");
  syncSprite3dSourceActionButtons();
}

async function addSprite3dToSource() {
  let result;
  try {
    ({ result } = await commitSpriteEditorMutation({
      state: sprite3d,
      allowActiveDocument: true,
      request: (source, document) => sprite3dEditMutationRequest(
        canReplaceCurrentSprite3dDefinition(source) ? "duplicate" : "insert",
        { cursor: spriteSourceCursorPosition(source, document) },
      ),
    }));
  } catch (error) {
    setSprite3dActionStatus(userFacingRuntimeError(error), "is-error");
    return;
  }
  sprite3dNameInput.value = result.name;
  setSprite3dActionStatus("Added 3D sprite", "is-ok");
  setStatus("Added 3D sprite", "is-ok");
  syncSprite3dSourceActionButtons();
}

function newSprite3dDraft() {
  const before = visualEditSnapshot("sprite3d");
  clearSprite3dEditSource();
  sprite3dNameInput.value = "VoxelSprite";
  sprite3d.palette = [{ color: "#ff004d" }];
  sprite3d.selectedColorIndex = 0;
  sprite3d.animationMode = false;
  resetSprite3dBuilder(5, 5, 5);
  setSprite3dActionStatus("Started new 3D sprite", "is-ok");
  pushVisualEditUndoSnapshot("sprite3d", before);
}

function activeSprite3dEditDocument() {
  return spriteEditorOwnedDocument(sprite3d);
}

function activeSprite3dEditSource() {
  return spriteEditorSourceSnapshot(sprite3d).source;
}

const SPRITE3D_SOURCE_INDENT = "";

function sprite3dSourceIndent(indent = "") {
  return String(indent || "").replace(/\t/g, SPRITE3D_SOURCE_INDENT);
}

async function loadSprite3dFromSourcePosition(position, options = {}) {
  if (!isPuzzleDocument(activeDocument()) || !isTextDocument(activeDocument())) {
    return null;
  }
  const source = sourceEditor.value || "";
  if (typeof resolveSourceTargetFromWasm !== "function") {
    return null;
  }
  const target = await resolveSourceTargetFromWasm(source, position);
  if (!sourceTargetMatches(target, "sprite", "3d")) {
    return null;
  }
  return loadSprite3dSourceTarget(target, options);
}

function loadSprite3dSourceTarget(target, options = {}) {
  if (!isPuzzleDocument(activeDocument()) || !isTextDocument(activeDocument())) {
    return null;
  }
  if (!Number.isInteger(target?.bodyStart) || !Number.isInteger(target?.bodyEnd)) {
    return null;
  }
  const loaded = sprite3dTargetPayload(target);
  if (!loaded) {
    if (target?.sourceSprite?.dimension === "3d" && target.sourceSprite.status === "incomplete") {
      applyIncompleteSprite3dSourceTarget(target.name || "", target);
      if (!options.silent) {
        setSprite3dActionStatus(`Loaded unfinished ${sprite3dNameInput.value || "3D sprite"}`, "is-ok");
        setStatus(`Loaded unfinished 3D sprite ${sprite3dNameInput.value || ""}`.trim(), "is-ok");
      }
      return `sprite3d:${target.name}:${target.start ?? target.bodyStart}`;
    } else if (!options.silent) {
      setSprite3dActionStatus("No editable 3D sprite here", "is-error");
    }
    return null;
  }
  if (options.recordHistory && typeof pushSourceNavigationHistory === "function") {
    pushSourceNavigationHistory();
  }
  if (options.switchMode && currentPreviewMode !== "sprite3d") {
    setPreviewMode("sprite3d");
  }
  setSprite3dEditSource(target, activeDocument());
  applyLoadedSprite3d(target.name || "VoxelSprite", loaded);
  if (!options.silent) {
    setSprite3dActionStatus(`Loaded ${sprite3dNameInput.value}`, "is-ok");
    setStatus(`Loaded 3D sprite ${sprite3dNameInput.value}`, "is-ok");
  }
  return `sprite3d:${target.name}:${target.start ?? target.bodyStart}`;
}

function sprite3dTargetPayload(target) {
  const payload = target?.sourceSprite?.dimension === "3d" ? target.sourceSprite : null;
  const documentContract = projectSpriteDocumentContract(payload);
  if (!documentContract || documentContract.dimension !== "3d") {
    return null;
  }
  const { width, height, depth } = documentContract.extent;
  const palette = documentContract.resolvedPalette
    .map((entry) => {
      const paletteEntry = { color: normalizeSpriteColor(entry?.color) };
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
    sourceSpatialOps: documentContract.spatialOps,
  };
}

function setSprite3dEditSource(target, document = activeDocument()) {
  setSpriteEditorSourceTarget(sprite3d, target, document);
  syncSprite3dSourceActionButtons();
}

function clearSprite3dEditSource() {
  clearSpriteEditorSourceTarget(sprite3d);
}

function invalidateSprite3dEditSourceForDocument(document = activeDocument()) {
  if (!document || !sprite3d.editDocumentId || document.id !== sprite3d.editDocumentId) {
    return false;
  }
  return invalidateSpriteEditorSourceTarget(sprite3d, document);
}

function canReplaceCurrentSprite3dDefinition(source) {
  return Boolean(currentSprite3dEditSourceRange(source));
}

function syncSprite3dSourceActionButtons() {
  const hasEditableSource = canReplaceCurrentSprite3dDefinition(activeSprite3dEditSource());
  if (sprite3dUpdateButton) {
    sprite3dUpdateButton.disabled = !hasEditableSource;
  }
  if (sprite3dInsertButton) {
    sprite3dInsertButton.disabled = false;
  }
}

function currentSprite3dEditSourceRange(source) {
  return spriteEditorSourceRange(sprite3d, source, sprite3dSourceIndent);
}

function applyIncompleteSprite3dSourceTarget(name, target) {
  resetSprite3dClipState({ clipboard: true });
  if (target && typeof target === "object") {
    setSprite3dEditSource(target, activeDocument());
  }
  sprite3dNameInput.value = name || "";
  sprite3d.width = clampSprite3dSize(sprite3d.width);
  sprite3d.height = clampSprite3dSize(sprite3d.height);
  sprite3d.depth = clampSprite3dSize(sprite3d.depth);
  sprite3d.axis = "z";
  sprite3d.slice = 0;
  sprite3d.hoverSlice = null;
  sprite3d.palette = [];
  sprite3d.cells = Array.from({ length: sprite3dFrameCellCount() }, () => null);
  sprite3d.frames = [sprite3d.cells.slice()];
  sprite3d.animationMode = false;
  sprite3d.animationFrameIndex = 0;
  sprite3d.animationFrameCount = 1;
  sprite3d.animationPlaybackIndex = 0;
  sprite3d.animationDurationMs = null;
  sprite3d.frameDurationMs = null;
  sprite3d.shapeBind = null;
  sprite3d.sourceSpatialOps = [];
  sprite3d.selectedColorIndex = null;
  sprite3d.addPaletteOpen = false;
  sprite3d.editPaletteOpen = false;
  sprite3d.customColorOpen = false;
  sprite3d.addDraftColorIndex = null;
  renderSprite3dBuilder();
}

function applyLoadedSprite3d(name, loaded) {
  resetSprite3dClipState({ clipboard: true });
  sprite3dNameInput.value = name || "VoxelSprite";
  sprite3d.width = loaded.width;
  sprite3d.height = loaded.height;
  sprite3d.depth = loaded.depth;
  sprite3d.axis = "z";
  sprite3d.slice = 0;
  sprite3d.hoverSlice = null;
  sprite3d.palette = loaded.palette;
  sprite3d.cells = loaded.cells;
  sprite3d.frames = loaded.frames;
  sprite3d.animationMode = loaded.frames.length > 1 || Number.isFinite(loaded.animationDurationMs);
  sprite3d.animationFrameIndex = 0;
  sprite3d.animationFrameCount = Math.max(1, loaded.frames.length);
  sprite3d.animationPlaybackIndex = 0;
  sprite3d.animationDurationMs = loaded.animationDurationMs;
  sprite3d.frameDurationMs = loaded.frameDurationMs;
  sprite3d.shapeBind = loaded.shapeBind;
  sprite3d.sourceSpatialOps = loaded.sourceSpatialOps;
  sprite3d.selectedColorIndex = sprite3d.palette.length ? 0 : null;
  sprite3d.addPaletteOpen = false;
  sprite3d.editPaletteOpen = false;
  sprite3d.customColorOpen = false;
  sprite3d.addDraftColorIndex = null;
  renderSprite3dBuilder();
}

function handleSprite3dSliceBoardShortcut(event) {
  if (!(event.metaKey || event.ctrlKey) || event.altKey || event.shiftKey) {
    return;
  }
  const key = event.key.toLowerCase();
  if (key === "c") {
    event.preventDefault();
    event.stopPropagation();
    runSprite3dEditCommand("copy");
  } else if (key === "v") {
    event.preventDefault();
    event.stopPropagation();
    runSprite3dEditCommand("paste");
  }
}

function resetSprite3dCamera() {
  sprite3d.camera = { ...SPRITE3D_CAMERA_DEFAULT };
  sprite3d.hoverSlice = null;
  renderSprite3dCameraControls();
  renderSprite3dPresentationSurfaces();
  setSprite3dActionStatus("Reset camera", "is-ok");
}

function sprite3dCameraScrubTarget(event) {
  return event.target?.closest?.("[data-sprite3d-camera]") || null;
}

function startSprite3dCameraScrub(event) {
  const target = sprite3dCameraScrubTarget(event);
  if (!target || event.button !== 0) {
    return;
  }
  event.preventDefault();
  event.stopPropagation();
  const kind = target.dataset.sprite3dCamera;
  sprite3dCameraScrubDrag = {
    pointerId: event.pointerId,
    target,
    kind,
    startX: event.clientX,
    startY: event.clientY,
    moved: false,
    value: sprite3dCameraValue(kind),
  };
  target.setPointerCapture?.(event.pointerId);
  target.classList.add("is-dragging");
  document.documentElement.classList.add("is-sprite3d-camera-scrubbing");
  document.documentElement.classList.add("is-vertical-scrubbing");
}

function continueSprite3dCameraScrub(event) {
  if (!sprite3dCameraScrubDrag || sprite3dCameraScrubDrag.pointerId !== event.pointerId) {
    return;
  }
  event.preventDefault();
  event.stopPropagation();
  const deltaY = sprite3dCameraScrubDrag.startY - event.clientY;
  if (Math.abs(deltaY) > 2) {
    sprite3dCameraScrubDrag.moved = true;
  }
  setSprite3dCameraValue(
    sprite3dCameraScrubDrag.kind,
    sprite3dCameraScrubDrag.value + deltaY * sprite3dCameraScrubScale(sprite3dCameraScrubDrag.kind),
  );
}

function stopSprite3dCameraScrub(event) {
  if (!sprite3dCameraScrubDrag || sprite3dCameraScrubDrag.pointerId !== event.pointerId) {
    return;
  }
  event.preventDefault();
  event.stopPropagation();
  finishSprite3dCameraScrub(event.pointerId);
}

function finishSprite3dCameraScrub(pointerId = null) {
  if (!sprite3dCameraScrubDrag) {
    return;
  }
  const { target } = sprite3dCameraScrubDrag;
  if (pointerId !== null && target.hasPointerCapture?.(pointerId)) {
    target.releasePointerCapture(pointerId);
  }
  target.classList.remove("is-dragging");
  document.documentElement.classList.remove("is-sprite3d-camera-scrubbing");
  document.documentElement.classList.remove("is-vertical-scrubbing");
  sprite3dCameraScrubDrag = null;
}

function adjustSprite3dCameraScrubWithKey(event) {
  const target = sprite3dCameraScrubTarget(event);
  if (!target || !["ArrowLeft", "ArrowDown", "ArrowRight", "ArrowUp"].includes(event.key)) {
    return;
  }
  event.preventDefault();
  const direction = event.key === "ArrowLeft" || event.key === "ArrowDown" ? -1 : 1;
  const kind = target.dataset.sprite3dCamera;
  const multiplier = event.shiftKey ? 10 : 1;
  setSprite3dCameraValue(kind, sprite3dCameraValue(kind) + direction * sprite3dCameraKeyStep(kind) * multiplier);
}

function sprite3dCameraValue(kind) {
  const camera = sprite3dCamera();
  if (kind === "yaw") {
    return camera.yawDegrees;
  }
  if (kind === "pitch") {
    return camera.pitchDegrees;
  }
  return camera.zoom;
}

function setSprite3dCameraValue(kind, value) {
  const camera = sprite3dCamera();
  if (kind === "yaw") {
    camera.yawDegrees = sprite3dNormalizeDegrees(value);
  } else if (kind === "pitch") {
    camera.pitchDegrees = sprite3dClampNumber(
      value,
      SPRITE3D_CAMERA_MIN_PITCH_DEGREES,
      SPRITE3D_CAMERA_MAX_PITCH_DEGREES,
    );
  } else if (kind === "zoom") {
    camera.zoom = sprite3dClampNumber(value, 0.25, 4);
  }
  renderSprite3dCameraControls();
  renderSprite3dPresentationSurfaces();
}

function sprite3dCameraScrubScale(kind) {
  return kind === "zoom" ? 0.01 : 0.5;
}

function sprite3dCameraKeyStep(kind) {
  return kind === "zoom" ? 0.05 : 1;
}

function setSprite3dActionStatus(text, className = "") {
  if (!sprite3dActionStatus) {
    return;
  }
  window.clearTimeout(sprite3dActionClearTimer);
  sprite3dActionStatus.className = `sprite-action-status tool-feedback-bar ${className}`.trim();
  sprite3dActionStatus.textContent = text;
  setPaneStatus("sprite", text, className);
  if (text && className === "is-ok") {
    sprite3dActionClearTimer = window.setTimeout(() => {
      if (sprite3dActionStatus.textContent === text && sprite3dActionStatus.classList.contains("is-ok")) {
        sprite3dActionStatus.className = "sprite-action-status tool-feedback-bar";
        sprite3dActionStatus.textContent = "";
      }
    }, 1800);
  }
}

function clearSprite3dActionError() {
  if (!sprite3dActionStatus?.classList.contains("is-error")) {
    return;
  }
  setSprite3dActionStatus("");
}

function startSprite3dPreviewDrag(event) {
  if (event.button !== 0) {
    return;
  }
  event.preventDefault();
  sprite3dPreviewDrag = {
    pointerId: event.pointerId,
    x: event.clientX,
    y: event.clientY,
    startX: event.clientX,
    startY: event.clientY,
    moved: false,
  };
  sprite3dPreviewCanvas.setPointerCapture?.(event.pointerId);
  sprite3dPreviewCanvas.classList.add("is-dragging");
}

function continueSprite3dPreviewDrag(event) {
  if (!sprite3dPreviewDrag || sprite3dPreviewDrag.pointerId !== event.pointerId) {
    setSprite3dHoverSliceFromEvent(event);
    return;
  }
  event.preventDefault();
  const camera = sprite3dCamera();
  const deltaX = event.clientX - sprite3dPreviewDrag.x;
  const deltaY = event.clientY - sprite3dPreviewDrag.y;
  sprite3dPreviewDrag.x = event.clientX;
  sprite3dPreviewDrag.y = event.clientY;
  if (Math.hypot(event.clientX - sprite3dPreviewDrag.startX, event.clientY - sprite3dPreviewDrag.startY) > 4) {
    sprite3dPreviewDrag.moved = true;
  }
  camera.yawDegrees = sprite3dNormalizeDegrees(camera.yawDegrees + deltaX * 0.35);
  camera.pitchDegrees = sprite3dClampNumber(
    camera.pitchDegrees - deltaY * 0.25,
    SPRITE3D_CAMERA_MIN_PITCH_DEGREES,
    SPRITE3D_CAMERA_MAX_PITCH_DEGREES,
  );
  renderSprite3dCameraControls();
  renderSprite3dPreview();
}

function stopSprite3dPreviewDrag(event) {
  if (!sprite3dPreviewDrag || sprite3dPreviewDrag.pointerId !== event.pointerId) {
    return;
  }
  if (sprite3dPreviewCanvas.hasPointerCapture?.(event.pointerId)) {
    sprite3dPreviewCanvas.releasePointerCapture(event.pointerId);
  }
  const wasClick = !sprite3dPreviewDrag.moved;
  sprite3dPreviewDrag = null;
  sprite3dPreviewCanvas.classList.remove("is-dragging");
  const hitSlice = setSprite3dHoverSliceFromEvent(event);
  if (wasClick && Number.isInteger(hitSlice)) {
    setSprite3dSlice(hitSlice);
  }
}

function sprite3dNormalizeDegrees(value) {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) {
    return 0;
  }
  return ((parsed % 360) + 360) % 360;
}

function setSprite3dHoverSliceFromEvent(event) {
  const next = sprite3dSliceFromPreviewEvent(event);
  if (sprite3d.hoverSlice !== next) {
    sprite3d.hoverSlice = next;
    renderSprite3dPreview();
  }
  return next;
}

function sprite3dSliceFromPreviewEvent(event) {
  const rect = sprite3dPreviewCanvas.getBoundingClientRect();
  const x = event.clientX - rect.left;
  const y = event.clientY - rect.top;
  const point = { x, y };
  const view = sprite3dPreviewCanvas._sprite3dPreviewView
    || sprite3dPreviewView(Math.max(1, Math.round(rect.width)), Math.max(1, Math.round(rect.height)));
  const ray = sprite3dPreviewRay(point, view);
  const voxelHit = sprite3dRaycastOccupiedVoxel(ray);
  if (voxelHit) {
    return sprite3dSliceIndexForVoxel(voxelHit.grid);
  }
  return sprite3dApproximateSliceFromRay(ray);
}

function sprite3dPreviewRay(point, view) {
  const camera = sprite3dCamera();
  const yaw = sprite3dDegreesToRadians(camera.yawDegrees ?? 0);
  const pitch = sprite3dDegreesToRadians(camera.pitchDegrees ?? 0);
  const scale = Math.max(0.000001, view.cellScale * (camera.zoom ?? 1));
  const screenU = (point.x - view.originX) / scale;
  const screenV = (point.y - view.originY) / scale;
  const sinYaw = Math.sin(yaw);
  const cosYaw = Math.cos(yaw);
  const sinPitch = Math.sin(pitch);
  const cosPitch = Math.cos(pitch);
  const yawYAtDepthZero = -sinPitch * screenV;
  const centerX = (sprite3d.width - 1) / 2;
  const centerY = (sprite3d.height - 1) / 2;
  const centerDepth = (sprite3d.depth - 1) / 2;
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

function sprite3dRaycastOccupiedVoxel(ray) {
  let best = null;
  for (let z = 0; z < sprite3d.depth; z += 1) {
    for (let y = 0; y < sprite3d.height; y += 1) {
      for (let x = 0; x < sprite3d.width; x += 1) {
        if (!validSprite3dColorIndex(sprite3d.cells[sprite3dCellIndex(x, y, z)])) {
          continue;
        }
        const hit = sprite3dRayAabbInterval(ray, {
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

function sprite3dApproximateSliceFromRay(ray) {
  const bounds = {
    min: { x: -0.5, y: -0.5, z: -0.5 },
    max: { x: sprite3d.width - 0.5, y: sprite3d.height - 0.5, z: sprite3d.depth - 0.5 },
  };
  const hit = sprite3dRayAabbInterval(ray, bounds);
  if (!hit) {
    return null;
  }
  const point = {
    x: ray.origin.x + ray.direction.x * hit.tMax,
    y: ray.origin.y + ray.direction.y * hit.tMax,
    z: ray.origin.z + ray.direction.z * hit.tMax,
  };
  return sprite3dSliceIndexForWorldPoint(point);
}

function sprite3dRayAabbInterval(ray, bounds) {
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

function sprite3dSliceIndexForVoxel(grid) {
  return sprite3dSliceIndexForWorldPoint(grid);
}

function sprite3dSliceIndexForWorldPoint(point) {
  const max = sprite3dAxisSize() - 1;
  if (sprite3d.axis === "x") {
    return sprite3dClamp(Math.round(point.x), 0, max);
  }
  if (sprite3d.axis === "y") {
    return sprite3dClamp(max - Math.round(point.y), 0, max);
  }
  return sprite3dClamp(max - Math.round(point.z), 0, max);
}

function sprite3dDegreesToRadians(value) {
  return (Number(value) * Math.PI) / 180;
}

function clearSprite3dHoverSlice() {
  if (sprite3d.hoverSlice === null) {
    return;
  }
  sprite3d.hoverSlice = null;
  renderSprite3dPreview();
}

function sprite3dNearestSliceEdgeHit(point, edges) {
  let best = null;
  for (const edge of edges) {
    const candidate = sprite3dSliceEdgeHit(point, edge);
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

function sprite3dSliceEdgeHit(point, edge) {
  const dx = edge.to.x - edge.from.x;
  const dy = edge.to.y - edge.from.y;
  const lengthSquared = dx * dx + dy * dy;
  if (lengthSquared <= 0.0001) {
    return null;
  }
  const rawT = ((point.x - edge.from.x) * dx + (point.y - edge.from.y) * dy) / lengthSquared;
  const t = sprite3dClamp(rawT, 0, 1);
  const nearest = {
    x: edge.from.x + dx * t,
    y: edge.from.y + dy * t,
  };
  const distance = Math.hypot(point.x - nearest.x, point.y - nearest.y);
  if (distance > edge.hitRadius) {
    return null;
  }
  const axisSize = sprite3dAxisSize();
  const world = Math.max(0, Math.min(axisSize - 1, Math.round(edge.min + (edge.max - edge.min) * t)));
  return {
    index: Math.max(0, Math.min(axisSize - 1, axisSize - 1 - world)),
    distance,
  };
}

function sprite3dPointInPolygon(point, polygon) {
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
  sprite3dNameInput,
  sprite3dWidthInput,
  sprite3dHeightInput,
  sprite3dDepthInput,
  sprite3dScaleInput,
  sprite3dSliceValue,
  sprite3dAnimationDurationInput,
  sprite3dAnimationFrameCountInput,
  sprite3dAnimationFrameInput,
]) {
  installSelectAllOnFocus(input);
}
sprite3dNameInput?.addEventListener("input", () => {
  renderSprite3dPreview();
  syncSprite3dSourceActionButtons();
});
sourceEditor.addEventListener("input", () => {
  invalidateSprite3dEditSourceForDocument(activeDocument());
  syncSprite3dSourceActionButtons();
});
function bindSprite3dDimensionInput(input, axis) {
  input?.addEventListener("input", () => {
    if (input.validity.valid && input.value !== "") {
      updateSprite3dDimension(axis, input.value);
    }
  });
  input?.addEventListener("change", () => updateSprite3dDimension(axis, input.value));
  input?.addEventListener("keydown", (event) => {
    if (event.key !== "Enter") return;
    event.preventDefault();
    updateSprite3dDimension(axis, input.value);
  });
}
bindSprite3dDimensionInput(sprite3dWidthInput, "width");
bindSprite3dDimensionInput(sprite3dHeightInput, "height");
bindSprite3dDimensionInput(sprite3dDepthInput, "depth");
sprite3dScaleInput?.addEventListener("input", () => {
  clearSprite3dActionError();
  renderSprite3dControls();
});
sprite3dScaleInput?.addEventListener("keydown", (event) => {
  if (event.key !== "Enter") {
    return;
  }
  event.preventDefault();
});
sprite3dSliceValue?.addEventListener("change", applySprite3dSliceInput);
sprite3dSliceValue?.addEventListener("keydown", (event) => {
  if (event.key !== "Enter") {
    return;
  }
  event.preventDefault();
  applySprite3dSliceInput();
});
sprite3dAnimationDurationInput?.addEventListener("change", () => setSprite3dAnimationDuration(sprite3dAnimationDurationInput.value));
sprite3dAnimationFrameCountInput?.addEventListener("change", () => setSprite3dAnimationFrameCount(sprite3dAnimationFrameCountInput.value));
const sprite3dSliceScrub = document.querySelector("[data-sprite3d-slice-scrub]");
sprite3dSliceScrub?.addEventListener("pointerdown", startSprite3dSliceScrub);
sprite3dSliceScrub?.addEventListener("pointermove", continueSprite3dSliceScrub);
sprite3dSliceScrub?.addEventListener("pointerup", stopSprite3dSliceScrub);
sprite3dSliceScrub?.addEventListener("pointercancel", stopSprite3dSliceScrub);
for (const scrub of [sprite3dCameraYawScrub, sprite3dCameraPitchScrub, sprite3dCameraZoomScrub]) {
  scrub?.addEventListener("pointerdown", startSprite3dCameraScrub);
  scrub?.addEventListener("pointermove", continueSprite3dCameraScrub);
  scrub?.addEventListener("pointerup", stopSprite3dCameraScrub);
  scrub?.addEventListener("pointercancel", stopSprite3dCameraScrub);
  scrub?.addEventListener("keydown", adjustSprite3dCameraScrubWithKey);
}
window.addEventListener("pointerup", stopSprite3dCameraScrub, true);
window.addEventListener("pointercancel", stopSprite3dCameraScrub, true);
window.addEventListener("pointerup", stopSprite3dSliceScrub, true);
window.addEventListener("pointercancel", stopSprite3dSliceScrub, true);
window.addEventListener("blur", () => {
  finishSprite3dCameraScrub();
  finishSprite3dSliceScrub();
});
for (const button of sprite3dAxisButtons) {
  button.addEventListener("click", () => setSprite3dAxis(button.dataset.sprite3dAxis));
}
sprite3dPalette?.addEventListener("keydown", (event) => {
  const token = event.target.closest(".sprite-token");
  if (!token || (event.key !== "Enter" && event.key !== " ")) {
    return;
  }
  const rawIndex = token.dataset.colorIndex;
  if (rawIndex === undefined) {
    return;
  }
  event.preventDefault();
  selectSprite3dColor(rawIndex === "erase" ? null : Number(rawIndex));
});
sprite3dSliceBoard?.addEventListener("pointerdown", startSprite3dPaint);
sprite3dSliceBoard?.addEventListener("pointermove", continueSprite3dPaint);
sprite3dSliceBoard?.addEventListener("pointerup", stopSprite3dPaint);
sprite3dSliceBoard?.addEventListener("pointercancel", stopSprite3dPaint);
sprite3dSliceBoard?.addEventListener("keydown", (event) => {
  if (sprite3dTranslateActive) {
    event.preventDefault();
    return;
  }
  handleSprite3dSliceBoardShortcut(event);
  if (event.defaultPrevented) {
    return;
  }
  if (event.key === "Enter" || event.key === " ") {
    const mutate = sprite3dBucketActive ? bucketFillSprite3dFromElement : paintSprite3dCellFromElement;
    if (withVisualEditHistory("sprite3d", () => mutate(event.target))) {
      event.preventDefault();
      event.stopPropagation();
    }
  }
});
sprite3dPreviousSliceButton?.addEventListener("click", () => moveSprite3dSlice(-1));
sprite3dNextSliceButton?.addEventListener("click", () => moveSprite3dSlice(1));
sprite3dScaleDownButton?.addEventListener("click", scaleDownSprite3d);
sprite3dScaleUpButton?.addEventListener("click", scaleUpSprite3d);
sprite3dRotatePlaneLeftButton?.addEventListener("click", rotateSprite3dPlaneLeft);
sprite3dRotatePlaneRightButton?.addEventListener("click", rotateSprite3dPlaneRight);
sprite3dFlipPlaneHorizontalButton?.addEventListener("click", flipSprite3dPlaneHorizontal);
sprite3dFlipPlaneVerticalButton?.addEventListener("click", flipSprite3dPlaneVertical);
sprite3dTranslateButton?.addEventListener("click", toggleSprite3dTranslateMode);
sprite3dScopeSliceButton?.addEventListener("click", () => setSprite3dEditScope("slice"));
sprite3dScopeAllButton?.addEventListener("click", () => setSprite3dEditScope("all"));
sprite3dFillButton?.addEventListener("click", toggleSprite3dBucketMode);
sprite3dUpdateButton?.addEventListener("click", () => {
  updateSprite3dInSource().catch((error) => {
    console.error(error);
    setSprite3dActionStatus("3D sprite source update failed", "is-error");
    setStatus("3D sprite source update failed", "is-error");
  });
});
newSprite3dButton?.addEventListener("click", newSprite3dDraft);
sprite3dInsertButton?.addEventListener("click", () => addSprite3dToSource().catch((error) => {
  console.error(error);
  setSprite3dActionStatus("Could not add 3D sprite", "is-error");
}));
sprite3dResetCameraButton?.addEventListener("click", resetSprite3dCamera);
sprite3dPreviewCanvas?.addEventListener("pointerdown", startSprite3dPreviewDrag);
sprite3dPreviewCanvas?.addEventListener("pointermove", continueSprite3dPreviewDrag);
sprite3dPreviewCanvas?.addEventListener("pointerup", stopSprite3dPreviewDrag);
sprite3dPreviewCanvas?.addEventListener("pointercancel", stopSprite3dPreviewDrag);
sprite3dPreviewCanvas?.addEventListener("pointerleave", clearSprite3dHoverSlice);
document.addEventListener("click", (event) => {
  if (!sprite3dTranslateActive || sprite3dSliceBoard?.contains(event.target)) {
    return;
  }
  if (event.target.closest?.("#sprite3dTranslateButton")) {
    return;
  }
  deactivateSprite3dTranslateMode();
});
document.addEventListener("keydown", (event) => {
  if (handleSprite3dClipKeyboard(event)) {
    return;
  }
  if (sprite3dTranslateActive && event.key === "Escape") {
    event.preventDefault();
    deactivateSprite3dTranslateMode();
  }
});
window.addEventListener("resize", () => {
  if (!sprite3dBuilder?.hidden) {
    renderSprite3dPreview();
  }
});
registerSourceEditableTarget?.("sprite3d", {
  load: loadSprite3dFromSourcePosition,
});

function syncSprite3dBuilderAfterScriptLoad() {
  if (currentPreviewMode === "sprite3d" && typeof loadFirstFocusedPuzzleEntry === "function") {
    loadFirstFocusedPuzzleEntry("sprite", "sprite3d");
  }
}

resetSprite3dBuilder();
syncSprite3dBuilderAfterScriptLoad();
