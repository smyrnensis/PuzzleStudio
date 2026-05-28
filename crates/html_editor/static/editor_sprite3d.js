let sprite3dActionClearTimer = 0;
let sprite3dPreviewDrag = null;
let sprite3dCameraScrubDrag = null;
let sprite3dSliceScrubDrag = null;
let sprite3dBucketActive = false;
const SPRITE3D_EDITOR_MAX_SIZE = 64;
const SPRITE3D_SLICE_SCRUB_STEP_PX = 18;
const SPRITE3D_CAMERA_MIN_PITCH_DEGREES = -90;
const SPRITE3D_CAMERA_MAX_PITCH_DEGREES = 90;
const SPRITE3D_CAMERA_DEFAULT = {
  yawDegrees: 15,
  pitchDegrees: 30,
  zoom: 1,
};

function resetSprite3dBuilder(size = sprite3d.size) {
  ensureSprite3dPalette();
  sprite3d.size = clampSprite3dSize(size);
  sprite3d.slice = Math.max(0, Math.min(sprite3d.size - 1, Number(sprite3d.slice) || 0));
  sprite3d.hoverSlice = null;
  sprite3d.cells = Array.from({ length: sprite3d.size * sprite3d.size * sprite3d.size }, () => null);
  if (!validSprite3dColorIndex(sprite3d.selectedColorIndex)) {
    sprite3d.selectedColorIndex = 0;
  }
  renderSprite3dBuilder();
}

function clampSprite3dSize(value) {
  const size = Math.trunc(Number(value) || 5);
  return Math.max(1, Math.min(SPRITE3D_EDITOR_MAX_SIZE, size));
}

function renderSprite3dBuilder() {
  if (!sprite3dBuilder || !sprite3dSliceBoard || !sprite3dPalette || !sprite3dPreviewCanvas) {
    return;
  }
  renderSprite3dControls();
  renderSprite3dPalette();
  renderSprite3dSliceBoard();
  renderSprite3dPreview();
}

function renderSprite3dControls() {
  sprite3dNameInput.value = sprite3dNameInput.value || "VoxelSprite";
  sprite3dSizeInput.value = String(sprite3d.size);
  syncSprite3dBucketButton();
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
      label: "Edit scope: current slice",
      title: "Scope: current slice",
    },
    {
      button: sprite3dScopeAllButton,
      scope: "all",
      label: "Edit scope: whole sprite",
      title: "Scope: whole sprite",
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
  setSprite3dButtonLabel(
    sprite3dRotatePlaneLeftButton,
    isAll ? "Rotate whole sprite counterclockwise" : "Rotate current slice counterclockwise",
  );
  setSprite3dButtonLabel(
    sprite3dRotatePlaneRightButton,
    isAll ? "Rotate whole sprite clockwise" : "Rotate current slice clockwise",
  );
  setSprite3dButtonLabel(
    sprite3dFlipPlaneHorizontalButton,
    isAll ? "Flip whole sprite horizontally" : "Flip current slice horizontally",
  );
  setSprite3dButtonLabel(
    sprite3dFlipPlaneVerticalButton,
    isAll ? "Flip whole sprite vertically" : "Flip current slice vertically",
  );
  setSprite3dButtonLabel(
    sprite3dFillButton,
    isAll ? "Bucket fill 3D connected component" : "Bucket fill current slice connected component",
  );
  setSprite3dButtonLabel(sprite3dClearButton, isAll ? "Clear whole sprite" : "Clear current slice");
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
  sprite3d.editScope = scope === "all" ? "all" : "slice";
  renderSprite3dScopeControl();
  setSprite3dActionStatus(
    sprite3d.editScope === "all" ? "3D edits affect the whole sprite" : "2D edits affect the current slice",
    "is-ok",
  );
}

function toggleSprite3dBucketMode() {
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

function renderSprite3dPalette() {
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
      positionSpriteColorMenu(pendingEditMenu, currentButton, { side: "right" });
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

    const colorInput = document.createElement("input");
    colorInput.type = "color";
    colorInput.className = "sprite-token-color-input";
    colorInput.value = spriteRgbHex(entry.color);
    colorInput.setAttribute("aria-label", `Edit 3D sprite color ${index + 1}`);
    colorInput.addEventListener("input", () => {
      sprite3d.selectedColorIndex = index;
      updateSelectedSprite3dColor(colorInput.value, { deferHistory: true });
    });
    colorInput.addEventListener("change", () => {
      sprite3d.selectedColorIndex = index;
      updateSelectedSprite3dColor(colorInput.value, { commitHistory: true });
    });
    item.append(colorInput);
    paletteGrid.append(item);
  }
  const addWrap = document.createElement("span");
  addWrap.className = "sprite-add-wrap";
  const addButton = document.createElement("button");
  addButton.type = "button";
  addButton.className = "sprite-token sprite-add-color-button";
  addButton.disabled = sprite3dPaletteEntries().length >= SPRITE_COLOR_TOKENS.length;
  addButton.title = "Add sprite color";
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
  sprite3dSliceBoard.replaceChildren();
  sprite3dSliceBoard.style.setProperty("--sprite-size", sprite3d.size);
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
    button.style.setProperty("--sprite-swatch-color", sprite3dColorForColorIndex(colorIndex));
    button.style.setProperty("--sprite-cell-ink", sprite3dInkForColorIndex(colorIndex));
    button.setAttribute("aria-label", `Voxel ${coords.x + 1}, ${coords.y + 1}, ${coords.z + 1}`);
    sprite3dSliceBoard.append(button);
  }
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
  sprite3dPreviewCanvas._sprite3dPreviewView = view;
  renderSprite3dCameraControls();
}

function sprite3dPreviewView(width, height, size) {
  const padding = 22;
  const overlayClearanceY = 14;
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
  const scale = Math.max(4, Math.min(availableWidth / projectedWidth, availableHeight / projectedHeight))
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
  if (!Array.isArray(sprite3d.palette) || sprite3d.palette.length === 0) {
    sprite3d.palette = [{ color: "#ff004d" }];
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
  const colorIndex = sprite3d.selectedColorIndex;
  const allScope = sprite3dEditScope() === "all";
  const count = allScope
    ? floodFillSprite3dComponentAtSliceIndex(index, colorIndex)
    : floodFillSprite3dSliceComponentAtIndex(index, colorIndex);
  if (!count) {
    setSprite3dActionStatus("Connected component already has that color", "is-ok");
    return false;
  }
  sprite3d.addPaletteOpen = false;
  sprite3d.editPaletteOpen = false;
  sprite3d.customColorOpen = false;
  sprite3d.addDraftColorIndex = null;
  sprite3d.hoverSlice = null;
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

function startSprite3dPaint(event) {
  if (event.button !== 0) {
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
  if (!sprite3dPaintDrag || sprite3dPaintDrag.pointerId !== event.pointerId) {
    return;
  }
  event.preventDefault();
  paintSprite3dDragIndex(sprite3dSliceCellIndexFromElement(document.elementFromPoint(event.clientX, event.clientY)));
}

function stopSprite3dPaint(event) {
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
  sprite3d.axis = ["x", "y", "z"].includes(axis) ? axis : "z";
  sprite3d.hoverSlice = null;
  renderSprite3dBuilder();
}

function setSprite3dSlice(value) {
  sprite3d.slice = Math.max(0, Math.min(sprite3d.size - 1, Math.trunc(Number(value) || 0)));
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

function sprite3dObjectDefinitionText(indent) {
  const lines = [
    `${indent}${sprite3dObjectName()}`,
    `${indent}${sprite3dPaletteSourceTokens().join(" ")}`,
    ...sprite3dVoxelRows().map((row) => (row ? `${indent}${row}` : "")),
  ];
  return lines.join("\n");
}

function sprite3dPaletteSourceTokens() {
  return sprite3dPaletteEntries().map((entry) => sprite3dPaletteSourceToken(entry));
}

function sprite3dPaletteSourceToken(entry) {
  const color = normalizeSpriteColor(entry?.color || "#00000000");
  return color === "#00000000" ? "transparent" : color;
}

function sprite3dVoxelRows() {
  const rows = [];
  for (let z = 0; z < sprite3d.size; z += 1) {
    if (z > 0) {
      rows.push("");
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

function addSprite3dToSource() {
  const document = activeSprite3dEditDocument();
  if (!document || !isTextDocument(document)) {
    setSprite3dActionStatus("No puzzle source", "is-error");
    setStatus("No puzzle source for 3D sprite", "is-error");
    return;
  }

  const result = insertSprite3dDefinition(activeSprite3dEditSource());
  applySprite3dSourceChange(document, result.source, "Added 3D sprite");
}

function updateSprite3dInSource() {
  const document = activeSprite3dEditDocument();
  if (!document || !isTextDocument(document)) {
    setSprite3dActionStatus("No puzzle source", "is-error");
    setStatus("No puzzle source for 3D sprite", "is-error");
    return;
  }

  const result = replaceSprite3dDefinition(activeSprite3dEditSource());
  if (!result) {
    const name = sprite3dObjectName();
    setSprite3dActionStatus(`No 3D sprite named ${name}`, "is-error");
    setStatus(`No 3D sprite named ${name}`, "is-error");
    return;
  }
  applySprite3dSourceChange(document, result.source, "Updated 3D sprite");
}

function activeSprite3dEditDocument() {
  if (typeof activeSpriteEditDocument === "function") {
    return activeSpriteEditDocument();
  }
  const document = activeDocument();
  return document && isTextDocument(document) && isPuzzleDocument(document) ? document : activePreviewDocument();
}

function activeSprite3dEditSource() {
  const document = activeSprite3dEditDocument();
  if (!document || !isTextDocument(document)) {
    return "";
  }
  return document.id === activeDocument()?.id ? sourceEditor.value : document.source || "";
}

function applySprite3dSourceChange(document, source, statusText) {
  document.source = source;
  if (document.id === activeDocument()?.id) {
    setSourceEditorValue(source, { resetUndo: false });
  }
  scheduleLocalSave();
  schedulePreview();
  sourceEditor.focus();
  setSprite3dActionStatus(statusText, "is-ok");
  setStatus(statusText, "is-ok");
}

function insertSprite3dDefinition(source) {
  const block = findSprites3dBlock(source);
  if (!block) {
    const puzzle3Block = findPuzzle3Block(source);
    if (puzzle3Block) {
      return {
        source: `${source.slice(0, puzzle3Block.end).trimEnd()}\n\nsprites3 generated of ${puzzle3Block.name} {\n${sprite3dObjectDefinitionText("  ")}\n}\n${source.slice(puzzle3Block.end)}`,
      };
    }
    const prefix = source.trimEnd() ? `${source.trimEnd()}\n\n` : "";
    return {
      source: `${prefix}sprites3 {\n${sprite3dObjectDefinitionText("  ")}\n}\n`,
    };
  }

  const indent = `${block.indent}  `;
  return {
    source: `${source.slice(0, block.bodyEnd).trimEnd()}\n\n${sprite3dObjectDefinitionText(indent)}\n${source.slice(block.bodyEnd)}`,
  };
}

function replaceSprite3dDefinition(source) {
  const entry = findSprite3dDefinitionByName(source, sprite3dObjectName());
  if (!entry) {
    return null;
  }
  const replacement = sprite3dObjectDefinitionText(entry.indent);
  return {
    source: replaceEditorSourceRangePreservingLineBoundary(source, entry.start, entry.end, replacement),
  };
}

function loadSprite3dFromSourcePosition(position, options = {}) {
  if (!isPuzzleDocument(activeDocument()) || !isTextDocument(activeDocument())) {
    return null;
  }
  const source = sourceEditor.value || "";
  const entry = findSprite3dDefinitionAtPosition(source, position);
  if (!entry) {
    return null;
  }
  const loaded = parseSprite3dDefinitionSource(source, entry);
  if (!loaded) {
    if (!options.silent) {
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
  applyLoadedSprite3d(entry.name, loaded);
  if (!options.silent) {
    setSprite3dActionStatus(`Loaded ${entry.name}`, "is-ok");
    setStatus(`Loaded 3D sprite ${entry.name}`, "is-ok");
  }
  return `sprite3d:${entry.name}:${entry.start}`;
}

function loadSprite3dSourceTarget(target, options = {}) {
  if (!isPuzzleDocument(activeDocument()) || !isTextDocument(activeDocument())) {
    return null;
  }
  const source = sourceEditor.value || "";
  if (!Number.isInteger(target?.bodyStart) || !Number.isInteger(target?.bodyEnd)) {
    return null;
  }
  const loaded = parseSprite3dDefinitionSource(source, {
    name: target.name || "VoxelSprite",
    bodyStart: target.bodyStart,
    bodyEnd: target.bodyEnd,
  });
  if (!loaded) {
    if (!options.silent) {
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
  applyLoadedSprite3d(target.name || "VoxelSprite", loaded);
  if (!options.silent) {
    setSprite3dActionStatus(`Loaded ${sprite3dNameInput.value}`, "is-ok");
    setStatus(`Loaded 3D sprite ${sprite3dNameInput.value}`, "is-ok");
  }
  return `sprite3d:${target.name}:${target.start ?? target.bodyStart}`;
}

function applyLoadedSprite3d(name, loaded) {
  sprite3dNameInput.value = name || "VoxelSprite";
  sprite3d.size = loaded.size;
  sprite3d.axis = "z";
  sprite3d.slice = 0;
  sprite3d.hoverSlice = null;
  sprite3d.palette = loaded.palette;
  sprite3d.cells = loaded.cells;
  sprite3d.selectedColorIndex = sprite3d.palette.length ? 0 : null;
  sprite3d.addPaletteOpen = false;
  sprite3d.editPaletteOpen = false;
  sprite3d.customColorOpen = false;
  sprite3d.addDraftColorIndex = null;
  renderSprite3dBuilder();
}

function findSprites3dBlock(source) {
  return findSprites3dBlocks(source)[0] || null;
}

function findSprites3dBlocks(source) {
  const pattern = /(^|\n)([\t ]*)sprites3(?:\s+[^\n{]+)?\s*\{/gm;
  const blocks = [];
  let match = null;
  while ((match = pattern.exec(source))) {
    const start = match.index + match[1].length;
    const openIndex = source.indexOf("{", start);
    const closeIndex = findMatchingBrace(source, openIndex);
    if (openIndex < 0 || closeIndex < 0) {
      continue;
    }
    blocks.push({
      start,
      openIndex,
      closeIndex,
      indent: match[2] || "",
      bodyStart: openIndex + 1,
      bodyEnd: closeIndex,
    });
    pattern.lastIndex = closeIndex + 1;
  }
  return blocks;
}

function findSprite3dDefinitionAtPosition(source, position) {
  for (const block of findSprites3dBlocks(source)) {
    if (position < block.bodyStart || position > block.bodyEnd) {
      continue;
    }
    const entry = findSprite3dDefinitions(source, block)
      .find((candidate) => position >= candidate.start && position <= candidate.end);
    if (entry) {
      return entry;
    }
  }
  return null;
}

function findSprite3dDefinitionByName(source, name) {
  for (const block of findSprites3dBlocks(source)) {
    const entry = findSprite3dDefinitionBlock(source, block, name);
    if (entry) {
      return entry;
    }
  }
  return null;
}

function findSprite3dDefinitionBlock(source, block, name) {
  return findSprite3dDefinitions(source, block)
    .find((entry) => entry.name === name) || null;
}

function findSprite3dDefinitions(source, block) {
  return findCanonicalSprite3dDefinitions(source, block).sort((a, b) => a.start - b.start);
}

function findCanonicalSprite3dDefinitions(source, block) {
  const lines = editorSourceLinesWithOffsets(source)
    .filter((line) => line.start >= block.bodyStart && line.start < block.bodyEnd);
  const entries = [];
  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    const code = stripLineCommentForWasm(line.raw).trim();
    if (!isCanonicalSprite3dName(code) || !nextSprite3dPaletteLine(lines, index)) {
      continue;
    }
    const start = firstEditorSourceCodeIndex(line);
    let end = line.absoluteEnd;
    for (let next = index + 1; next < lines.length; next += 1) {
      const nextCode = stripLineCommentForWasm(lines[next].raw).trim();
      if (
        nextCode.startsWith("sprite ")
        || (isCanonicalSprite3dName(nextCode) && nextSprite3dPaletteLine(lines, next))
      ) {
        break;
      }
      end = lines[next].absoluteEnd;
    }
    entries.push({
      name: code,
      start,
      end,
      indent: line.raw.match(/^[\t ]*/)?.[0] || `${block.indent}  `,
      bodyStart: line.absoluteEnd,
      bodyEnd: end,
      format: "canonical",
    });
  }
  return entries;
}

function nextSprite3dPaletteLine(lines, index) {
  for (const line of lines.slice(index + 1)) {
    const code = stripLineCommentForWasm(line.raw).trim();
    if (!code) {
      continue;
    }
    return isSprite3dPaletteRow(code);
  }
  return false;
}

function isCanonicalSprite3dName(value) {
  return /^@?[A-Za-z_][\w:]*$/.test(value || "") && !isSprite3dPaletteRow(value);
}

function parseSprite3dDefinitionSource(source, entry) {
  const body = source.slice(entry.bodyStart, entry.bodyEnd);
  const rows = body.split("\n").map((row) => row.trim());
  const firstPaletteIndex = rows.findIndex((row) => row && isSprite3dPaletteRow(row));
  if (firstPaletteIndex < 0) {
    return null;
  }
  return parseSprite3dRows(
    parseSprite3dPaletteTokens(rows[firstPaletteIndex].split(/\s+/).filter(Boolean)),
    rows.slice(firstPaletteIndex + 1),
  );
}

function parseSprite3dPaletteTokens(tokens) {
  const entries = [];
  for (const [index, token] of tokens.entries()) {
    const color = parseSprite3dColorToken(token);
    if (!color || !SPRITE_COLOR_TOKENS[index]) {
      return null;
    }
    entries.push({ key: SPRITE_COLOR_TOKENS[index], color });
  }
  return entries.length ? entries : null;
}

function parseSprite3dColorToken(token) {
  if (String(token || "").toLowerCase() === "transparent") {
    return "#00000000";
  }
  return parseSpriteHexColor(token);
}

function isSprite3dPaletteRow(line) {
  const tokens = String(line || "").split(/\s+/).filter(Boolean);
  return tokens.length > 0 && tokens.every((token) => Boolean(parseSprite3dColorToken(token)));
}

function parseSprite3dRows(paletteEntries, rawRows) {
  if (!paletteEntries?.length) {
    return null;
  }
  const slices = [];
  let current = [];
  for (const row of rawRows) {
    if (!row) {
      if (current.length) {
        slices.push(current);
        current = [];
      }
      continue;
    }
    current.push(row);
  }
  if (current.length) {
    slices.push(current);
  }
  if (!slices.length || !slices[0].length || !slices[0][0].length) {
    return null;
  }
  const depth = slices.length;
  const height = slices[0].length;
  const width = slices[0][0].length;
  for (const slice of slices) {
    if (slice.length !== height || slice.some((row) => row.length !== width)) {
      return null;
    }
  }
  const size = clampSprite3dSize(Math.max(width, height, depth));
  if (size !== Math.max(width, height, depth)) {
    return null;
  }
  const palette = paletteEntries.map((entry) => ({ color: normalizeSpriteColor(entry.color) }));
  const keyToIndex = new Map(paletteEntries.map((entry, index) => [entry.key, index]));
  const cells = Array.from({ length: size * size * size }, () => null);
  for (let z = 0; z < depth; z += 1) {
    for (let y = 0; y < height; y += 1) {
      for (let x = 0; x < width; x += 1) {
        const char = slices[z][y][x];
        if (char === "." || char === " ") {
          continue;
        }
        if (!keyToIndex.has(char)) {
          return null;
        }
        const target = sprite3dCoordsFromPlaneForSize(size, "z", z, x, y);
        cells[((target.z * size + target.y) * size) + target.x] = keyToIndex.get(char);
      }
    }
  }
  return { size, palette, cells };
}

function findPuzzle3Block(source) {
  const pattern = /(^|\n)([\t ]*)puzzle3\s+(@?[A-Za-z_][\w:]*)[^{]*\{/m;
  const match = pattern.exec(source);
  if (!match) {
    return null;
  }
  const start = match.index + match[1].length;
  const openIndex = source.indexOf("{", start);
  const closeIndex = findMatchingBrace(source, openIndex);
  if (openIndex < 0 || closeIndex < 0) {
    return null;
  }
  return {
    name: match[3],
    indent: match[2] || "",
    start,
    end: closeIndex + 1,
  };
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
  setStatus(text, className);
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
  input?.addEventListener("focus", () => input.select());
  input?.addEventListener("pointerup", (event) => {
    if (document.activeElement === input) {
      event.preventDefault();
    }
  });
}
sprite3dNameInput?.addEventListener("input", () => {
  renderSprite3dPreview();
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
  event.preventDefault();
  const rawIndex = token.dataset.colorIndex;
  selectSprite3dColor(rawIndex === "erase" ? null : Number(rawIndex));
});
sprite3dSliceBoard?.addEventListener("pointerdown", startSprite3dPaint);
sprite3dSliceBoard?.addEventListener("pointermove", continueSprite3dPaint);
sprite3dSliceBoard?.addEventListener("pointerup", stopSprite3dPaint);
sprite3dSliceBoard?.addEventListener("pointercancel", stopSprite3dPaint);
sprite3dSliceBoard?.addEventListener("keydown", (event) => {
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
sprite3dScopeSliceButton?.addEventListener("click", () => setSprite3dEditScope("slice"));
sprite3dScopeAllButton?.addEventListener("click", () => setSprite3dEditScope("all"));
sprite3dFillButton?.addEventListener("click", toggleSprite3dBucketMode);
sprite3dExportButton?.addEventListener("click", exportSprite3dSource);
sprite3dInsertButton?.addEventListener("click", addSprite3dToSource);
sprite3dUpdateButton?.addEventListener("click", updateSprite3dInSource);
sprite3dResetCameraButton?.addEventListener("click", resetSprite3dCamera);
sprite3dPreviewCanvas?.addEventListener("pointerdown", startSprite3dPreviewDrag);
sprite3dPreviewCanvas?.addEventListener("pointermove", continueSprite3dPreviewDrag);
sprite3dPreviewCanvas?.addEventListener("pointerup", stopSprite3dPreviewDrag);
sprite3dPreviewCanvas?.addEventListener("pointercancel", stopSprite3dPreviewDrag);
sprite3dPreviewCanvas?.addEventListener("pointerleave", clearSprite3dHoverSlice);
window.addEventListener("resize", () => {
  if (!sprite3dBuilder?.hidden) {
    renderSprite3dPreview();
  }
});
registerSourceEditableTarget?.("sprite3d", {
  find: findSprite3dDefinitionAtPosition,
  load: loadSprite3dFromSourcePosition,
});

function syncSprite3dBuilderAfterScriptLoad() {
  if (currentPreviewMode === "sprite3d" && typeof loadFirstFocusedPuzzleEntry === "function") {
    loadFirstFocusedPuzzleEntry("sprite", "sprite3d");
  }
}

resetSprite3dBuilder();
syncSprite3dBuilderAfterScriptLoad();
