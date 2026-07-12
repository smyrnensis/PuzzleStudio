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
const SPRITE3D_EDITOR_MAX_SIZE = 64;
const SPRITE3D_SLICE_SCRUB_STEP_PX = 18;
const SPRITE3D_CAMERA_MIN_PITCH_DEGREES = -90;
const SPRITE3D_CAMERA_MAX_PITCH_DEGREES = 90;
const SPRITE3D_PREVIEW_BASE_ZOOM = 0.92;
const SPRITE3D_CAMERA_DEFAULT = {
  yawDegrees: 15,
  pitchDegrees: 30,
  zoom: 1,
};

function resetSprite3dBuilder(size = sprite3d.size) {
  resetSprite3dClipState({ clipboard: true });
  ensureSprite3dPalette();
  sprite3d.size = clampSprite3dSize(size);
  sprite3d.slice = Math.max(0, Math.min(sprite3d.size - 1, Number(sprite3d.slice) || 0));
  sprite3d.hoverSlice = null;
  sprite3d.cells = Array.from({ length: sprite3d.size * sprite3d.size * sprite3d.size }, () => null);
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

function sprite3dPaneScrollElement() {
  return sprite3dBuilder?.querySelector(":scope > .tool-pane-scroll") || null;
}

function shouldPreserveSprite3dPaneScroll() {
  if (!sprite3dBuilder || sprite3dBuilder.hidden) {
    return false;
  }
  const active = document.activeElement;
  return Boolean(
    (active && sprite3dBuilder.contains(active))
    || sprite3dPaintDrag
    || sprite3dTranslateDrag
    || sprite3dPreviewDrag
    || sprite3dCameraScrubDrag
    || sprite3dSliceScrubDrag
  );
}

function captureSprite3dPaneScroll() {
  if (!shouldPreserveSprite3dPaneScroll()) {
    return null;
  }
  const scroll = sprite3dPaneScrollElement();
  return scroll ? { top: scroll.scrollTop, left: scroll.scrollLeft } : null;
}

function restoreSprite3dPaneScroll(state) {
  if (!state) {
    return;
  }
  const apply = () => {
    const scroll = sprite3dPaneScrollElement();
    if (!scroll) {
      return;
    }
    scroll.scrollTop = Math.max(0, Math.min(state.top, scroll.scrollHeight - scroll.clientHeight));
    scroll.scrollLeft = Math.max(0, Math.min(state.left, scroll.scrollWidth - scroll.clientWidth));
  };
  apply();
  window.requestAnimationFrame?.(apply);
}

function withSprite3dPaneScrollPreserved(render) {
  const scroll = captureSprite3dPaneScroll();
  const result = render();
  restoreSprite3dPaneScroll(scroll);
  return result;
}

function renderSprite3dBuilder() {
  if (!sprite3dBuilder || !sprite3dSliceBoard || !sprite3dPalette || !sprite3dPreviewCanvas) {
    return;
  }
  withSprite3dPaneScrollPreserved(() => {
    renderSprite3dControls();
    renderSprite3dPalette();
    renderSprite3dSliceBoard();
    renderSprite3dPreview();
    syncSprite3dSourceActionButtons();
  });
}

function renderSprite3dControls() {
  withSprite3dPaneScrollPreserved(() => {
    sprite3dNameInput.value = sprite3dNameInput.value || "VoxelSprite";
    sprite3dSizeInput.value = String(sprite3d.size);
    syncSprite3dBucketButton();
    syncSprite3dTranslateButton();
    renderSprite3dClipActions();
    renderSprite3dScopeControl();
    renderSprite3dCameraControls();
    renderSpriteScaleControl({
      size: sprite3d.size,
      maxSize: SPRITE3D_EDITOR_MAX_SIZE,
      scaleInput: sprite3dScaleInput,
      scaleUpButton: sprite3dScaleUpButton,
      scaleDownButton: sprite3dScaleDownButton,
      canScaleDown: canScaleDownSprite3d,
      noun: "3D sprite",
    });
    if (sprite3dSliceValue instanceof HTMLInputElement) {
      sprite3dSliceValue.min = "1";
      sprite3dSliceValue.max = String(sprite3d.size);
      sprite3dSliceValue.value = String(sprite3d.slice + 1);
    } else if (sprite3dSliceValue) {
      sprite3dSliceValue.textContent = `${sprite3d.slice + 1} / ${sprite3d.size}`;
    }
    const sliceTotal = document.querySelector("#sprite3dSliceTotal");
    if (sliceTotal) {
      sliceTotal.textContent = String(sprite3d.size);
    }
    if (sprite3dPasteSliceButton) {
      sprite3dPasteSliceButton.disabled = !sprite3d.sliceClipboard;
    }
    if (sprite3dPreviousSliceButton) {
      sprite3dPreviousSliceButton.disabled = sprite3d.slice <= 0;
    }
    if (sprite3dNextSliceButton) {
      sprite3dNextSliceButton.disabled = sprite3d.slice >= sprite3d.size - 1;
    }
    for (const button of sprite3dAxisButtons) {
      const active = button.dataset.sprite3dAxis === sprite3d.axis;
      button.classList.toggle("is-active", active);
      button.setAttribute("aria-pressed", String(active));
    }
  });
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
      label: "Scope slice",
      title: "Scope slice",
    },
    {
      button: sprite3dScopeAllButton,
      scope: "all",
      label: "Scope all",
      title: "Scope all",
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
  setSprite3dButtonLabel(
    sprite3dFillButton,
    isAll ? "Fill connected 3D component" : "Fill connected area in current slice",
  );
  setSprite3dButtonLabel(sprite3dClearButton, isAll ? "Clear whole sprite" : "Clear current slice");
  syncSprite3dTranslateButton();
}

function syncSprite3dTranslateButton() {
  if (!sprite3dTranslateButton) {
    return;
  }
  const label = sprite3dEditScope() === "all" ? "Translate whole sprite" : "Translate current slice";
  sprite3dTranslateButton.classList.toggle("is-active", sprite3dTranslateActive);
  sprite3dTranslateButton.setAttribute("aria-pressed", String(sprite3dTranslateActive));
  sprite3dTranslateButton.setAttribute("aria-label", sprite3dTranslateActive ? "Stop translating 3D sprite" : label);
  sprite3dTranslateButton.title = sprite3dTranslateActive ? "Stop translating 3D sprite" : label;
}

function renderSprite3dClipActions() {
  if (!sprite3dClipActions) {
    return;
  }
  const actions = document.createElement("span");
  actions.className = "sprite-clip-actions";
  actions.classList.toggle("is-expanded", sprite3dClipActive);
  actions.append(renderSpriteClipButton({
    title: sprite3dClipActive ? "Close clip tools" : "Clip",
    ariaLabel: sprite3dClipActive ? "Close 3D clip tools" : "Open 3D clip tools",
    active: sprite3dClipActive,
    onClick: toggleSprite3dClipMode,
    icon: spriteLucideIconSvg("mouse-pointer-2"),
  }));
  if (sprite3dClipActive) {
    const expanded = document.createElement("span");
    expanded.className = "sprite-clip-expanded-actions";
    expanded.append(
      renderSpriteClipButton({
        title: "Copy clip",
        ariaLabel: "Copy selected 3D sprite area",
        disabled: !sprite3dClipSelection,
        onClick: copySprite3dClipSelection,
        icon: spriteLucideIconSvg("copy"),
      }),
      renderSpriteClipButton({
        title: "Cut clip",
        ariaLabel: "Cut selected 3D sprite area",
        disabled: !sprite3dClipSelection,
        onClick: cutSprite3dClipSelection,
        icon: spriteLucideIconSvg("scissors"),
      }),
      renderSpriteClipButton({
        title: "Paste clip",
        ariaLabel: "Paste copied 3D sprite area",
        disabled: !sprite3dClipClipboard,
        onClick: pasteSprite3dClipClipboard,
        icon: spriteLucideIconSvg("clipboard-paste"),
      }),
      renderSpriteClipButton({
        title: sprite3dClipFloating ? "Discard clip preview" : "Clear clip",
        ariaLabel: sprite3dClipFloating ? "Discard 3D clip preview" : "Clear selected 3D sprite area",
        disabled: !sprite3dClipSelection && !sprite3dClipFloating,
        danger: true,
        onClick: clearSprite3dClipSelection,
        icon: spriteLucideIconSvg("trash-2"),
      }),
    );
    actions.append(expanded);
  }
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
    if (!Number.isInteger(min) || !Number.isInteger(max) || min < 0 || max < min || max >= sprite3d.size) {
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
  const box = existing || { minX: 0, maxX: sprite3d.size - 1, minY: 0, maxY: sprite3d.size - 1, minZ: 0, maxZ: sprite3d.size - 1 };
  for (const worldAxis of ["x", "y", "z"]) {
    if (worldAxis === sprite3d.axis) {
      if (!existing) {
        box[`min${worldAxis.toUpperCase()}`] = fullDepth ? 0 : fixedStack;
        box[`max${worldAxis.toUpperCase()}`] = fullDepth ? sprite3d.size - 1 : fixedStack;
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
  return {
    x: Math.max(0, Math.min(sprite3d.size - 1, Math.floor(((clientX - geometry.left) / geometry.width) * sprite3d.size))),
    y: Math.max(0, Math.min(sprite3d.size - 1, Math.floor(((clientY - geometry.top) / geometry.height) * sprite3d.size))),
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
    return { scope: "slice", width: rect.width, height: rect.height, depth: 1, cells: sprite3dSliceClipCells(rect) };
  }
  return { scope: "all", ...dimensions, cells: sprite3dClipCells(box) };
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

function copySprite3dClipSelection() {
  const box = normalizeSprite3dClipBox(sprite3dClipSelection);
  const dimensions = sprite3dClipBoxDimensions(box);
  if (!box || !dimensions) {
    setSprite3dActionStatus("No clip selection", "is-error");
    return false;
  }
  sprite3dClipClipboard = sprite3dClipClipboardFromSelection(box, dimensions);
  sprite3dClipFloating = { kind: "copy" };
  sprite3dClipActive = true;
  renderSprite3dBuilder();
  setSprite3dActionStatus(`Copied ${dimensions.width}x${dimensions.height}x${dimensions.depth} clip`, "is-ok");
  return true;
}

function cutSprite3dClipSelection() {
  const box = normalizeSprite3dClipBox(sprite3dClipSelection);
  const dimensions = sprite3dClipBoxDimensions(box);
  if (!box || !dimensions) {
    setSprite3dActionStatus("No clip selection", "is-error");
    return false;
  }
  const before = visualEditSnapshot("sprite3d");
  sprite3dClipClipboard = sprite3dClipClipboardFromSelection(box, dimensions);
  sprite3dClipFloating = { kind: "cut" };
  const changed = clearSprite3dClipBox(box);
  commitSprite3dClipMutation(before, changed, `Cut ${dimensions.width}x${dimensions.height}x${dimensions.depth} clip`);
  return true;
}

function clearSprite3dClipSelection() {
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
  return commitSprite3dClipMutation(before, clearSprite3dClipBox(box), "Cleared 3D clip");
}

function pasteSprite3dClipClipboard() {
  if (!sprite3dClipClipboard) {
    setSprite3dActionStatus("No copied clip", "is-error");
    return false;
  }
  if (sprite3dClipClipboard.scope === "slice") {
    const baseRect = sprite3dClipPlaneRect() || { x: 0, y: 0, width: 1, height: 1 };
    const rect = {
      x: baseRect.x,
      y: baseRect.y,
      width: sprite3dClipClipboard.width,
      height: sprite3dClipClipboard.height,
    };
    if (rect.x + rect.width > sprite3d.size || rect.y + rect.height > sprite3d.size) {
      setSprite3dActionStatus("Copied slice clip does not fit at selection", "is-error");
      return false;
    }
    const target = sprite3dClipBoxFromPlaneRect(rect, { fullDepth: false });
    const before = visualEditSnapshot("sprite3d");
    const changed = setSprite3dSliceClipCells(rect, sprite3dClipClipboard);
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
    maxX: base.minX + sprite3dClipClipboard.width - 1,
    minY: base.minY,
    maxY: base.minY + sprite3dClipClipboard.height - 1,
    minZ: base.minZ,
    maxZ: base.minZ + sprite3dClipClipboard.depth - 1,
  });
  if (!target) {
    setSprite3dActionStatus("Copied clip does not fit at selection", "is-error");
    return false;
  }
  const before = visualEditSnapshot("sprite3d");
  const changed = setSprite3dClipCells(target, sprite3dClipClipboard);
  sprite3dClipSelection = target;
  sprite3dClipFloating = null;
  const dimensions = sprite3dClipBoxDimensions(target);
  commitSprite3dClipMutation(before, changed, `Pasted ${dimensions.width}x${dimensions.height}x${dimensions.depth} clip`);
  return true;
}

function sprite3dClipBoxShiftedInPlane(box, du, dv) {
  const rect = sprite3dClipPlaneRect(box);
  if (!rect) {
    return null;
  }
  const targetRect = { ...rect, x: rect.x + du, y: rect.y + dv };
  if (targetRect.x < 0 || targetRect.y < 0
    || targetRect.x + targetRect.width > sprite3d.size
    || targetRect.y + targetRect.height > sprite3d.size) {
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
  if (edge.includes("e")) right = Math.min(sprite3d.size - 1, Math.max(cell.x, left));
  if (edge.includes("n")) top = Math.max(0, Math.min(cell.y, bottom));
  if (edge.includes("s")) bottom = Math.min(sprite3d.size - 1, Math.max(cell.y, top));
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
  const size = sprite3d.size;
  const next = scope === "all"
    ? Array.from({ length: size * size * size }, () => null)
    : [...originCells];
  const firstStack = scope === "all" ? 0 : sprite3d.slice;
  const lastStack = scope === "all" ? size - 1 : sprite3d.slice;
  for (let stack = firstStack; stack <= lastStack; stack += 1) {
    for (let v = 0; v < size; v += 1) {
      for (let u = 0; u < size; u += 1) {
        const source = sprite3dCoordsFromPlane(sprite3d.axis, stack, u, v);
        const target = sprite3dCoordsFromPlane(
          sprite3d.axis,
          stack,
          sprite3dPositiveModulo(u + du, size),
          sprite3dPositiveModulo(v + dv, size),
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
  const du = Math.round((event.clientX - sprite3dTranslateDrag.startClientX) / (sprite3dTranslateDrag.width / sprite3d.size));
  const dv = Math.round((event.clientY - sprite3dTranslateDrag.startClientY) / (sprite3dTranslateDrag.height / sprite3d.size));
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
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <rect x="5" y="5" width="14" height="14" rx="1.8"></rect>
    </svg>
  `;
}

function sprite3dCubeIconSvg() {
  return `
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M12 2.8 20 7.3v9.4l-8 4.5-8-4.5V7.3Z"></path>
      <path d="M4 7.3 12 12l8-4.7"></path>
      <path d="M12 12v9.2"></path>
    </svg>
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

function renderSprite3dPaletteContent() {
  ensureSprite3dPalette();
  sprite3dPalette.replaceChildren();
  const selectedIsTransparent = sprite3d.selectedColorIndex === null;
  if (selectedIsTransparent || validSprite3dColorIndex(sprite3d.selectedColorIndex)) {
    const selected = selectedIsTransparent ? { color: "#00000000" } : sprite3dPaletteEntries()[sprite3d.selectedColorIndex];
    const currentWrap = document.createElement("span");
    currentWrap.className = "sprite-current-color-wrap";
    const currentButton = document.createElement("button");
    currentButton.type = "button";
    currentButton.className = "sprite-current-color-button";
    currentButton.classList.toggle("is-transparent", selectedIsTransparent);
    currentButton.style.setProperty("--sprite-current-color", normalizeSpriteColor(selected.color));
    currentButton.title = selectedIsTransparent ? "Transparent eraser cannot be edited" : "Pick selected color";
    currentButton.setAttribute(
      "aria-label",
      selectedIsTransparent ? "Selected transparent eraser color #00000000, not editable" : `Pick selected color ${selected.color}`,
    );
    currentButton.setAttribute("aria-disabled", String(selectedIsTransparent));
    currentButton.setAttribute("aria-expanded", String(!selectedIsTransparent && sprite3d.editPaletteOpen));
    currentButton.innerHTML = `<span class="sprite-current-color-swatch" aria-hidden="true"></span>`;
    if (selectedIsTransparent) {
      currentButton.insertAdjacentHTML("beforeend", `
        <span class="sprite-current-transparent-icon" aria-hidden="true">
          <svg viewBox="0 0 24 24">
            <path d="m7 21-4.3-4.3c-1-1-1-2.5 0-3.4l9.6-9.6c1-1 2.5-1 3.4 0l5.6 5.6c1 1 1 2.5 0 3.4L13 21"></path>
            <path d="M22 21H7"></path>
            <path d="m5 11 9 9"></path>
          </svg>
        </span>
      `);
    } else {
      currentButton.insertAdjacentHTML("beforeend", `
        <span class="sprite-current-edit-icon" aria-hidden="true">
          <svg viewBox="0 0 24 24">
            <path d="M12 20h9"></path>
            <path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4Z"></path>
          </svg>
        </span>
      `);
    }
    const currentHexInput = document.createElement("input");
    currentHexInput.type = "text";
    currentHexInput.className = "sprite-current-hex-input";
    currentHexInput.value = selectedIsTransparent ? "#00000000" : normalizeSpriteColor(selected.color);
    currentHexInput.placeholder = "#rrggbbaa";
    currentHexInput.spellcheck = false;
    currentHexInput.autocomplete = "off";
    currentHexInput.readOnly = selectedIsTransparent;
    currentHexInput.setAttribute(
      "aria-label",
      selectedIsTransparent ? "Transparent 3D sprite color code" : "Selected 3D sprite color hex code",
    );
    const applyCurrentHex = (options = {}) => {
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
    sprite3dPalette.append(currentWrap);
    if (pendingEditMenu) {
      positionSpriteColorMenu(pendingEditMenu, currentButton, { side: "left" });
    }
  }

  const paletteGrid = document.createElement("span");
  paletteGrid.className = "sprite-palette-grid";

  const eraseButton = document.createElement("button");
  eraseButton.type = "button";
  eraseButton.className = "sprite-token sprite-color-swatch sprite-token-erase";
  eraseButton.classList.toggle("is-selected", sprite3d.selectedColorIndex === null);
  eraseButton.dataset.colorIndex = "erase";
  eraseButton.style.setProperty("--sprite-swatch-color", "#00000000");
  eraseButton.title = "Paint empty voxel";
  eraseButton.setAttribute("aria-label", "Paint empty voxel");
  eraseButton.innerHTML = `
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="m7 21-4.3-4.3c-1-1-1-2.5 0-3.4l9.6-9.6c1-1 2.5-1 3.4 0l5.6 5.6c1 1 1 2.5 0 3.4L13 21"></path>
      <path d="M22 21H7"></path>
      <path d="m5 11 9 9"></path>
    </svg>
  `;
  eraseButton.addEventListener("click", () => selectSprite3dColor(null));
  paletteGrid.append(eraseButton);

  for (const [index, entry] of sprite3dPaletteEntries().entries()) {
    const item = document.createElement("span");
    item.className = "sprite-token-item";
    item.classList.toggle("is-selected", index === sprite3d.selectedColorIndex);

    const button = document.createElement("button");
    button.type = "button";
    button.className = "sprite-token sprite-color-swatch";
    button.classList.toggle("is-selected", index === sprite3d.selectedColorIndex);
    button.dataset.colorIndex = String(index);
    button.style.setProperty("--sprite-swatch-color", normalizeSpriteColor(entry.color));
    button.style.setProperty("--sprite-token-ink", readableInkForColor(entry.color));
    button.title = `Paint ${entry.color}`;
    button.setAttribute("aria-label", `Paint 3D sprite color ${index + 1}`);
    button.addEventListener("click", () => selectSprite3dColor(index));
    item.append(button);

    paletteGrid.append(item);
  }
  const addWrap = document.createElement("span");
  addWrap.className = "sprite-add-wrap";
  const addButton = document.createElement("button");
  addButton.type = "button";
  addButton.className = "sprite-token sprite-add-color-button";
  addButton.disabled = sprite3dPaletteEntries().length >= SPRITE_COLOR_TOKENS.length;
  addButton.title = "Add color";
  addButton.setAttribute("aria-label", "Add sprite color");
  addButton.setAttribute("aria-expanded", String(sprite3d.addPaletteOpen));
  addButton.innerHTML = `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14"></path><path d="M5 12h14"></path></svg>`;
  addButton.addEventListener("click", toggleSprite3dAddPalette);
  addWrap.append(addButton);
  paletteGrid.append(addWrap);

  const removeButton = document.createElement("button");
  removeButton.type = "button";
  removeButton.className = "sprite-token sprite-remove-color-button";
  removeButton.disabled = !validSprite3dColorIndex(sprite3d.selectedColorIndex) || sprite3dPaletteEntries().length <= 1;
  removeButton.title = "Remove selected color";
  removeButton.setAttribute("aria-label", "Remove selected color");
  removeButton.innerHTML = `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 12h14"></path></svg>`;
  removeButton.addEventListener("click", removeSprite3dColor);
  paletteGrid.append(removeButton);
  sprite3dPalette.append(paletteGrid);

  if (sprite3d.addPaletteOpen) {
    const draft = validSprite3dColorIndex(sprite3d.addDraftColorIndex)
      ? sprite3dPaletteEntries()[sprite3d.addDraftColorIndex].color
      : nextSpritePresetColor(sprite3dPaletteEntries());
    const addMenu = renderSpriteColorMenu({
      mode: "add",
      customValue: draft,
      onDiscard: cancelSprite3dColorAdd,
      onChange: previewNewSprite3dColor,
      onPreset: previewNewSprite3dColor,
      renderPalette: renderSprite3dPalette,
    });
    addMenu.classList.add("is-add-menu");
    sprite3dPalette.append(addMenu);
    positionSpriteColorMenu(addMenu, paletteGrid, { side: "left" });
  }
}

function renderSprite3dSliceBoard() {
  withSprite3dPaneScrollPreserved(() => {
    sprite3dSliceBoard.replaceChildren();
    sprite3dSliceBoard.classList.toggle("is-translate-active", sprite3dTranslateActive);
    sprite3dSliceBoard.classList.toggle("is-clip-active", sprite3dClipActive);
    sprite3dSliceBoard.classList.toggle("is-clip-floating", Boolean(sprite3dClipFloating));
    sprite3dSliceBoard.style.setProperty("--sprite-size", sprite3d.size);
    const selectionRect = sprite3dClipPlaneRect();
    const fixed = sprite3dPlaneWorldSlice(sprite3d.axis, sprite3d.slice);
    const normalKey = `${sprite3d.axis.toUpperCase()}`;
    const selectionIntersectsSlice = Boolean(
      sprite3dClipSelection
      && fixed >= sprite3dClipSelection[`min${normalKey}`]
      && fixed <= sprite3dClipSelection[`max${normalKey}`],
    );
    const cellCount = sprite3d.size * sprite3d.size;
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
      const u = index % sprite3d.size;
      const v = Math.floor(index / sprite3d.size);
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
  const canvas = sprite3dPreviewCanvas;
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

  const size = sprite3d.size;
  const view = sprite3dPreviewView(width, height, size);
  drawSprite3dBounds(ctx, view);

  const occupied = sprite3dOccupancyMap();
  const faces = sprite3dMergedVoxelFaces(occupied, view);
  const previewOwner = sprite3dPreviewRenderOwner();
  const sceneFaces = [
    ...faces.map((face) => ({ ...face, kind: "voxel", ownerCell: previewOwner, renderPriority: 0 })),
    ...sprite3dSliceSurfaceFaces(sprite3d.hoverSlice, view, "hover", occupied, 1)
      .map((face) => ({ ...face, ownerCell: previewOwner })),
    ...sprite3dSliceSurfaceFaces(sprite3d.slice, view, "active", occupied, 2)
      .map((face) => ({ ...face, ownerCell: previewOwner })),
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
  drawSprite3dClipBounds(ctx, view);
  sprite3dPreviewCanvas._sprite3dPreviewView = view;
  renderSprite3dCameraControls();
}

function sprite3dPreviewView(width, height, size) {
  const padding = 22;
  const overlayClearanceY = 0;
  const boundsView = {
    cellScale: 1,
    originX: 0,
    originY: 0,
    size,
  };
  const points = sprite3dBoundsCorners(size).map((corner) => sprite3dProject(corner, boundsView));
  const minX = Math.min(...points.map((point) => point.x));
  const maxX = Math.max(...points.map((point) => point.x));
  const minY = Math.min(...points.map((point) => point.y));
  const maxY = Math.max(...points.map((point) => point.y));
  const projectedWidth = Math.max(1, maxX - minX);
  const projectedHeight = Math.max(1, maxY - minY);
  const availableWidth = Math.max(1, width - padding * 2);
  const availableHeight = Math.max(1, height - padding * 2);
  const scale = Math.max(4, Math.min(availableWidth / projectedWidth, availableHeight / projectedHeight) * SPRITE3D_PREVIEW_BASE_ZOOM)
    * sprite3dCamera().zoom;
  return {
    cellScale: scale,
    originX: width / 2 - ((minX + maxX) / 2) * scale,
    originY: height / 2 - ((minY + maxY) / 2) * scale + overlayClearanceY,
    size,
  };
}

function sprite3dBoundsCorners(size) {
  const min = -0.5;
  const max = size - 0.5;
  return [
    { x: min, y: min, z: min },
    { x: max, y: min, z: min },
    { x: max, y: max, z: min },
    { x: min, y: max, z: min },
    { x: min, y: min, z: max },
    { x: max, y: min, z: max },
    { x: max, y: max, z: max },
    { x: min, y: max, z: max },
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

function sprite3dOccupancyMap() {
  const occupied = new Map();
  for (let z = 0; z < sprite3d.size; z += 1) {
    for (let y = 0; y < sprite3d.size; y += 1) {
      for (let x = 0; x < sprite3d.size; x += 1) {
        const colorIndex = sprite3d.cells[sprite3dCellIndex(x, y, z)];
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
    sprite3dProject({ x: view.size - 0.5, y: -0.5, z }, view),
    sprite3dProject({ x: view.size - 0.5, y: view.size - 0.5, z }, view),
    sprite3dProject({ x: -0.5, y: view.size - 0.5, z }, view),
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
  for (let index = 0; index < sprite3d.size; index += 1) {
    hitPlanes.push({ index, points: sprite3dSliceHitPlaneCorners(index, view) });
  }
  return hitPlanes;
}

function sprite3dSliceHitEdges(view) {
  if (sprite3d.axis !== "z") {
    return [];
  }
  const min = -0.5;
  const max = sprite3d.size - 0.5;
  return [
    { x: min, y: min },
    { x: max, y: min },
    { x: max, y: max },
    { x: min, y: max },
  ].map((edge) => {
    const from = sprite3dProject({ x: edge.x, y: edge.y, z: min }, view);
    const to = sprite3dProject({ x: edge.x, y: edge.y, z: max }, view);
    return {
      axis: "z",
      from,
      to,
      min,
      max,
      hitRadius: sprite3dClamp(view.cellScale * 0.34, 8, 18),
    };
  });
}

function sprite3dSliceHitPlaneCorners(slice, view) {
  const min = -0.5;
  const max = sprite3d.size - 0.5;
  const fixed = sprite3dPlaneWorldSlice(sprite3d.axis, slice);
  let corners = [];
  if (sprite3d.axis === "x") {
    corners = [
      { x: fixed, y: min, z: min },
      { x: fixed, y: min, z: max },
      { x: fixed, y: max, z: max },
      { x: fixed, y: max, z: min },
    ];
  } else if (sprite3d.axis === "y") {
    corners = [
      { x: min, y: fixed, z: min },
      { x: max, y: fixed, z: min },
      { x: max, y: fixed, z: max },
      { x: min, y: fixed, z: max },
    ];
  } else {
    corners = [
      { x: min, y: min, z: fixed },
      { x: max, y: min, z: fixed },
      { x: max, y: max, z: fixed },
      { x: min, y: max, z: fixed },
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
  for (let row = 0; row < sprite3d.size; row += 1) {
    for (let col = 0; col < sprite3d.size; col += 1) {
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
    && grid.x < sprite3d.size
    && grid.y < sprite3d.size
    && grid.z < sprite3d.size
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
  const center = (view.size - 1) / 2;
  return Puzzle3VisualCore.projectOrthographic(position, {
    camera,
    center: { x: center, y: center, z: center },
    origin: { x: view.originX, y: view.originY },
    scale: view.cellScale,
  });
}

function sprite3dMergedVoxelFaces(occupied, view) {
  const voxels = [];
  for (let z = 0; z < sprite3d.size; z += 1) {
    for (let y = 0; y < sprite3d.size; y += 1) {
      for (let x = 0; x < sprite3d.size; x += 1) {
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
  return ((z * sprite3d.size + y) * sprite3d.size) + x;
}

function sprite3dCoordsFromSliceCell(index) {
  const u = index % sprite3d.size;
  const v = Math.floor(index / sprite3d.size);
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
  return Number.isInteger(index) && index >= 0 && index < sprite3d.size * sprite3d.size ? index : -1;
}

function paintSprite3dCellAtSliceIndex(index, colorIndex) {
  if (!Number.isInteger(index) || index < 0 || index >= sprite3d.size * sprite3d.size) {
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
  if (!Number.isInteger(index) || index < 0 || index >= sprite3d.size * sprite3d.size) {
    return 0;
  }
  const startCoords = sprite3dCoordsFromSliceCell(index);
  const startVoxelIndex = sprite3dCellIndex(startCoords.x, startCoords.y, startCoords.z);
  const nextColorIndex = validSprite3dColorIndex(colorIndex) ? colorIndex : null;
  const targetColorIndex = normalizedSprite3dCellColorIndex(startVoxelIndex);
  if (targetColorIndex === nextColorIndex) {
    return 0;
  }
  const size = sprite3d.size;
  const visited = new Uint8Array(size * size);
  const stack = [index];
  let changed = 0;
  while (stack.length) {
    const current = stack.pop();
    if (visited[current]) {
      continue;
    }
    const coords = sprite3dCoordsFromSliceCell(current);
    const voxelIndex = sprite3dCellIndex(coords.x, coords.y, coords.z);
    if (normalizedSprite3dCellColorIndex(voxelIndex) !== targetColorIndex) {
      continue;
    }
    visited[current] = 1;
    sprite3d.cells[voxelIndex] = nextColorIndex;
    changed += 1;
    const u = current % size;
    const v = Math.floor(current / size);
    if (u > 0) {
      stack.push(current - 1);
    }
    if (u < size - 1) {
      stack.push(current + 1);
    }
    if (v > 0) {
      stack.push(current - size);
    }
    if (v < size - 1) {
      stack.push(current + size);
    }
  }
  return changed;
}

function floodFillSprite3dComponentAtSliceIndex(index, colorIndex) {
  if (!Number.isInteger(index) || index < 0 || index >= sprite3d.size * sprite3d.size) {
    return 0;
  }
  const startCoords = sprite3dCoordsFromSliceCell(index);
  const startVoxelIndex = sprite3dCellIndex(startCoords.x, startCoords.y, startCoords.z);
  const nextColorIndex = validSprite3dColorIndex(colorIndex) ? colorIndex : null;
  const targetColorIndex = normalizedSprite3dCellColorIndex(startVoxelIndex);
  if (targetColorIndex === nextColorIndex) {
    return 0;
  }
  const size = sprite3d.size;
  const visited = new Uint8Array(sprite3d.cells.length);
  const stack = [startCoords];
  let changed = 0;
  while (stack.length) {
    const current = stack.pop();
    if (
      current.x < 0 || current.y < 0 || current.z < 0
      || current.x >= size || current.y >= size || current.z >= size
    ) {
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
  if (!Number.isInteger(index) || index < 0 || index >= sprite3d.size * sprite3d.size) {
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
  if (currentPreviewMode !== "sprite3d" || !sprite3dClipActive || sprite3dBuilder.hidden
    || spriteClipShortcutTargetIsText(event.target)) {
    return false;
  }
  const key = event.key.length === 1 ? event.key.toLowerCase() : event.key;
  const modifier = (event.metaKey && !event.ctrlKey) || (event.ctrlKey && !event.metaKey);
  let handled = false;
  if (modifier && !event.altKey && !event.shiftKey && key === "c") {
    handled = copySprite3dClipSelection();
  } else if (modifier && !event.altKey && !event.shiftKey && key === "x") {
    handled = cutSprite3dClipSelection();
  } else if (modifier && !event.altKey && !event.shiftKey && key === "v") {
    handled = pasteSprite3dClipClipboard();
  } else if (!modifier && !event.altKey && (key === "Backspace" || key === "Delete")) {
    handled = clearSprite3dClipSelection();
  } else if (!modifier && !event.altKey && key === "Escape") {
    deactivateSprite3dClipMode();
    setSprite3dActionStatus("Brush: paint individual voxels", "is-ok");
    handled = true;
  } else if (!modifier && !event.altKey && ["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(key)) {
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
  sprite3dPaintDrag.lastIndex = index;
  if (paintSprite3dCellAtSliceIndex(index, sprite3dPaintDrag.colorIndex)) {
    sprite3dPaintDrag.changed = true;
  }
}

function updateSprite3dSize(value) {
  const before = visualEditSnapshot("sprite3d");
  const nextSize = clampSprite3dSize(value);
  if (nextSize === sprite3d.size) {
    renderSprite3dControls();
    return;
  }
  const previousSize = sprite3d.size;
  const previousCells = sprite3d.cells;
  const nextCells = Array.from({ length: nextSize * nextSize * nextSize }, () => null);
  const copySize = Math.min(previousSize, nextSize);
  for (let z = 0; z < copySize; z += 1) {
    for (let y = 0; y < copySize; y += 1) {
      for (let x = 0; x < copySize; x += 1) {
        nextCells[((z * nextSize + y) * nextSize) + x] = previousCells[((z * previousSize + y) * previousSize) + x];
      }
    }
  }
  sprite3d.size = nextSize;
  resetSprite3dClipState();
  sprite3d.slice = Math.min(sprite3d.slice, nextSize - 1);
  sprite3d.cells = nextCells;
  renderSprite3dBuilder();
  pushVisualEditUndoSnapshot("sprite3d", before);
}

function sprite3dScaleFactor() {
  return spriteEditorScaleFactor(sprite3dScaleInput, SPRITE3D_EDITOR_MAX_SIZE);
}

function canScaleDownSprite3d(factor = sprite3dScaleFactor()) {
  return factor > 1 && sprite3d.size >= factor && sprite3d.size % factor === 0;
}

function scaleUpSprite3d() {
  const before = visualEditSnapshot("sprite3d");
  const factor = sprite3dScaleFactor();
  const previousSize = sprite3d.size;
  const nextSize = previousSize * factor;
  if (nextSize > SPRITE3D_EDITOR_MAX_SIZE) {
    setSprite3dActionStatus(`3D sprite size limit is ${SPRITE3D_EDITOR_MAX_SIZE}`, "is-error");
    renderSprite3dControls();
    return;
  }

  const previousCells = sprite3d.cells;
  const nextCells = Array.from({ length: nextSize * nextSize * nextSize }, () => null);
  for (let z = 0; z < previousSize; z += 1) {
    for (let y = 0; y < previousSize; y += 1) {
      for (let x = 0; x < previousSize; x += 1) {
        const sourceIndex = ((z * previousSize + y) * previousSize) + x;
        const colorIndex = validSprite3dColorIndex(previousCells[sourceIndex])
          ? previousCells[sourceIndex]
          : null;
        const nextX = x * factor;
        const nextY = y * factor;
        const nextZ = z * factor;
        for (let dz = 0; dz < factor; dz += 1) {
          for (let dy = 0; dy < factor; dy += 1) {
            for (let dx = 0; dx < factor; dx += 1) {
              nextCells[(((nextZ + dz) * nextSize + (nextY + dy)) * nextSize) + nextX + dx] = colorIndex;
            }
          }
        }
      }
    }
  }

  sprite3d.size = nextSize;
  resetSprite3dClipState();
  sprite3d.slice = Math.min(sprite3d.slice * factor, nextSize - 1);
  sprite3d.hoverSlice = null;
  sprite3d.cells = nextCells;
  renderSprite3dBuilder();
  const message = `Scaled ${factor}x to ${nextSize}x${nextSize}x${nextSize}`;
  setSprite3dActionStatus(message, "is-ok");
  setStatus(`Scaled 3D sprite ${factor}x to ${nextSize}x${nextSize}x${nextSize}`, "is-ok");
  pushVisualEditUndoSnapshot("sprite3d", before);
}

function scaleDownSprite3d() {
  const before = visualEditSnapshot("sprite3d");
  const factor = sprite3dScaleFactor();
  if (!canScaleDownSprite3d(factor)) {
    setSprite3dActionStatus(`Size ${sprite3d.size} is not divisible by ${factor}`, "is-error");
    renderSprite3dControls();
    return;
  }

  const previousSize = sprite3d.size;
  const nextSize = previousSize / factor;
  const previousCells = sprite3d.cells;
  const nextCells = Array.from({ length: nextSize * nextSize * nextSize }, () => null);
  for (let z = 0; z < nextSize; z += 1) {
    for (let y = 0; y < nextSize; y += 1) {
      for (let x = 0; x < nextSize; x += 1) {
        const sourceIndex = (((z * factor) * previousSize + (y * factor)) * previousSize) + (x * factor);
        const colorIndex = previousCells[sourceIndex];
        nextCells[((z * nextSize + y) * nextSize) + x] = validSprite3dColorIndex(colorIndex) ? colorIndex : null;
      }
    }
  }

  sprite3d.size = nextSize;
  resetSprite3dClipState();
  sprite3d.slice = Math.min(Math.floor(sprite3d.slice / factor), nextSize - 1);
  sprite3d.hoverSlice = null;
  sprite3d.cells = nextCells;
  renderSprite3dBuilder();
  const message = `Scaled down ${factor}x to ${nextSize}x${nextSize}x${nextSize}`;
  setSprite3dActionStatus(message, "is-ok");
  setStatus(`Scaled 3D sprite down ${factor}x to ${nextSize}x${nextSize}x${nextSize}`, "is-ok");
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
  sprite3d.hoverSlice = null;
  renderSprite3dBuilder();
}

function setSprite3dSlice(value) {
  const nextSlice = Math.max(0, Math.min(sprite3d.size - 1, Math.trunc(Number(value) || 0)));
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

function clearSprite3dSlice() {
  const before = visualEditSnapshot("sprite3d");
  for (let index = 0; index < sprite3d.size * sprite3d.size; index += 1) {
    const coords = sprite3dCoordsFromSliceCell(index);
    sprite3d.cells[sprite3dCellIndex(coords.x, coords.y, coords.z)] = null;
  }
  renderSprite3dBuilder();
  setSprite3dActionStatus("Cleared current 2D slice", "is-ok");
  pushVisualEditUndoSnapshot("sprite3d", before);
}

function clearSprite3dBuilder() {
  const before = visualEditSnapshot("sprite3d");
  resetSprite3dBuilder(sprite3d.size);
  setSprite3dActionStatus("Cleared whole 3D sprite", "is-ok");
  pushVisualEditUndoSnapshot("sprite3d", before);
}

function clearSprite3dScoped() {
  if (sprite3dEditScope() === "all") {
    clearSprite3dBuilder();
  } else {
    clearSprite3dSlice();
  }
}

function transformSprite3dCells(mapper, message) {
  const before = visualEditSnapshot("sprite3d");
  const size = sprite3d.size;
  const previousCells = sprite3d.cells;
  const nextCells = Array.from({ length: size * size * size }, () => null);
  for (let z = 0; z < size; z += 1) {
    for (let y = 0; y < size; y += 1) {
      for (let x = 0; x < size; x += 1) {
        const sourceIndex = sprite3dCellIndex(x, y, z);
        const colorIndex = previousCells[sourceIndex];
        if (!validSprite3dColorIndex(colorIndex)) {
          continue;
        }
        const target = mapper(x, y, z, size);
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
  const max = sprite3d.size - 1;
  if (axis === "x") {
    return { stack: x, u: max - y, v: max - z };
  }
  if (axis === "y") {
    return { stack: max - y, u: x, v: max - z };
  }
  return { stack: max - z, u: x, v: max - y };
}

function sprite3dCoordsFromPlane(axis, stack, u, v) {
  return sprite3dCoordsFromPlaneForSize(sprite3d.size, axis, stack, u, v);
}

function sprite3dCoordsFromPlaneForSize(size, axis, stack, u, v) {
  const max = size - 1;
  const fixed = sprite3dPlaneWorldSlice(axis, stack, size);
  if (axis === "x") {
    return { x: fixed, y: max - u, z: max - v };
  }
  if (axis === "y") {
    return { x: u, y: fixed, z: max - v };
  }
  return { x: u, y: max - v, z: fixed };
}

function sprite3dPlaneWorldSlice(axis, stack, size = sprite3d.size) {
  const normalized = Math.max(0, Math.min(size - 1, Math.trunc(Number(stack) || 0)));
  return axis === "x" ? normalized : size - 1 - normalized;
}

function sprite3dCurrentSliceDescriptor() {
  return {
    axis: ["x", "y", "z"].includes(sprite3d.axis) ? sprite3d.axis : "z",
    slice: Math.max(0, Math.min(sprite3d.size - 1, Math.trunc(Number(sprite3d.slice) || 0))),
    size: sprite3d.size,
  };
}

function readSprite3dSliceCells(axis, slice, size = sprite3d.size) {
  const cells = [];
  for (let v = 0; v < size; v += 1) {
    for (let u = 0; u < size; u += 1) {
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
  for (let v = 0; v < sprite3d.size; v += 1) {
    for (let u = 0; u < sprite3d.size; u += 1) {
      const colorIndex = cells[(v * sprite3d.size) + u];
      const target = sprite3dCoordsFromPlane(axis, slice, u, v);
      sprite3d.cells[sprite3dCellIndex(target.x, target.y, target.z)] = validSprite3dColorIndex(colorIndex)
        ? colorIndex
        : null;
    }
  }
}

function transformSprite3dCurrentPlane(mapper, message) {
  const axis = ["x", "y", "z"].includes(sprite3d.axis) ? sprite3d.axis : "z";
  transformSprite3dCells((x, y, z, size) => {
    const plane = sprite3dPlaneCoordinates(axis, x, y, z);
    const next = mapper(plane.u, plane.v, size);
    return sprite3dCoordsFromPlane(axis, plane.stack, next.u, next.v);
  }, `${message} all ${axis.toUpperCase()} slices`);
}

function transformSprite3dCurrentSlice(mapper, message) {
  const before = visualEditSnapshot("sprite3d");
  const source = sprite3dCurrentSliceDescriptor();
  const previousCells = readSprite3dSliceCells(source.axis, source.slice, source.size);
  const nextCells = Array.from({ length: source.size * source.size }, () => null);
  for (let v = 0; v < source.size; v += 1) {
    for (let u = 0; u < source.size; u += 1) {
      const colorIndex = previousCells[(v * source.size) + u];
      if (!validSprite3dColorIndex(colorIndex)) {
        continue;
      }
      const next = mapper(u, v, source.size);
      nextCells[(next.v * source.size) + next.u] = colorIndex;
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
  transformSprite3dScoped((u, v, size) => ({ u: v, v: size - 1 - u }), "Rotated left");
}

function rotateSprite3dPlaneRight() {
  transformSprite3dScoped((u, v, size) => ({ u: size - 1 - v, v: u }), "Rotated right");
}

function flipSprite3dPlaneHorizontal() {
  transformSprite3dScoped((u, v, size) => ({ u: size - 1 - u, v }), "Flipped horizontal");
}

function flipSprite3dPlaneVertical() {
  transformSprite3dScoped((u, v, size) => ({ u, v: size - 1 - v }), "Flipped vertical");
}

function copySprite3dSlice() {
  const source = sprite3dCurrentSliceDescriptor();
  const cells = readSprite3dSliceCells(source.axis, source.slice, source.size);
  sprite3d.sliceClipboard = {
    ...source,
    cells,
    colors: sprite3dSliceCellColors(cells),
    palette: sprite3dPaletteColors(),
  };
  renderSprite3dControls();
  setSprite3dActionStatus(`Copied 2D ${source.axis.toUpperCase()} slice ${source.slice + 1}`, "is-ok");
}

function pasteSprite3dSlice() {
  const before = visualEditSnapshot("sprite3d");
  const copied = sprite3d.sliceClipboard;
  if (!copied) {
    setSprite3dActionStatus("No copied slice", "is-error");
    return;
  }
  const target = sprite3dCurrentSliceDescriptor();
  if (copied.size !== target.size || !Array.isArray(copied.cells) || copied.cells.length !== target.size * target.size) {
    setSprite3dActionStatus(`Copied slice is ${copied.size}x${copied.size}; current sprite is ${target.size}x${target.size}`, "is-error");
    return;
  }
  const pasted = sprite3dPastedSliceCells(copied, target.size);
  if (pasted.error) {
    setSprite3dActionStatus(pasted.error, "is-error");
    return;
  }
  writeSprite3dSliceCells(target.axis, target.slice, pasted.cells);
  sprite3d.hoverSlice = null;
  renderSprite3dBuilder();
  const paletteMessage = pasted.addedColors > 0 ? `, added ${pasted.addedColors} color${pasted.addedColors === 1 ? "" : "s"}` : "";
  setSprite3dActionStatus(
    `Pasted 2D ${copied.axis.toUpperCase()} slice ${copied.slice + 1} to ${target.axis.toUpperCase()} slice ${target.slice + 1}${paletteMessage}`,
    "is-ok",
  );
  pushVisualEditUndoSnapshot("sprite3d", before);
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
  for (let z = 0; z < sprite3d.size; z += 1) {
    if (z > 0) {
      rows.push("-");
    }
    for (let y = 0; y < sprite3d.size; y += 1) {
      const row = [];
      for (let x = 0; x < sprite3d.size; x += 1) {
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
  const frames = Array.isArray(sprite3d.frames) && sprite3d.frames.length
    ? sprite3d.frames.map((frame) => Array.isArray(frame) ? frame.slice() : [])
    : [[]];
  frames[0] = sprite3d.cells.slice();
  return frames.map((frame) => Array.from({ length: sprite3d.size }, (_, z) =>
    Array.from({ length: sprite3d.size }, (_, y) =>
      Array.from({ length: sprite3d.size }, (_, x) => {
        const cell = frame[sprite3dCellIndex(x, y, z)];
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
    durationMs: sprite3d.animationDurationMs,
    frameDurationMs: sprite3d.frameDurationMs,
    shapeRef: shape.linked ? shape.name : null,
    spatialOps: sprite3d.sourceSpatialOps || [],
    colorBindings,
  };
}

async function exportSprite3dSource() {
  const text = sprite3dClipboardText();
  try {
    window.focus();
    sprite3dExportButton?.focus({ preventScroll: true });
    await copyTextToClipboard(text);
    setSprite3dActionStatus("Copied 3D sprite", "is-ok");
    setStatus("Copied 3D sprite", "is-ok");
  } catch (error) {
    setSprite3dActionStatus("Copy failed", "is-error");
    setStatus(`Could not copy 3D sprite: ${error?.message || error}`, "is-error");
  }
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

async function duplicateSprite3dInSource() {
  let result;
  try {
    ({ result } = await commitSpriteEditorMutation({
      state: sprite3d,
      request: () => sprite3dEditMutationRequest("duplicate"),
    }));
  } catch (error) {
    setSprite3dActionStatus("No selected 3D sprite source range", "is-error");
    setStatus("No selected 3D sprite source range", "is-error");
    setSprite3dActionStatus(userFacingRuntimeError(error), "is-error");
    return;
  }
  sprite3dNameInput.value = result.name;
  syncSprite3dSourceActionButtons();
  setSprite3dActionStatus("Duplicated 3D sprite", "is-ok");
  setStatus("Duplicated 3D sprite", "is-ok");
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
  if (target?.kind !== "sprite3d") {
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
  if (width < 1 || width !== height || width !== depth || !palette.length || !frames.length || frames.some((frame) => frame.length !== frameCellCount)) {
    return null;
  }
  return {
    size: width,
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
  if (duplicateSprite3dButton) {
    duplicateSprite3dButton.disabled = !hasEditableSource;
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
  sprite3d.size = clampSprite3dSize(sprite3d.size);
  sprite3d.axis = "z";
  sprite3d.slice = 0;
  sprite3d.hoverSlice = null;
  sprite3d.palette = [];
  sprite3d.cells = Array.from({ length: sprite3d.size * sprite3d.size * sprite3d.size }, () => null);
  sprite3d.frames = [sprite3d.cells.slice()];
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
  sprite3d.size = loaded.size;
  sprite3d.axis = "z";
  sprite3d.slice = 0;
  sprite3d.hoverSlice = null;
  sprite3d.palette = loaded.palette;
  sprite3d.cells = loaded.cells;
  sprite3d.frames = loaded.frames;
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
    copySprite3dSlice();
  } else if (key === "v") {
    event.preventDefault();
    event.stopPropagation();
    pasteSprite3dSlice();
  }
}

function resetSprite3dCamera() {
  sprite3d.camera = { ...SPRITE3D_CAMERA_DEFAULT };
  sprite3d.hoverSlice = null;
  renderSprite3dCameraControls();
  renderSprite3dPreview();
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
  renderSprite3dPreview();
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
    || sprite3dPreviewView(Math.max(1, Math.round(rect.width)), Math.max(1, Math.round(rect.height)), sprite3d.size);
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
  const center = (view.size - 1) / 2;
  return {
    origin: {
      x: center + screenU * cosYaw + yawYAtDepthZero * sinYaw,
      y: center - screenU * sinYaw + yawYAtDepthZero * cosYaw,
      z: center - cosPitch * screenV,
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
  for (let z = 0; z < sprite3d.size; z += 1) {
    for (let y = 0; y < sprite3d.size; y += 1) {
      for (let x = 0; x < sprite3d.size; x += 1) {
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
    max: { x: sprite3d.size - 0.5, y: sprite3d.size - 0.5, z: sprite3d.size - 0.5 },
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
  const max = sprite3d.size - 1;
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
  const world = Math.max(0, Math.min(sprite3d.size - 1, Math.round(edge.min + (edge.max - edge.min) * t)));
  return {
    index: Math.max(0, Math.min(sprite3d.size - 1, sprite3d.size - 1 - world)),
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
  sprite3dSizeInput,
  sprite3dScaleInput,
  sprite3dSliceValue,
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
sprite3dSizeInput?.addEventListener("change", () => updateSprite3dSize(sprite3dSizeInput.value));
sprite3dSizeInput?.addEventListener("keydown", (event) => {
  if (event.key !== "Enter") {
    return;
  }
  event.preventDefault();
  updateSprite3dSize(sprite3dSizeInput.value);
});
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
sprite3dClearButton?.addEventListener("click", clearSprite3dScoped);
sprite3dScaleDownButton?.addEventListener("click", scaleDownSprite3d);
sprite3dScaleUpButton?.addEventListener("click", scaleUpSprite3d);
sprite3dRotatePlaneLeftButton?.addEventListener("click", rotateSprite3dPlaneLeft);
sprite3dRotatePlaneRightButton?.addEventListener("click", rotateSprite3dPlaneRight);
sprite3dFlipPlaneHorizontalButton?.addEventListener("click", flipSprite3dPlaneHorizontal);
sprite3dFlipPlaneVerticalButton?.addEventListener("click", flipSprite3dPlaneVertical);
sprite3dCopySliceButton?.addEventListener("click", copySprite3dSlice);
sprite3dPasteSliceButton?.addEventListener("click", pasteSprite3dSlice);
sprite3dTranslateButton?.addEventListener("click", toggleSprite3dTranslateMode);
sprite3dScopeSliceButton?.addEventListener("click", () => setSprite3dEditScope("slice"));
sprite3dScopeAllButton?.addEventListener("click", () => setSprite3dEditScope("all"));
sprite3dFillButton?.addEventListener("click", toggleSprite3dBucketMode);
sprite3dExportButton?.addEventListener("click", exportSprite3dSource);
sprite3dUpdateButton?.addEventListener("click", () => {
  updateSprite3dInSource().catch((error) => {
    console.error(error);
    setSprite3dActionStatus("3D sprite source update failed", "is-error");
    setStatus("3D sprite source update failed", "is-error");
  });
});
duplicateSprite3dButton?.addEventListener("click", () => {
  duplicateSprite3dInSource().catch((error) => {
    console.error(error);
    setSprite3dActionStatus("3D sprite duplication failed", "is-error");
    setStatus("3D sprite duplication failed", "is-error");
  });
});
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
