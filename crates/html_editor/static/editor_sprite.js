let spriteActionClearTimer = 0;
let spriteBucketActive = false;
let spriteGridVisible = true;
let spriteBrushPreset = "pixel";
let spriteLastPaintColorIndex = 0;
let spriteClipActive = false;
let spriteClipSelection = null;
let spriteClipDrag = null;
let spriteClipClipboard = null;
let spriteClipFloating = null;
let spriteAnimationPlaybackTimer = 0;
let spriteAnimationPlaybackDurationMs = 0;
let spriteAnimationInsertMode = false;
let spriteAnimationRemoveMode = false;
const spriteColorEditSessions = {
  sprite: null,
  sprite3d: null,
};
const SPRITE_EDITOR_MAX_SIZE = 64;
const SPRITE_ANIMATION_MAX_FRAMES = 24;
const SPRITE_ANIMATION_MIN_DURATION_MS = 20;
const SPRITE_ANIMATION_MAX_DURATION_MS = 5000;
const SPRITE_BRUSH_PRESETS = {
  pixel: { label: "1px", diameterCells: 1 },
  thin: { label: "Marker S", ratio: 1 / 32 },
  medium: { label: "Marker M", ratio: 1 / 20 },
  thick: { label: "Marker L", ratio: 1 / 12 },
};

function resetSpriteBuilder(size = sprite.size) {
  sprite.size = clampSpriteSize(size);
  sprite.cells = Array.from({ length: sprite.size * sprite.size }, () => null);
  resetSpriteAnimationFramesFromCurrentCells();
  sprite.shapeBind = null;
  sprite.solidSource = false;
  sprite.sourcePreludeRows = [];
  if (!Number.isInteger(sprite.selectedColorIndex) || !sprite.palette[sprite.selectedColorIndex]) {
    sprite.selectedColorIndex = 0;
  }
  renderSpriteBuilder();
}

function clampSpriteSize(value) {
  const parsed = Math.trunc(Number(value));
  const size = Number.isFinite(parsed) ? parsed : 5;
  return Math.max(1, Math.min(SPRITE_EDITOR_MAX_SIZE, size));
}

function renderSpriteBuilder() {
  if (!spriteBoard || !spritePalette) {
    return;
  }
  ensureSpriteAnimationFrames();
  renderSpriteControls();
  renderSpritePalette();
  renderSpriteBoard();
  renderSpriteAnimationControls();
  syncSpriteSourceActionButtons();
}

function setSpriteAnimationMode(enabled, options = {}) {
  sprite.animationMode = Boolean(enabled);
  ensureSpriteAnimationFrames();
  if (!sprite.animationMode) {
    stopSpriteAnimationPlayback({ render: false });
  }
  if (options.render !== false) {
    renderSpriteBuilder();
  }
  if (typeof syncPreviewModeButtonState === "function") {
    syncPreviewModeButtonState();
  }
}

function resetSpriteAnimationFramesFromCurrentCells() {
  sprite.animationFrameIndex = 0;
  sprite.animationFrameCount = 1;
  sprite.animationPlaybackIndex = 0;
  sprite.animationFrames = [cloneSpriteCells(sprite.cells)];
  spriteAnimationInsertMode = false;
  spriteAnimationRemoveMode = false;
}

function cloneSpriteCells(cells = sprite.cells) {
  const length = sprite.size * sprite.size;
  return Array.from({ length }, (_, index) => {
    const colorIndex = cells[index];
    return validSpriteColorIndex(colorIndex) ? colorIndex : null;
  });
}

function normalizedSpriteAnimationFrameCount(value = sprite.animationFrameCount) {
  const parsed = Math.trunc(Number(value));
  const count = Number.isFinite(parsed) ? parsed : 1;
  return Math.max(1, Math.min(SPRITE_ANIMATION_MAX_FRAMES, count));
}

function normalizedSpriteAnimationDuration(value = sprite.animationDurationMs) {
  const parsed = Math.trunc(Number(value));
  const duration = Number.isFinite(parsed) ? parsed : 120;
  return Math.max(SPRITE_ANIMATION_MIN_DURATION_MS, Math.min(SPRITE_ANIMATION_MAX_DURATION_MS, duration));
}

function ensureSpriteAnimationFrames() {
  sprite.animationFrameCount = normalizedSpriteAnimationFrameCount(sprite.animationFrameCount);
  sprite.animationDurationMs = normalizedSpriteAnimationDuration(sprite.animationDurationMs);
  if (!Array.isArray(sprite.animationFrames) || !sprite.animationFrames.length) {
    sprite.animationFrames = [cloneSpriteCells(sprite.cells)];
  }
  while (sprite.animationFrames.length < sprite.animationFrameCount) {
    sprite.animationFrames.push(cloneSpriteCells(sprite.cells));
  }
  if (sprite.animationFrames.length > sprite.animationFrameCount) {
    sprite.animationFrames.length = sprite.animationFrameCount;
  }
  for (let index = 0; index < sprite.animationFrames.length; index += 1) {
    sprite.animationFrames[index] = normalizeSpriteAnimationFrameCells(sprite.animationFrames[index]);
  }
  sprite.animationFrameIndex = Math.max(0, Math.min(sprite.animationFrameCount - 1, Math.trunc(Number(sprite.animationFrameIndex) || 0)));
  sprite.animationPlaybackIndex = Math.max(0, Math.min(sprite.animationFrameCount - 1, Math.trunc(Number(sprite.animationPlaybackIndex) || 0)));
  if (sprite.animationMode) {
    sprite.cells = sprite.animationFrames[sprite.animationFrameIndex];
  }
}

function normalizeSpriteAnimationFrameCells(cells) {
  const length = sprite.size * sprite.size;
  return Array.from({ length }, (_, index) => {
    const colorIndex = Array.isArray(cells) ? cells[index] : null;
    return validSpriteColorIndex(colorIndex) ? colorIndex : null;
  });
}

function resizeSpriteAnimationCells(cells, previousSize, nextSize) {
  const nextCells = Array.from({ length: nextSize * nextSize }, () => null);
  const copySize = Math.min(previousSize, nextSize);
  for (let y = 0; y < copySize; y += 1) {
    for (let x = 0; x < copySize; x += 1) {
      const colorIndex = cells[y * previousSize + x];
      nextCells[y * nextSize + x] = validSpriteColorIndex(colorIndex) ? colorIndex : null;
    }
  }
  return nextCells;
}

function syncSpriteAnimationFramesAfterSizeChange(previousSize, nextSize, activeCells) {
  if (!sprite.animationMode) {
    resetSpriteAnimationFramesFromCurrentCells();
    return;
  }
  ensureSpriteAnimationFrames();
  sprite.animationFrames = sprite.animationFrames.map((cells, index) => (
    index === sprite.animationFrameIndex
      ? cloneSpriteCells(activeCells)
      : resizeSpriteAnimationCells(cells, previousSize, nextSize)
  ));
  sprite.cells = sprite.animationFrames[sprite.animationFrameIndex];
}

function renderSpriteAnimationControls() {
  if (!spriteBuilder) {
    return;
  }
  ensureSpriteAnimationFrames();
  spriteBuilder.classList.toggle("is-animation-mode", sprite.animationMode);
  if (!sprite.animationMode) {
    spriteAnimationInsertMode = false;
    spriteAnimationRemoveMode = false;
    return;
  }
  syncSpriteAnimationInputValues({ preserveActive: true });
  if (spriteAnimationFrameTotal) {
    spriteAnimationFrameTotal.textContent = String(sprite.animationFrameCount);
  }
  if (spriteAnimationPreviousFrameButton) {
    spriteAnimationPreviousFrameButton.disabled = sprite.animationFrameCount <= 1;
  }
  if (spriteAnimationNextFrameButton) {
    spriteAnimationNextFrameButton.disabled = sprite.animationFrameCount <= 1;
  }
  if (spriteAnimationInsertFrameButton) {
    const canInsertFrame = sprite.animationFrameCount < SPRITE_ANIMATION_MAX_FRAMES;
    spriteAnimationInsertFrameButton.disabled = !canInsertFrame;
    spriteAnimationInsertFrameButton.classList.toggle("is-active", spriteAnimationInsertMode && canInsertFrame);
    spriteAnimationInsertFrameButton.setAttribute("aria-pressed", spriteAnimationInsertMode && canInsertFrame ? "true" : "false");
  }
  if (spriteAnimationRemoveFrameButton) {
    const canRemoveFrame = sprite.animationFrameCount > 1;
    spriteAnimationRemoveFrameButton.disabled = !canRemoveFrame;
    spriteAnimationRemoveFrameButton.classList.toggle("is-active", spriteAnimationRemoveMode && canRemoveFrame);
    spriteAnimationRemoveFrameButton.setAttribute("aria-pressed", spriteAnimationRemoveMode && canRemoveFrame ? "true" : "false");
  }
  renderSpriteAnimationSurfaces();
  syncSpriteAnimationPlayback();
}

function syncSpriteAnimationInputValues(options = {}) {
  const preserveActive = options.preserveActive === true;
  if (spriteAnimationDurationInput && (!preserveActive || document.activeElement !== spriteAnimationDurationInput)) {
    spriteAnimationDurationInput.value = String(sprite.animationDurationMs);
  }
  if (spriteAnimationFrameCountInput && (!preserveActive || document.activeElement !== spriteAnimationFrameCountInput)) {
    spriteAnimationFrameCountInput.value = String(sprite.animationFrameCount);
  }
  if (spriteAnimationFrameInput && (!preserveActive || document.activeElement !== spriteAnimationFrameInput)) {
    spriteAnimationFrameInput.value = String(sprite.animationFrameIndex + 1);
  }
  if (spriteAnimationFrameInput) {
    spriteAnimationFrameInput.max = String(sprite.animationFrameCount);
  }
}

function renderSpriteAnimationSurfaces() {
  if (!sprite.animationMode) {
    return;
  }
  renderSpriteAnimationPlaybackView(sprite.animationFrames[sprite.animationPlaybackIndex] || sprite.cells);
  renderSpriteAnimationFrameStrip();
}

function renderSpriteAnimationPlaybackView(cells) {
  if (!spriteAnimationPlaybackView) {
    return;
  }
  spriteAnimationPlaybackView.style.setProperty("--sprite-size", sprite.size);
  spriteAnimationPlaybackView.replaceChildren(...spriteAnimationFrameCells(cells));
}

function spriteAnimationFrameCells(cells) {
  return Array.from({ length: sprite.size * sprite.size }, (_, index) => {
    const colorIndex = validSpriteColorIndex(cells?.[index]) ? cells[index] : null;
    const cell = document.createElement("span");
    cell.className = "sprite-animation-frame-cell";
    cell.style.setProperty("--sprite-swatch-color", spriteColorForColorIndex(colorIndex));
    return cell;
  });
}

function renderSpriteAnimationFrameStrip() {
  if (!spriteAnimationFrameStrip) {
    return;
  }
  const showInsertTargets = spriteAnimationInsertMode && sprite.animationFrameCount < SPRITE_ANIMATION_MAX_FRAMES;
  const showRemoveTargets = spriteAnimationRemoveMode && sprite.animationFrameCount > 1;
  spriteAnimationFrameStrip.classList.toggle("is-insert-mode", showInsertTargets);
  spriteAnimationFrameStrip.classList.toggle("is-remove-mode", showRemoveTargets);
  const fragment = document.createDocumentFragment();
  for (let index = 0; index < sprite.animationFrameCount; index += 1) {
    if (showInsertTargets) {
      fragment.append(spriteAnimationInsertTargetButton(index));
    }
    const button = document.createElement("button");
    button.type = "button";
    button.className = "sprite-animation-frame-button";
    button.classList.toggle("is-active", index === sprite.animationFrameIndex);
    button.classList.toggle("is-playing-frame", sprite.animationMode && index === sprite.animationPlaybackIndex);
    button.style.setProperty("--sprite-size", sprite.size);
    button.setAttribute("aria-label", showRemoveTargets ? `Remove sprite animation frame ${index + 1}` : `Edit sprite animation frame ${index + 1}`);
    button.title = showRemoveTargets ? "Remove frame" : `Frame ${index + 1}`;
    button.append(...spriteAnimationFrameCells(sprite.animationFrames[index]));
    const label = document.createElement("span");
    label.className = "sprite-animation-frame-index";
    label.textContent = String(index + 1);
    button.append(label);
    button.addEventListener("click", () => {
      if (spriteAnimationRemoveMode) {
        removeSpriteAnimationFrameAt(index);
        return;
      }
      setSpriteAnimationFrame(index);
    });
    fragment.append(button);
  }
  if (showInsertTargets) {
    fragment.append(spriteAnimationInsertTargetButton(sprite.animationFrameCount));
  }
  spriteAnimationFrameStrip.replaceChildren(fragment);
}

function spriteAnimationInsertTargetButton(index) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "sprite-animation-insert-target";
  button.setAttribute("aria-label", `Insert sprite animation frame at position ${index + 1}`);
  button.title = "Add frame";
  button.addEventListener("click", () => insertSpriteAnimationFrameAt(index));
  return button;
}

function setSpriteAnimationFrame(index) {
  ensureSpriteAnimationFrames();
  spriteAnimationInsertMode = false;
  spriteAnimationRemoveMode = false;
  const nextIndex = Math.max(0, Math.min(sprite.animationFrameCount - 1, Math.trunc(Number(index) || 0)));
  if (nextIndex === sprite.animationFrameIndex) {
    renderSpriteAnimationControls();
    return;
  }
  sprite.animationFrameIndex = nextIndex;
  sprite.cells = sprite.animationFrames[nextIndex];
  deactivateSpriteClipMode({ render: false });
  renderSpriteBuilder();
  setSpriteActionStatus(`Frame ${nextIndex + 1}`, "is-ok");
}

function moveSpriteAnimationFrame(delta) {
  ensureSpriteAnimationFrames();
  const count = sprite.animationFrameCount;
  const next = (sprite.animationFrameIndex + delta + count) % count;
  setSpriteAnimationFrame(next);
}

function updateSpriteAnimationFrameCount(value) {
  const before = visualEditSnapshot("sprite");
  sprite.animationFrameCount = normalizedSpriteAnimationFrameCount(value);
  spriteAnimationInsertMode = false;
  spriteAnimationRemoveMode = false;
  ensureSpriteAnimationFrames();
  renderSpriteBuilder();
  pushVisualEditUndoSnapshot("sprite", before);
}

function toggleSpriteAnimationInsertMode() {
  ensureSpriteAnimationFrames();
  if (sprite.animationFrameCount >= SPRITE_ANIMATION_MAX_FRAMES) {
    spriteAnimationInsertMode = false;
    renderSpriteAnimationControls();
    setSpriteActionStatus(`Maximum ${SPRITE_ANIMATION_MAX_FRAMES} frames`, "is-error");
    return;
  }
  spriteAnimationRemoveMode = false;
  spriteAnimationInsertMode = !spriteAnimationInsertMode;
  renderSpriteAnimationControls();
  setSpriteActionStatus(spriteAnimationInsertMode ? "Click a frame gap" : "Add frame canceled", "is-ok");
}

function toggleSpriteAnimationRemoveMode() {
  ensureSpriteAnimationFrames();
  if (sprite.animationFrameCount <= 1) {
    spriteAnimationRemoveMode = false;
    renderSpriteAnimationControls();
    setSpriteActionStatus("At least 1 frame is required", "is-error");
    return;
  }
  spriteAnimationInsertMode = false;
  spriteAnimationRemoveMode = !spriteAnimationRemoveMode;
  renderSpriteAnimationControls();
  setSpriteActionStatus(spriteAnimationRemoveMode ? "Click a frame to remove" : "Remove frame canceled", "is-ok");
}

function insertSpriteAnimationFrameAt(index) {
  ensureSpriteAnimationFrames();
  if (sprite.animationFrameCount >= SPRITE_ANIMATION_MAX_FRAMES) {
    spriteAnimationInsertMode = false;
    renderSpriteAnimationControls();
    setSpriteActionStatus(`Maximum ${SPRITE_ANIMATION_MAX_FRAMES} frames`, "is-error");
    return;
  }
  const before = visualEditSnapshot("sprite");
  const insertIndex = Math.max(0, Math.min(sprite.animationFrameCount, Math.trunc(Number(index) || 0)));
  const copyIndex = Math.max(0, Math.min(sprite.animationFrameCount - 1, insertIndex - 1));
  const insertedCells = cloneSpriteCells(sprite.animationFrames[copyIndex]);
  stopSpriteAnimationPlayback({ render: false });
  sprite.animationFrames.splice(insertIndex, 0, insertedCells);
  sprite.animationFrameCount = sprite.animationFrames.length;
  sprite.animationFrameIndex = insertIndex;
  sprite.animationPlaybackIndex = insertIndex;
  sprite.cells = sprite.animationFrames[insertIndex];
  spriteAnimationInsertMode = false;
  spriteAnimationRemoveMode = false;
  deactivateSpriteClipMode({ render: false });
  renderSpriteBuilder();
  setSpriteActionStatus(`Added frame ${insertIndex + 1}`, "is-ok");
  pushVisualEditUndoSnapshot("sprite", before);
}

function removeSpriteAnimationFrameAt(index) {
  ensureSpriteAnimationFrames();
  if (sprite.animationFrameCount <= 1) {
    spriteAnimationRemoveMode = false;
    renderSpriteAnimationControls();
    setSpriteActionStatus("At least 1 frame is required", "is-error");
    return;
  }
  const before = visualEditSnapshot("sprite");
  const removeIndex = Math.max(0, Math.min(sprite.animationFrameCount - 1, Math.trunc(Number(index) || 0)));
  stopSpriteAnimationPlayback({ render: false });
  sprite.animationFrames.splice(removeIndex, 1);
  sprite.animationFrameCount = sprite.animationFrames.length;
  sprite.animationFrameIndex = Math.max(0, Math.min(removeIndex, sprite.animationFrameCount - 1));
  sprite.animationPlaybackIndex = sprite.animationFrameIndex;
  sprite.cells = sprite.animationFrames[sprite.animationFrameIndex];
  spriteAnimationInsertMode = false;
  spriteAnimationRemoveMode = false;
  deactivateSpriteClipMode({ render: false });
  renderSpriteBuilder();
  setSpriteActionStatus(`Removed frame ${removeIndex + 1}`, "is-ok");
  pushVisualEditUndoSnapshot("sprite", before);
}

function updateSpriteAnimationDuration(value, options = {}) {
  const nextDuration = normalizedSpriteAnimationDuration(value);
  const changed = nextDuration !== sprite.animationDurationMs;
  const before = options.recordHistory === false || !changed ? null : visualEditSnapshot("sprite");
  sprite.animationDurationMs = nextDuration;
  if (
    spriteAnimationDurationInput
    && !(options.preserveInput === true && document.activeElement === spriteAnimationDurationInput)
  ) {
    spriteAnimationDurationInput.value = String(sprite.animationDurationMs);
  }
  if (changed && sprite.animationMode && sprite.animationFrameCount > 1) {
    stopSpriteAnimationPlayback({ render: false });
    startSpriteAnimationPlayback();
  }
  if (before) {
    pushVisualEditUndoSnapshot("sprite", before);
  }
}

function isSpriteVisualEditUndoTarget(target) {
  return target === spriteAnimationDurationInput || target === spriteAnimationFrameCountInput;
}

function spriteAnimationFrameDelayMs() {
  ensureSpriteAnimationFrames();
  return Math.max(1, Math.round(sprite.animationDurationMs / sprite.animationFrameCount));
}

function syncSpriteAnimationPlayback() {
  if (!sprite.animationMode || sprite.animationFrameCount <= 1) {
    stopSpriteAnimationPlayback({ render: false });
    sprite.animationPlaybackIndex = sprite.animationFrameIndex;
    renderSpriteAnimationPlaybackView(sprite.cells);
    renderSpriteAnimationFrameStrip();
    return;
  }
  if (
    !sprite.animationPlaying
    || !spriteAnimationPlaybackTimer
    || spriteAnimationPlaybackDurationMs !== spriteAnimationFrameDelayMs()
  ) {
    startSpriteAnimationPlayback();
  }
}

function startSpriteAnimationPlayback() {
  ensureSpriteAnimationFrames();
  if (sprite.animationFrameCount <= 1) {
    stopSpriteAnimationPlayback({ render: false });
    return;
  }
  stopSpriteAnimationPlayback({ render: false });
  sprite.animationPlaying = true;
  spriteAnimationPlaybackDurationMs = spriteAnimationFrameDelayMs();
  sprite.animationPlaybackIndex = sprite.animationFrameIndex;
  renderSpriteAnimationPlaybackView(sprite.animationFrames[sprite.animationPlaybackIndex] || sprite.cells);
  const tick = () => {
    if (!sprite.animationPlaying) {
      return;
    }
    sprite.animationPlaybackIndex = (sprite.animationPlaybackIndex + 1) % sprite.animationFrameCount;
    renderSpriteAnimationPlaybackView(sprite.animationFrames[sprite.animationPlaybackIndex] || sprite.cells);
    spriteAnimationPlaybackDurationMs = spriteAnimationFrameDelayMs();
    spriteAnimationPlaybackTimer = window.setTimeout(tick, spriteAnimationPlaybackDurationMs);
  };
  spriteAnimationPlaybackTimer = window.setTimeout(tick, spriteAnimationPlaybackDurationMs);
}

function stopSpriteAnimationPlayback(options = {}) {
  window.clearTimeout(spriteAnimationPlaybackTimer);
  spriteAnimationPlaybackTimer = 0;
  spriteAnimationPlaybackDurationMs = 0;
  sprite.animationPlaying = false;
  if (options.render !== false) {
    renderSpriteAnimationSurfaces();
  }
}

function renderSpriteControls() {
  spriteSizeInput.value = String(sprite.size);
  syncSpritePaintToolControls();
  syncSpriteGridButton();
  renderSpriteShapeBindRow(spriteShapeField);
  renderSpriteScaleControl({
    size: sprite.size,
    maxSize: SPRITE_EDITOR_MAX_SIZE,
    scaleInput: spriteScaleInput,
    scaleUpButton: spriteScaleUpButton,
    scaleDownButton: spriteScaleDownButton,
    canScaleDown: canScaleDownSprite,
    noun: "sprite",
  });
}

function syncSpritePaintToolControls() {
  syncSpriteBucketButton();
  syncSpriteMarkerButton();
}

function syncSpriteBucketButton() {
  if (!spriteFillButton) {
    return;
  }
  spriteFillButton.classList.toggle("is-active", spriteBucketActive);
  spriteFillButton.setAttribute("aria-pressed", String(spriteBucketActive));
  spriteFillButton.setAttribute("aria-label", "Bucket fill");
  spriteFillButton.title = spriteBucketActive ? "Bucket active" : "Bucket fill";
}

function toggleSpriteBucketMode() {
  const wasClipActive = spriteClipActive || spriteClipSelection;
  deactivateSpriteClipMode({ render: false });
  spriteBucketActive = !spriteBucketActive;
  syncSpritePaintToolControls();
  renderSpritePalette();
  if (wasClipActive) {
    renderSpriteBoard();
  }
  setSpriteActionStatus(
    spriteBucketActive ? "Bucket: click a connected area" : spritePaintToolStatusText(),
    "is-ok",
  );
}

function deactivateSpriteBucketModeAfterUse() {
  if (!spriteBucketActive) {
    return;
  }
  spriteBucketActive = false;
  syncSpritePaintToolControls();
}

function syncSpriteMarkerButton() {
  spriteBrushPreset = normalizeSpriteBrushPreset(spriteBrushPreset);
  for (const button of spriteBrushPresetButtons()) {
    const preset = normalizeSpriteBrushPreset(button.dataset.spriteBrushPreset);
    const selected = !spriteBucketActive && !spriteClipActive && preset === spriteBrushPreset;
    const label = spriteBrushPresetLabel(preset);
    button.classList.toggle("is-active", selected);
    button.setAttribute("aria-pressed", String(selected));
    button.title = label;
    button.setAttribute("aria-label", `Brush: ${label}`);
  }
}

function selectSpriteBrushPreset(preset) {
  const wasBucketActive = spriteBucketActive;
  const wasClipActive = spriteClipActive || spriteClipSelection;
  spriteBrushPreset = normalizeSpriteBrushPreset(preset);
  spriteBucketActive = false;
  deactivateSpriteClipMode({ render: false });
  if (!validSpriteColorIndex(sprite.selectedColorIndex)) {
    sprite.selectedColorIndex = validSpriteColorIndex(spriteLastPaintColorIndex) ? spriteLastPaintColorIndex : 0;
  }
  syncSpritePaintToolControls();
  if (wasBucketActive || wasClipActive) {
    renderSpritePalette();
  }
  if (wasClipActive) {
    renderSpriteBoard();
  }
  setSpriteActionStatus(spritePaintToolStatusText(), "is-ok");
}

function normalizeSpriteBrushPreset(preset) {
  return Object.prototype.hasOwnProperty.call(SPRITE_BRUSH_PRESETS, preset) ? preset : "pixel";
}

function spriteBrushPresetLabel(preset = spriteBrushPreset) {
  return SPRITE_BRUSH_PRESETS[normalizeSpriteBrushPreset(preset)].label;
}

function spriteBrushPresetButtons() {
  return spriteMarkerTool.querySelectorAll("[data-sprite-brush-preset]");
}

function spriteBrushIsPixel(preset = spriteBrushPreset) {
  return normalizeSpriteBrushPreset(preset) === "pixel";
}

function spritePaintToolStatusText() {
  return `Brush: ${spriteBrushPresetLabel(spriteBrushPreset).toLowerCase()}`;
}

function beginSpriteColorEditHistory(kind) {
  if (!spriteColorEditSessions[kind]) {
    spriteColorEditSessions[kind] = visualEditSnapshot(kind);
  }
}

function commitSpriteColorEditHistory(kind) {
  const before = spriteColorEditSessions[kind];
  spriteColorEditSessions[kind] = null;
  if (!before) {
    return false;
  }
  return pushVisualEditUndoSnapshot(kind, before);
}

function discardSpriteColorEditHistory(kind) {
  spriteColorEditSessions[kind] = null;
}

function clearSpriteColorEditorState({ commitHistory = true } = {}) {
  if (commitHistory) {
    commitSpriteColorEditHistory("sprite");
  }
  sprite.addPaletteOpen = false;
  sprite.editPaletteOpen = false;
  sprite.customColorOpen = false;
  sprite.addDraftColorIndex = null;
}

function clearSpriteTagPickerState() {
  sprite.colorTagPickerOpen = false;
  sprite.shapeTagPickerOpen = false;
}

function renderSpriteColorAdjuster({ color, ariaLabel, onChange }) {
  const editor = window.PuzzleStudioColorEditor.create({
    color,
    ariaLabel,
    className: "sprite-color-adjuster",
    onInput: onChange,
  });
  return editor;
}

function renderSpritePalette() {
  const sourceActions = document.querySelector("#spriteBuilder .sprite-source-actions");
  spritePalette.replaceChildren();
  const selectedIsTransparent = sprite.selectedColorIndex === null;
  if (selectedIsTransparent || validSpriteColorIndex(sprite.selectedColorIndex)) {
    const currentWrap = document.createElement("span");
    currentWrap.className = "sprite-current-color-wrap";
    const selected = selectedIsTransparent ? { color: "#00000000" } : sprite.palette[sprite.selectedColorIndex];
    const selectedBind = selectedIsTransparent ? { available: false, linked: false, label: "" } : spritePaletteEntryBindInfo(selected);
    const selectedDisplayName = selectedIsTransparent ? "" : spritePaletteEntryDisplayName(selected);
    const currentButton = document.createElement("button");
    currentButton.type = "button";
    currentButton.className = "sprite-current-color-button";
    currentButton.classList.toggle("is-transparent", selectedIsTransparent);
    currentButton.classList.toggle("is-bound", selectedBind.available && selectedBind.linked);
    currentButton.classList.toggle("is-unlinked", selectedBind.available && !selectedBind.linked);
    currentButton.style.setProperty("--sprite-current-color", normalizeSpriteColor(selected.color));
    currentButton.title = selectedIsTransparent
      ? "Transparent eraser cannot be edited"
      : selectedDisplayName ? `Pick selected color ${selectedDisplayName}` : selectedBind.available ? `Pick selected color (${selectedBind.label})` : "Pick selected color";
    currentButton.setAttribute(
      "aria-label",
      selectedIsTransparent
        ? "Selected transparent eraser color #00000000, not editable"
        : selectedDisplayName ? `Pick selected color ${selectedDisplayName}` : `Pick selected color ${selected.color}`,
    );
    currentButton.setAttribute("aria-disabled", String(selectedIsTransparent));
    currentButton.setAttribute("aria-expanded", String(!selectedIsTransparent && sprite.editPaletteOpen));
    currentButton.innerHTML = `
      <span class="sprite-current-color-swatch" aria-hidden="true"></span>
    `;
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
    currentHexInput.className = "sprite-current-value-input sprite-current-hex-input";
    currentHexInput.value = selectedDisplayName || (selectedIsTransparent
      ? "#00000000"
      : normalizeSpriteColor(selected.color));
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
    const currentTagButton = selectedIsTransparent ? null : renderSpriteCurrentColorTagButton(selected);
    const currentTagUnlinkButton = !selectedIsTransparent && selectedBind.linked && selectedBind.name
      ? renderSpriteCurrentColorUnlinkButton(sprite.selectedColorIndex, selectedBind)
      : null;
    const colorNames = selectedIsTransparent ? [] : spriteColorAssetNames();
    const applyCurrentColorValue = (color) => {
      beginSpriteColorEditHistory("sprite");
      const normalized = normalizeSpriteColor(color);
      clearSpriteActionError();
      selected.color = normalized;
      updateSpriteBoundColorDefinition(selected, normalized);
      currentButton.style.setProperty("--sprite-current-color", normalized);
      currentButton.setAttribute("aria-label", selectedDisplayName ? `Pick selected color ${selectedDisplayName}` : `Pick selected color ${normalized}`);
      currentHexInput.value = selectedDisplayName || normalized;
      renderSpriteColorSurfaces();
    };
    let pendingEditMenu = null;
    const applyCurrentHex = (options = {}) => {
      if (currentHexInput.classList.contains("is-name-mode")) {
        const ok = applyCurrentColorName(sprite.selectedColorIndex, currentHexInput.value, { reportError: true });
        if (ok && options.commitHistory) {
          commitSpriteColorEditHistory("sprite");
        }
        return;
      }
      const parsed = parseSpriteHexColor(currentHexInput.value);
      if (!parsed) {
        if (options.reportError) {
          setSpriteActionStatus("Use #rrggbb or #rrggbbaa", "is-error");
        }
        return;
      }
      applyCurrentColorValue(parsed);
      if (options.commitHistory) {
        commitSpriteColorEditHistory("sprite");
      }
    };

    if (!selectedIsTransparent) {
      currentButton.addEventListener("click", () => {
        const opening = !sprite.editPaletteOpen;
        if (!opening) {
          commitSpriteColorEditHistory("sprite");
        }
        sprite.editPaletteOpen = opening;
        sprite.addPaletteOpen = false;
        sprite.addDraftColorIndex = null;
        sprite.customColorOpen = opening;
        if (opening) {
          clearSpriteTagPickerState();
          renderSpriteControls();
        }
        renderSpritePalette();
      });
      currentHexInput.addEventListener("input", () => {
        if (!currentHexInput.classList.contains("is-name-mode")) {
          applyCurrentHex();
        }
      });
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
    currentWrap.append(currentButton);
    currentWrap.append(currentHexInput);
    if (currentTagButton) {
      currentWrap.append(currentTagButton);
    }
    if (currentTagUnlinkButton) {
      currentWrap.append(currentTagUnlinkButton);
    }
    if (!selectedIsTransparent && sprite.colorTagPickerOpen) {
      const colorAssets = spriteSourceColorAssets();
      const tagPicker = renderSpriteAssetNamePicker({
        className: "sprite-color-tag-picker",
        names: colorNames,
        value: selectedBind.name || defaultSpriteAssetName("color", sprite.selectedColorIndex),
        placeholder: "color_name",
        ariaLabel: "Color tag name",
        emptyText: "No named colors yet",
        optionMeta: (name) => ({ color: colorAssets.get(name) }),
        onCommit: (name) => {
          const wasOpen = sprite.colorTagPickerOpen;
          sprite.colorTagPickerOpen = false;
          const ok = applyCurrentColorName(sprite.selectedColorIndex, name, { reportError: true });
          if (!ok) {
            sprite.colorTagPickerOpen = wasOpen;
            return false;
          }
          clearSpriteColorEditorState();
          renderSpriteBuilder();
          return true;
        },
        onCancel: () => {
          sprite.colorTagPickerOpen = false;
          renderSpritePalette();
        },
      });
      currentWrap.append(tagPicker);
      requestAnimationFrame(() => {
        focusSpriteTagPickerInput(tagPicker);
      });
    }
    if (!selectedIsTransparent && sprite.editPaletteOpen) {
      const editorPanel = document.createElement("span");
      editorPanel.className = "sprite-current-editor-panel";
      const editMenu = renderSpriteColorMenu({
        mode: "edit",
        customValue: selected.color,
        customOnly: true,
      });
      editorPanel.append(editMenu);
      currentWrap.append(editorPanel);
      pendingEditMenu = editMenu;
    }
    spritePalette.append(currentWrap);
    if (pendingEditMenu) {
      positionSpriteColorMenu(pendingEditMenu, currentButton, { side: "left" });
    }
  }

  const paletteGrid = document.createElement("span");
  paletteGrid.className = "sprite-palette-grid";

  const eraseButton = document.createElement("button");
  eraseButton.type = "button";
  eraseButton.className = "sprite-token sprite-token-erase sprite-icon-button";
  eraseButton.classList.toggle("is-selected", sprite.selectedColorIndex === null && !spriteBucketActive);
  eraseButton.dataset.colorIndex = "erase";
  eraseButton.style.setProperty("--sprite-swatch-color", "#00000000");
  eraseButton.title = "Paint transparent";
  eraseButton.setAttribute("aria-label", "Paint transparent sprite cell");
  eraseButton.innerHTML = `
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="m7 21-4.3-4.3c-1-1-1-2.5 0-3.4l9.6-9.6c1-1 2.5-1 3.4 0l5.6 5.6c1 1 1 2.5 0 3.4L13 21"></path>
      <path d="M22 21H7"></path>
      <path d="m5 11 9 9"></path>
    </svg>
  `;
  eraseButton.addEventListener("click", () => {
    spriteBucketActive = false;
    selectSpriteColor(null);
  });
  paletteGrid.append(eraseButton);

  for (const [index, entry] of sprite.palette.entries()) {
    const item = document.createElement("span");
    item.className = "sprite-token-item";
    item.classList.toggle("is-selected", index === sprite.selectedColorIndex);

    const button = document.createElement("button");
    button.type = "button";
    button.className = "sprite-token sprite-color-swatch";
    button.classList.toggle("is-selected", index === sprite.selectedColorIndex);
    button.dataset.colorIndex = String(index);
    button.style.setProperty("--sprite-swatch-color", normalizeSpriteColor(entry.color));
    button.style.setProperty("--sprite-token-ink", readableInkForColor(entry.color));
    const bind = spritePaletteEntryBindInfo(entry);
    button.classList.toggle("is-bound", bind.available && bind.linked);
    button.classList.toggle("is-unlinked", bind.available && !bind.linked);
    const displayName = spritePaletteEntryDisplayName(entry);
    button.title = displayName ? `Paint ${displayName} (${entry.color})` : `Paint ${entry.color}`;
    button.setAttribute("aria-label", displayName ? `Paint color ${index}: ${displayName}` : `Paint color ${index}`);
    button.addEventListener("click", () => selectSpriteColor(index));
    item.append(button);

    const bindMarker = renderSpriteBindMarker(entry);
    if (bindMarker) {
      item.append(bindMarker);
    }

    paletteGrid.append(item);
  }

  const addWrap = document.createElement("span");
  addWrap.className = "sprite-add-wrap";
  const addButton = document.createElement("button");
  addButton.type = "button";
  addButton.className = "sprite-token sprite-add-color-button";
  addButton.disabled = sprite.palette.length >= SPRITE_COLOR_TOKENS.length;
  addButton.title = "Add color";
  addButton.setAttribute("aria-label", "Add sprite color");
  addButton.setAttribute("aria-expanded", String(sprite.addPaletteOpen));
  addButton.innerHTML = `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14"></path><path d="M5 12h14"></path></svg>`;
  addButton.addEventListener("click", toggleSpriteAddPalette);
  addWrap.append(addButton);
  paletteGrid.append(addWrap);

  const removeButton = document.createElement("button");
  removeButton.type = "button";
  removeButton.className = "sprite-token sprite-remove-color-button";
  removeButton.disabled = !validSpriteColorIndex(sprite.selectedColorIndex) || sprite.palette.length <= 1;
  removeButton.title = "Remove selected color";
  removeButton.setAttribute("aria-label", "Remove selected color");
  removeButton.innerHTML = `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 12h14"></path></svg>`;
  removeButton.addEventListener("click", deleteSelectedSpriteColor);
  paletteGrid.append(removeButton);

  spritePalette.append(paletteGrid);

  if (sprite.addPaletteOpen) {
    const draft = validSpriteColorIndex(sprite.addDraftColorIndex)
      ? sprite.palette[sprite.addDraftColorIndex].color
      : nextSpritePresetColor();
    const addMenu = renderSpriteColorMenu({
      mode: "add",
      customValue: draft,
    });
    addMenu.classList.add("is-add-menu");
    spritePalette.append(addMenu);
    positionSpriteColorMenu(addMenu, paletteGrid, { side: "left" });
  }

  const paintToolRow = document.createElement("span");
  paintToolRow.className = "sprite-paint-tool-row";

  const brushActions = document.createElement("span");
  brushActions.className = "sprite-paint-tool-group sprite-brush-actions";
  brushActions.append(spriteMarkerTool);
  if (spriteFillButton) {
    brushActions.append(spriteFillButton);
  }
  brushActions.append(renderSpriteClipActions());
  paintToolRow.append(brushActions);

  const globalEditActions = document.createElement("span");
  globalEditActions.className = "sprite-paint-tool-group sprite-global-edit-actions";

  if (spriteGridButton) {
    globalEditActions.append(spriteGridButton);
  }

  const transformActions = document.createElement("span");
  transformActions.className = "sprite-paint-transform-actions";
  for (const button of [
    spriteRotateLeftButton,
    spriteRotateRightButton,
    spriteFlipHorizontalButton,
    spriteFlipVerticalButton,
    spriteClearButton,
  ]) {
    if (button) {
      transformActions.append(button);
    }
  }
  globalEditActions.append(transformActions);
  if (sourceActions) {
    globalEditActions.append(sourceActions);
  }
  paintToolRow.append(globalEditActions);
  spritePalette.append(paintToolRow);
}

function renderSpriteClipActions() {
  const clipActions = document.createElement("span");
  clipActions.className = "sprite-clip-actions";
  clipActions.classList.toggle("is-expanded", spriteClipActive);
  clipActions.append(renderSpriteClipButton({
    title: spriteClipActive ? "Close clip tools" : "Clip",
    ariaLabel: spriteClipActive ? "Close clip tools" : "Open clip tools",
    active: spriteClipActive,
    onClick: toggleSpriteClipMode,
    icon: spriteLucideIconSvg("mouse-pointer-2"),
  }));
  if (spriteClipActive) {
    const expandedActions = document.createElement("span");
    expandedActions.className = "sprite-clip-expanded-actions";
    expandedActions.append(
      renderSpriteClipButton({
        title: "Copy clip",
        ariaLabel: "Copy selected sprite area",
        disabled: !spriteClipSelection,
        onClick: copySpriteClipSelection,
        icon: spriteLucideIconSvg("copy"),
      }),
      renderSpriteClipButton({
        title: "Cut clip",
        ariaLabel: "Cut selected sprite area",
        disabled: !spriteClipSelection,
        onClick: cutSpriteClipSelection,
        icon: spriteLucideIconSvg("scissors"),
      }),
      renderSpriteClipButton({
        title: "Paste clip",
        ariaLabel: "Paste copied sprite area",
        disabled: !spriteClipClipboard,
        onClick: pasteSpriteClipClipboard,
        icon: spriteLucideIconSvg("clipboard-paste"),
      }),
      renderSpriteClipButton({
        title: spriteClipFloating ? "Discard clip preview" : "Clear clip",
        ariaLabel: spriteClipFloating ? "Discard clipped sprite preview" : "Clear selected sprite area",
        disabled: !spriteClipSelection && !spriteClipFloating,
        danger: true,
        onClick: clearSpriteClipSelection,
        icon: spriteLucideIconSvg("trash-2"),
      }),
    );
    clipActions.append(expandedActions);
  }
  return clipActions;
}

function spritePaletteEntryBindInfo(entry) {
  const bind = entry?.bind ?? entry?.bound ?? entry?.sourceRef ?? null;
  if (!bind) {
    return { available: true, linked: false, name: "", label: "Unlinked color" };
  }
  if (typeof bind === "string") {
    return { available: true, linked: true, name: bind, label: `Bound to ${bind}` };
  }
  if (typeof bind === "object") {
    const name = bind.name || bind.ref || bind.source || bind.color || "";
    const linked = !(bind.linked === false || bind.unlinked === true || bind.detached === true);
    return { available: true, linked, name, label: name ? `Bound to ${name}` : "Bound color" };
  }
  return { available: true, linked: true, name: "", label: "Bound color" };
}

function spritePaletteEntryDisplayName(entry) {
  const bind = spritePaletteEntryBindInfo(entry);
  return bind.linked && bind.name ? bind.name : "";
}

function renderSpriteCurrentColorTagButton(entry) {
  const bind = spritePaletteEntryBindInfo(entry);
  const button = document.createElement("button");
  button.type = "button";
  button.className = "sprite-current-tag-button sprite-icon-button";
  button.classList.toggle("is-active", bind.linked);
  button.title = bind.name ? `Color tag: ${bind.name}` : "Tag selected color";
  button.setAttribute("aria-label", button.title);
  button.setAttribute("aria-pressed", String(bind.linked));
  button.setAttribute("aria-haspopup", "listbox");
  button.setAttribute("aria-expanded", String(Boolean(sprite.colorTagPickerOpen)));
  button.innerHTML = spriteTagIconSvg();
  button.addEventListener("click", () => {
    const opening = !sprite.colorTagPickerOpen;
    if (opening) {
      clearSpriteColorEditorState();
      sprite.shapeTagPickerOpen = false;
      renderSpriteControls();
    }
    sprite.colorTagPickerOpen = opening;
    renderSpritePalette();
  });
  return button;
}

function renderSpriteCurrentColorUnlinkButton(index, bind) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "sprite-current-tag-unlink-button sprite-icon-button";
  button.title = bind?.name ? `Unlink color tag ${bind.name}` : "Unlink color tag";
  button.setAttribute("aria-label", button.title);
  button.innerHTML = spriteUnlinkIconSvg();
  button.addEventListener("click", (event) => {
    event.preventDefault();
    event.stopPropagation();
    sprite.colorTagPickerOpen = false;
    clearSpriteColorEditorState();
    toggleSpritePaletteEntryBinding(index);
  });
  return button;
}

function renderSpriteAssetNamePicker({ className, names, value, placeholder, ariaLabel, emptyText, optionMeta, onCommit, onCancel }) {
  const picker = document.createElement("form");
  picker.className = ["sprite-tag-picker", className || ""].filter(Boolean).join(" ");
  picker.noValidate = true;
  const input = document.createElement("input");
  input.type = "text";
  input.className = "sprite-tag-picker-input";
  input.value = value || "";
  input.placeholder = placeholder;
  input.spellcheck = false;
  input.autocomplete = "off";
  input.setAttribute("aria-label", ariaLabel);
  let submitted = false;
  const commit = (rawValue = input.value) => {
    if (submitted) {
      return;
    }
    submitted = Boolean(onCommit(rawValue) !== false);
  };
  picker.addEventListener("submit", (event) => {
    event.preventDefault();
    event.stopPropagation();
    commit();
  });
  input.addEventListener("change", () => commit());
  input.addEventListener("keydown", (event) => {
    event.stopPropagation();
    if (event.key === "Escape") {
      event.preventDefault();
      onCancel?.();
      return;
    }
    if (event.key !== "Enter") {
      return;
    }
    event.preventDefault();
    commit();
  });

  const options = document.createElement("span");
  options.className = "sprite-tag-options";
  options.setAttribute("role", "listbox");
  if (names.length) {
    for (const name of names) {
      const option = document.createElement("button");
      option.type = "button";
      option.className = "sprite-tag-option";
      option.setAttribute("role", "option");
      const meta = typeof optionMeta === "function" ? optionMeta(name) : null;
      if (meta && Object.prototype.hasOwnProperty.call(meta, "color")) {
        const color = parseSpriteHexColor(meta.color);
        if (color) {
          option.classList.add("has-color");
          option.style.setProperty("--sprite-tag-option-color", color);
          option.style.setProperty("--sprite-tag-option-ink", readableInkForColor(color));
          option.title = `${name} ${color}`;
          option.setAttribute("aria-label", `Use color tag ${name} ${color}`);
          const swatch = document.createElement("span");
          swatch.className = "sprite-tag-option-swatch";
          swatch.setAttribute("aria-hidden", "true");
          const label = document.createElement("span");
          label.className = "sprite-tag-option-name";
          label.textContent = name;
          const hexLabel = document.createElement("span");
          hexLabel.className = "sprite-tag-option-value";
          hexLabel.textContent = color;
          option.append(swatch, label, hexLabel);
        } else {
          option.classList.add("has-invalid-color");
          option.disabled = true;
          option.textContent = name;
          option.title = `Invalid color tag ${name}`;
          option.setAttribute("aria-label", `Invalid color tag ${name}`);
        }
      } else {
        option.textContent = name;
      }
      option.addEventListener("mousedown", (event) => event.preventDefault());
      option.addEventListener("click", () => commit(name));
      options.append(option);
    }
  } else {
    const empty = document.createElement("span");
    empty.className = "sprite-tag-empty";
    empty.textContent = emptyText;
    options.append(empty);
  }
  picker.append(input, options);
  return picker;
}

function focusSpriteTagPickerInput(tagPicker) {
  const input = tagPicker.querySelector(".sprite-tag-picker-input");
  if (!input) {
    return;
  }
  input.focus();
  input.select();
}

function spriteColorAssetNames() {
  return [...spriteSourceColorAssets().keys()].sort((a, b) => a.localeCompare(b));
}

function spriteShapeAssetNames() {
  return [...spriteSourceShapeAssets().keys()].sort((a, b) => a.localeCompare(b));
}

function activeSpriteSourceContract() {
  return sprite.sourceSpriteContract && typeof sprite.sourceSpriteContract === "object"
    ? sprite.sourceSpriteContract
    : null;
}

function spriteSourceColorAssets() {
  const assets = new Map();
  const contract = activeSpriteSourceContract();
  for (const entry of Array.isArray(contract?.colorAssets) ? contract.colorAssets : []) {
    const name = String(entry?.name || "").trim();
    const color = String(entry?.color || "").trim();
    if (name && color) {
      assets.set(name, color);
    }
  }
  for (const entry of Array.isArray(contract?.resolvedPalette) ? contract.resolvedPalette : []) {
    const name = String(entry?.source || "").trim();
    const color = String(entry?.color || "").trim();
    if (entry?.linked && name && color) {
      assets.set(name, color);
    }
  }
  return assets;
}

function spriteSourceShapeAssets() {
  const assets = new Map();
  const contract = activeSpriteSourceContract();
  for (const entry of Array.isArray(contract?.shapeAssets) ? contract.shapeAssets : []) {
    const name = String(entry?.name || "").trim();
    const rows = Array.isArray(entry?.rows)
      ? entry.rows.map((row) => String(row || "").trim()).filter(Boolean)
      : [];
    if (name && rows.length) {
      assets.set(name, rows);
    }
  }
  const shapeName = typeof contract?.shapeRef === "string" ? contract.shapeRef.trim() : "";
  const resolvedRows = Array.isArray(contract?.resolvedShapeRows)
    ? contract.resolvedShapeRows.map((row) => String(row || "").trim()).filter(Boolean)
    : [];
  if (shapeName && resolvedRows.length) {
    assets.set(shapeName, resolvedRows);
  }
  return assets;
}

function applyCurrentColorName(index, rawName, options = {}) {
  if (!validSpriteColorIndex(index)) {
    return false;
  }
  const entry = sprite.palette[index];
  const name = sanitizeSpriteColorAssetRef(rawName);
  if (!name) {
    if (options.reportError) {
      setSpriteActionStatus("Enter a color name", "is-error");
    }
    return false;
  }
  const colorAssets = spriteSourceColorAssets();
  let status = `Using color ${name}`;
  if (colorAssets.has(name)) {
    const resolved = colorAssets.get(name);
    if (!resolved) {
      if (options.reportError) {
        setSpriteActionStatus(`Cannot resolve color ${name}`, "is-error");
      }
      return false;
    }
    entry.color = resolved;
  } else {
    if (name.includes(":")) {
      if (options.reportError) {
        setSpriteActionStatus(`Cannot resolve color ${name}`, "is-error");
      }
      return false;
    }
    const staged = sprite.palette.find((candidate, candidateIndex) => {
      const bind = spritePaletteEntryBindInfo(candidate);
      return candidateIndex !== index && bind.linked && bind.name === name;
    });
    if (staged) {
      entry.color = normalizeSpriteColor(staged.color);
    }
    status = `Tagged color ${name}`;
  }
  entry.bind = { type: "color", name, linked: true };
  entry.editMode = "name";
  syncSpritePaletteEntriesForColorName(name, entry.color);
  setSpriteActionStatus(status, "is-ok");
  renderSpriteBuilder();
  return true;
}

function renderSpriteBindToggle(entry, index, options = {}) {
  const bind = spritePaletteEntryBindInfo(entry);
  if (!bind.available) {
    return null;
  }
  const button = document.createElement("button");
  button.type = "button";
  button.className = ["sprite-bind-toggle", options.className || ""].filter(Boolean).join(" ");
  button.classList.toggle("is-linked", bind.linked);
  button.classList.toggle("is-unlinked", !bind.linked);
  button.dataset.colorIndex = String(index);
  button.title = bind.linked ? `Unlink ${bind.label}` : (bind.name ? `Relink ${bind.label}` : "Link color");
  button.setAttribute("aria-label", bind.linked ? `Unlink color ${index}` : (bind.name ? `Relink color ${index}` : `Link color ${index}`));
  button.innerHTML = spriteLinkIconSvg();
  button.addEventListener("click", (event) => {
    event.preventDefault();
    event.stopPropagation();
    toggleSpritePaletteEntryBinding(index);
  });
  return button;
}

function renderSpriteBindMarker(entry) {
  const bind = spritePaletteEntryBindInfo(entry);
  if (!bind.available || !bind.linked) {
    return null;
  }
  const marker = document.createElement("span");
  marker.className = "sprite-bind-marker is-linked";
  marker.title = bind.label;
  marker.setAttribute("aria-label", bind.label);
  marker.innerHTML = spriteTagIconSvg();
  return marker;
}

function renderSpriteAssetBindToggle({ bind, className, label, linkedTitle, unlinkedTitle, onClick }) {
  const info = spriteAssetBindInfo(bind, label);
  const button = document.createElement("button");
  button.type = "button";
  button.className = ["sprite-bind-toggle", "sprite-asset-bind-toggle", className || ""].filter(Boolean).join(" ");
  button.classList.toggle("is-linked", info.linked);
  button.classList.toggle("is-unlinked", !info.linked);
  button.title = info.linked ? `${linkedTitle}: ${info.name}` : unlinkedTitle;
  button.setAttribute("aria-label", info.linked ? `${linkedTitle} ${label}` : unlinkedTitle);
  button.innerHTML = spriteLinkIconSvg();
  button.addEventListener("click", (event) => {
    event.preventDefault();
    event.stopPropagation();
    onClick();
  });
  return button;
}

function spriteAssetBindInfo(bind, label) {
  if (!bind) {
    return { linked: false, name: "", label: `Unlinked ${label}` };
  }
  if (typeof bind === "string") {
    return { linked: true, name: bind, label: `Bound to ${bind}` };
  }
  const name = bind.name || bind.ref || bind.source || "";
  const linked = !(bind.linked === false || bind.unlinked === true || bind.detached === true);
  return { linked, name, label: name ? `Bound to ${name}` : `Bound ${label}` };
}

function toggleSpritePaletteEntryBinding(index) {
  if (!validSpriteColorIndex(index)) {
    return;
  }
  const entry = sprite.palette[index];
  const rawBind = entry?.bind ?? entry?.bound ?? entry?.sourceRef ?? null;
  if (!rawBind) {
    linkSpritePaletteEntryToNewColor(index);
    return;
  }
  if (typeof rawBind === "string") {
    entry.bind = { type: "color", name: rawBind, linked: false };
  } else {
    rawBind.linked = rawBind.linked === false || rawBind.unlinked === true || rawBind.detached === true;
    delete rawBind.unlinked;
    delete rawBind.detached;
  }
  sprite.selectedColorIndex = index;
  const bind = spritePaletteEntryBindInfo(entry);
  rewriteCurrentSpriteDefinitionFromBuilder(bind.linked ? "Linked color" : "Unlinked color");
  renderSpritePalette();
  renderSpriteColorSurfaces();
}

function linkSpritePaletteEntryToNewColor(index) {
  const entry = sprite.palette[index];
  if (!entry) {
    return;
  }
  const name = promptSpriteAssetName("Color name", defaultSpriteAssetName("color", index));
  if (!name) {
    return;
  }
  const source = activeSpriteEditSource();
  const nextSource = ensureSpriteColorDefinition(source, name, normalizeSpriteColor(entry.color));
  if (!nextSource) {
    return;
  }
  entry.bind = { type: "color", name, linked: true };
  const rewritten = replaceSpriteDefinition(nextSource);
  if (!rewritten) {
    entry.bind = null;
    setSpriteActionStatus(`No sprite named ${spriteObjectName()}`, "is-error");
    return;
  }
  sprite.selectedColorIndex = index;
  applySpriteSourceChange(rewritten.source, `Linked color ${name}`);
  renderSpriteBuilder();
}

function rewriteCurrentSpriteDefinitionFromBuilder(status) {
  const result = replaceSpriteDefinition(activeSpriteEditSource());
  if (!result) {
    return false;
  }
  applySpriteSourceChange(result.source, status);
  return true;
}

function updateSpriteBoundColorDefinition(entry, color) {
  const bind = spritePaletteEntryBindInfo(entry);
  if (!bind.linked || !bind.name) {
    return false;
  }
  const source = activeSpriteEditSource();
  const nextSource = replaceSpriteColorDefinition(source, bind.name, color);
  if (!nextSource || nextSource === source) {
    return false;
  }
  const applied = applySpriteSourceChange(nextSource);
  if (applied) {
    syncSpritePaletteEntriesForColorName(bind.name, color);
  }
  return applied;
}

function toggleSpriteShapeBinding() {
  const info = spriteAssetBindInfo(sprite.shapeBind, "shape");
  if (!info.name) {
    linkSpriteShapeToNewShape();
    return;
  }
  sprite.shapeBind = { type: "shape", name: info.name, linked: !info.linked };
  rewriteCurrentSpriteDefinitionFromBuilder(sprite.shapeBind.linked ? "Linked shape" : "Unlinked shape");
  renderSpriteBuilder();
}

function linkSpriteShapeToNewShape() {
  const name = promptSpriteShapeAssetName("Shape name", defaultSpriteAssetName("shape"));
  if (!name) {
    return;
  }
  const source = activeSpriteEditSource();
  const nextSource = ensureSpriteShapeDefinition(source, name, spriteAscii().split("\n"));
  if (!nextSource) {
    return;
  }
  sprite.shapeBind = { type: "shape", name, linked: true };
  const rewritten = replaceSpriteDefinition(nextSource);
  if (!rewritten) {
    sprite.shapeBind = null;
    setSpriteActionStatus(`No sprite named ${spriteObjectName()}`, "is-error");
    return;
  }
  applySpriteSourceChange(rewritten.source, `Linked shape ${name}`);
  renderSpriteBuilder();
}

function updateSpriteBoundShapeDefinition() {
  const info = spriteAssetBindInfo(sprite.shapeBind, "shape");
  if (!info.linked || !info.name) {
    return false;
  }
  const source = activeSpriteEditSource();
  const nextSource = replaceSpriteShapeDefinition(source, info.name, spriteAscii().split("\n"));
  if (!nextSource || nextSource === source) {
    return false;
  }
  applySpriteSourceChange(nextSource);
  return true;
}

function promptSpriteAssetName(label, defaultValue) {
  let raw = defaultValue;
  try {
    raw = window.prompt(label, defaultValue);
  } catch {
    raw = defaultValue;
  }
  if (raw === null) {
    return null;
  }
  const name = sanitizeSpriteAssetName(raw);
  if (!name) {
    setSpriteActionStatus("Use an asset name like wall_color", "is-error");
    return null;
  }
  return name;
}

function promptSpriteShapeAssetName(label, defaultValue) {
  let raw = defaultValue;
  try {
    raw = window.prompt(label, defaultValue);
  } catch {
    raw = defaultValue;
  }
  if (raw === null) {
    return null;
  }
  const name = sanitizeSpriteShapeRef(raw);
  if (!name) {
    setSpriteActionStatus("Use a shape name like wall-shape or shape:tag", "is-error");
    return null;
  }
  return name;
}

function sanitizeSpriteAssetName(value) {
  const cleaned = String(value || "")
    .trim()
    .replace(/[^\w]+/g, "_")
    .replace(/^_+|_+$/g, "");
  if (!cleaned) {
    return "";
  }
  return /^[A-Za-z_]/.test(cleaned) ? cleaned : `color_${cleaned}`;
}

function sanitizeSpriteColorAssetRef(value) {
  const raw = String(value || "").trim();
  if (!raw.includes(":")) {
    return sanitizeSpriteAssetName(raw);
  }
  const parts = raw.split(":");
  if (parts.length !== 2) {
    return "";
  }
  const tableName = sanitizeSpriteAssetName(parts[0]);
  const rowName = sanitizeSpriteAssetName(parts[1]);
  return tableName && rowName ? `${tableName}:${rowName}` : "";
}

function sanitizeSpriteShapeRef(value) {
  const raw = String(value || "").trim();
  if (!raw || /[\s{}#]/.test(raw)) {
    return "";
  }
  if (!raw.includes(":")) {
    return isSpritePlainShapeName(raw) ? raw : "";
  }
  const parts = raw.split(":");
  if (parts.length !== 2) {
    return "";
  }
  return isSpriteShapeTableRef(parts[0], parts[1]) ? raw : "";
}

function isSpritePlainShapeName(value) {
  return /^[A-Za-z_][A-Za-z0-9_+*()/-]*$/.test(String(value || ""));
}

function isSpriteShapeTableRef(tableName, valueName) {
  return /^[A-Za-z_]\w*$/.test(String(tableName || ""))
    && /^[A-Za-z0-9_+*()/-]+$/.test(String(valueName || ""));
}

function defaultSpriteAssetName(kind, index = 0) {
  if (kind === "color") {
    const objectName = String(spriteObjectName()).split(":")[0];
    const base = sanitizeSpriteAssetName(objectName) || "sprite";
    return `${base}_${Number(index) + 1}`;
  }
  const base = sanitizeSpriteAssetName(spriteObjectName()).replace(new RegExp(`_${kind}$`), "") || "sprite";
  return `${base}_${kind}_${Number(index) + 1}`;
}

function syncSpritePaletteEntriesForColorName(name, color) {
  const normalized = normalizeSpriteColor(color);
  for (const entry of sprite.palette) {
    const bind = spritePaletteEntryBindInfo(entry);
    if (bind.linked && bind.name === name) {
      entry.color = normalized;
    }
  }
}

function applySpriteSourceChange(source, statusText = "") {
  const document = activeSpriteEditDocument();
  if (!document || !isTextDocument(document)) {
    setSpriteActionStatus("No puzzle source", "is-error");
    setStatus("No puzzle source for sprite", "is-error");
    return false;
  }
  document.source = source;
  if (document.id === activeDocument()?.id) {
    setSourceEditorValue(source, { resetUndo: false });
  }
  scheduleLocalSave();
  schedulePreview();
  if (statusText) {
    setSpriteActionStatus(statusText, "is-ok");
    setStatus(statusText, "is-ok");
  }
  return true;
}

function activeSpriteEditDocument() {
  const editDocument = sprite.editDocumentId
    ? documents.find((candidate) => candidate.id === sprite.editDocumentId)
    : null;
  if (editDocument && isTextDocument(editDocument) && isPuzzleDocument(editDocument)) {
    return editDocument;
  }
  const document = activeDocument();
  if (document && isTextDocument(document) && isPuzzleDocument(document)) {
    return document;
  }
  return activePreviewDocument();
}

function setSpriteEditDocument(document = activeDocument()) {
  sprite.editDocumentId = document && isTextDocument(document) && isPuzzleDocument(document)
    ? document.id
    : null;
}

function setSpriteEditSource(entry, document = activeDocument()) {
  setSpriteEditDocument(document);
  sprite.editSourceStart = Number.isInteger(entry?.start)
    ? entry.start
    : Number.isInteger(entry?.openIndex)
      ? entry.openIndex
      : null;
  sprite.editSourceEnd = Number.isInteger(entry?.end) ? entry.end : null;
  sprite.editSourceBodyStart = Number.isInteger(entry?.bodyStart) ? entry.bodyStart : null;
  sprite.editSourceBodyEnd = Number.isInteger(entry?.bodyEnd) ? entry.bodyEnd : null;
  sprite.editSourceName = entry?.name || "";
  sprite.sourceSpriteContract = entry?.sourceSprite && typeof entry.sourceSprite === "object"
    ? cloneVisualEditValue(entry.sourceSprite)
    : null;
}

function clearSpriteEditSource() {
  sprite.editSourceStart = null;
  sprite.editSourceEnd = null;
  sprite.editSourceBodyStart = null;
  sprite.editSourceBodyEnd = null;
  sprite.editSourceName = "";
  sprite.sourceSpriteContract = null;
}

function invalidateSpriteEditSourceForDocument(document = activeDocument()) {
  if (!document || !sprite.editDocumentId || document.id !== sprite.editDocumentId) {
    return false;
  }
  clearSpriteEditSource();
  return true;
}

function activeSpriteEditSource() {
  const document = activeSpriteEditDocument();
  if (!document || !isTextDocument(document)) {
    return "";
  }
  return document.id === activeDocument()?.id
    ? sourceEditorDocumentValue()
    : document.source || "";
}

function spriteLinkIconSvg() {
  return `
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M10 13a5 5 0 0 0 7.1.1l2-2a5 5 0 0 0-7.1-7.1l-1.1 1.1"></path>
      <path d="M14 11a5 5 0 0 0-7.1-.1l-2 2a5 5 0 0 0 7.1 7.1l1.1-1.1"></path>
    </svg>
  `;
}

function spriteUnlinkIconSvg() {
  return `
    <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
      <path d="M12.586 2.586A2 2 0 0 0 11.172 2H4a2 2 0 0 0-2 2v7.172a2 2 0 0 0 .586 1.414l8.704 8.704a2.426 2.426 0 0 0 3.42 0l6.58-6.58a2.426 2.426 0 0 0 0-3.42z"></path>
      <circle cx="7.5" cy="7.5" r=".5" fill="currentColor"></circle>
      <path d="M3 21 21 3"></path>
    </svg>
  `;
}

function spriteTagIconSvg() {
  return `
    <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-tag-icon lucide-tag" aria-hidden="true">
      <path d="M12.586 2.586A2 2 0 0 0 11.172 2H4a2 2 0 0 0-2 2v7.172a2 2 0 0 0 .586 1.414l8.704 8.704a2.426 2.426 0 0 0 3.42 0l6.58-6.58a2.426 2.426 0 0 0 0-3.42z"></path>
      <circle cx="7.5" cy="7.5" r=".5" fill="currentColor"></circle>
    </svg>
  `;
}

function positionSpriteColorMenu(menu, anchor, options = {}) {
  const menuRect = menu.getBoundingClientRect();
  const anchorRect = anchor.getBoundingClientRect();
  const gap = 6;
  const margin = 8;
  const viewportWidth = document.documentElement.clientWidth || window.innerWidth;
  const viewportHeight = document.documentElement.clientHeight || window.innerHeight;
  const menuWidth = menuRect.width > 0 && menuRect.width < 420 ? menuRect.width : 274;
  const menuHeight = menuRect.height > 0 && menuRect.height < viewportHeight ? menuRect.height : 224;
  const preferLeft = options.side === "left";
  let left = preferLeft
    ? anchorRect.left - menuWidth - gap
    : anchorRect.right + gap;
  if (left < margin) {
    left = anchorRect.right + gap;
  }
  if (left + menuWidth > viewportWidth - margin) {
    left = anchorRect.left - menuWidth - gap;
  }
  left = Math.max(margin, Math.min(left, viewportWidth - menuWidth - margin));
  const top = Math.max(
    margin,
    Math.min(anchorRect.top, viewportHeight - menuHeight - margin),
  );
  menu.style.position = "fixed";
  menu.style.left = `${left}px`;
  menu.style.right = "auto";
  menu.style.top = `${top}px`;
  menu.style.zIndex = "50";
}

function renderSpriteColorMenu({
  mode,
  customValue,
  customOnly = false,
  inline = false,
  onPreset = null,
  onChange = null,
  onDiscard = cancelSpriteColorAdd,
  renderPalette = renderSpritePalette,
}) {
  const presetList = document.createElement("span");
  presetList.className = [
    "sprite-color-menu",
    "is-adjuster",
    customOnly ? "is-custom-only" : "",
    inline ? "is-inline-custom" : "",
  ].filter(Boolean).join(" ");

  if (!customOnly) {
    const presetGrid = document.createElement("span");
    presetGrid.className = "sprite-preset-grid";
    for (const color of SPRITE_COLOR_PRESETS) {
      const preset = document.createElement("button");
      preset.type = "button";
      preset.className = "sprite-color-preset sprite-color-swatch";
      preset.classList.toggle("is-selected", normalizeSpriteColor(color) === normalizeSpriteColor(customValue));
      preset.style.setProperty("--sprite-swatch-color", normalizeSpriteColor(color));
      preset.title = mode === "add" ? `Start from ${color}` : `Use ${color}`;
      preset.setAttribute("aria-label", mode === "add" ? `Start from color ${color}` : `Use color ${color}`);
      preset.addEventListener("click", () => {
        if (onPreset) {
          onPreset(color, { deferHistory: true });
        } else if (mode === "add") {
          previewNewSpriteColor(color, { deferHistory: true });
        } else {
          updateSelectedSpriteColor(color, { deferHistory: true });
        }
        renderPalette();
      });
      presetGrid.append(preset);
    }
    presetList.append(presetGrid);
  }
  presetList.append(renderSpriteColorAdjuster({
    color: customValue,
    ariaLabel: mode === "add" ? "New color" : "Selected color",
    onChange: (color) => {
      if (onChange) {
        onChange(color, { deferHistory: true });
      } else if (mode === "add") {
        previewNewSpriteColor(color, { deferHistory: true });
      } else {
        updateSelectedSpriteColor(color, { deferHistory: true });
      }
    },
  }));
  const actionRow = document.createElement("span");
  actionRow.className = "sprite-color-actions";
  if (mode === "add") {
    actionRow.classList.add("is-floating");
    const discardButton = document.createElement("button");
    discardButton.type = "button";
    discardButton.className = "sprite-color-action-button sprite-color-trash-button";
    discardButton.title = "Discard new color";
    discardButton.setAttribute("aria-label", "Discard new color");
    discardButton.innerHTML = spriteTrashIconSvg();
    discardButton.addEventListener("click", onDiscard);
    actionRow.append(discardButton);
  } else {
    actionRow.hidden = true;
  }
  presetList.append(actionRow);
  return presetList;
}

function spriteTrashIconSvg() {
  return `
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M3 6h18"></path>
      <path d="M8 6V4h8v2"></path>
      <path d="M6 6l1 15h10l1-15"></path>
      <path d="M10 11v6"></path>
      <path d="M14 11v6"></path>
    </svg>
  `;
}

function spriteLucideIconSvg(name) {
  const icons = {
    "mouse-pointer-2": `
      <path d="M4.037 4.688a.495.495 0 0 1 .651-.651l16 6.5a.5.5 0 0 1-.063.947l-6.124 1.58a2 2 0 0 0-1.438 1.435l-1.579 6.126a.5.5 0 0 1-.947.063z"></path>
    `,
    copy: `
      <rect width="14" height="14" x="8" y="8" rx="2" ry="2"></rect>
      <path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"></path>
    `,
    scissors: `
      <circle cx="6" cy="6" r="3"></circle>
      <path d="M8.12 8.12 12 12"></path>
      <path d="M20 4 8.12 15.88"></path>
      <circle cx="6" cy="18" r="3"></circle>
      <path d="M14.47 14.48 20 20"></path>
    `,
    "clipboard-paste": `
      <path d="M15 2H9a1 1 0 0 0-1 1v2c0 .6.4 1 1 1h6c.6 0 1-.4 1-1V3c0-.6-.4-1-1-1Z"></path>
      <path d="M8 4H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-2"></path>
      <path d="M16 4h2a2 2 0 0 1 2 2v4"></path>
      <path d="M21 14H11"></path>
      <path d="m15 10-4 4 4 4"></path>
    `,
    "trash-2": `
      <path d="M3 6h18"></path>
      <path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"></path>
      <path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"></path>
      <line x1="10" x2="10" y1="11" y2="17"></line>
      <line x1="14" x2="14" y1="11" y2="17"></line>
    `,
  };
  const paths = icons[name];
  if (!paths) {
    throw new Error(`Unknown sprite lucide icon ${name}`);
  }
  return `
    <svg xmlns="http://www.w3.org/2000/svg" class="lucide lucide-${name}-icon lucide-${name}" viewBox="0 0 24 24" aria-hidden="true">
      ${paths}
    </svg>
  `;
}

function renderSpriteClipButton({ title, ariaLabel, icon, active = false, disabled = false, danger = false, onClick }) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "sprite-icon-button sprite-clip-button";
  button.classList.toggle("is-active", active);
  button.classList.toggle("is-danger", danger);
  button.disabled = Boolean(disabled);
  button.title = title;
  button.setAttribute("aria-label", ariaLabel);
  button.setAttribute("aria-pressed", String(active));
  button.innerHTML = icon;
  button.addEventListener("click", onClick);
  return button;
}

function toggleSpriteClipMode() {
  if (spriteClipActive) {
    deactivateSpriteClipMode();
    setSpriteActionStatus(spritePaintToolStatusText(), "is-ok");
    return;
  }
  spriteBucketActive = false;
  spriteClipActive = true;
  spriteClipSelection = normalizeSpriteClipRect(spriteClipSelection);
  spriteClipDrag = null;
  renderSpriteBuilder();
  setSpriteActionStatus(
    spriteClipSelection ? "Clip: drag selection to move it" : "Clip: drag to select sprite area",
    "is-ok",
  );
}

function deactivateSpriteClipMode(options = {}) {
  const wasActive = spriteClipActive || spriteClipSelection || spriteClipDrag || spriteClipFloating;
  const clearSelection = options.clearSelection !== false;
  spriteClipActive = false;
  if (clearSelection) {
    spriteClipSelection = null;
  } else {
    spriteClipSelection = normalizeSpriteClipRect(spriteClipSelection);
  }
  spriteClipDrag = null;
  spriteClipFloating = null;
  if (options.render === false || !wasActive) {
    return;
  }
  renderSpriteBuilder();
}

function normalizeSpriteClipRect(rect) {
  if (!rect) {
    return null;
  }
  const x = Math.trunc(Number(rect.x));
  const y = Math.trunc(Number(rect.y));
  const width = Math.trunc(Number(rect.width));
  const height = Math.trunc(Number(rect.height));
  if (width <= 0 || height <= 0) {
    return null;
  }
  if (x < 0 || y < 0 || x + width > sprite.size || y + height > sprite.size) {
    return null;
  }
  return { x, y, width, height };
}

function spriteClipRectFromCells(start, end) {
  return normalizeSpriteClipRect({
    x: Math.min(start.x, end.x),
    y: Math.min(start.y, end.y),
    width: Math.abs(end.x - start.x) + 1,
    height: Math.abs(end.y - start.y) + 1,
  });
}

function spriteClipSelectionContainsCell(cell, rect = spriteClipSelection) {
  const normalized = normalizeSpriteClipRect(rect);
  return Boolean(
    normalized
    && cell
    && cell.x >= normalized.x
    && cell.x < normalized.x + normalized.width
    && cell.y >= normalized.y
    && cell.y < normalized.y + normalized.height
  );
}

function spriteClipRectContainsIndex(rect, index) {
  const normalized = normalizeSpriteClipRect(rect);
  if (!normalized || !Number.isInteger(index) || index < 0) {
    return false;
  }
  const x = index % sprite.size;
  const y = Math.floor(index / sprite.size);
  return x >= normalized.x
    && x < normalized.x + normalized.width
    && y >= normalized.y
    && y < normalized.y + normalized.height;
}

function spriteClipCellFromClient(clientX, clientY, geometry = spriteBoardGeometry()) {
  if (geometry.width <= 0 || geometry.height <= 0) {
    return null;
  }
  return {
    x: Math.max(0, Math.min(sprite.size - 1, Math.floor(((clientX - geometry.left) / geometry.width) * geometry.size))),
    y: Math.max(0, Math.min(sprite.size - 1, Math.floor(((clientY - geometry.top) / geometry.height) * geometry.size))),
  };
}

function spriteClipRectCells(rect) {
  const normalized = normalizeSpriteClipRect(rect);
  if (!normalized) {
    return [];
  }
  const cells = [];
  for (let y = 0; y < normalized.height; y += 1) {
    for (let x = 0; x < normalized.width; x += 1) {
      const index = (normalized.y + y) * sprite.size + normalized.x + x;
      const colorIndex = sprite.cells[index];
      cells.push(validSpriteColorIndex(colorIndex) ? colorIndex : null);
    }
  }
  return cells;
}

function setSpriteClipRectCells(rect, cells) {
  const normalized = normalizeSpriteClipRect(rect);
  if (!normalized || !Array.isArray(cells) || cells.length !== normalized.width * normalized.height) {
    return [];
  }
  const changedIndices = [];
  for (let y = 0; y < normalized.height; y += 1) {
    for (let x = 0; x < normalized.width; x += 1) {
      const index = (normalized.y + y) * sprite.size + normalized.x + x;
      const next = cells[y * normalized.width + x];
      if (setSpriteCellColorAtIndex(index, next)) {
        changedIndices.push(index);
      }
    }
  }
  return changedIndices;
}

function clearSpriteClipRect(rect) {
  const normalized = normalizeSpriteClipRect(rect);
  if (!normalized) {
    return [];
  }
  const changedIndices = [];
  for (let y = normalized.y; y < normalized.y + normalized.height; y += 1) {
    for (let x = normalized.x; x < normalized.x + normalized.width; x += 1) {
      const index = y * sprite.size + x;
      if (setSpriteCellColorAtIndex(index, null)) {
        changedIndices.push(index);
      }
    }
  }
  return changedIndices;
}

function commitSpriteClipMutation(before, changedIndices, message) {
  if (!changedIndices.length) {
    setSpriteActionStatus("Clip did not change sprite", "is-ok");
    renderSpriteBuilder();
    return false;
  }
  sprite.solidSource = false;
  updateSpriteBoundShapeDefinition();
  renderSpriteBuilder();
  syncSpriteSourceActionButtons();
  setSpriteActionStatus(message, "is-ok");
  setStatus(message, "is-ok");
  pushVisualEditUndoSnapshot("sprite", before);
  return true;
}

function copySpriteClipSelection() {
  const rect = normalizeSpriteClipRect(spriteClipSelection);
  if (!rect) {
    setSpriteActionStatus("No clip selection", "is-error");
    return false;
  }
  spriteClipClipboard = {
    width: rect.width,
    height: rect.height,
    cells: spriteClipRectCells(rect),
  };
  spriteClipFloating = { kind: "copy" };
  spriteClipActive = true;
  spriteClipSelection = rect;
  renderSpriteBuilder();
  setSpriteActionStatus(`Copied ${rect.width}x${rect.height} clip: drag target, Command+V to paste`, "is-ok");
  return true;
}

function cutSpriteClipSelection() {
  const rect = normalizeSpriteClipRect(spriteClipSelection);
  if (!rect) {
    setSpriteActionStatus("No clip selection", "is-error");
    return false;
  }
  const before = visualEditSnapshot("sprite");
  spriteClipClipboard = {
    width: rect.width,
    height: rect.height,
    cells: spriteClipRectCells(rect),
  };
  spriteClipFloating = { kind: "cut" };
  spriteClipActive = true;
  spriteClipSelection = rect;
  const changedIndices = clearSpriteClipRect(rect);
  commitSpriteClipMutation(before, changedIndices, `Cut ${rect.width}x${rect.height} clip`);
  setSpriteActionStatus(`Cut ${rect.width}x${rect.height} clip: drag target, Command+V to paste`, "is-ok");
  return true;
}

function clearSpriteClipSelection() {
  if (spriteClipFloating) {
    spriteClipFloating = null;
    spriteClipSelection = null;
    spriteClipDrag = null;
    renderSpriteBuilder();
    setSpriteActionStatus("Clip preview discarded", "is-ok");
    return true;
  }
  const rect = normalizeSpriteClipRect(spriteClipSelection);
  if (!rect) {
    setSpriteActionStatus("No clip selection", "is-error");
    return false;
  }
  const before = visualEditSnapshot("sprite");
  const changedIndices = clearSpriteClipRect(rect);
  return commitSpriteClipMutation(before, changedIndices, "Cleared clip");
}

function pasteSpriteClipClipboard() {
  if (!spriteClipClipboard) {
    setSpriteActionStatus("No copied clip", "is-error");
    return false;
  }
  if (spriteClipClipboard.width > sprite.size || spriteClipClipboard.height > sprite.size) {
    setSpriteActionStatus("Copied clip is larger than sprite", "is-error");
    return false;
  }
  const base = normalizeSpriteClipRect(spriteClipSelection) || { x: 0, y: 0, width: 1, height: 1 };
  const rect = normalizeSpriteClipRect({
    x: base.x,
    y: base.y,
    width: spriteClipClipboard.width,
    height: spriteClipClipboard.height,
  });
  if (!rect) {
    setSpriteActionStatus("Copied clip does not fit at selection", "is-error");
    return false;
  }
  const before = visualEditSnapshot("sprite");
  const changedIndices = setSpriteClipRectCells(rect, spriteClipClipboard.cells);
  spriteClipActive = true;
  spriteClipSelection = rect;
  spriteClipFloating = null;
  commitSpriteClipMutation(before, changedIndices, `Pasted ${rect.width}x${rect.height} clip`);
  setSpriteActionStatus(`Pasted ${rect.width}x${rect.height} clip`, "is-ok");
  return true;
}

function moveSpriteClipRange(target, message = "Moved clip range") {
  spriteClipSelection = target;
  renderSpriteBuilder();
  setSpriteActionStatus(message, "is-ok");
}

function spriteClipFloatingRectAtCell(cell) {
  if (!cell || !spriteClipClipboard) {
    return null;
  }
  const width = Math.min(sprite.size, spriteClipClipboard.width);
  const height = Math.min(sprite.size, spriteClipClipboard.height);
  return normalizeSpriteClipRect({
    x: Math.max(0, Math.min(sprite.size - width, cell.x)),
    y: Math.max(0, Math.min(sprite.size - height, cell.y)),
    width,
    height,
  });
}

function spriteClipFloatingCellsForSelection(rect) {
  const normalized = normalizeSpriteClipRect(rect);
  if (!normalized || !spriteClipFloating || !spriteClipClipboard) {
    return null;
  }
  if (spriteClipClipboard.width !== normalized.width || spriteClipClipboard.height !== normalized.height) {
    return null;
  }
  if (!Array.isArray(spriteClipClipboard.cells) || spriteClipClipboard.cells.length !== normalized.width * normalized.height) {
    return null;
  }
  return spriteClipClipboard.cells;
}

function renderSpriteBoard() {
  spriteBoard.style.setProperty("--sprite-size", sprite.size);
  syncSpriteGridVisibility();
  spriteBoard.classList.toggle("is-clip-active", spriteClipActive);
  spriteBoard.classList.toggle("is-clip-floating", Boolean(spriteClipActive && spriteClipFloating && spriteClipClipboard));
  spriteClipSelection = normalizeSpriteClipRect(spriteClipSelection);
  const nextBoard = document.createDocumentFragment();
  for (let index = 0; index < sprite.cells.length; index += 1) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "sprite-cell sprite-color-swatch";
    syncSpriteCellElement(button, index);
    nextBoard.append(button);
  }
  renderSpriteClipSelectionFrame(nextBoard);
  spriteBoard.replaceChildren(nextBoard);
  renderSpriteAnimationSurfaces();
}

function syncSpriteGridVisibility() {
  if (!spriteBoard) {
    return;
  }
  spriteBoard.classList.toggle("is-grid-hidden", !spriteGridVisible);
  syncSpriteGridButton();
}

function syncSpriteGridButton() {
  if (!spriteGridButton) {
    return;
  }
  spriteGridButton.classList.toggle("is-active", spriteGridVisible);
  spriteGridButton.setAttribute("aria-pressed", spriteGridVisible ? "true" : "false");
  spriteGridButton.title = "Toggle grid";
  spriteGridButton.setAttribute("aria-label", "Toggle sprite grid");
}

function toggleSpriteGrid() {
  spriteGridVisible = !spriteGridVisible;
  syncSpriteGridVisibility();
  setSpriteActionStatus(spriteGridVisible ? "Sprite grid visible" : "Sprite grid hidden", "is-ok");
}

function renderSpriteClipSelectionFrame(target = spriteBoard) {
  const rect = normalizeSpriteClipRect(spriteClipSelection);
  if (!rect) {
    return;
  }
  renderSpriteClipFloatingPreview(rect, target);
  const frame = document.createElement("div");
  frame.className = "sprite-clip-selection-frame";
  frame.style.setProperty("--sprite-clip-x", String(rect.x));
  frame.style.setProperty("--sprite-clip-y", String(rect.y));
  frame.style.setProperty("--sprite-clip-width", String(rect.width));
  frame.style.setProperty("--sprite-clip-height", String(rect.height));
  frame.setAttribute("aria-hidden", "true");
  if (!spriteClipFloating) {
    for (const edge of ["n", "e", "s", "w"]) {
      const node = document.createElement("span");
      node.className = `sprite-clip-selection-edge sprite-clip-selection-edge-${edge}`;
      node.dataset.spriteClipResize = edge;
      frame.append(node);
    }
  }
  for (const handle of ["nw", "ne", "sw", "se"]) {
    const node = document.createElement("span");
    node.className = `sprite-clip-selection-handle sprite-clip-selection-handle-${handle}`;
    if (!spriteClipFloating) {
      node.dataset.spriteClipResize = handle;
    }
    frame.append(node);
  }
  target.append(frame);
}

function renderSpriteClipFloatingPreview(rect, target = spriteBoard) {
  const cells = spriteClipFloatingCellsForSelection(rect);
  if (!cells) {
    return;
  }
  const preview = document.createElement("div");
  preview.className = `sprite-clip-floating-preview is-${spriteClipFloating.kind || "copy"}`;
  preview.style.setProperty("--sprite-clip-x", String(rect.x));
  preview.style.setProperty("--sprite-clip-y", String(rect.y));
  preview.style.setProperty("--sprite-clip-width", String(rect.width));
  preview.style.setProperty("--sprite-clip-height", String(rect.height));
  preview.style.setProperty("--sprite-clip-preview-cols", String(rect.width));
  preview.setAttribute("aria-hidden", "true");
  for (const colorIndex of cells) {
    const cell = document.createElement("span");
    const validIndex = validSpriteColorIndex(colorIndex) ? colorIndex : null;
    cell.className = "sprite-clip-preview-cell sprite-color-swatch";
    cell.dataset.colorIndex = validIndex === null ? "erase" : String(validIndex);
    cell.style.setProperty("--sprite-swatch-color", spriteColorForColorIndex(validIndex));
    cell.style.setProperty("--sprite-cell-ink", spriteInkForColorIndex(validIndex));
    cell.style.setProperty("--sprite-puzzle-line", spriteGridLineForColorIndex(validIndex));
    preview.append(cell);
  }
  target.append(preview);
}

function syncSpriteCellElement(button, index) {
  const colorIndex = validSpriteColorIndex(sprite.cells[index]) ? sprite.cells[index] : null;
  const char = spriteExportCharForColorIndex(colorIndex);
  const isClipSelected = spriteClipRectContainsIndex(spriteClipSelection, index);
  button.dataset.index = String(index);
  button.dataset.colorIndex = colorIndex === null ? "erase" : String(colorIndex);
  button.classList.toggle("is-clip-selected", isClipSelected);
  button.style.setProperty("--sprite-swatch-color", spriteColorForColorIndex(colorIndex));
  button.style.setProperty("--sprite-cell-ink", spriteInkForColorIndex(colorIndex));
  button.style.setProperty("--sprite-puzzle-line", spriteGridLineForColorIndex(colorIndex));
  button.setAttribute("aria-label", `Sprite cell ${index + 1}: ${char}`);
}

function renderSpriteCellsAtIndices(indices) {
  for (const index of new Set(indices)) {
    const cell = spriteBoard.children[index];
    if (!cell || !cell.classList.contains("sprite-cell") || cell.dataset.index !== String(index)) {
      throw new Error(`Sprite cell element missing for changed cell ${index}`);
    }
    syncSpriteCellElement(cell, index);
  }
}

function selectSpriteColor(index) {
  commitSpriteColorEditHistory("sprite");
  const wasClipActive = spriteClipActive || spriteClipSelection;
  deactivateSpriteClipMode({ render: false });
  sprite.selectedColorIndex = validSpriteColorIndex(index) ? index : null;
  if (validSpriteColorIndex(sprite.selectedColorIndex)) {
    spriteLastPaintColorIndex = sprite.selectedColorIndex;
  }
  sprite.addPaletteOpen = false;
  sprite.editPaletteOpen = false;
  sprite.customColorOpen = false;
  sprite.addDraftColorIndex = null;
  renderSpriteControls();
  renderSpritePalette();
  if (wasClipActive) {
    renderSpriteBoard();
  }
}

function updateSelectedSpriteColor(value, options = {}) {
  const before = options.deferHistory || options.commitHistory ? null : visualEditSnapshot("sprite");
  if (options.deferHistory || options.commitHistory) {
    beginSpriteColorEditHistory("sprite");
  }
  if (!validSpriteColorIndex(sprite.selectedColorIndex)) {
    sprite.selectedColorIndex = 0;
  }
  const selected = sprite.palette[sprite.selectedColorIndex];
  if (!selected) {
    return;
  }
  const normalized = normalizeSpriteColor(value);
  selected.color = normalized;
  updateSpriteBoundColorDefinition(selected, normalized);
  if (options.closeMenu) {
    sprite.editPaletteOpen = false;
    sprite.customColorOpen = false;
    sprite.addDraftColorIndex = null;
    renderSpriteBuilder();
    if (options.deferHistory || options.commitHistory) {
      commitSpriteColorEditHistory("sprite");
    } else {
      pushVisualEditUndoSnapshot("sprite", before);
    }
    return;
  }
  renderSpriteColorSurfaces();
  if (options.deferHistory) {
    return;
  }
  if (options.commitHistory) {
    commitSpriteColorEditHistory("sprite");
    return;
  }
  pushVisualEditUndoSnapshot("sprite", before);
}

function toggleSpriteAddPalette() {
  commitSpriteColorEditHistory("sprite");
  const before = visualEditSnapshot("sprite");
  const opening = !sprite.addPaletteOpen;
  if (opening && sprite.palette.length >= SPRITE_COLOR_TOKENS.length) {
    setSpriteActionStatus(`Palette limit is ${SPRITE_COLOR_TOKENS.length} colors`, "is-error");
    return;
  }
  sprite.addPaletteOpen = opening;
  sprite.editPaletteOpen = false;
  sprite.customColorOpen = opening;
  if (opening) {
    if (!validSpriteColorIndex(sprite.addDraftColorIndex)) {
      sprite.palette.push({ color: normalizeSpriteColor(nextSpritePresetColor()) });
      sprite.addDraftColorIndex = sprite.palette.length - 1;
    }
    sprite.selectedColorIndex = sprite.addDraftColorIndex;
    renderSpriteBuilder();
    pushVisualEditUndoSnapshot("sprite", before);
    return;
  }
  sprite.addDraftColorIndex = null;
  renderSpriteBuilder();
  pushVisualEditUndoSnapshot("sprite", before);
}

function addSpriteColor(color = nextSpritePresetColor()) {
  const before = visualEditSnapshot("sprite");
  const draftIndex = validSpriteColorIndex(sprite.addDraftColorIndex) ? sprite.addDraftColorIndex : null;
  if (draftIndex === null && sprite.palette.length >= SPRITE_COLOR_TOKENS.length) {
    setSpriteActionStatus(`Palette limit is ${SPRITE_COLOR_TOKENS.length} colors`, "is-error");
    return;
  }
  if (draftIndex === null) {
    sprite.palette.push({ color: normalizeSpriteColor(color) });
    sprite.selectedColorIndex = sprite.palette.length - 1;
  } else {
    sprite.palette[draftIndex].color = normalizeSpriteColor(color);
    sprite.selectedColorIndex = draftIndex;
  }
  sprite.addPaletteOpen = false;
  sprite.editPaletteOpen = false;
  sprite.customColorOpen = false;
  sprite.addDraftColorIndex = null;
  renderSpriteBuilder();
  pushVisualEditUndoSnapshot("sprite", before);
}

function previewNewSpriteColor(color, options = {}) {
  const before = options.deferHistory ? null : visualEditSnapshot("sprite");
  if (options.deferHistory) {
    beginSpriteColorEditHistory("sprite");
  }
  if (!validSpriteColorIndex(sprite.addDraftColorIndex) && sprite.palette.length >= SPRITE_COLOR_TOKENS.length) {
    return;
  }
  if (!validSpriteColorIndex(sprite.addDraftColorIndex)) {
    sprite.palette.push({ color: normalizeSpriteColor(color) });
    sprite.addDraftColorIndex = sprite.palette.length - 1;
    sprite.selectedColorIndex = sprite.addDraftColorIndex;
    renderSpriteBuilder();
  } else {
    sprite.palette[sprite.addDraftColorIndex].color = normalizeSpriteColor(color);
    sprite.selectedColorIndex = sprite.addDraftColorIndex;
    renderSpriteColorSurfaces();
  }
  if (options.closeMenu) {
    sprite.addPaletteOpen = false;
    sprite.editPaletteOpen = false;
    sprite.customColorOpen = false;
    sprite.addDraftColorIndex = null;
    renderSpriteBuilder();
  }
  if (options.deferHistory) {
    return;
  }
  pushVisualEditUndoSnapshot("sprite", before);
}

function closeSpriteColorEditor() {
  clearSpriteColorEditorState();
  renderSpritePalette();
}

function confirmSpriteColorAdd() {
  if (!validSpriteColorIndex(sprite.addDraftColorIndex)) {
    return;
  }
  commitSpriteColorEditHistory("sprite");
  sprite.selectedColorIndex = sprite.addDraftColorIndex;
  sprite.addPaletteOpen = false;
  sprite.editPaletteOpen = false;
  sprite.customColorOpen = false;
  sprite.addDraftColorIndex = null;
  renderSpriteBuilder();
}

function cancelSpriteColorAdd() {
  discardSpriteColorEditHistory("sprite");
  const before = visualEditSnapshot("sprite");
  if (validSpriteColorIndex(sprite.addDraftColorIndex)) {
    removeSpritePaletteColor(sprite.addDraftColorIndex);
  }
  sprite.addPaletteOpen = false;
  sprite.editPaletteOpen = false;
  sprite.customColorOpen = false;
  sprite.addDraftColorIndex = null;
  renderSpriteBuilder();
  pushVisualEditUndoSnapshot("sprite", before);
}

function closeSpriteColorEditorFromOutside(event) {
  const target = event.target;
  const spritePopupOpen = Boolean(
    sprite.addPaletteOpen
    || sprite.editPaletteOpen
    || sprite.colorTagPickerOpen
    || sprite.shapeTagPickerOpen
  );
  const sprite3dPopupOpen = Boolean(sprite3d.addPaletteOpen || sprite3d.editPaletteOpen);
  if (spritePopupOpen && !spritePalette.contains(target) && !spriteShapeField?.contains(target)) {
    clearSpriteColorEditorState();
    clearSpriteTagPickerState();
    renderSpriteControls();
    renderSpritePalette();
  }
  if (
    sprite3dPopupOpen
    && !sprite3dPalette?.contains(target)
    && typeof closeSprite3dColorEditor === "function"
  ) {
    closeSprite3dColorEditor();
  }
}

function nextSpritePresetColor(palette = sprite.palette) {
  const used = new Set(palette.map((entry) => normalizeSpriteColor(entry.color)));
  return SPRITE_COLOR_PRESETS.find((color) => !used.has(color)) || "#e94f64";
}

function deleteSelectedSpriteColor() {
  commitSpriteColorEditHistory("sprite");
  const before = visualEditSnapshot("sprite");
  if (!validSpriteColorIndex(sprite.selectedColorIndex) || sprite.palette.length <= 1) {
    return;
  }
  sprite.addPaletteOpen = false;
  sprite.editPaletteOpen = false;
  sprite.customColorOpen = false;
  sprite.addDraftColorIndex = null;
  removeSpritePaletteColor(sprite.selectedColorIndex);
  updateSpriteBoundShapeDefinition();
  renderSpriteBuilder();
  pushVisualEditUndoSnapshot("sprite", before);
}

function removeSpritePaletteColor(deletedIndex) {
  if (!validSpriteColorIndex(deletedIndex) || sprite.palette.length <= 1) {
    return;
  }
  const oldPaletteLength = sprite.palette.length;
  sprite.palette.splice(deletedIndex, 1);
  const normalizeCell = (colorIndex) => {
    if (!Number.isInteger(colorIndex) || colorIndex < 0 || colorIndex >= oldPaletteLength) {
      return null;
    }
    if (colorIndex === deletedIndex) {
      return null;
    }
    return colorIndex > deletedIndex ? colorIndex - 1 : colorIndex;
  };
  sprite.cells = sprite.cells.map(normalizeCell);
  if (Array.isArray(sprite.animationFrames)) {
    sprite.animationFrames = sprite.animationFrames.map((frame) => (
      Array.isArray(frame) ? frame.map(normalizeCell) : frame
    ));
    if (sprite.animationMode) {
      ensureSpriteAnimationFrames();
      sprite.animationFrames[sprite.animationFrameIndex] = sprite.cells;
    }
  }
  sprite.selectedColorIndex = Math.min(deletedIndex, sprite.palette.length - 1);
}

function normalizeSpriteColor(value) {
  return parseSpriteHexColor(value) || "#e94f64";
}

function parseSpriteHexColor(value) {
  const color = String(value || "").trim();
  if (color.toLowerCase() === "transparent") {
    return "#00000000";
  }
  const full = color.startsWith("#") ? color : `#${color}`;
  if (/^#[0-9a-f]{8}$/i.test(full)) {
    return full.toLowerCase();
  }
  if (/^#[0-9a-f]{6}$/i.test(full)) {
    return full.toLowerCase();
  }
  const shortAlpha = full.match(/^#([0-9a-f])([0-9a-f])([0-9a-f])([0-9a-f])$/i);
  if (shortAlpha) {
    return `#${shortAlpha[1]}${shortAlpha[1]}${shortAlpha[2]}${shortAlpha[2]}${shortAlpha[3]}${shortAlpha[3]}${shortAlpha[4]}${shortAlpha[4]}`.toLowerCase();
  }
  const short = full.match(/^#([0-9a-f])([0-9a-f])([0-9a-f])$/i);
  if (short) {
    return `#${short[1]}${short[1]}${short[2]}${short[2]}${short[3]}${short[3]}`.toLowerCase();
  }
  return "";
}

function spriteRgbHex(value) {
  return normalizeSpriteColor(value).slice(0, 7);
}

function spriteAlphaPercent(value) {
  const normalized = normalizeSpriteColor(value);
  if (normalized.length !== 9) {
    return 100;
  }
  return Math.round((Number.parseInt(normalized.slice(7, 9), 16) / 255) * 100);
}

function spriteColorWithAlpha(rgb, alphaPercent) {
  const base = spriteRgbHex(rgb);
  const percent = Math.max(0, Math.min(100, Math.round(Number(alphaPercent) || 0)));
  if (percent >= 100) {
    return base;
  }
  const alpha = Math.round((percent / 100) * 255).toString(16).padStart(2, "0");
  return `${base}${alpha}`;
}

function renderSpriteColorSurfaces() {
  syncSpritePaletteSwatches();
  syncSpriteColorAdjusters();
  renderSpriteBoard();
  syncSpriteSourceActionButtons();
}

function syncSpritePaletteSwatches() {
  for (const [index, entry] of sprite.palette.entries()) {
    const color = normalizeSpriteColor(entry.color);
    const displayName = spritePaletteEntryDisplayName(entry);
    for (const token of spritePalette.querySelectorAll(`[data-color-index="${index}"]`)) {
      token.style.setProperty("--sprite-swatch-color", color);
      token.style.setProperty("--sprite-token-ink", readableInkForColor(color));
      token.title = displayName ? `Paint ${displayName} (${color})` : `Paint ${color}`;
      token.setAttribute("aria-label", displayName ? `Paint color ${index}: ${displayName}` : `Paint color ${index}`);
    }
  }
  const selected = sprite.palette[sprite.selectedColorIndex];
  const currentButton = spritePalette.querySelector(".sprite-current-color-button");
  if (currentButton && selected) {
    const normalized = normalizeSpriteColor(selected.color);
    const displayName = spritePaletteEntryDisplayName(selected);
    currentButton.style.setProperty("--sprite-current-color", normalized);
    currentButton.setAttribute("aria-label", displayName ? `Pick selected color ${displayName}` : `Pick selected color ${normalized}`);
    const currentHexInput = spritePalette.querySelector(".sprite-current-hex-input");
    if (currentHexInput && !currentHexInput.classList.contains("is-name-mode") && document.activeElement !== currentHexInput) {
      currentHexInput.value = normalized;
    }
  }
}

function syncSpriteColorAdjusters() {
  const selected = validSpriteColorIndex(sprite.selectedColorIndex)
    ? sprite.palette[sprite.selectedColorIndex]
    : null;
  if (!selected) {
    return;
  }
  const normalized = normalizeSpriteColor(selected.color);
  for (const adjuster of spritePalette.querySelectorAll(".sprite-color-adjuster")) {
    if (adjuster.contains(document.activeElement)) {
      continue;
    }
    adjuster.syncColor?.(normalized);
  }
}

function validSpriteColorIndex(index) {
  return Number.isInteger(index) && index >= 0 && index < sprite.palette.length;
}

function spriteExportCharForColorIndex(index) {
  if (!validSpriteColorIndex(index)) {
    return ".";
  }
  return SPRITE_COLOR_TOKENS[index] || ".";
}

function spriteColorForColorIndex(index) {
  return validSpriteColorIndex(index) ? normalizeSpriteColor(sprite.palette[index].color) : "#00000000";
}

function spriteInkForColorIndex(index) {
  return validSpriteColorIndex(index) ? readableInkForColor(sprite.palette[index].color) : "#8d969f";
}

function spriteGridLineForColorIndex(index) {
  return validSpriteColorIndex(index) ? readableInkForColor(sprite.palette[index].color) : "#1d242b";
}

function readableInkForColor(color) {
  const normalized = normalizeSpriteColor(color).slice(1);
  const red = Number.parseInt(normalized.slice(0, 2), 16);
  const green = Number.parseInt(normalized.slice(2, 4), 16);
  const blue = Number.parseInt(normalized.slice(4, 6), 16);
  const alpha = normalized.length >= 8 ? Number.parseInt(normalized.slice(6, 8), 16) / 255 : 1;
  const base = 190;
  const mixedRed = red * alpha + base * (1 - alpha);
  const mixedGreen = green * alpha + base * (1 - alpha);
  const mixedBlue = blue * alpha + base * (1 - alpha);
  const luminance = (mixedRed * 299 + mixedGreen * 587 + mixedBlue * 114) / 1000;
  return luminance > 150 ? "#1d242b" : "#ffffff";
}

function updateSpriteSize(value) {
  const before = visualEditSnapshot("sprite");
  const previousSize = sprite.size;
  const nextSize = clampSpriteSize(value);
  if (nextSize === sprite.size) {
    renderSpriteControls();
    return;
  }
  const nextCells = Array.from({ length: nextSize * nextSize }, () => null);
  const copySize = Math.min(sprite.size, nextSize);
  for (let y = 0; y < copySize; y += 1) {
    for (let x = 0; x < copySize; x += 1) {
      const value = sprite.cells[y * sprite.size + x];
      nextCells[y * nextSize + x] = validSpriteColorIndex(value) ? value : null;
    }
  }
  sprite.size = nextSize;
  sprite.cells = nextCells;
  syncSpriteAnimationFramesAfterSizeChange(previousSize, nextSize, nextCells);
  updateSpriteBoundShapeDefinition();
  renderSpriteBuilder();
  pushVisualEditUndoSnapshot("sprite", before);
}

function spriteScaleFactor() {
  return spriteEditorScaleFactor(spriteScaleInput, SPRITE_EDITOR_MAX_SIZE);
}

function canScaleDownSprite(factor = spriteScaleFactor()) {
  return factor > 1 && sprite.size >= factor && sprite.size % factor === 0;
}

function scaleUpSprite() {
  const before = visualEditSnapshot("sprite");
  const factor = spriteScaleFactor();
  const previousSize = sprite.size;
  const nextSize = sprite.size * factor;
  if (nextSize > SPRITE_EDITOR_MAX_SIZE) {
    setSpriteActionStatus(`Sprite size limit is ${SPRITE_EDITOR_MAX_SIZE}`, "is-error");
    renderSpriteControls();
    return;
  }

  const nextCells = Array.from({ length: nextSize * nextSize }, () => null);
  for (let y = 0; y < sprite.size; y += 1) {
    for (let x = 0; x < sprite.size; x += 1) {
      const colorIndex = validSpriteColorIndex(sprite.cells[y * sprite.size + x])
        ? sprite.cells[y * sprite.size + x]
        : null;
      const nextX = x * factor;
      const nextY = y * factor;
      for (let dy = 0; dy < factor; dy += 1) {
        for (let dx = 0; dx < factor; dx += 1) {
          nextCells[(nextY + dy) * nextSize + nextX + dx] = colorIndex;
        }
      }
    }
  }

  sprite.size = nextSize;
  sprite.cells = nextCells;
  syncSpriteAnimationFramesAfterSizeChange(previousSize, nextSize, nextCells);
  updateSpriteBoundShapeDefinition();
  renderSpriteBuilder();
  const message = `Scaled ${factor}x to ${nextSize}x${nextSize}`;
  setSpriteActionStatus(message, "is-ok");
  setStatus(`Scaled sprite ${factor}x to ${nextSize}x${nextSize}`, "is-ok");
  pushVisualEditUndoSnapshot("sprite", before);
}

function scaleDownSprite() {
  const before = visualEditSnapshot("sprite");
  const factor = spriteScaleFactor();
  if (!canScaleDownSprite(factor)) {
    setSpriteActionStatus(`Size ${sprite.size} is not divisible by ${factor}`, "is-error");
    renderSpriteControls();
    return;
  }

  const previousSize = sprite.size;
  const nextSize = sprite.size / factor;
  const nextCells = Array.from({ length: nextSize * nextSize }, () => null);
  for (let y = 0; y < nextSize; y += 1) {
    for (let x = 0; x < nextSize; x += 1) {
      nextCells[y * nextSize + x] = validSpriteColorIndex(sprite.cells[(y * factor) * sprite.size + (x * factor)])
        ? sprite.cells[(y * factor) * sprite.size + (x * factor)]
        : null;
    }
  }

  sprite.size = nextSize;
  sprite.cells = nextCells;
  syncSpriteAnimationFramesAfterSizeChange(previousSize, nextSize, nextCells);
  updateSpriteBoundShapeDefinition();
  renderSpriteBuilder();
  const message = `Scaled down ${factor}x to ${nextSize}x${nextSize}`;
  setSpriteActionStatus(message, "is-ok");
  setStatus(`Scaled sprite down ${factor}x to ${nextSize}x${nextSize}`, "is-ok");
  pushVisualEditUndoSnapshot("sprite", before);
}

function transformSpriteCells(mapper, message) {
  const before = visualEditSnapshot("sprite");
  const size = sprite.size;
  const previousCells = sprite.cells;
  const nextCells = Array.from({ length: size * size }, () => null);
  for (let y = 0; y < size; y += 1) {
    for (let x = 0; x < size; x += 1) {
      const source = mapper(x, y, size);
      const colorIndex = previousCells[source.y * size + source.x];
      nextCells[y * size + x] = validSpriteColorIndex(colorIndex) ? colorIndex : null;
    }
  }
  sprite.cells = nextCells;
  if (sprite.animationMode) {
    ensureSpriteAnimationFrames();
    sprite.animationFrames[sprite.animationFrameIndex] = sprite.cells;
  }
  sprite.addPaletteOpen = false;
  sprite.editPaletteOpen = false;
  sprite.customColorOpen = false;
  sprite.addDraftColorIndex = null;
  updateSpriteBoundShapeDefinition();
  renderSpriteBuilder();
  setSpriteActionStatus(message, "is-ok");
  setStatus(message, "is-ok");
  pushVisualEditUndoSnapshot("sprite", before);
}

function rotateSpriteLeft() {
  transformSpriteCells((x, y, size) => ({ x: size - 1 - y, y: x }), "Rotated left");
}

function rotateSpriteRight() {
  transformSpriteCells((x, y, size) => ({ x: y, y: size - 1 - x }), "Rotated right");
}

function flipSpriteHorizontal() {
  transformSpriteCells((x, y, size) => ({ x: size - 1 - x, y }), "Flipped horizontal");
}

function flipSpriteVertical() {
  transformSpriteCells((x, y, size) => ({ x, y: size - 1 - y }), "Flipped vertical");
}

function normalizedSpriteCellColorIndex(index) {
  const colorIndex = sprite.cells[index];
  return validSpriteColorIndex(colorIndex) ? colorIndex : null;
}

function floodFillSpriteComponentAtIndex(index, colorIndex) {
  if (!Number.isInteger(index) || index < 0 || index >= sprite.cells.length) {
    return 0;
  }
  const nextColorIndex = validSpriteColorIndex(colorIndex) ? colorIndex : null;
  const targetColorIndex = normalizedSpriteCellColorIndex(index);
  if (targetColorIndex === nextColorIndex) {
    return 0;
  }
  const size = sprite.size;
  const visited = new Uint8Array(sprite.cells.length);
  const stack = [index];
  let changed = 0;
  while (stack.length) {
    const current = stack.pop();
    if (visited[current] || normalizedSpriteCellColorIndex(current) !== targetColorIndex) {
      continue;
    }
    visited[current] = 1;
    sprite.cells[current] = nextColorIndex;
    changed += 1;
    const x = current % size;
    const y = Math.floor(current / size);
    if (x > 0) {
      stack.push(current - 1);
    }
    if (x < size - 1) {
      stack.push(current + 1);
    }
    if (y > 0) {
      stack.push(current - size);
    }
    if (y < size - 1) {
      stack.push(current + size);
    }
  }
  if (!changed) {
    return 0;
  }
  sprite.solidSource = false;
  sprite.addPaletteOpen = false;
  sprite.editPaletteOpen = false;
  sprite.customColorOpen = false;
  sprite.addDraftColorIndex = null;
  updateSpriteBoundShapeDefinition();
  renderSpriteBoard();
  syncSpriteSourceActionButtons();
  return changed;
}

function bucketFillSpriteFromIndex(index) {
  const count = floodFillSpriteComponentAtIndex(index, sprite.selectedColorIndex);
  if (!count) {
    setSpriteActionStatus("Connected area already has that color", "is-ok");
    deactivateSpriteBucketModeAfterUse();
    return false;
  }
  const colorIndex = validSpriteColorIndex(sprite.selectedColorIndex) ? sprite.selectedColorIndex : null;
  const message = colorIndex === null ? "Filled connected area with transparent" : "Filled connected area";
  deactivateSpriteBucketModeAfterUse();
  setSpriteActionStatus(message, "is-ok");
  setStatus(message, "is-ok");
  return true;
}

function bucketFillSpriteFromElement(element) {
  return bucketFillSpriteFromIndex(spriteCellIndexFromElement(element));
}

function paintSpriteCellFromElement(element) {
  const index = spriteCellIndexFromElement(element);
  return paintSpriteAtPoint(spritePointForCellIndex(index), sprite.selectedColorIndex);
}

function spriteCellIndexFromElement(element) {
  const cell = element?.closest?.(".sprite-cell");
  if (!cell || !spriteBoard.contains(cell)) {
    return -1;
  }
  const index = Number(cell.dataset.index);
  return Number.isInteger(index) && index >= 0 && index < sprite.cells.length ? index : -1;
}

function spritePointForCellIndex(index) {
  if (!Number.isInteger(index) || index < 0 || index >= sprite.cells.length) {
    return null;
  }
  return {
    x: (index % sprite.size) + 0.5,
    y: Math.floor(index / sprite.size) + 0.5,
  };
}

function paintSpriteAtPoint(point, colorIndex) {
  const indices = spritePaintIndicesForPoint(point);
  const changedIndices = paintSpriteCellsAtIndices(indices, colorIndex);
  if (!changedIndices.length) {
    return false;
  }
  finishSpritePaintMutation(changedIndices);
  return true;
}

function paintSpriteCellsAtIndices(indices, colorIndex) {
  const changedIndices = [];
  for (const index of indices) {
    if (setSpriteCellColorAtIndex(index, colorIndex)) {
      changedIndices.push(index);
    }
  }
  return changedIndices;
}

function setSpriteCellColorAtIndex(index, colorIndex) {
  if (!Number.isInteger(index) || index < 0 || index >= sprite.cells.length) {
    return false;
  }
  const nextColorIndex = validSpriteColorIndex(colorIndex) ? colorIndex : null;
  if (sprite.cells[index] === nextColorIndex) {
    return false;
  }
  sprite.cells[index] = nextColorIndex;
  return true;
}

function finishSpritePaintMutation(changedIndices, options = {}) {
  sprite.solidSource = false;
  if (!options.deferSourceSync) {
    updateSpriteBoundShapeDefinition();
  }
  renderSpriteCellsAtIndices(changedIndices);
  renderSpriteAnimationSurfaces();
  if (!options.deferSourceSync) {
    syncSpriteSourceActionButtons();
  }
}

function paintSpriteCellFromPoint(clientX, clientY, colorIndex) {
  return paintSpriteAtPoint(spriteBoardPointFromClient(clientX, clientY), colorIndex);
}

function spriteBoardGeometry() {
  const rect = spriteBoard.getBoundingClientRect();
  return {
    left: rect.left,
    top: rect.top,
    right: rect.right,
    bottom: rect.bottom,
    width: rect.width,
    height: rect.height,
    size: sprite.size,
  };
}

function spriteBoardPointFromClient(clientX, clientY, geometry = spriteBoardGeometry()) {
  if (
    geometry.width <= 0
    || geometry.height <= 0
    || clientX < geometry.left
    || clientX > geometry.right
    || clientY < geometry.top
    || clientY > geometry.bottom
  ) {
    return null;
  }
  return {
    x: ((clientX - geometry.left) / geometry.width) * geometry.size,
    y: ((clientY - geometry.top) / geometry.height) * geometry.size,
  };
}

function spriteCellIndexFromPoint(point) {
  if (!point || !Number.isFinite(point.x) || !Number.isFinite(point.y)) {
    return -1;
  }
  const x = Math.floor(point.x);
  const y = Math.floor(point.y);
  if (x < 0 || x >= sprite.size || y < 0 || y >= sprite.size) {
    return -1;
  }
  return y * sprite.size + x;
}

function spriteBrushDiameterCells(preset = spriteBrushPreset) {
  const config = SPRITE_BRUSH_PRESETS[normalizeSpriteBrushPreset(preset)];
  if (Number.isFinite(config.diameterCells)) {
    return Math.min(sprite.size, config.diameterCells);
  }
  return Math.min(sprite.size, sprite.size * config.ratio);
}

function spritePaintIndicesForPoint(point) {
  if (!point || !Number.isFinite(point.x) || !Number.isFinite(point.y)) {
    return [];
  }
  if (spriteBrushIsPixel()) {
    const index = spriteCellIndexFromPoint(point);
    return index >= 0 ? [index] : [];
  }
  const diameter = spriteBrushDiameterCells();
  const radius = diameter / 2;
  const minX = Math.max(0, Math.floor(point.x - radius - 0.5));
  const maxX = Math.min(sprite.size - 1, Math.ceil(point.x + radius - 0.5));
  const minY = Math.max(0, Math.floor(point.y - radius - 0.5));
  const maxY = Math.min(sprite.size - 1, Math.ceil(point.y + radius - 0.5));
  const indices = [];
  for (let y = minY; y <= maxY; y += 1) {
    for (let x = minX; x <= maxX; x += 1) {
      const dx = x + 0.5 - point.x;
      const dy = y + 0.5 - point.y;
      if ((dx * dx) + (dy * dy) <= radius * radius) {
        indices.push(y * sprite.size + x);
      }
    }
  }
  if (!indices.length) {
    const index = spriteCellIndexFromPoint(point);
    if (index >= 0) {
      indices.push(index);
    }
  }
  return indices;
}

function spritePaintDragIndices(point) {
  if (!spritePaintDrag || !point) {
    return [];
  }
  const lastPoint = spritePaintDrag.lastPoint;
  if (!lastPoint) {
    return spritePaintIndicesForPoint(point);
  }
  const points = spriteInterpolatedBrushPoints(lastPoint, point);
  const indices = new Set();
  for (const brushPoint of points) {
    for (const cellIndex of spritePaintIndicesForPoint(brushPoint)) {
      indices.add(cellIndex);
    }
  }
  return [...indices];
}

function spriteInterpolatedBrushPoints(fromPoint, toPoint) {
  const dx = toPoint.x - fromPoint.x;
  const dy = toPoint.y - fromPoint.y;
  const distance = Math.hypot(dx, dy);
  const steps = Math.max(1, Math.ceil(distance * 2));
  if (steps <= 0) {
    return [toPoint];
  }
  const points = [];
  for (let step = 1; step <= steps; step += 1) {
    const t = step / steps;
    points.push({
      x: fromPoint.x + dx * t,
      y: fromPoint.y + dy * t,
    });
  }
  return points;
}

function startSpriteClip(event, geometry, cell) {
  event.preventDefault();
  spriteClipActive = true;
  const resizeHandle = !spriteClipFloating && spriteClipSelection
    ? event.target.closest("[data-sprite-clip-resize]")
    : null;
  if (resizeHandle) {
    spriteClipDrag = {
      mode: "resize",
      pointerId: event.pointerId,
      geometry,
      startCell: cell,
      origin: spriteClipSelection,
      preview: spriteClipSelection,
      edge: resizeHandle.dataset.spriteClipResize,
    };
  } else if (spriteClipSelectionContainsCell(cell)) {
    spriteClipDrag = {
      mode: "move",
      pointerId: event.pointerId,
      geometry,
      startCell: cell,
      origin: spriteClipSelection,
      preview: spriteClipSelection,
    };
  } else if (spriteClipFloating && spriteClipClipboard) {
    const target = spriteClipFloatingRectAtCell(cell);
    if (!target) {
      return;
    }
    spriteClipSelection = target;
    spriteClipDrag = {
      mode: "move",
      pointerId: event.pointerId,
      geometry,
      startCell: cell,
      origin: target,
      preview: target,
    };
  } else {
    spriteClipSelection = spriteClipRectFromCells(cell, cell);
    spriteClipDrag = {
      mode: "select",
      pointerId: event.pointerId,
      geometry,
      startCell: cell,
    };
  }
  if (spriteBoard.setPointerCapture) {
    spriteBoard.setPointerCapture(event.pointerId);
  }
  renderSpriteBoard();
}

function continueSpriteClip(event) {
  if (!spriteClipDrag || spriteClipDrag.pointerId !== event.pointerId) {
    return false;
  }
  const cell = spriteClipCellFromClient(event.clientX, event.clientY, spriteClipDrag.geometry);
  if (!cell) {
    return true;
  }
  event.preventDefault();
  if (spriteClipDrag.mode === "select") {
    spriteClipSelection = spriteClipRectFromCells(spriteClipDrag.startCell, cell);
    renderSpriteBoard();
    return true;
  }
  if (spriteClipDrag.mode === "move") {
    const origin = spriteClipDrag.origin;
    const dx = cell.x - spriteClipDrag.startCell.x;
    const dy = cell.y - spriteClipDrag.startCell.y;
    const nextX = Math.max(0, Math.min(sprite.size - origin.width, origin.x + dx));
    const nextY = Math.max(0, Math.min(sprite.size - origin.height, origin.y + dy));
    const next = normalizeSpriteClipRect({ ...origin, x: nextX, y: nextY });
    if (next && (!spriteClipDrag.preview || next.x !== spriteClipDrag.preview.x || next.y !== spriteClipDrag.preview.y)) {
      spriteClipSelection = next;
      spriteClipDrag.preview = next;
      renderSpriteBoard();
    }
    return true;
  }
  if (spriteClipDrag.mode === "resize") {
    const next = spriteClipResizeRect(spriteClipDrag.origin, spriteClipDrag.edge, cell);
    if (next && (!spriteClipDrag.preview
      || next.x !== spriteClipDrag.preview.x
      || next.y !== spriteClipDrag.preview.y
      || next.width !== spriteClipDrag.preview.width
      || next.height !== spriteClipDrag.preview.height)) {
      spriteClipSelection = next;
      spriteClipDrag.preview = next;
      renderSpriteBoard();
    }
    return true;
  }
  return true;
}

function stopSpriteClip(event) {
  if (!spriteClipDrag || spriteClipDrag.pointerId !== event.pointerId) {
    return false;
  }
  if (spriteBoard.hasPointerCapture?.(event.pointerId)) {
    spriteBoard.releasePointerCapture(event.pointerId);
  }
  event.preventDefault();
  const drag = spriteClipDrag;
  spriteClipDrag = null;
  spriteClipSelection = normalizeSpriteClipRect(spriteClipSelection);
  renderSpriteBuilder();
  if (!spriteClipSelection) {
    return true;
  }
  const verb = drag.mode === "move"
    ? "Clip range moved"
    : drag.mode === "resize"
      ? "Clip range resized"
      : "Clip range selected";
  setSpriteActionStatus(`${verb} ${spriteClipSelection.width}x${spriteClipSelection.height}`, "is-ok");
  return true;
}

function spriteClipResizeRect(origin, edge, cell) {
  const rect = normalizeSpriteClipRect(origin);
  if (!rect || !edge || !cell) {
    return null;
  }
  let left = rect.x;
  let right = rect.x + rect.width - 1;
  let top = rect.y;
  let bottom = rect.y + rect.height - 1;
  if (edge.includes("w")) {
    left = Math.max(0, Math.min(cell.x, right));
  }
  if (edge.includes("e")) {
    right = Math.min(sprite.size - 1, Math.max(cell.x, left));
  }
  if (edge.includes("n")) {
    top = Math.max(0, Math.min(cell.y, bottom));
  }
  if (edge.includes("s")) {
    bottom = Math.min(sprite.size - 1, Math.max(cell.y, top));
  }
  return normalizeSpriteClipRect({
    x: left,
    y: top,
    width: right - left + 1,
    height: bottom - top + 1,
  });
}

function spriteClipShortcutTargetIsText(target) {
  const tagName = target?.tagName || "";
  return target?.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(tagName);
}

function spriteClipShortcutsAreActive() {
  return currentPreviewMode === "sprite" && spriteClipActive && !spriteBuilder.hidden;
}

function moveSpriteClipRangeBy(dx, dy) {
  const origin = normalizeSpriteClipRect(spriteClipSelection);
  if (!origin) {
    return false;
  }
  const target = normalizeSpriteClipRect({
    ...origin,
    x: origin.x + dx,
    y: origin.y + dy,
  });
  if (!target) {
    setSpriteActionStatus("Clip must stay inside sprite", "is-error");
    return true;
  }
  moveSpriteClipRange(target);
  return true;
}

function handleSpriteClipKeyboard(event) {
  if (!spriteClipShortcutsAreActive() || spriteClipShortcutTargetIsText(event.target)) {
    return false;
  }
  const key = event.key.length === 1 ? event.key.toLowerCase() : event.key;
  const modifier = (event.metaKey && !event.ctrlKey) || (event.ctrlKey && !event.metaKey);
  let handled = false;
  if (modifier && !event.altKey && !event.shiftKey && key === "c") {
    handled = copySpriteClipSelection();
  } else if (modifier && !event.altKey && !event.shiftKey && key === "x") {
    handled = cutSpriteClipSelection();
  } else if (modifier && !event.altKey && !event.shiftKey && key === "v") {
    handled = pasteSpriteClipClipboard();
  } else if (!modifier && !event.altKey && (key === "Backspace" || key === "Delete")) {
    handled = clearSpriteClipSelection();
  } else if (!modifier && !event.altKey && key === "Escape") {
    deactivateSpriteClipMode();
    setSpriteActionStatus(spritePaintToolStatusText(), "is-ok");
    handled = true;
  } else if (!modifier && !event.altKey && key === "ArrowLeft") {
    handled = moveSpriteClipRangeBy(-1, 0);
  } else if (!modifier && !event.altKey && key === "ArrowRight") {
    handled = moveSpriteClipRangeBy(1, 0);
  } else if (!modifier && !event.altKey && key === "ArrowUp") {
    handled = moveSpriteClipRangeBy(0, -1);
  } else if (!modifier && !event.altKey && key === "ArrowDown") {
    handled = moveSpriteClipRangeBy(0, 1);
  }
  if (!handled) {
    return false;
  }
  event.preventDefault();
  event.stopPropagation();
  return true;
}

function startSpritePaint(event) {
  if (event.button !== 0) {
    return;
  }
  const geometry = spriteBoardGeometry();
  const point = spriteBoardPointFromClient(event.clientX, event.clientY, geometry);
  const index = spriteCellIndexFromPoint(point);
  if (spriteClipActive) {
    const cell = spriteClipCellFromClient(event.clientX, event.clientY, geometry);
    if (!cell) {
      return;
    }
    startSpriteClip(event, geometry, cell);
    return;
  }
  if (!point || index < 0) {
    return;
  }
  event.preventDefault();
  if (spriteBucketActive) {
    const before = visualEditSnapshot("sprite");
    if (bucketFillSpriteFromIndex(index)) {
      pushVisualEditUndoSnapshot("sprite", before);
    }
    return;
  }
  spritePaintDrag = {
    pointerId: event.pointerId,
    colorIndex: sprite.selectedColorIndex,
    lastPoint: null,
    geometry,
    beforeSnapshot: visualEditSnapshot("sprite"),
    changed: false,
  };
  if (spriteBoard.setPointerCapture) {
    spriteBoard.setPointerCapture(event.pointerId);
  }
  paintSpriteDragPoint(point);
}

function continueSpritePaint(event) {
  if (continueSpriteClip(event)) {
    return;
  }
  const geometry = spritePaintDrag?.geometry || spriteBoardGeometry();
  const point = spriteBoardPointFromClient(event.clientX, event.clientY, geometry);
  if (!spritePaintDrag || spritePaintDrag.pointerId !== event.pointerId) {
    return;
  }
  event.preventDefault();
  paintSpriteDragPoint(point);
}

function stopSpritePaint(event) {
  if (stopSpriteClip(event)) {
    return;
  }
  if (!spritePaintDrag || spritePaintDrag.pointerId !== event.pointerId) {
    return;
  }
  if (spriteBoard.hasPointerCapture?.(event.pointerId)) {
    spriteBoard.releasePointerCapture(event.pointerId);
  }
  if (spritePaintDrag.changed) {
    updateSpriteBoundShapeDefinition();
    syncSpriteSourceActionButtons();
    pushVisualEditUndoSnapshot("sprite", spritePaintDrag.beforeSnapshot);
  }
  spritePaintDrag = null;
}

function paintSpriteDragPoint(point) {
  if (!spritePaintDrag || !point) {
    return;
  }
  const indices = spritePaintDragIndices(point);
  spritePaintDrag.lastPoint = point;
  const changedIndices = paintSpriteCellsAtIndices(indices, spritePaintDrag.colorIndex);
  if (changedIndices.length) {
    finishSpritePaintMutation(changedIndices, { deferSourceSync: true });
    spritePaintDrag.changed = true;
  }
}

function spriteAscii() {
  const rows = [];
  for (let y = 0; y < sprite.size; y += 1) {
    const row = [];
    for (let x = 0; x < sprite.size; x += 1) {
      row.push(spriteExportCharForColorIndex(sprite.cells[y * sprite.size + x]));
    }
    rows.push(row.join(""));
  }
  return rows.join("\n");
}

function spriteClipboardText() {
  return puzzleSpriteText();
}

function puzzleSpriteText() {
  return spriteObjectDefinitionText("");
}

function spriteObjectName() {
  const cleaned = String(spriteNameInput.value || "")
    .trim()
    .replace(/[^\w:@]+/g, "_")
    .replace(/(?!^)@/g, "_")
    .replace(/^_+|_+$/g, "");
  return cleaned || "Sprite";
}

function renderSpriteShapeBindRow(target) {
  if (!target) {
    return;
  }
  target.replaceChildren();
  const info = spriteAssetBindInfo(sprite.shapeBind, "shape");
  const row = document.createElement("div");
  row.className = "sprite-shape-bind-row";
  row.classList.toggle("has-unlink", info.linked && info.name);
  const label = document.createElement("span");
  label.className = "sprite-shape-bind-label";
  label.textContent = "Shape";
  const input = document.createElement("input");
  input.type = "text";
  input.className = "sprite-shape-name-input";
  input.value = info.name || "";
  input.placeholder = "";
  input.spellcheck = false;
  input.autocomplete = "off";
  input.setAttribute("aria-label", "Shape name");
  const tagButton = document.createElement("button");
  tagButton.type = "button";
  tagButton.className = "sprite-shape-tag-button sprite-icon-button";
  tagButton.classList.toggle("is-active", info.linked);
  tagButton.innerHTML = spriteTagIconSvg();
  tagButton.setAttribute("aria-pressed", String(info.linked));
  tagButton.setAttribute("aria-haspopup", "listbox");
  tagButton.setAttribute("aria-expanded", String(Boolean(sprite.shapeTagPickerOpen)));
  tagButton.title = info.name ? `Shape tag: ${info.name}` : "Tag shape by name";
  tagButton.setAttribute("aria-label", tagButton.title);
  const commitName = (options = {}) => {
    commitSpriteShapeName(input.value, {
      sync: Boolean(sprite.shapeTagPickerOpen) || spriteAssetBindInfo(sprite.shapeBind, "shape").linked,
      ...options,
    });
  };
  input.addEventListener("change", () => commitName({ reportError: true }));
  input.addEventListener("blur", () => commitName({ reportError: false }));
  input.addEventListener("keydown", (event) => {
    event.stopPropagation();
    if (event.key !== "Enter") {
      return;
    }
    event.preventDefault();
    commitName({ reportError: true });
  });
  tagButton.addEventListener("click", () => {
    const opening = !sprite.shapeTagPickerOpen;
    if (opening) {
      clearSpriteColorEditorState();
      sprite.colorTagPickerOpen = false;
      renderSpritePalette();
    }
    sprite.shapeTagPickerOpen = opening;
    renderSpriteControls();
  });
  row.append(label, input, tagButton);
  if (info.linked && info.name) {
    row.append(renderSpriteShapeUnlinkButton(info));
  }
  if (sprite.shapeTagPickerOpen) {
    const tagPicker = renderSpriteAssetNamePicker({
      className: "sprite-shape-tag-picker",
      names: spriteShapeAssetNames(),
      value: info.name || "",
      placeholder: "",
      ariaLabel: "Shape tag name",
      emptyText: "No named shapes yet",
      onCommit: (name) => {
        const wasOpen = sprite.shapeTagPickerOpen;
        sprite.shapeTagPickerOpen = false;
        const ok = setSpriteShapeSync(true, name);
        if (!ok) {
          sprite.shapeTagPickerOpen = wasOpen;
          return false;
        }
        clearSpriteColorEditorState();
        renderSpriteBuilder();
        return true;
      },
      onCancel: () => {
        sprite.shapeTagPickerOpen = false;
        renderSpriteControls();
      },
    });
    row.append(tagPicker);
    requestAnimationFrame(() => {
      focusSpriteTagPickerInput(tagPicker);
    });
  }
  target.append(row);
}

function renderSpriteShapeUnlinkButton(info) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "sprite-shape-tag-unlink-button sprite-icon-button";
  button.title = info?.name ? `Unlink shape tag ${info.name}` : "Unlink shape tag";
  button.setAttribute("aria-label", button.title);
  button.innerHTML = spriteUnlinkIconSvg();
  button.addEventListener("click", (event) => {
    event.preventDefault();
    event.stopPropagation();
    sprite.shapeTagPickerOpen = false;
    clearSpriteColorEditorState();
    toggleSpriteShapeBinding();
  });
  return button;
}

function commitSpriteShapeName(rawName, options = {}) {
  const name = sanitizeSpriteShapeRef(rawName);
  const info = spriteAssetBindInfo(sprite.shapeBind, "shape");
  if (!name) {
    if (info.name) {
      sprite.shapeBind = null;
      rewriteCurrentSpriteDefinitionFromBuilder("Shape sync off");
      renderSpriteBuilder();
    } else if (options.reportError && options.sync) {
      setSpriteActionStatus("Enter a shape name", "is-error");
    }
    return false;
  }
  sprite.shapeBind = { type: "shape", name, linked: Boolean(options.sync) };
  if (options.sync) {
    return setSpriteShapeSync(true, name);
  }
  syncSpriteSourceActionButtons();
  return true;
}

function setSpriteShapeSync(sync, rawName) {
  const name = sanitizeSpriteShapeRef(rawName || spriteAssetBindInfo(sprite.shapeBind, "shape").name);
  if (!sync) {
    sprite.shapeBind = name ? { type: "shape", name, linked: false } : null;
    renderSpriteBuilder();
    return true;
  }
  if (!name) {
    setSpriteActionStatus("Enter a shape name", "is-error");
    return false;
  }
  const shapes = spriteSourceShapeAssets();
  let status = `Using shape ${name}`;
  if (shapes.has(name)) {
    const parsed = spriteCellsFromAsciiRows(shapes.get(name), sprite.palette.length);
    if (!parsed) {
      setSpriteActionStatus(`Cannot use shape ${name}`, "is-error");
      return false;
    }
    sprite.size = parsed.size;
    sprite.cells = parsed.cells;
  } else {
    status = `Tagged shape ${name}`;
  }
  sprite.shapeBind = { type: "shape", name, linked: true };
  setSpriteActionStatus(status, "is-ok");
  renderSpriteBuilder();
  return true;
}

function spriteCellsFromAsciiRows(rows, paletteLength) {
  if (!Array.isArray(rows) || rows.length === 0) {
    return null;
  }
  const width = Math.max(...rows.map((row) => row.length));
  const height = rows.length;
  const size = clampSpriteSize(Math.max(width, height));
  const cells = Array.from({ length: size * size }, () => null);
  for (let y = 0; y < Math.min(height, size); y += 1) {
    for (let x = 0; x < Math.min(rows[y].length, size); x += 1) {
      const colorIndex = spriteColorIndexForPaletteChar(rows[y][x], paletteLength);
      if (colorIndex === undefined) {
        return null;
      }
      cells[y * size + x] = colorIndex;
    }
  }
  return { size, cells };
}

function loadSpriteSourceTarget(target, options = {}) {
  if (!isPuzzleDocument(activeDocument()) || !isTextDocument(activeDocument())) {
    return null;
  }
  const source = sourceEditorDocumentValue();
  if (!Number.isInteger(target?.bodyStart) || !Number.isInteger(target?.bodyEnd)) {
    return null;
  }
  const targetName = target.name || spriteObjectName();
  const loaded = parseSpriteDefinitionSource(target.sourceSprite, targetName);
  if (!loaded) {
    const contractError = spriteSourceContractError(target.sourceSprite);
    if (contractError) {
      if (!options.silent) {
        setSpriteActionStatus(contractError, "is-error");
        setStatus(contractError, "is-error");
      }
      return null;
    }
    if (isIncompleteSpriteSourceTarget(source, target)) {
      applyIncompleteSpriteSourceTarget(targetName, target);
      if (!options.silent) {
        setSpriteActionStatus(`Loaded unfinished ${spriteNameInput.value || "sprite"}`, "is-ok");
        setStatus(`Loaded unfinished sprite ${spriteNameInput.value || ""}`.trim(), "is-ok");
      }
      return `sprite:${targetName}:${target.start ?? target.bodyStart}`;
    } else if (!options.silent) {
      setSpriteActionStatus("No editable sprite here", "is-error");
    }
    return null;
  }
  if (options.recordHistory && typeof pushSourceNavigationHistory === "function") {
    pushSourceNavigationHistory();
  }
  if (options.switchMode && currentPreviewMode !== "sprite") {
    setPreviewMode("sprite");
  }
  spriteNameInput.value = targetName || "Sprite";
  setSpriteEditSource(target, activeDocument());
  sprite.size = loaded.size;
  sprite.palette = loaded.palette;
  sprite.shapeBind = loaded.shapeBind || null;
  sprite.solidSource = Boolean(loaded.solid);
  sprite.sourcePreludeRows = Array.isArray(loaded.sourcePreludeRows) ? loaded.sourcePreludeRows : [];
  sprite.cells = loaded.cells;
  if (loaded.animationMode) {
    sprite.animationMode = true;
    sprite.animationDurationMs = normalizedSpriteAnimationDuration(loaded.animationDurationMs);
    sprite.animationFrameCount = normalizedSpriteAnimationFrameCount(loaded.animationFrameCount);
    sprite.animationFrameIndex = 0;
    sprite.animationPlaybackIndex = 0;
    sprite.animationPlaying = false;
    sprite.animationFrames = Array.isArray(loaded.animationFrames)
      ? loaded.animationFrames.map((frame) => cloneSpriteCells(frame))
      : [cloneSpriteCells(sprite.cells)];
    ensureSpriteAnimationFrames();
  } else {
    sprite.animationMode = false;
    resetSpriteAnimationFramesFromCurrentCells();
  }
  sprite.selectedColorIndex = sprite.palette.length ? 0 : null;
  sprite.addPaletteOpen = false;
  sprite.editPaletteOpen = false;
  sprite.customColorOpen = false;
  sprite.addDraftColorIndex = null;
  renderSpriteBuilder();
  if (!options.silent) {
    setSpriteActionStatus(`Loaded ${spriteNameInput.value}`, "is-ok");
    setStatus(`Loaded sprite ${spriteNameInput.value}`, "is-ok");
  }
  return `sprite:${targetName}:${target.start ?? target.bodyStart}`;
}

function isIncompleteSpriteSourceTarget(_source, target) {
  const paletteTokens = target?.sourceSprite?.paletteTokens;
  return Array.isArray(paletteTokens) && paletteTokens.length === 0;
}

function applyIncompleteSpriteSourceTarget(name, target) {
  if (target && typeof target === "object") {
    setSpriteEditSource(target, activeDocument());
  }
  spriteNameInput.value = name || "";
  sprite.size = clampSpriteSize(sprite.size);
  sprite.palette = [];
  sprite.shapeBind = null;
  sprite.solidSource = false;
  sprite.sourcePreludeRows = [];
  sprite.animationMode = false;
  sprite.cells = Array.from({ length: sprite.size * sprite.size }, () => null);
  resetSpriteAnimationFramesFromCurrentCells();
  sprite.selectedColorIndex = null;
  sprite.addPaletteOpen = false;
  sprite.editPaletteOpen = false;
  sprite.customColorOpen = false;
  sprite.addDraftColorIndex = null;
  renderSpriteBuilder();
}

function parseSpriteDefinitionSource(contract, selectorName = "") {
  if (!contract || typeof contract !== "object") {
    return null;
  }
  const sourcePreludeRows = Array.isArray(contract.preludeRows)
    ? contract.preludeRows.map((row) => String(row || "").trim()).filter(Boolean)
    : [];
  const paletteTokens = Array.isArray(contract.paletteTokens)
    ? contract.paletteTokens.map((token) => String(token || "").trim()).filter(Boolean)
    : [];
  const asciiRows = Array.isArray(contract.pixelRows)
    ? contract.pixelRows.map((line) => String(line || "").trim()).filter(Boolean)
    : [];
  const animationRows = Array.isArray(contract.animationFrames)
    ? contract.animationFrames
        .map((frame) => (
          Array.isArray(frame)
            ? frame.map((line) => String(line || "").trim()).filter(Boolean)
            : []
        ))
        .filter((frame) => frame.length)
    : [];
  const shapeName = typeof contract.shapeRef === "string" ? contract.shapeRef.trim() : "";
  const resolvedPalette = Array.isArray(contract.resolvedPalette)
    ? contract.resolvedPalette
    : [];
  if (!paletteTokens.length || !resolvedPalette.length) {
    return null;
  }
  let shapeBind = null;
  const palette = resolvedPalette.map((entry, index) => {
    const color = String(entry?.color || "").trim();
    const source = String(entry?.source || paletteTokens[index] || "").trim();
    if (!color) {
      return null;
    }
    const paletteEntry = { color };
    if (entry?.linked && source) {
      paletteEntry.bind = { type: "color", name: source, linked: true };
    }
    return paletteEntry;
  });
  if (!palette.length || palette.some((entry) => !entry)) {
    return null;
  }
  if (shapeName) {
    const shapeRows = Array.isArray(contract.resolvedShapeRows)
      ? contract.resolvedShapeRows.map((row) => String(row || "").trim()).filter(Boolean)
      : [];
    if (!shapeRows.length) {
      return null;
    }
    shapeBind = { type: "shape", name: shapeName, linked: true };
    asciiRows.splice(0, asciiRows.length, ...shapeRows);
  }
  if (animationRows.length >= 2 && !shapeBind) {
    const parsedFrames = animationRows.map((frame) => spriteCellsFromAsciiRows(frame, palette.length));
    if (parsedFrames.some((frame) => !frame)) {
      return null;
    }
    const size = parsedFrames[0].size;
    if (parsedFrames.some((frame) => frame.size !== size)) {
      return null;
    }
    const frameDurationMs = Number.isFinite(Number(contract.frameDurationMs))
      ? Number(contract.frameDurationMs)
      : null;
    const durationMs = Number.isFinite(Number(contract.durationMs))
      ? normalizedSpriteAnimationDuration(contract.durationMs)
      : normalizedSpriteAnimationDuration(frameDurationMs === null ? undefined : frameDurationMs * parsedFrames.length);
    return {
      size,
      palette,
      shapeBind: null,
      sourcePreludeRows,
      animationMode: true,
      animationDurationMs: durationMs,
      animationFrameCount: parsedFrames.length,
      animationFrames: parsedFrames.map((frame) => frame.cells),
      cells: parsedFrames[0].cells,
    };
  }
  if (asciiRows.length === 0) {
    if (palette.length !== 1) {
      return null;
    }
    return {
      size: 1,
      palette,
      shapeBind: null,
      solid: true,
      sourcePreludeRows,
      cells: [0],
    };
  }
  const width = Math.max(...asciiRows.map((row) => row.length));
  const height = asciiRows.length;
  const size = clampSpriteSize(Math.max(width, height));
  const cells = Array.from({ length: size * size }, () => null);
  for (let y = 0; y < Math.min(height, size); y += 1) {
    for (let x = 0; x < Math.min(asciiRows[y].length, size); x += 1) {
      const colorIndex = spriteColorIndexForPaletteChar(asciiRows[y][x], palette.length);
      if (colorIndex === undefined) {
        return null;
      }
      cells[y * size + x] = colorIndex;
    }
  }
  return {
    size,
    palette,
    shapeBind,
    sourcePreludeRows,
    cells,
  };
}

function spriteSourceContractError(contract) {
  if (!contract || typeof contract !== "object") {
    return "";
  }
  const shapeName = typeof contract.shapeRef === "string" ? contract.shapeRef.trim() : "";
  const shapeRows = Array.isArray(contract.resolvedShapeRows)
    ? contract.resolvedShapeRows.map((row) => String(row || "").trim()).filter(Boolean)
    : [];
  if (shapeName && !shapeRows.length) {
    return `Cannot resolve shape ${shapeName}`;
  }
  return "";
}

function spritePaletteEntrySourceToken(entry) {
  const bind = spritePaletteEntryBindInfo(entry);
  if (bind.linked && bind.name) {
    return bind.name;
  }
  return normalizeSpriteColor(entry.color);
}

function collectSpriteUnbracedShapeDefinitions(source, shapesBlock, callback) {
  const body = source.slice(shapesBlock.bodyStart, shapesBlock.bodyEnd);
  const lines = spriteSourceBlockLines(body, shapesBlock.bodyStart);
  let index = 0;
  while (index < lines.length) {
    const line = lines[index];
    const bodyLineStart = line.start - shapesBlock.bodyStart;
    const name = stripSpriteAssetComment(line.text).trim();
    const nameMatch = /^([A-Za-z_][A-Za-z0-9_+*()/-]*)$/.exec(name);
    if (!nameMatch || topLevelDepthAt(body, bodyLineStart) !== 0) {
      index += 1;
      continue;
    }

    const rows = [];
    let rowIndex = index + 1;
    let bodyStart = null;
    let bodyEnd = null;
    while (rowIndex < lines.length) {
      const rowLine = lines[rowIndex];
      const rowLineStart = rowLine.start - shapesBlock.bodyStart;
      const row = stripSpriteAssetComment(rowLine.text).trim();
      if (
        !row
        || topLevelDepthAt(body, rowLineStart) !== 0
        || spriteUnbracedShapeRowIsBoundary(lines, rowIndex, rows, body, shapesBlock)
      ) {
        break;
      }
      if (bodyStart === null) {
        bodyStart = rowLine.start;
      }
      rows.push(row);
      bodyEnd = rowLine.start + rowLine.text.length;
      rowIndex += 1;
    }
    if (rows.length) {
      callback(nameMatch[1], rows, {
        indent: /^\s*/.exec(line.text)?.[0] || spriteSourceChildIndent(shapesBlock.indent),
        braced: false,
        tableRow: false,
        bodyStart,
        bodyEnd,
      });
      index = rowIndex;
    } else {
      index += 1;
    }
  }
}

function spriteUnbracedShapeRowIsBoundary(lines, rowIndex, rows, body, shapesBlock) {
  const row = stripSpriteAssetComment(lines[rowIndex]?.text || "").trim();
  if (!row) {
    return true;
  }
  if (row.includes("{") || row.includes("}")) {
    return true;
  }
  if (!rows.length) {
    return false;
  }
  const width = spriteAsciiRowWidth(rows[0]);
  const rowWidth = spriteAsciiRowWidth(row);
  if (!/^([A-Za-z_][A-Za-z0-9_+*()/-]*)$/.test(row)) {
    return false;
  }
  if (rowWidth !== width) {
    return true;
  }
  const next = spriteNextTopLevelShapeLine(lines, rowIndex + 1, body, shapesBlock);
  if (!next) {
    return false;
  }
  return spriteAsciiRowWidth(next) !== width;
}

function spriteNextTopLevelShapeLine(lines, startIndex, body, shapesBlock) {
  for (let index = startIndex; index < lines.length; index += 1) {
    const line = lines[index];
    const lineStart = line.start - shapesBlock.bodyStart;
    const text = stripSpriteAssetComment(line.text).trim();
    if (!text) {
      continue;
    }
    if (topLevelDepthAt(body, lineStart) !== 0) {
      return "";
    }
    return text;
  }
  return "";
}

function spriteAsciiRowWidth(row) {
  return Array.from(String(row || "")).length;
}

function spriteSourceBlockLines(text, absoluteStart) {
  const lines = [];
  let start = 0;
  while (start <= text.length) {
    const newline = text.indexOf("\n", start);
    const end = newline < 0 ? text.length : newline;
    const lineEnd = end > start && text[end - 1] === "\r" ? end - 1 : end;
    lines.push({
      text: text.slice(start, lineEnd),
      start: absoluteStart + start,
      end: absoluteStart + lineEnd,
    });
    if (newline < 0) {
      break;
    }
    start = newline + 1;
  }
  return lines;
}

function spriteSelectorSingleTagValue(selectorName, bindingName = "") {
  const parts = String(selectorName || "").split(":").filter(Boolean);
  if (parts.length !== 2) {
    return "";
  }
  const value = parts[1];
  return value && value !== bindingName ? value : "";
}

function collectSpriteFlatAssetRows(source, block, callback) {
  const body = source.slice(block.bodyStart, block.bodyEnd);
  const pattern = /(^|\n)([\t ]*)([A-Za-z_][\w]*)\s*=\s*([^\n{}]+)/g;
  let match = null;
  while ((match = pattern.exec(body))) {
    const bodyMatchStart = match.index + match[1].length;
    if (topLevelDepthAt(body, bodyMatchStart) !== 0) {
      continue;
    }
    callback(match[3], stripSpriteAssetComment(match[4]).trim(), {
      lineStart: block.bodyStart + bodyMatchStart,
      valueStart: block.bodyStart + match.index + match[0].lastIndexOf(match[4]),
      valueEnd: block.bodyStart + match.index + match[0].length,
    });
  }
}

function collectSpriteAssetTables(source, block, callback) {
  const body = source.slice(block.bodyStart, block.bodyEnd);
  const pattern = /(^|\n)([\t ]*)([A-Za-z_][\w]*)(?::[A-Za-z_][\w]*)?\s*\{/g;
  let match = null;
  while ((match = pattern.exec(body))) {
    const bodyMatchStart = match.index + match[1].length;
    if (topLevelDepthAt(body, bodyMatchStart) !== 0) {
      continue;
    }
    const openIndex = source.indexOf("{", block.bodyStart + bodyMatchStart);
    const closeIndex = findMatchingBrace(source, openIndex);
    if (openIndex < 0 || closeIndex < 0 || closeIndex > block.bodyEnd) {
      continue;
    }
    const tableBlock = {
      bodyStart: openIndex + 1,
      bodyEnd: closeIndex,
    };
    collectSpriteFlatAssetRows(source, tableBlock, (rowName, value, range) => {
      callback(match[3], rowName, value, range);
    });
  }
}

function stripSpriteAssetComment(value) {
  return String(value || "").replace(/\s+\/\/.*$/, "");
}

function spriteColorIndexForPaletteChar(char, paletteLength) {
  if (char === ".") {
    return null;
  }
  const index = SPRITE_COLOR_TOKENS.indexOf(char);
  return index >= 0 && index < paletteLength ? index : undefined;
}

async function exportSpriteAscii() {
  const text = spriteClipboardText();
  try {
    window.focus();
    spriteExportButton.focus({ preventScroll: true });
    await copyTextToClipboard(text);
    setSpriteActionStatus("Copied", "is-ok");
    setStatus("Copied sprite", "is-ok");
  } catch (error) {
    setSpriteActionStatus("Copy failed", "is-error");
    setStatus(`Could not copy sprite: ${error?.message || error}`, "is-error");
  }
}

function addSpriteToSource() {
  const document = activeSpriteEditDocument();
  if (!document || !isTextDocument(document)) {
    setSpriteActionStatus("No puzzle source", "is-error");
    setStatus("No puzzle source for sprite", "is-error");
    return;
  }

  const inserted = insertSpriteDefinition(activeSpriteEditSource());
  const source = sourceWithStagedSpriteAssetDefinitions(inserted.source);
  if (!source) {
    return;
  }
  const result = { source, start: inserted.start };
  document.source = result.source;
  if (document.id === activeDocument()?.id) {
    setSourceEditorValue(result.source, { resetUndo: false });
    revealSpriteSourceResult(document, result);
  }
  scheduleLocalSave();
  schedulePreview();
  setSpriteEditSource({ start: result.start, name: spriteObjectName() }, document);
  sourceEditor.focus({ preventScroll: true });
  setSpriteActionStatus("Added sprite", "is-ok");
  setStatus("Added sprite", "is-ok");
}

async function updateSpriteInSource() {
  const document = activeSpriteEditDocument();
  if (!document || !isTextDocument(document)) {
    setSpriteActionStatus("No puzzle source", "is-error");
    setStatus("No puzzle source for sprite", "is-error");
    return;
  }

  const stagedSource = sourceWithStagedSpriteAssetDefinitions(activeSpriteEditSource());
  if (!stagedSource) {
    return;
  }
  const result = await replaceCurrentSpriteDefinitionFromParser(stagedSource);
  if (!result) {
    setSpriteActionStatus("No selected sprite source range", "is-error");
    setStatus("No selected sprite source range", "is-error");
    return;
  }
  document.source = result.source;
  if (document.id === activeDocument()?.id) {
    setSourceEditorValue(result.source, { resetUndo: false });
    revealSpriteSourceResult(document, result);
  }
  scheduleLocalSave();
  schedulePreview();
  setSpriteEditSource({ ...result.target, start: result.start, end: result.end, name: spriteObjectName() }, document);
  sourceEditor.focus({ preventScroll: true });
  setSpriteActionStatus("Updated sprite", "is-ok");
  setStatus("Updated sprite", "is-ok");
}

function duplicateSpriteInSource() {
  const document = activeSpriteEditDocument();
  if (!document || !isTextDocument(document)) {
    setSpriteActionStatus("No puzzle source", "is-error");
    setStatus("No puzzle source for sprite", "is-error");
    return;
  }

  const result = duplicateCurrentSpriteDefinition(activeSpriteEditSource());
  if (!result) {
    setSpriteActionStatus("No selected sprite source range", "is-error");
    setStatus("No selected sprite source range", "is-error");
    return;
  }
  document.source = result.source;
  if (document.id === activeDocument()?.id) {
    setSourceEditorValue(result.source, { resetUndo: false });
    revealSpriteSourceResult(document, result);
  }
  scheduleLocalSave();
  schedulePreview();
  spriteNameInput.value = result.name;
  setSpriteEditSource({ start: result.start, end: result.end, name: result.name }, document);
  syncSpriteSourceActionButtons();
  sourceEditor.focus({ preventScroll: true });
  setSpriteActionStatus("Duplicated sprite", "is-ok");
  setStatus("Duplicated sprite", "is-ok");
}

function sourceWithStagedSpriteAssetDefinitions(source) {
  let nextSource = source;
  for (const entry of sprite.palette) {
    const bind = spritePaletteEntryBindInfo(entry);
    if (!bind.linked || !bind.name || findSpriteColorDefinitionRange(nextSource, bind.name)) {
      continue;
    }
    const withColor = ensureSpriteColorDefinition(nextSource, bind.name, normalizeSpriteColor(entry.color));
    if (!withColor) {
      return null;
    }
    nextSource = withColor;
  }

  const shape = spriteAssetBindInfo(sprite.shapeBind, "shape");
  if (shape.linked && shape.name && !findSpriteShapeDefinitionRange(nextSource, shape.name)) {
    const withShape = ensureSpriteShapeDefinition(nextSource, shape.name, spriteAscii().split("\n"));
    if (!withShape) {
      return null;
    }
    nextSource = withShape;
  }
  return nextSource;
}

function clearSpriteBuilder() {
  const before = visualEditSnapshot("sprite");
  if (sprite.animationMode) {
    ensureSpriteAnimationFrames();
    sprite.cells = Array.from({ length: sprite.size * sprite.size }, () => null);
    sprite.animationFrames[sprite.animationFrameIndex] = sprite.cells;
    updateSpriteBoundShapeDefinition();
    renderSpriteBuilder();
    setSpriteActionStatus(`Cleared frame ${sprite.animationFrameIndex + 1}`, "is-ok");
    pushVisualEditUndoSnapshot("sprite", before);
    return;
  }
  resetSpriteBuilder(sprite.size);
  setSpriteActionStatus("Cleared", "is-ok");
  pushVisualEditUndoSnapshot("sprite", before);
}

function setSpriteActionStatus(text, className = "") {
  if (!spriteActionStatus) {
    return;
  }
  window.clearTimeout(spriteActionClearTimer);
  spriteActionStatus.className = `sprite-action-status tool-feedback-bar ${className}`.trim();
  spriteActionStatus.textContent = text;
  setPaneStatus("sprite", text, className);
  if (text && className === "is-ok") {
    spriteActionClearTimer = window.setTimeout(() => {
      if (spriteActionStatus.textContent === text && spriteActionStatus.classList.contains("is-ok")) {
        spriteActionStatus.className = "sprite-action-status tool-feedback-bar";
        spriteActionStatus.textContent = "";
      }
    }, 1800);
  }
}

function clearSpriteActionError() {
  if (!spriteActionStatus?.classList.contains("is-error")) {
    return;
  }
  setSpriteActionStatus("");
}

function insertSpriteDefinition(source) {
  const block = findSpritesBlock(source);
  if (!block) {
    const puzzleBlock = findPuzzleBlock(source);
    if (puzzleBlock) {
      const blockIndent = spriteSourceChildIndent(puzzleBlock.indent);
      const insertStart = source.slice(0, puzzleBlock.bodyEnd).trimEnd().length + 2 + blockIndent.length + "sprites {\n".length;
      return {
        source: `${source.slice(0, puzzleBlock.bodyEnd).trimEnd()}\n\n${blockIndent}sprites {\n${spriteObjectDefinitionText(spriteSourceChildIndent(blockIndent))}\n${blockIndent}}\n${source.slice(puzzleBlock.bodyEnd)}`,
        start: insertStart,
        updated: false,
      };
    }

    const prefix = source.trimEnd() ? `${source.trimEnd()}\n\n` : "";
    return {
      source: `${prefix}sprites {\n${spriteObjectDefinitionText(SPRITE_SOURCE_INDENT)}\n}\n`,
      start: `${prefix}sprites {\n`.length,
      updated: false,
    };
  }

  const indent = spriteSourceChildIndent(block.indent);
  const before = source.slice(0, block.bodyEnd).trimEnd();
  return {
    source: `${before}\n\n${spriteObjectDefinitionText(indent)}\n${source.slice(block.bodyEnd)}`,
    start: before.length + 2,
  };
}

function spriteSourceCursorPosition(source, document = activeDocument()) {
  if (document?.id === activeDocument()?.id && sourceEditor) {
    return Math.max(
      0,
      Math.min(String(source || "").length, Math.max(sourceEditor.selectionStart || 0, sourceEditor.selectionEnd || 0)),
    );
  }
  return String(source || "").length;
}

async function spriteSourceTargetAtCursor(source, cursor) {
  if (typeof resolveSourceTargetFromWasm !== "function") {
    return null;
  }
  return resolveSourceTargetFromWasm(source, cursor);
}

async function spriteSourceTargetByName(source, name) {
  const targetName = String(name || "").trim();
  if (!targetName) {
    return null;
  }
  const text = String(source || "");
  const candidates = [];
  let index = text.indexOf(targetName);
  while (index >= 0) {
    candidates.push(index);
    index = text.indexOf(targetName, index + Math.max(1, targetName.length));
  }
  const unique = new Map();
  for (const candidate of candidates) {
    const target = await spriteSourceTargetAtCursor(text, candidate);
    if (target?.kind !== "sprite" || target.name !== targetName) {
      continue;
    }
    const key = `${target.start}:${target.end}`;
    if (!unique.has(key)) {
      unique.set(key, target);
    }
  }
  return unique.size === 1 ? [...unique.values()][0] : null;
}

async function replaceCurrentSpriteDefinitionFromParser(source, name = sprite.editSourceName || spriteObjectName()) {
  const target = await spriteSourceTargetByName(source, name);
  if (!target || !Number.isInteger(target.start) || !Number.isInteger(target.end)) {
    return null;
  }
  const range = {
    ...target,
    indent: spriteSourceIndent(source.slice(source.lastIndexOf("\n", target.start - 1) + 1, target.start)),
  };
  const replaced = replaceSpriteDefinition(source, range);
  return replaced ? { ...replaced, target } : null;
}

function spriteSourceInsertionLineEnd(source, position) {
  const text = String(source || "");
  const safePosition = Math.max(0, Math.min(text.length, Math.trunc(Number(position) || 0)));
  const newline = text.indexOf("\n", safePosition);
  return newline < 0 ? text.length : newline + 1;
}

function insertSpriteSourceTextAt(source, position, text, innerOffset = 0) {
  const original = String(source || "");
  const safePosition = Math.max(0, Math.min(original.length, Math.trunc(Number(position) || 0)));
  const before = original.slice(0, safePosition).trimEnd();
  const after = original.slice(safePosition).replace(/^[\t ]*\n?/, "");
  const snippet = String(text || "").trimEnd();
  let next = before;
  if (next) {
    next += "\n\n";
  }
  const start = next.length + Math.max(0, Math.min(snippet.length, innerOffset));
  next += snippet;
  const end = next.length;
  if (after) {
    next += `${snippet ? "\n\n" : "\n"}${after}`;
  } else {
    next += "\n";
  }
  return { source: next, start, end };
}

function replaceSpriteDefinition(source, sourceRange = currentSpriteEditSourceRange(source)) {
  const entry = sourceRange;
  if (!entry) {
    return null;
  }
  const replacement = spriteObjectDefinitionText(spriteSourceIndent(entry.indent));
  return {
    source: replaceEditorSourceRangePreservingLineBoundary(source, entry.start, entry.end, replacement),
    start: entry.start,
    end: entry.start + replacement.length,
  };
}

function duplicateCurrentSpriteDefinition(source) {
  const entry = currentSpriteEditSourceRange(source);
  const block = findSpritesBlock(source);
  if (!entry || !block || entry.start < block.bodyStart || entry.start > block.bodyEnd) {
    return null;
  }
  const originalName = spriteObjectName();
  if (!originalName) {
    return null;
  }
  const name = uniqueSpriteDuplicateName(source, originalName);
  if (!name) {
    return null;
  }
  const duplicateText = spriteObjectDefinitionText(spriteSourceIndent(entry.indent), name);
  const inserted = insertSpriteSourceTextAt(
    source,
    spriteSourceInsertionLineEnd(source, entry.end),
    duplicateText,
    0,
  );
  return {
    source: inserted.source,
    start: inserted.start,
    end: inserted.end,
    name,
  };
}

function uniqueSpriteDuplicateName(source, originalName) {
  const base = String(originalName || "Sprite").trim().replace(/_copy(?:_\d+)?$/, "") || "Sprite";
  const existing = spriteSourceDefinitionNames(source);
  for (let index = 1; index <= 10000; index += 1) {
    const suffix = index === 1 ? "_copy" : `_copy_${index}`;
    const candidate = `${base}${suffix}`;
    if (!existing.has(candidate)) {
      return candidate;
    }
  }
  return "";
}

function spriteSourceDefinitionNames(source) {
  const names = new Set();
  for (const entry of surfaceEntriesForSource(source).filter((entry) => entry.kind === "sprite")) {
    names.add(entry.name);
  }
  return names;
}

function canReplaceCurrentSpriteDefinition(source) {
  return Boolean(currentSpriteEditSourceRange(source));
}

function syncSpriteSourceActionButtons() {
  const hasEditableSource = canReplaceCurrentSpriteDefinition(activeSpriteEditSource());
  if (spriteUpdateButton) {
    spriteUpdateButton.disabled = !hasEditableSource;
  }
  if (duplicateSpriteButton) {
    duplicateSpriteButton.disabled = !hasEditableSource;
  }
}

function currentSpriteEditSourceRange(source) {
  const start = sprite.editSourceStart;
  const end = sprite.editSourceEnd;
  if (
    !Number.isInteger(start)
    || !Number.isInteger(end)
    || start < 0
    || end < start
    || end > String(source || "").length
  ) {
    return null;
  }
  return {
    start,
    end,
    indent: spriteSourceIndent(source.slice(source.lastIndexOf("\n", start - 1) + 1, start)),
  };
}

function revealSpriteSourceResult(document, result) {
  if (!Number.isInteger(result?.start) || typeof revealSourceLocation !== "function") {
    return;
  }
  revealSourceLocation({
    document,
    start: result.start,
    end: Number.isInteger(result.end) ? result.end : result.start,
  }, {
    recordHistory: false,
    revealPane: false,
  });
}

function findSpritesBlock(source) {
  const pattern = /(^|\n)([\t ]*)sprites(?:\s+[^\n{]+)?\s*\{/m;
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
    start,
    openIndex,
    closeIndex,
    indent: match[2] || "",
    bodyStart: openIndex + 1,
    bodyEnd: closeIndex,
  };
}

function findVisualAssetBlock(source, spritesBlock, name) {
  const body = source.slice(spritesBlock.bodyStart, spritesBlock.bodyEnd);
  const pattern = new RegExp(`(^|\\n)([\\t ]*)${escapeRegExp(name)}\\s*\\{`, "g");
  let match = null;
  while ((match = pattern.exec(body))) {
    const bodyMatchStart = match.index + match[1].length;
    if (topLevelDepthAt(body, bodyMatchStart) !== 0) {
      continue;
    }
    const start = spritesBlock.bodyStart + bodyMatchStart;
    const openIndex = source.indexOf("{", start);
    const closeIndex = findMatchingBrace(source, openIndex);
    if (openIndex < 0 || closeIndex < 0 || closeIndex > spritesBlock.bodyEnd) {
      continue;
    }
    return {
      start,
      openIndex,
      closeIndex,
      indent: match[2] || "",
      bodyStart: openIndex + 1,
      bodyEnd: closeIndex,
    };
  }
  return null;
}

function escapeRegExp(value) {
  return String(value).replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function ensureSpriteColorDefinition(source, name, color) {
  const spritesBlock = findSpritesBlock(source);
  if (!spritesBlock) {
    setSpriteActionStatus("No sprites block", "is-error");
    return null;
  }
  if (findSpriteColorDefinitionRange(source, name)) {
    setSpriteActionStatus(`${name} already exists`, "is-error");
    return null;
  }
  const normalized = normalizeSpriteColor(color);
  const paletteBlock = findVisualAssetBlock(source, spritesBlock, "palette");
  if (paletteBlock) {
    const rowIndent = spriteSourceChildIndent(paletteBlock.indent);
    return `${source.slice(0, paletteBlock.bodyEnd).trimEnd()}\n${rowIndent}${name} = ${normalized}\n${source.slice(paletteBlock.bodyEnd)}`;
  }
  const blockIndent = spriteSourceChildIndent(spritesBlock.indent);
  const rowIndent = spriteSourceChildIndent(blockIndent);
  const paletteText = `\n${blockIndent}palette {\n${rowIndent}${name} = ${normalized}\n${blockIndent}}\n`;
  return `${source.slice(0, spritesBlock.bodyStart)}${paletteText}${source.slice(spritesBlock.bodyStart)}`;
}

function replaceSpriteColorDefinition(source, name, color) {
  const range = findSpriteColorDefinitionRange(source, name);
  if (!range) {
    return null;
  }
  return `${source.slice(0, range.valueStart)} ${normalizeSpriteColor(color)}${source.slice(range.valueEnd)}`;
}

function ensureSpriteShapeDefinition(source, name, rows) {
  const shapeRows = spriteShapeDefinitionRows(rows);
  if (!shapeRows) {
    setSpriteActionStatus("Draw shape pixels before registering shape", "is-error");
    return null;
  }
  const spritesBlock = findSpritesBlock(source);
  if (!spritesBlock) {
    setSpriteActionStatus("No sprites block", "is-error");
    return null;
  }
  if (findSpriteShapeDefinitionRange(source, name)) {
    setSpriteActionStatus(`${name} already exists`, "is-error");
    return null;
  }
  const shapesBlock = findVisualAssetBlock(source, spritesBlock, "shapes");
  if (shapesBlock) {
    const indent = spriteSourceChildIndent(shapesBlock.indent);
    const bodyHasContent = source.slice(shapesBlock.bodyStart, shapesBlock.bodyEnd).trim().length > 0;
    const text = `${bodyHasContent ? "\n\n" : "\n"}${spritePlainShapeDefinitionText(indent, name, shapeRows)}\n`;
    return `${source.slice(0, shapesBlock.bodyEnd).trimEnd()}${text}${source.slice(shapesBlock.bodyEnd)}`;
  }
  const blockIndent = spriteSourceChildIndent(spritesBlock.indent);
  const shapeIndent = spriteSourceChildIndent(blockIndent);
  const text = `\n${blockIndent}shapes {\n${spritePlainShapeDefinitionText(shapeIndent, name, shapeRows)}\n${blockIndent}}\n`;
  return `${source.slice(0, spritesBlock.bodyStart)}${text}${source.slice(spritesBlock.bodyStart)}`;
}

function replaceSpriteShapeDefinition(source, name, rows) {
  const shapeRows = spriteShapeDefinitionRows(rows);
  if (!shapeRows) {
    setSpriteActionStatus("Draw shape pixels before updating shape", "is-error");
    return null;
  }
  const range = findSpriteShapeDefinitionRange(source, name);
  if (!range) {
    return null;
  }
  if (range.braced && !range.tableRow) {
    const replacement = spritePlainShapeDefinitionText(range.indent, name, shapeRows);
    const boundary = spritePlainShapeDefinitionTrailingBoundary(source, range.declarationEnd);
    return `${source.slice(0, range.declarationStart)}${replacement}${boundary}${source.slice(range.declarationEnd)}`;
  }
  if (!range.braced) {
    const indent = spriteSourceChildIndent(range.indent);
    const body = shapeRows.map((row) => `${indent}${row}`).join("\n");
    return `${source.slice(0, range.bodyStart)}${body}${source.slice(range.bodyEnd)}`;
  }
  const rangeIndent = spriteSourceIndent(range.indent);
  const indent = spriteSourceChildIndent(rangeIndent);
  const body = `\n${shapeRows.map((row) => `${indent}${row}`).join("\n")}\n${rangeIndent}`;
  return `${source.slice(0, range.bodyStart)}${body}${source.slice(range.bodyEnd)}`;
}

function spritePlainShapeDefinitionText(indent, name, rows) {
  const rowIndent = spriteSourceChildIndent(indent);
  return `${indent}${name}\n${rows.map((row) => `${rowIndent}${row}`).join("\n")}`;
}

function spritePlainShapeDefinitionTrailingBoundary(source, position) {
  const suffix = String(source || "").slice(position);
  if (!suffix) {
    return "";
  }
  if (/^\r?\n[\t ]*\r?\n/.test(suffix) || /^\r?\n[\t ]*\}/.test(suffix)) {
    return "";
  }
  return "\n";
}

function spriteShapeDefinitionRows(rows) {
  const normalized = Array.isArray(rows)
    ? rows.map((row) => String(row || "").trim()).filter(Boolean)
    : [];
  if (!normalized.length || !normalized.some((row) => /[0-9A-Za-z]/.test(row))) {
    return null;
  }
  return normalized;
}

function findSpriteShapeDefinitionRange(source, name) {
  const spritesBlock = findSpritesBlock(source);
  const shapesBlock = spritesBlock ? findVisualAssetBlock(source, spritesBlock, "shapes") : null;
  if (!shapesBlock) {
    return null;
  }
  const body = source.slice(shapesBlock.bodyStart, shapesBlock.bodyEnd);
  const pattern = new RegExp(`(^|\\n)([\\t ]*)${escapeRegExp(name)}\\s*\\{`, "g");
  let match = null;
  while ((match = pattern.exec(body))) {
    const bodyMatchStart = match.index + match[1].length;
    if (topLevelDepthAt(body, bodyMatchStart) !== 0) {
      continue;
    }
    const openIndex = source.indexOf("{", shapesBlock.bodyStart + bodyMatchStart);
    const closeIndex = findMatchingBrace(source, openIndex);
    if (openIndex < 0 || closeIndex < 0 || closeIndex > shapesBlock.bodyEnd) {
      continue;
    }
    return {
      name,
      indent: match[2] || spriteSourceChildIndent(shapesBlock.indent),
      braced: true,
      tableRow: false,
      declarationStart: shapesBlock.bodyStart + bodyMatchStart,
      declarationEnd: closeIndex + 1,
      bodyStart: openIndex + 1,
      bodyEnd: closeIndex,
    };
  }
  let unbracedRange = null;
  collectSpriteUnbracedShapeDefinitions(source, shapesBlock, (shapeName, _rows, range) => {
    if (!unbracedRange && shapeName === name) {
      unbracedRange = range;
    }
  });
  if (unbracedRange) {
    return unbracedRange;
  }
  const tableSeparator = name.indexOf(":");
  if (tableSeparator > 0) {
    const tableName = name.slice(0, tableSeparator);
    const value = spriteSelectorSingleTagValue(spriteObjectName(), name.slice(tableSeparator + 1));
    return findSpriteShapeTableRowRange(source, shapesBlock, tableName, value);
  }
  return null;
}

function findSpriteShapeTableRowRange(source, shapesBlock, tableName, value = "") {
  const body = source.slice(shapesBlock.bodyStart, shapesBlock.bodyEnd);
  const tablePattern = new RegExp(`(^|\\n)([\\t ]*)${escapeRegExp(tableName)}:[A-Za-z_][\\w]*\\s*\\{`, "g");
  let tableMatch = null;
  while ((tableMatch = tablePattern.exec(body))) {
    const tableMatchStart = tableMatch.index + tableMatch[1].length;
    if (topLevelDepthAt(body, tableMatchStart) !== 0) {
      continue;
    }
    const tableOpenIndex = source.indexOf("{", shapesBlock.bodyStart + tableMatchStart);
    const tableCloseIndex = findMatchingBrace(source, tableOpenIndex);
    if (tableOpenIndex < 0 || tableCloseIndex < 0 || tableCloseIndex > shapesBlock.bodyEnd) {
      continue;
    }
    const tableBodyStart = tableOpenIndex + 1;
    const tableBody = source.slice(tableBodyStart, tableCloseIndex);
    const rowPattern = /(^|\n)([\t ]*)([A-Za-z_][\w]*)\s*\{/g;
    let firstRange = null;
    let rowMatch = null;
    while ((rowMatch = rowPattern.exec(tableBody))) {
      const rowMatchStart = rowMatch.index + rowMatch[1].length;
      if (topLevelDepthAt(tableBody, rowMatchStart) !== 0) {
        continue;
      }
      const openIndex = source.indexOf("{", tableBodyStart + rowMatchStart);
      const closeIndex = findMatchingBrace(source, openIndex);
      if (openIndex < 0 || closeIndex < 0 || closeIndex > tableCloseIndex) {
        continue;
      }
      const range = {
        indent: rowMatch[2] || spriteSourceChildIndent(tableMatch[2]),
        braced: true,
        tableRow: true,
        bodyStart: openIndex + 1,
        bodyEnd: closeIndex,
      };
      if (!firstRange) {
        firstRange = range;
      }
      if (value && rowMatch[3] === value) {
        return range;
      }
    }
    return firstRange;
  }
  return null;
}

function findSpriteColorDefinitionRange(source, name) {
  const spritesBlock = findSpritesBlock(source);
  const paletteBlock = spritesBlock ? findVisualAssetBlock(source, spritesBlock, "palette") : null;
  if (!paletteBlock) {
    return null;
  }
  const tableSeparator = name.indexOf(":");
  if (tableSeparator > 0) {
    return findSpriteColorTableRowRange(source, paletteBlock, name.slice(0, tableSeparator), name.slice(tableSeparator + 1));
  }
  return findSpriteFlatAssetRowRange(source, paletteBlock, name);
}

function findSpriteColorTableRowRange(source, paletteBlock, tableName, rowName) {
  const body = source.slice(paletteBlock.bodyStart, paletteBlock.bodyEnd);
  const pattern = new RegExp(`(^|\\n)([\\t ]*)${escapeRegExp(tableName)}(?::[A-Za-z_][\\w]*)?\\s*\\{`, "g");
  let match = null;
  while ((match = pattern.exec(body))) {
    const bodyMatchStart = match.index + match[1].length;
    if (topLevelDepthAt(body, bodyMatchStart) !== 0) {
      continue;
    }
    const openIndex = source.indexOf("{", paletteBlock.bodyStart + bodyMatchStart);
    const closeIndex = findMatchingBrace(source, openIndex);
    if (openIndex < 0 || closeIndex < 0 || closeIndex > paletteBlock.bodyEnd) {
      continue;
    }
    const rowRange = findSpriteFlatAssetRowRange(source, { bodyStart: openIndex + 1, bodyEnd: closeIndex }, rowName);
    if (rowRange) {
      return rowRange;
    }
  }
  return null;
}

function findSpriteFlatAssetRowRange(source, block, name) {
  const body = source.slice(block.bodyStart, block.bodyEnd);
  const pattern = new RegExp(`(^|\\n)([\\t ]*)${escapeRegExp(name)}\\s*=\\s*([^\\n{}]+)`, "g");
  let match = null;
  while ((match = pattern.exec(body))) {
    const bodyMatchStart = match.index + match[1].length;
    if (topLevelDepthAt(body, bodyMatchStart) !== 0) {
      continue;
    }
    const lineStart = block.bodyStart + bodyMatchStart;
    const lineEndIndex = source.indexOf("\n", lineStart);
    const lineEnd = lineEndIndex < 0 || lineEndIndex > block.bodyEnd ? block.bodyEnd : lineEndIndex;
    const equalsIndex = source.indexOf("=", lineStart);
    if (equalsIndex < 0 || equalsIndex > lineEnd) {
      continue;
    }
    return {
      lineStart,
      lineEnd,
      valueStart: equalsIndex + 1,
      valueEnd: lineEnd,
    };
  }
  return null;
}

function findPuzzleBlock(source) {
  const pattern = /(^|\n)([\t ]*)puzzle(?:\s+[^\s{]+)?\s*\{/m;
  const match = pattern.exec(source);
  if (!match) {
    return null;
  }
  const openIndex = source.indexOf("{", match.index + match[0].lastIndexOf("puzzle"));
  const closeIndex = findMatchingBrace(source, openIndex);
  if (closeIndex < 0) {
    return null;
  }
  return {
    indent: match[2] || "",
    bodyStart: openIndex + 1,
    bodyEnd: closeIndex,
  };
}

const SPRITE_SOURCE_INDENT = "";

function spriteSourceIndent(indent = "") {
  return String(indent || "").replace(/\t/g, SPRITE_SOURCE_INDENT);
}

function spriteSourceChildIndent(indent = "") {
  return `${spriteSourceIndent(indent)}${SPRITE_SOURCE_INDENT}`;
}

function spriteObjectDefinitionText(indent, name = spriteObjectName()) {
  const normalizedIndent = spriteSourceIndent(indent);
  const rowIndent = spriteSourceChildIndent(normalizedIndent);
  const shapeInfo = spriteAssetBindInfo(sprite.shapeBind, "shape");
  const colorRow = spritePaletteSourceTokens().join(" ");
  const solidRow = sprite.solidSource ? spriteSolidDefinitionRow(shapeInfo) : null;
  const animationSource = spriteAnimationSourceFrames();
  const preludeRows = spriteSourcePreludeRows({ omitDuration: Boolean(animationSource) }).map((row) => `${rowIndent}${row}`);
  if (solidRow) {
    return [
      `${normalizedIndent}${name}`,
      ...preludeRows,
      `${rowIndent}${solidRow}`,
    ].join("\n");
  }
  const lines = [
    `${normalizedIndent}${name}`,
    ...preludeRows,
    `${rowIndent}${colorRow}`,
  ];
  if (shapeInfo.linked && shapeInfo.name) {
    lines.push(`${rowIndent}shape ${shapeInfo.name}`);
  } else if (animationSource) {
    lines.splice(lines.length - 1, 0, `${rowIndent}duration ${sprite.animationDurationMs}ms`);
    animationSource.forEach((frame, index) => {
      if (index > 0) {
        lines.push(`${rowIndent}>`);
      }
      lines.push(...frame.map((row) => `${rowIndent}${row}`));
    });
  } else {
    lines.push(...spriteAscii().split("\n").map((row) => `${rowIndent}${row}`));
  }
  return lines.join("\n");
}

function spriteAnimationSourceFrames() {
  if (!sprite.animationMode) {
    return null;
  }
  ensureSpriteAnimationFrames();
  if (sprite.animationFrameCount < 2 || sprite.solidSource) {
    return null;
  }
  return sprite.animationFrames
    .slice(0, sprite.animationFrameCount)
    .map((frame) => spriteAsciiFromCells(frame));
}

function spriteAsciiFromCells(cells) {
  return Array.from({ length: sprite.size }, (_, y) => (
    Array.from({ length: sprite.size }, (_, x) => {
      const cell = Array.isArray(cells) ? cells[y * sprite.size + x] : null;
      return spriteExportCharForColorIndex(cell);
    }).join("")
  )).join("\n");
}

function spriteSourcePreludeRows(options = {}) {
  return (Array.isArray(sprite.sourcePreludeRows) ? sprite.sourcePreludeRows : [])
    .map((row) => String(row || "").trim())
    .filter((row) => !isSpriteSelectorPreludeRow(row))
    .filter((row) => !(options.omitDuration && isSpriteTimingPreludeRow(row)))
    .filter(Boolean);
}

function isSpriteSelectorPreludeRow(row) {
  return /^selector(?:\s*=)?\s+\S+$/i.test(String(row || "").trim());
}

function isSpriteTimingPreludeRow(row) {
  return /^(?:duration|frame_duration)(?:\s*=)?\s+\S+$/i.test(String(row || "").trim());
}

function spriteSolidDefinitionRow(shapeInfo) {
  if (shapeInfo.linked || sprite.palette.length !== 1 || !sprite.cells.length) {
    return null;
  }
  if (!sprite.cells.every((cell) => cell === 0)) {
    return null;
  }
  return spritePaletteSourceTokens()[0] || null;
}

function spritePaletteSourceTokens() {
  return sprite.palette.map((entry) => spritePaletteEntrySourceToken(entry));
}

function topLevelDepthAt(text, endIndex) {
  let depth = 0;
  for (let index = 0; index < endIndex; index += 1) {
    if (text[index] === "{") {
      depth += 1;
    } else if (text[index] === "}") {
      depth = Math.max(0, depth - 1);
    }
  }
  return depth;
}

for (const input of [
  spriteNameInput,
  spriteSizeInput,
  spriteScaleInput,
  spriteAnimationDurationInput,
  spriteAnimationFrameCountInput,
  spriteAnimationFrameInput,
]) {
  if (input) {
    installSelectAllOnFocus(input);
  }
}
spriteSizeInput.addEventListener("change", () => updateSpriteSize(spriteSizeInput.value));
spriteSizeInput.addEventListener("keydown", (event) => {
  if (event.key !== "Enter") {
    return;
  }
  event.preventDefault();
  updateSpriteSize(spriteSizeInput.value);
});
spriteScaleInput.addEventListener("input", () => {
  clearSpriteActionError();
  renderSpriteControls();
});
spriteScaleInput.addEventListener("keydown", (event) => {
  if (event.key !== "Enter") {
    return;
  }
  event.preventDefault();
});
spriteAnimationDurationInput?.addEventListener("input", () => updateSpriteAnimationDuration(spriteAnimationDurationInput.value, { preserveInput: true, recordHistory: false }));
spriteAnimationDurationInput?.addEventListener("change", () => updateSpriteAnimationDuration(spriteAnimationDurationInput.value));
spriteAnimationDurationInput?.addEventListener("keydown", (event) => {
  if (event.key !== "Enter") {
    return;
  }
  event.preventDefault();
  updateSpriteAnimationDuration(spriteAnimationDurationInput.value);
});
spriteAnimationFrameCountInput?.addEventListener("change", () => updateSpriteAnimationFrameCount(spriteAnimationFrameCountInput.value));
spriteAnimationFrameCountInput?.addEventListener("keydown", (event) => {
  if (event.key !== "Enter") {
    return;
  }
  event.preventDefault();
  updateSpriteAnimationFrameCount(spriteAnimationFrameCountInput.value);
});
spriteAnimationFrameInput?.addEventListener("change", () => setSpriteAnimationFrame(Number(spriteAnimationFrameInput.value) - 1));
spriteAnimationFrameInput?.addEventListener("keydown", (event) => {
  if (event.key !== "Enter") {
    return;
  }
  event.preventDefault();
  setSpriteAnimationFrame(Number(spriteAnimationFrameInput.value) - 1);
});
spriteAnimationPreviousFrameButton?.addEventListener("click", () => moveSpriteAnimationFrame(-1));
spriteAnimationNextFrameButton?.addEventListener("click", () => moveSpriteAnimationFrame(1));
spriteAnimationInsertFrameButton?.addEventListener("click", toggleSpriteAnimationInsertMode);
spriteAnimationRemoveFrameButton?.addEventListener("click", toggleSpriteAnimationRemoveMode);
for (const button of spriteBrushPresetButtons()) {
  button.addEventListener("click", () => selectSpriteBrushPreset(button.dataset.spriteBrushPreset));
}
spriteNameInput.addEventListener("input", syncSpriteSourceActionButtons);
sourceEditor.addEventListener("input", () => {
  invalidateSpriteEditSourceForDocument(activeDocument());
  syncSpriteSourceActionButtons();
});
spritePalette.addEventListener("mousedown", (event) => {
  const button = event.target.closest("button");
  if (!button || !spritePalette.contains(button)) {
    return;
  }
  event.preventDefault();
});
spritePalette.addEventListener("keydown", (event) => {
  const token = event.target.closest(".sprite-token");
  if (!token) {
    return;
  }
  if (event.key === "Enter" || event.key === " ") {
    const rawIndex = token.dataset.colorIndex;
    if (rawIndex === undefined) {
      return;
    }
    event.preventDefault();
    const nextIndex = rawIndex === "erase" ? null : Number(rawIndex);
    if (nextIndex === null) {
      spriteBucketActive = false;
    }
    selectSpriteColor(nextIndex);
  }
});
spriteBoard.addEventListener("pointerdown", startSpritePaint);
spriteBoard.addEventListener("pointermove", continueSpritePaint);
spriteBoard.addEventListener("pointerup", stopSpritePaint);
spriteBoard.addEventListener("pointercancel", stopSpritePaint);
spriteBoard.addEventListener("keydown", (event) => {
  if (handleSpriteClipKeyboard(event)) {
    return;
  }
  if (event.key === "Enter" || event.key === " ") {
    const mutate = spriteBucketActive ? bucketFillSpriteFromElement : paintSpriteCellFromElement;
    if (withVisualEditHistory("sprite", () => mutate(event.target))) {
      event.preventDefault();
      event.stopPropagation();
    }
  }
});
document.addEventListener("keydown", handleSpriteClipKeyboard);
document.addEventListener("pointerdown", closeSpriteColorEditorFromOutside);
spriteClearButton.addEventListener("click", clearSpriteBuilder);
spriteExportButton.addEventListener("click", exportSpriteAscii);
spriteUpdateButton.addEventListener("click", () => {
  updateSpriteInSource().catch((error) => {
    console.error(error);
    setSpriteActionStatus("Sprite source update failed", "is-error");
    setStatus("Sprite source update failed", "is-error");
  });
});
duplicateSpriteButton?.addEventListener("click", duplicateSpriteInSource);
spriteScaleDownButton.addEventListener("click", scaleDownSprite);
spriteScaleUpButton.addEventListener("click", scaleUpSprite);
spriteRotateLeftButton.addEventListener("click", rotateSpriteLeft);
spriteRotateRightButton.addEventListener("click", rotateSpriteRight);
spriteFlipHorizontalButton.addEventListener("click", flipSpriteHorizontal);
spriteFlipVerticalButton.addEventListener("click", flipSpriteVertical);
spriteFillButton.addEventListener("click", toggleSpriteBucketMode);
spriteGridButton?.addEventListener("click", toggleSpriteGrid);
resetSpriteBuilder();
