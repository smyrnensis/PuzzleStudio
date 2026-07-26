let visualActionClearTimer = 0;
let visualBucketActive = false;
let visualTranslateActive = false;
let visualTranslateDrag = null;
let visualGridVisible = true;
let visualBrushSizePx = 1;
let visualLastPaintColorIndex = 0;
let visualClipActive = false;
let visualClipSelection = null;
let visualClipDrag = null;
let visualClipClipboard = null;
let visualClipFloating = null;
let visualAnimationPlaybackTimer = 0;
let visualAnimationPlaybackDurationMs = 0;
const visualColorEditSessions = {
  visual: null,
  visual3d: null,
};
const VISUAL_EDITOR_MAX_SIZE = 64;
const VISUAL_ANIMATION_MAX_FRAMES = 24;
const VISUAL_ANIMATION_MIN_DURATION_MS = 20;
const VISUAL_ANIMATION_MAX_DURATION_MS = 5000;

function visualFrameCellCount() {
  return visual.width * visual.height;
}

function resetVisualBuilder(width = visual.width, height = visual.height) {
  visual.width = clampVisualSize(width);
  visual.height = clampVisualSize(height);
  visual.cells = Array.from({ length: visualFrameCellCount() }, () => null);
  resetVisualAnimationFramesFromCurrentCells();
  visual.shapeBind = null;
  visual.solidSource = false;
  visual.sourcePreludeRows = [];
  visual.sourceSpatialOps = [];
  if (!Number.isInteger(visual.selectedColorIndex) || !visual.palette[visual.selectedColorIndex]) {
    visual.selectedColorIndex = 0;
  }
  renderVisualBuilder();
}

function clampVisualSize(value) {
  const parsed = Math.trunc(Number(value));
  const size = Number.isFinite(parsed) ? parsed : 5;
  return Math.max(1, Math.min(VISUAL_EDITOR_MAX_SIZE, size));
}

function visualPaneScrollElement(builder) {
  return builder?.querySelector(":scope > .tool-pane-scroll") || null;
}

function captureVisualPaneScroll(builder) {
  if (!builder || builder.hidden) {
    return null;
  }
  const scroll = visualPaneScrollElement(builder);
  return scroll ? { top: scroll.scrollTop, left: scroll.scrollLeft } : null;
}

function restoreVisualPaneScroll(builder, state) {
  if (!state) {
    return;
  }
  const apply = () => {
    const scroll = visualPaneScrollElement(builder);
    if (!scroll) {
      return;
    }
    scroll.scrollTop = Math.max(0, Math.min(state.top, scroll.scrollHeight - scroll.clientHeight));
    scroll.scrollLeft = Math.max(0, Math.min(state.left, scroll.scrollWidth - scroll.clientWidth));
  };
  apply();
  window.requestAnimationFrame?.(apply);
}

function withVisualPaneScrollPreserved(builder, render) {
  const scroll = captureVisualPaneScroll(builder);
  const result = render();
  restoreVisualPaneScroll(builder, scroll);
  return result;
}

function withVisual2dPaneScrollPreserved(render) {
  return withVisualPaneScrollPreserved(visualBuilder, render);
}

function renderVisualBuilder() {
  if (!visualBoard || !visualPalette) {
    return;
  }
  withVisual2dPaneScrollPreserved(() => {
    ensureVisualAnimationFrames();
    renderVisualControls();
    renderVisualPalette();
    renderVisualBoard();
    renderVisualAnimationControls();
    syncVisualSourceActionButtons();
  });
}

function setVisualAnimationMode(enabled, options = {}) {
  visual.animationMode = Boolean(enabled);
  ensureVisualAnimationFrames();
  if (!visual.animationMode) {
    stopVisualAnimationPlayback({ render: false });
  }
  if (options.render !== false) {
    renderVisualBuilder();
  }
  if (typeof syncPreviewModeButtonState === "function") {
    syncPreviewModeButtonState();
  }
}

function resetVisualAnimationFramesFromCurrentCells() {
  visual.animationFrameIndex = 0;
  visual.animationFrameCount = 1;
  visual.animationPlaybackIndex = 0;
  visual.animationFrames = [cloneVisualCells(visual.cells)];
}

function cloneVisualCells(cells = visual.cells) {
  const length = visualFrameCellCount();
  return Array.from({ length }, (_, index) => {
    const colorIndex = cells[index];
    return validVisualColorIndex(colorIndex) ? colorIndex : null;
  });
}

function normalizedVisualAnimationFrameCount(value = visual.animationFrameCount) {
  const parsed = Math.trunc(Number(value));
  const count = Number.isFinite(parsed) ? parsed : 1;
  return Math.max(1, Math.min(VISUAL_ANIMATION_MAX_FRAMES, count));
}

function normalizedVisualAnimationDuration(value = visual.animationDurationMs) {
  const parsed = Math.trunc(Number(value));
  const duration = Number.isFinite(parsed) ? parsed : 120;
  return Math.max(VISUAL_ANIMATION_MIN_DURATION_MS, Math.min(VISUAL_ANIMATION_MAX_DURATION_MS, duration));
}

function ensureVisualAnimationFrames() {
  visual.animationFrameCount = normalizedVisualAnimationFrameCount(visual.animationFrameCount);
  visual.animationDurationMs = normalizedVisualAnimationDuration(visual.animationDurationMs);
  if (!Array.isArray(visual.animationFrames) || !visual.animationFrames.length) {
    visual.animationFrames = [cloneVisualCells(visual.cells)];
  }
  while (visual.animationFrames.length < visual.animationFrameCount) {
    visual.animationFrames.push(cloneVisualCells(visual.cells));
  }
  if (visual.animationFrames.length > visual.animationFrameCount) {
    visual.animationFrames.length = visual.animationFrameCount;
  }
  for (let index = 0; index < visual.animationFrames.length; index += 1) {
    visual.animationFrames[index] = normalizeVisualAnimationFrameCells(visual.animationFrames[index]);
  }
  visual.animationFrameIndex = Math.max(0, Math.min(visual.animationFrameCount - 1, Math.trunc(Number(visual.animationFrameIndex) || 0)));
  visual.animationPlaybackIndex = Math.max(0, Math.min(visual.animationFrameCount - 1, Math.trunc(Number(visual.animationPlaybackIndex) || 0)));
  if (visual.animationMode) {
    visual.cells = visual.animationFrames[visual.animationFrameIndex];
  }
}

function normalizeVisualAnimationFrameCells(cells) {
  const length = visualFrameCellCount();
  return Array.from({ length }, (_, index) => {
    const colorIndex = Array.isArray(cells) ? cells[index] : null;
    return validVisualColorIndex(colorIndex) ? colorIndex : null;
  });
}

function resizeVisualAnimationCells(cells, previous, next) {
  const nextCells = Array.from({ length: next.width * next.height }, () => null);
  const copyWidth = Math.min(previous.width, next.width);
  const copyHeight = Math.min(previous.height, next.height);
  for (let y = 0; y < copyHeight; y += 1) {
    for (let x = 0; x < copyWidth; x += 1) {
      const colorIndex = cells[y * previous.width + x];
      nextCells[y * next.width + x] = validVisualColorIndex(colorIndex) ? colorIndex : null;
    }
  }
  return nextCells;
}

function syncVisualAnimationFramesAfterSizeChange(previous, next, activeCells) {
  if (!visual.animationMode) {
    resetVisualAnimationFramesFromCurrentCells();
    return;
  }
  ensureVisualAnimationFrames();
  visual.animationFrames = visual.animationFrames.map((cells, index) => (
    index === visual.animationFrameIndex
      ? cloneVisualCells(activeCells)
      : resizeVisualAnimationCells(cells, previous, next)
  ));
  visual.cells = visual.animationFrames[visual.animationFrameIndex];
}

function renderVisualAnimationControls() {
  if (!visualBuilder) {
    return;
  }
  withVisual2dPaneScrollPreserved(() => renderVisualAnimationControlsContent());
}

function renderVisualAnimationControlsContent() {
  mountSharedVisualAnimationUi("2d");
  ensureVisualAnimationFrames();
  visualBuilder.classList.toggle("is-animation-mode", visual.animationMode);
  if (!visual.animationMode) {
    return;
  }
  syncVisualAnimationInputValues({ preserveActive: true });
  syncSharedVisualAnimationToolbarState(visual.animationFrameCount, VISUAL_ANIMATION_MAX_FRAMES);
  renderVisualAnimationSurfaces();
  syncVisualAnimationPlayback();
}

function syncSharedVisualAnimationToolbarState(frameCount, maxFrames) {
  if (visualAnimationFrameTotal) visualAnimationFrameTotal.textContent = String(frameCount);
  if (visualAnimationPreviousFrameButton) visualAnimationPreviousFrameButton.disabled = frameCount <= 1;
  if (visualAnimationNextFrameButton) visualAnimationNextFrameButton.disabled = frameCount <= 1;
  if (visualAnimationInsertFrameButton) {
    visualAnimationInsertFrameButton.disabled = frameCount >= maxFrames;
  }
  if (visualAnimationRemoveFrameButton) {
    visualAnimationRemoveFrameButton.disabled = frameCount <= 1;
  }
}

function mountSharedVisualAnimationUi(dimension) {
  const toolbar = visualAnimationFrameInput?.closest(".visual-animation-toolbar");
  const panel = visualAnimationFrameStrip?.closest(".visual-animation-panel");
  const playbackPanel = visualAnimationPlaybackView?.closest(".visual-animation-playback-panel");
  const sidecar = panel?.closest(".visual-animation-sidecar");
  if (!toolbar || !panel || !playbackPanel || !sidecar) {
    throw new Error("Shared visual animation UI is unavailable");
  }
  if (dimension === "3d") {
    const previewColumn = visual3dBuilder?.querySelector(".visual3d-preview-column");
    const previewStage = visual3dBuilder?.querySelector(".visual3d-preview-stage");
    if (!previewColumn || !previewStage) {
      throw new Error("3D visual animation hosts are unavailable");
    }
    previewColumn.insertBefore(toolbar, previewStage);
    toolbar.classList.add("is-visual3d-shared");
    panel.classList.add("visual3d-animation-panel");
    previewStage.append(sidecar);
    toolbar.setAttribute("aria-label", "3D visual animation frame controls");
    playbackPanel.setAttribute("aria-label", "3D visual animation playback preview");
    visualAnimationPlaybackView.setAttribute("aria-label", "3D visual animation playback preview");
    panel.setAttribute("aria-label", "3D visual animation frames");
    visualAnimationDurationInput?.closest(".visual-animation-duration-control")?.toggleAttribute("hidden", true);
    visual3dAnimationDurationInput?.closest(".visual-animation-duration-control")?.toggleAttribute("hidden", false);
    return;
  }
  const boardWrap = visualBuilder.querySelector(".visual-board-wrap");
  const workspace = visualBuilder.querySelector(".visual-animation-workspace");
  if (!boardWrap || !workspace) {
    throw new Error("2D visual animation hosts are unavailable");
  }
  boardWrap.insertBefore(toolbar, workspace);
  toolbar.classList.remove("is-visual3d-shared");
  panel.classList.remove("visual3d-animation-panel");
  workspace.append(sidecar);
  toolbar.setAttribute("aria-label", "Visual animation frame controls");
  playbackPanel.setAttribute("aria-label", "Visual animation playback preview");
  visualAnimationPlaybackView.setAttribute("aria-label", "Visual animation playback preview");
  panel.setAttribute("aria-label", "Visual animation frames");
  visualAnimationDurationInput?.closest(".visual-animation-duration-control")?.toggleAttribute("hidden", false);
  visual3dAnimationDurationInput?.closest(".visual-animation-duration-control")?.toggleAttribute("hidden", true);
}

function syncVisualAnimationInputValues(options = {}) {
  const preserveActive = options.preserveActive === true;
  if (visualAnimationDurationInput && (!preserveActive || document.activeElement !== visualAnimationDurationInput)) {
    visualAnimationDurationInput.value = String(visual.animationDurationMs);
  }
  if (visualAnimationFrameInput && (!preserveActive || document.activeElement !== visualAnimationFrameInput)) {
    visualAnimationFrameInput.value = String(visual.animationFrameIndex + 1);
  }
  if (visualAnimationFrameInput) {
    visualAnimationFrameInput.max = String(visual.animationFrameCount);
  }
}

function renderVisualAnimationSurfaces() {
  if (!visual.animationMode) {
    return;
  }
  renderVisualAnimationPlaybackView(visual.animationFrames[visual.animationPlaybackIndex] || visual.cells);
  renderVisualAnimationFrameStrip();
}

function renderVisualAnimationPlaybackView(cells) {
  renderSharedVisualAnimationPlaybackView(sharedVisualAnimationController("visual"), cells);
}

function visualAnimationFrameCells(cells) {
  return Array.from({ length: visualFrameCellCount() }, (_, index) => {
    const colorIndex = validVisualColorIndex(cells?.[index]) ? cells[index] : null;
    const cell = document.createElement("span");
    cell.className = "visual-animation-frame-cell";
    cell.style.setProperty("--visual-swatch-color", visualColorForColorIndex(colorIndex));
    return cell;
  });
}

function renderVisualAnimationFrameStrip() {
  if (!visualAnimationFrameStrip) {
    return;
  }
  renderVisualAnimationFrameStripView({
    target: visualAnimationFrameStrip,
    frameCount: visual.animationFrameCount,
    activeIndex: visual.animationFrameIndex,
    playingIndex: visual.animationPlaybackIndex,
    size: Math.max(visual.width, visual.height),
    columns: visual.width,
    rows: visual.height,
    renderCells: (index) => visualAnimationFrameCells(visual.animationFrames[index]),
    onSelect: setVisualAnimationFrame,
    noun: "visual animation",
  });
}

function renderVisualAnimationFrameStripView(options) {
  const target = options.target;
  if (!target) {
    return;
  }
  const fragment = document.createDocumentFragment();
  for (let index = 0; index < options.frameCount; index += 1) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "visual-animation-frame-button";
    button.classList.toggle("is-active", index === options.activeIndex);
    button.classList.toggle("is-playing-frame", index === options.playingIndex);
    button.style.setProperty("--visual-size", options.size);
    if (options.columns) {
      button.style.setProperty("--visual-preview-cols", options.columns);
    }
    if (options.rows) {
      button.style.setProperty("--visual-preview-rows", options.rows);
    }
    button.setAttribute("aria-label", `Edit ${options.noun} frame ${index + 1}`);
    button.title = `Frame ${index + 1}`;
    button.append(...options.renderCells(index));
    const label = document.createElement("span");
    label.className = "visual-animation-frame-index";
    label.textContent = String(index + 1);
    button.append(label);
    button.addEventListener("click", () => options.onSelect(index));
    fragment.append(button);
  }
  target.replaceChildren(fragment);
}

function sharedVisualAnimationController(dimension = currentVisualPaneMode) {
  const is3d = dimension === "visual3d" || dimension === "3d";
  if (is3d) ensureVisual3dAnimationState();
  else ensureVisualAnimationFrames();
  const state = is3d ? visual3d : visual;
  return {
    dimension: is3d ? "visual3d" : "visual",
    state,
    get frames() {
      return is3d ? state.frames : state.animationFrames;
    },
    maxFrames: is3d ? VISUAL3D_ANIMATION_MAX_FRAMES : VISUAL_ANIMATION_MAX_FRAMES,
    noun: is3d ? "3D visual animation" : "visual animation",
    durationMs: () => is3d ? normalizedVisual3dAnimationDuration() : normalizedVisualAnimationDuration(),
    renderPlaybackFrame: is3d
      ? (frame) => visual3dAnimationFramePreview(frame)
      : (frame) => visualAnimationFrameCells(frame),
    commit: is3d ? commitVisual3dActiveFrame : () => {},
    stopPlayback: () => stopVisualAnimationPlayback({ render: false }),
    deactivateClip: is3d
      ? () => deactivateVisual3dClipMode({ render: false })
      : () => deactivateVisualClipMode({ render: false }),
    render: is3d ? renderVisual3dBuilder : renderVisualBuilder,
  };
}

function selectSharedVisualAnimationFrame(dimension, index) {
  const context = sharedVisualAnimationController(dimension);
  context.commit();
  const nextIndex = Math.max(0, Math.min(context.state.animationFrameCount - 1, Math.trunc(Number(index) || 0)));
  context.state.animationFrameIndex = nextIndex;
  context.state.animationPlaybackIndex = nextIndex;
  context.state.cells = context.frames[nextIndex];
  context.deactivateClip();
  context.render();
  setVisualActionStatus(`Frame ${nextIndex + 1}`, "is-ok");
}

function moveSharedVisualAnimationFrame(dimension, delta) {
  const context = sharedVisualAnimationController(dimension);
  const count = context.state.animationFrameCount;
  selectSharedVisualAnimationFrame(dimension, (context.state.animationFrameIndex + delta + count) % count);
}

function insertSharedVisualAnimationFrameAt(dimension, index) {
  const context = sharedVisualAnimationController(dimension);
  if (context.state.animationFrameCount >= context.maxFrames) {
    context.render();
    setVisualActionStatus(`Maximum ${context.maxFrames} frames`, "is-error");
    return false;
  }
  const before = visualEditSnapshot(context.dimension);
  context.commit();
  const insertIndex = Math.max(0, Math.min(context.state.animationFrameCount, Math.trunc(Number(index) || 0)));
  const copyIndex = Math.max(0, Math.min(context.state.animationFrameCount - 1, insertIndex - 1));
  context.stopPlayback();
  context.frames.splice(insertIndex, 0, context.frames[copyIndex].slice());
  context.state.animationFrameCount = context.frames.length;
  context.state.animationFrameIndex = insertIndex;
  context.state.animationPlaybackIndex = insertIndex;
  context.state.cells = context.frames[insertIndex];
  context.deactivateClip();
  context.render();
  setVisualActionStatus(`Added frame ${insertIndex + 1}`, "is-ok");
  pushVisualEditUndoSnapshot(context.dimension, before);
  return true;
}

function removeSharedVisualAnimationFrameAt(dimension, index) {
  const context = sharedVisualAnimationController(dimension);
  if (context.state.animationFrameCount <= 1) {
    context.render();
    setVisualActionStatus("At least 1 frame is required", "is-error");
    return false;
  }
  const before = visualEditSnapshot(context.dimension);
  context.commit();
  const removeIndex = Math.max(0, Math.min(context.state.animationFrameCount - 1, Math.trunc(Number(index) || 0)));
  context.stopPlayback();
  context.frames.splice(removeIndex, 1);
  context.state.animationFrameCount = context.frames.length;
  context.state.animationFrameIndex = Math.min(removeIndex, context.frames.length - 1);
  context.state.animationPlaybackIndex = context.state.animationFrameIndex;
  context.state.cells = context.frames[context.state.animationFrameIndex];
  context.deactivateClip();
  context.render();
  setVisualActionStatus(`Removed frame ${removeIndex + 1}`, "is-ok");
  pushVisualEditUndoSnapshot(context.dimension, before);
  return true;
}

function insertSharedVisualAnimationFrameAfterCurrent(dimension = currentVisualPaneMode) {
  const context = sharedVisualAnimationController(dimension);
  return insertSharedVisualAnimationFrameAt(
    context.dimension,
    context.state.animationFrameIndex + 1,
  );
}

function removeSharedVisualAnimationCurrentFrame(dimension = currentVisualPaneMode) {
  const context = sharedVisualAnimationController(dimension);
  return removeSharedVisualAnimationFrameAt(
    context.dimension,
    context.state.animationFrameIndex,
  );
}

function setVisualAnimationFrame(index) {
  selectSharedVisualAnimationFrame("visual", index);
}

function moveVisualAnimationFrame(delta) {
  moveSharedVisualAnimationFrame("visual", delta);
}

<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
function updateVisualAnimationFrameCount(value) {
  const before = visualEditSnapshot("visual");
  visual.animationFrameCount = normalizedVisualAnimationFrameCount(value);
  ensureVisualAnimationFrames();
  renderVisualBuilder();
  pushVisualEditUndoSnapshot("visual", before);
}

=======
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
function insertVisualAnimationFrameAt(index) {
  return insertSharedVisualAnimationFrameAt("visual", index);
}

function removeVisualAnimationFrameAt(index) {
  return removeSharedVisualAnimationFrameAt("visual", index);
}

function updateVisualAnimationDuration(value, options = {}) {
  const nextDuration = normalizedVisualAnimationDuration(value);
  const changed = nextDuration !== visual.animationDurationMs;
  const before = options.recordHistory === false || !changed ? null : visualEditSnapshot("visual");
  visual.animationDurationMs = nextDuration;
  if (
    visualAnimationDurationInput
    && !(options.preserveInput === true && document.activeElement === visualAnimationDurationInput)
  ) {
    visualAnimationDurationInput.value = String(visual.animationDurationMs);
  }
  if (changed && visual.animationMode && visual.animationFrameCount > 1) {
    stopVisualAnimationPlayback({ render: false });
    startVisualAnimationPlayback();
  }
  if (before) {
    pushVisualEditUndoSnapshot("visual", before);
  }
}

function isVisualEditUndoTarget(target) {
  return target === visualAnimationDurationInput || target === visual3dAnimationDurationInput;
}

function visualAnimationFrameDelayMs() {
  const context = sharedVisualAnimationController();
  return Math.max(1, Math.round(context.durationMs() / context.state.animationFrameCount));
}

function syncVisualAnimationPlayback() {
  const context = sharedVisualAnimationController();
  if (!context.state.animationMode || context.state.animationFrameCount <= 1) {
    stopVisualAnimationPlayback({ render: false });
    context.state.animationPlaybackIndex = context.state.animationFrameIndex;
    renderSharedVisualAnimationPlaybackView(context, context.state.cells);
    return;
  }
  if (
    !visual.animationPlaying
    || !visualAnimationPlaybackTimer
    || visualAnimationPlaybackDurationMs !== visualAnimationFrameDelayMs()
  ) {
    startVisualAnimationPlayback();
  }
}

function startVisualAnimationPlayback() {
  const dimension = currentVisualPaneMode;
  const context = sharedVisualAnimationController(dimension);
  if (context.state.animationFrameCount <= 1) {
    stopVisualAnimationPlayback({ render: false });
    return;
  }
  stopVisualAnimationPlayback({ render: false });
  context.state.animationPlaying = true;
  visualAnimationPlaybackDurationMs = visualAnimationFrameDelayMs();
  context.state.animationPlaybackIndex = context.state.animationFrameIndex;
  renderSharedVisualAnimationPlaybackView(context, context.frames[context.state.animationPlaybackIndex] || context.state.cells);
  const tick = () => {
    if (!context.state.animationPlaying || currentVisualPaneMode !== dimension) {
      return;
    }
    context.state.animationPlaybackIndex = (context.state.animationPlaybackIndex + 1) % context.state.animationFrameCount;
    renderSharedVisualAnimationPlaybackView(context, context.frames[context.state.animationPlaybackIndex] || context.state.cells);
    visualAnimationPlaybackDurationMs = visualAnimationFrameDelayMs();
    visualAnimationPlaybackTimer = window.setTimeout(tick, visualAnimationPlaybackDurationMs);
  };
  visualAnimationPlaybackTimer = window.setTimeout(tick, visualAnimationPlaybackDurationMs);
}

function stopVisualAnimationPlayback(options = {}) {
  window.clearTimeout(visualAnimationPlaybackTimer);
  visualAnimationPlaybackTimer = 0;
  visualAnimationPlaybackDurationMs = 0;
  visual.animationPlaying = false;
  visual3d.animationPlaying = false;
  if (options.render !== false) {
    const context = sharedVisualAnimationController();
    renderSharedVisualAnimationPlaybackView(context, context.frames[context.state.animationPlaybackIndex] || context.state.cells);
  }
}

function renderSharedVisualAnimationPlaybackView(context, frame) {
  if (!visualAnimationPlaybackView) {
    return;
  }
  visualAnimationPlaybackView.classList.toggle("is-visual3d", context.dimension === "visual3d");
  visualAnimationPlaybackView.style.setProperty(
    "--visual-size",
    Math.max(context.state.width, context.state.height),
  );
  visualAnimationPlaybackView.style.setProperty("--visual-preview-cols", context.state.width);
  visualAnimationPlaybackView.style.setProperty("--visual-preview-rows", context.state.height);
  visualAnimationPlaybackView.replaceChildren(...context.renderPlaybackFrame(frame));
}

function renderVisualEditorUpperControls(target, controls) {
  if (!target) {
    return;
  }

  const labeledControl = (labelText, control, className) => {
    const label = document.createElement("label");
    label.className = `visual-compact-control ${className}`;
    const caption = document.createElement("span");
    caption.textContent = labelText;
    label.append(caption, control);
    return label;
  };

  const root = document.createElement("div");
  root.className = "visual-editor-upper-controls";
  root.setAttribute("aria-label", "Visual source fields");

  const nameRow = document.createElement("div");
  nameRow.className = "visual-editor-name-row";
  const sizeControl = controls.extentInputs;
  const sizeEditor = document.createElement("span");
  sizeEditor.className = "visual-size-editor";
  const sizeBindButton = document.createElement("button");
  sizeBindButton.type = "button";
  sizeBindButton.className = "icon-button visual-size-bind-button visual-icon-button";
  sizeBindButton.classList.toggle("is-active", controls.sizeBound);
  sizeBindButton.setAttribute("aria-label", controls.sizeBound ? "Unbind size axes" : "Bind size axes");
  sizeBindButton.setAttribute("aria-pressed", String(controls.sizeBound));
  sizeBindButton.dataset.tooltip = controls.sizeBound ? "Size axes bound" : "Bind size axes";
  sizeBindButton.innerHTML = visualLucideIconSvg("link-2");
  sizeBindButton.addEventListener("click", controls.toggleSizeBound);
  sizeEditor.append(sizeControl, sizeBindButton);
  nameRow.append(
    labeledControl("Visual for", controls.nameInput, "visual-name-control"),
    labeledControl("Size", sizeEditor, "visual-size-control"),
  );

  const geometry = document.createElement("div");
  geometry.className = "visual-editor-geometry-group";

  const scale = document.createElement("div");
  scale.className = "visual-scale-control visual-compact-control";
  scale.setAttribute("role", "group");
  scale.setAttribute("aria-label", "Visual scale");
  scale.dataset.tooltip = "Uniform scale";
  const scaleGroup = document.createElement("div");
  scaleGroup.className = "visual-scale-group";
  const scalePrefix = document.createElement("span");
  scalePrefix.className = "visual-scale-prefix";
  scalePrefix.setAttribute("aria-hidden", "true");
  scalePrefix.textContent = "×";
  scaleGroup.append(
    scalePrefix,
    controls.scaleInput,
    controls.scaleUpButton,
    controls.scaleDownButton,
  );
  scale.append(scaleGroup);
  geometry.append(scale);

  root.append(nameRow, geometry);
  target.replaceChildren(root);
}

function visualEditorUpperControls2d() {
  return {
    dimension: "2d",
    nameInput: visualNameInput,
    extentInputs: visualWidthInput.closest(".visual-extent-inputs"),
    sizeBound: visual.sizeBound,
    toggleSizeBound: toggleVisualSizeBound,
    scaleInput: visualScaleInput,
    scaleDownButton: visualScaleDownButton,
    scaleUpButton: visualScaleUpButton,
    shapeField: visualShapeField,
  };
}

function visualEditorUpperControls3d() {
  return {
    dimension: "3d",
    nameInput: visual3dNameInput,
    extentInputs: visual3dWidthInput.closest(".visual3d-extent-inputs"),
    sizeBound: visual3d.sizeBound,
    toggleSizeBound: toggleVisual3dSizeBound,
    scaleInput: visual3dScaleInput,
    scaleDownButton: visual3dScaleDownButton,
    scaleUpButton: visual3dScaleUpButton,
    shapeField: visual3dShapeField,
  };
}

function toggleVisualSizeBound() {
  visual.sizeBound = !visual.sizeBound;
  renderVisualControls();
  setVisualActionStatus(visual.sizeBound ? "Size axes bound" : "Size axes independent", "is-ok");
}

function toggleVisual3dSizeBound() {
  visual3d.sizeBound = !visual3d.sizeBound;
  renderVisual3dControls();
  setVisual3dActionStatus(visual3d.sizeBound ? "Size axes bound" : "Size axes independent", "is-ok");
}

function renderVisualControls() {
  withVisual2dPaneScrollPreserved(() => renderVisualControlsContent());
}

function renderVisualControlsContent() {
  renderVisualEditorUpperControls(
    visualBuilder.querySelector(".visual-controls"),
    visualEditorUpperControls2d(),
  );
  visualWidthInput.value = String(visual.width);
  visualHeightInput.value = String(visual.height);
  syncVisualPaintToolControls();
  syncVisualGridButton();
  renderVisualShapeBindControl(visualShapeField, {
    state: visual,
    render: renderVisualControls,
    onChange: () => {
      const bind = visualAssetBindInfo(visual.shapeBind, "shape");
      if (bind.linked && bind.name) {
        setVisualShapeSync(true, bind.name);
        return;
      }
      void syncCurrentVisualDefinitionFromBuilder("Updated shape tag");
      renderVisualBuilder();
    },
  });
  renderVisualScaleControl({
    size: Math.max(visual.width, visual.height),
    maxSize: VISUAL_EDITOR_MAX_SIZE,
    scaleInput: visualScaleInput,
    scaleUpButton: visualScaleUpButton,
    scaleDownButton: visualScaleDownButton,
    canScaleDown: canScaleDownVisual,
    noun: "visual",
  });
}

function syncVisualPaintToolControls() {
  syncVisualBucketButton();
  syncVisualMarkerControl();
}

function syncVisualBucketButton() {
  if (!visualFillButton) {
    return;
  }
  visualFillButton.classList.toggle("is-active", visualBucketActive);
  visualFillButton.setAttribute("aria-pressed", String(visualBucketActive));
  visualFillButton.setAttribute("aria-label", "Fill");
  visualFillButton.title = "Fill";
  visualFillButton.dataset.tooltip = "Fill";
}

function toggleVisualBucketMode() {
  visualBucketActive = !visualBucketActive;
  syncVisualPaintToolControls();
  renderVisualPalette();
  setVisualActionStatus(
    visualBucketActive
      ? visualClipActive ? "Bucket: click a connected area inside the clip region" : "Bucket: click a connected area"
      : visualClipActive ? "Clip: drag selection to move it" : visualPaintToolStatusText(),
    "is-ok",
  );
}

function deactivateVisualBucketModeAfterUse() {
  if (!visualBucketActive) {
    return;
  }
  visualBucketActive = false;
  syncVisualPaintToolControls();
}

function syncVisualMarkerControl() {
  visualBrushSizePx = normalizeVisualBrushSize(visualBrushSizePx);
  visualBrushSizeInput.value = String(visualBrushSizePx);
}

function selectVisualBrushSize(size) {
  const wasBucketActive = visualBucketActive;
  const wasClipActive = visualClipActive || visualClipSelection;
  visualBrushSizePx = normalizeVisualBrushSize(size);
  visualBucketActive = false;
  deactivateVisualClipMode({ render: false });
  if (!validVisualColorIndex(visual.selectedColorIndex)) {
    visual.selectedColorIndex = validVisualColorIndex(visualLastPaintColorIndex) ? visualLastPaintColorIndex : 0;
  }
  syncVisualPaintToolControls();
  if (wasBucketActive || wasClipActive) {
    renderVisualPalette();
  }
  if (wasClipActive) {
    renderVisualBoard();
  }
  setVisualActionStatus(visualPaintToolStatusText(), "is-ok");
}

function normalizeVisualBrushSize(size) {
  const parsed = Number(size);
  return Number.isInteger(parsed) ? Math.max(1, Math.min(VISUAL_EDITOR_MAX_SIZE, parsed)) : 1;
}

function visualPaintToolStatusText() {
  return `Brush: ${visualBrushSizePx}px`;
}

function beginVisualColorEditHistory(kind) {
  if (!visualColorEditSessions[kind]) {
    visualColorEditSessions[kind] = visualEditSnapshot(kind);
  }
}

function commitVisualColorEditHistory(kind) {
  const before = visualColorEditSessions[kind];
  visualColorEditSessions[kind] = null;
  if (!before) {
    return false;
  }
  return pushVisualEditUndoSnapshot(kind, before);
}

function discardVisualColorEditHistory(kind) {
  visualColorEditSessions[kind] = null;
}

function clearVisualColorEditorState({ commitHistory = true } = {}) {
  if (commitHistory) {
    commitVisualColorEditHistory("visual");
  }
  visual.addPaletteOpen = false;
  visual.editPaletteOpen = false;
  visual.customColorOpen = false;
  visual.addDraftColorIndex = null;
}

function clearVisualTagPickerState() {
  visual.colorTagPickerOpen = false;
  visual.shapeTagPickerOpen = false;
}

function renderVisualColorAdjuster({ color, ariaLabel, onChange }) {
  const editor = window.PuzzleStudioColorEditor.create({
    color,
    ariaLabel,
    className: "visual-color-adjuster",
    onInput: onChange,
  });
  return editor;
}

function renderVisualPaletteGrid({
  target,
  leadingControl,
  entries,
  selectedIndex,
  bucketActive,
  emptyTitle,
  emptyAriaLabel,
  colorAriaLabel,
  onSelect,
  onAdd,
  onRemove,
  addOpen,
  renderAddMenu,
}) {
  const paletteGrid = document.createElement("span");
  paletteGrid.className = "visual-palette-grid";
  if (leadingControl) {
    paletteGrid.append(leadingControl);
  }
  const eraseButton = document.createElement("button");
  eraseButton.type = "button";
  eraseButton.className = "icon-button visual-token visual-token-erase visual-icon-button";
  eraseButton.classList.toggle("is-selected", selectedIndex === null && !bucketActive);
  eraseButton.dataset.colorIndex = "erase";
  eraseButton.style.setProperty("--visual-swatch-color", "#00000000");
  eraseButton.title = emptyTitle;
  eraseButton.setAttribute("aria-label", emptyAriaLabel);
  eraseButton.innerHTML = `
    ${editorIconSvg("eraser")}
  `;
  eraseButton.addEventListener("click", () => onSelect(null));
  paletteGrid.append(eraseButton);

  for (const [index, entry] of entries.entries()) {
    const item = document.createElement("span");
    item.className = "visual-token-item";
    item.classList.toggle("is-selected", index === selectedIndex);
    const button = document.createElement("button");
    button.type = "button";
    button.className = "visual-token visual-color-swatch";
    button.classList.toggle("is-selected", index === selectedIndex);
    button.dataset.colorIndex = String(index);
    button.style.setProperty("--visual-swatch-color", normalizeVisualColor(entry.color));
    button.style.setProperty("--visual-token-ink", readableInkForColor(entry.color));
    const bind = visualPaletteEntryBindInfo(entry);
    button.classList.toggle("is-bound", bind.available && bind.linked);
    button.classList.toggle("is-unlinked", bind.available && !bind.linked);
    const displayName = bind.linked && bind.name ? bind.name : "";
    button.setAttribute("aria-label", colorAriaLabel(index, displayName));
    button.addEventListener("click", () => onSelect(index));
    item.append(button);
    const marker = renderVisualBindMarker(entry);
    if (marker) {
      item.append(marker);
    }
    paletteGrid.append(item);
  }

  const addWrap = document.createElement("span");
  addWrap.className = "visual-add-wrap";
  const addButton = document.createElement("button");
  addButton.type = "button";
  addButton.className = "visual-token visual-add-color-button";
  addButton.disabled = entries.length >= VISUAL_COLOR_TOKENS.length;
  addButton.title = "Add color";
  addButton.setAttribute("aria-label", "Add visual color");
  addButton.setAttribute("aria-expanded", String(addOpen));
  addButton.innerHTML = `${editorIconSvg("plus")}`;
  addButton.addEventListener("click", onAdd);
  addWrap.append(addButton);
  paletteGrid.append(addWrap);

  const removeButton = document.createElement("button");
  removeButton.type = "button";
  removeButton.className = "visual-token visual-remove-color-button";
  removeButton.disabled = !Number.isInteger(selectedIndex) || entries.length <= 1;
  removeButton.title = "Remove selected color";
  removeButton.setAttribute("aria-label", "Remove selected color");
  removeButton.innerHTML = `${editorIconSvg("minus")}`;
  removeButton.addEventListener("click", onRemove);
  paletteGrid.append(removeButton);
  target.append(paletteGrid);
  if (addOpen) {
    const menu = renderAddMenu();
    menu.classList.add("is-add-menu");
    target.append(menu);
    positionVisualColorMenu(menu, paletteGrid, { side: "left" });
  }
  return paletteGrid;
}

function renderVisualPalette() {
  withVisual2dPaneScrollPreserved(() => renderVisualPaletteContent());
}

function renderVisualPaletteContent() {
  visualPalette.replaceChildren();
  const selectedIsTransparent = visual.selectedColorIndex === null;
  if (selectedIsTransparent || validVisualColorIndex(visual.selectedColorIndex)) {
    const currentWrap = document.createElement("span");
    currentWrap.className = "visual-current-color-wrap";
    const selected = selectedIsTransparent ? { color: "#00000000" } : visual.palette[visual.selectedColorIndex];
    const selectedBind = selectedIsTransparent ? { available: false, linked: false, label: "" } : visualPaletteEntryBindInfo(selected);
    const selectedDisplayName = selectedIsTransparent ? "" : visualPaletteEntryDisplayName(selected);
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
    currentButton.setAttribute("aria-expanded", String(!selectedIsTransparent && visual.editPaletteOpen));
    currentButton.innerHTML = `
      <span class="visual-current-color-swatch" aria-hidden="true"></span>
    `;
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
    currentHexInput.value = selectedDisplayName || (selectedIsTransparent
      ? "#00000000"
      : normalizeVisualColor(selected.color));
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
    const currentTagButton = selectedIsTransparent ? null : renderVisualCurrentColorTagButton({
      state: visual,
      entry: selected,
      onToggle: (opening) => {
        if (opening) {
          clearVisualColorEditorState();
          visual.shapeTagPickerOpen = false;
          renderVisualControls();
        }
        renderVisualPalette();
      },
    });
    const currentTagUnlinkButton = !selectedIsTransparent && selectedBind.linked && selectedBind.name
      ? renderVisualCurrentColorUnlinkButton(visual.selectedColorIndex, selectedBind)
      : null;
    const colorNames = selectedIsTransparent ? [] : visualColorAssetNames();
    const applyCurrentColorValue = (color) => {
      beginVisualColorEditHistory("visual");
      const normalized = normalizeVisualColor(color);
      clearVisualActionError();
      selected.color = normalized;
      updateVisualBoundColorDefinition(selected, normalized);
      currentButton.style.setProperty("--visual-current-color", normalized);
      currentButton.setAttribute("aria-label", selectedDisplayName ? `Edit selected color ${selectedDisplayName}` : `Edit selected color ${normalized}`);
      currentHexInput.value = selectedDisplayName || normalized;
      renderVisualColorSurfaces();
    };
    let pendingEditMenu = null;
    const applyCurrentHex = (options = {}) => {
      if (currentHexInput.classList.contains("is-name-mode")) {
        const ok = applyCurrentColorName(visual.selectedColorIndex, currentHexInput.value, { reportError: true });
        if (ok && options.commitHistory) {
          commitVisualColorEditHistory("visual");
        }
        return;
      }
      const parsed = parseVisualHexColor(currentHexInput.value);
      if (!parsed) {
        if (options.reportError) {
          setVisualActionStatus("Use #rrggbb or #rrggbbaa", "is-error");
        }
        return;
      }
      applyCurrentColorValue(parsed);
      if (options.commitHistory) {
        commitVisualColorEditHistory("visual");
      }
    };

    if (!selectedIsTransparent) {
      currentButton.addEventListener("click", () => {
        const opening = !visual.editPaletteOpen;
        if (!opening) {
          commitVisualColorEditHistory("visual");
        }
        visual.editPaletteOpen = opening;
        visual.addPaletteOpen = false;
        visual.addDraftColorIndex = null;
        visual.customColorOpen = opening;
        if (opening) {
          clearVisualTagPickerState();
          renderVisualControls();
        }
        renderVisualPalette();
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
    if (!selectedIsTransparent && visual.colorTagPickerOpen) {
      const colorAssets = visualSourceColorAssets();
      const tagPicker = renderVisualAssetNamePicker({
        className: "visual-color-tag-picker",
        names: colorNames,
        value: selectedBind.name || defaultVisualAssetName("color", visual.selectedColorIndex),
        placeholder: "color_name",
        ariaLabel: "Color tag name",
        emptyText: "No named colors yet",
        optionMeta: (name) => ({ color: colorAssets.get(name) }),
        onCommit: (name) => {
          const wasOpen = visual.colorTagPickerOpen;
          visual.colorTagPickerOpen = false;
          const ok = applyCurrentColorName(visual.selectedColorIndex, name, { reportError: true });
          if (!ok) {
            visual.colorTagPickerOpen = wasOpen;
            return false;
          }
          clearVisualColorEditorState();
          renderVisualBuilder();
          return true;
        },
        onCancel: () => {
          visual.colorTagPickerOpen = false;
          renderVisualPalette();
        },
      });
      currentWrap.append(tagPicker);
      requestAnimationFrame(() => {
        focusVisualTagPickerInput(tagPicker);
      });
    }
    if (!selectedIsTransparent && visual.editPaletteOpen) {
      const editorPanel = document.createElement("span");
      editorPanel.className = "visual-current-editor-panel";
      const editMenu = renderVisualColorMenu({
        mode: "edit",
        customValue: selected.color,
      });
      editorPanel.append(editMenu);
      currentWrap.append(editorPanel);
      pendingEditMenu = editMenu;
    }
    currentWrap.append(visualShapeField);
    visualPalette.append(currentWrap);
    if (pendingEditMenu) {
      positionVisualColorMenu(pendingEditMenu, currentButton, { side: "left" });
    }
  }

  renderVisualPaletteGrid({
    target: visualPalette,
    leadingControl: visualMarkerTool,
    entries: visual.palette,
    selectedIndex: visual.selectedColorIndex,
    bucketActive: visualBucketActive,
    emptyTitle: "Paint transparent",
    emptyAriaLabel: "Paint transparent visual cell",
    colorAriaLabel: (index, name) => name ? `Paint color ${index}: ${name}` : `Paint color ${index}`,
    onSelect: (index) => {
      visualBucketActive = false;
      selectVisualColor(index);
    },
    onAdd: toggleVisualAddPalette,
    onRemove: deleteSelectedVisualColor,
    addOpen: visual.addPaletteOpen,
    renderAddMenu: () => renderVisualColorMenu({
      mode: "add",
      customValue: validVisualColorIndex(visual.addDraftColorIndex)
        ? visual.palette[visual.addDraftColorIndex].color
        : nextVisualPresetColor(),
    }),
  });

  renderVisualEditorToolbar({ dimension: "2d", target: visualToolbarHost });
}

const VISUAL_EDIT_COMMANDS = Object.freeze([
  Object.freeze({
    id: "copy",
    group: "clipboard",
    icon: "copy",
    label: "Copy",
    execute2d: () => copyVisualEditRegion(),
  }),
  Object.freeze({
    id: "cut",
    group: "clipboard",
    icon: "scissors",
    label: "Cut",
    execute2d: () => cutVisualEditRegion(),
  }),
  Object.freeze({
    id: "paste",
    group: "clipboard",
    icon: "clipboard-paste",
    label: "Paste into",
    execute2d: () => pasteVisualEditRegion(),
  }),
  Object.freeze({
    id: "delete",
    group: "clipboard",
    icon: "trash-2",
    label: "Delete",
    execute2d: () => deleteVisualEditRegion(),
  }),
]);

function visualEditCommandDefinition(command) {
  const definition = VISUAL_EDIT_COMMANDS.find((candidate) => candidate.id === command);
  if (!definition) {
    throw new Error(`Unknown visual edit command ${command}`);
  }
  return definition;
}

const VISUAL_EDITOR_TOOL_SCHEMA = Object.freeze([
  { key: "scope", group: "context" },
  { key: "grid", group: "context" },
  { key: "clip", group: "context" },
  { key: "fill", group: "paint" },
  { key: "translate", group: "paint" },
  { key: "rotate-left", group: "transform" },
  { key: "rotate-right", group: "transform" },
  { key: "flip-horizontal", group: "transform" },
  { key: "flip-vertical", group: "transform" },
  ...VISUAL_EDIT_COMMANDS.map(({ id, group }) => Object.freeze({ key: id, group })),
]);

function visualEditorToolbarParts(dimension) {
  const is3d = dimension === "3d";
  return {
    marker: visualMarkerTool,
    fill: is3d ? visual3dFillButton : visualFillButton,
    translate: is3d ? visual3dTranslateButton : renderVisualTranslateButton(),
    grid: visualGridButton,
    "rotate-left": is3d ? visual3dRotatePlaneLeftButton : visualRotateLeftButton,
    "rotate-right": is3d ? visual3dRotatePlaneRightButton : visualRotateRightButton,
    "flip-horizontal": is3d ? visual3dFlipPlaneHorizontalButton : visualFlipHorizontalButton,
    "flip-vertical": is3d ? visual3dFlipPlaneVerticalButton : visualFlipVerticalButton,
    copy: renderVisualEditCommandButton(dimension, "copy"),
    cut: renderVisualEditCommandButton(dimension, "cut"),
    paste: renderVisualEditCommandButton(dimension, "paste"),
    delete: renderVisualEditCommandButton(dimension, "delete"),
    scope: is3d ? document.querySelector(".visual3d-scope-toggle") : null,
    clip: is3d ? visual3dClipActions : renderVisualClipActions(),
  };
}

function renderVisualEditCommandButton(dimension, command) {
  const definition = visualEditCommandDefinition(command);
  const label = visualEditCommandLabel(dimension, command);
  const button = renderVisualClipButton({
    title: label,
    ariaLabel: label,
    danger: command === "delete",
    icon: visualLucideIconSvg(definition.icon),
  });
  button.classList.add("visual-edit-command-button", `is-${command}`);
  button.dataset.visualEditCommand = command;
  return button;
}

function visualEditTargetLabel(dimension) {
  if (dimension === "3d") {
    if (visual3dClipActive && visual3dClipSelection) return "selected 3D area";
    return visual3dEditScope() === "all" ? "whole 3D visual" : "current slice";
  }
  return visualClipActive && visualClipSelection ? "selected area" : "whole visual";
}

function visualEditCommandLabel(dimension, command) {
  const target = visualEditTargetLabel(dimension);
  return `${visualEditCommandDefinition(command).label} ${target}`;
}

function syncVisualEditCommandLabels(dimension) {
  const host = dimension === "3d" ? visual3dToolbarHost : visualToolbarHost;
  for (const button of host?.querySelectorAll?.("[data-visual-edit-command]") || []) {
    const label = visualEditCommandLabel(dimension, button.dataset.visualEditCommand);
    button.title = label;
    button.setAttribute("aria-label", label);
  }
}

function runVisualEditCommand(dimension, command) {
  const definition = visualEditCommandDefinition(command);
  if (dimension === "3d") {
    return runVisual3dEditCommand(definition.id);
  }
  if (visualClipActive && !normalizeVisualClipRect(visualClipSelection)) {
    setVisualActionStatus("Select a clip region first", "is-error");
    return false;
  }
  return definition.execute2d();
}

function renderVisualEditorToolbar({ dimension, target }) {
  if (!target) {
    throw new Error(`Missing ${dimension} visual toolbar host`);
  }
  const parts = visualEditorToolbarParts(dimension);
  const row = document.createElement("div");
  row.className = "visual-paint-tool-row visual-editor-toolbar";
  row.setAttribute("role", "toolbar");
  const context = document.createElement("span");
  context.className = "visual-paint-tool-group visual-context-actions";
  const paint = document.createElement("span");
  paint.className = "visual-paint-tool-group visual-scoped-paint-actions";
  const transform = document.createElement("span");
  transform.className = "visual-paint-tool-group visual-scoped-transform-actions";
  const clipboard = document.createElement("span");
  clipboard.className = "visual-paint-tool-group visual-scoped-clipboard-actions";
  const groups = { context, paint, transform, clipboard };
  for (const { key, group } of VISUAL_EDITOR_TOOL_SCHEMA) {
    const part = parts[key];
    if (!part) {
      continue;
    }
    groups[group].append(part);
  }
  row.append(context, paint, transform, clipboard);
  target.replaceChildren(row);
  return row;
}

function renderVisualTranslateButton() {
  const button = renderVisualClipButton({
    title: "Move",
    ariaLabel: "Move",
    active: visualTranslateActive,
    icon: visualLucideIconSvg("move"),
  });
  button.classList.add("visual-translate-button");
  button.dataset.tooltip = "Move";
  return button;
}

function renderVisualClipActions() {
  const clipActions = document.createElement("span");
  clipActions.className = "visual-clip-actions";
  const button = renderVisualClipButton({
    title: "Clip",
    ariaLabel: "Clip",
    active: visualClipActive,
    icon: visualLucideIconSvg("mouse-pointer-2"),
  });
  button.dataset.tooltip = "Clip";
  clipActions.append(button);
  return clipActions;
}

function visualPaletteEntryDisplayName(entry) {
  const bind = visualPaletteEntryBindInfo(entry);
  return bind.linked && bind.name ? bind.name : "";
}

function renderVisualCurrentColorTagButton({ state, entry, onToggle }) {
  const bind = visualPaletteEntryBindInfo(entry);
  const button = document.createElement("button");
  button.type = "button";
  button.className = "icon-button visual-current-tag-button visual-icon-button";
  button.classList.toggle("is-active", bind.linked);
  button.title = bind.name ? `Color tag: ${bind.name}` : "Tag selected color";
  button.setAttribute("aria-label", button.title);
  button.setAttribute("aria-pressed", String(bind.linked));
  button.setAttribute("aria-haspopup", "listbox");
  button.setAttribute("aria-expanded", String(Boolean(state.colorTagPickerOpen)));
  button.innerHTML = visualTagIconSvg();
  button.addEventListener("click", () => {
    const opening = !state.colorTagPickerOpen;
    state.colorTagPickerOpen = opening;
    onToggle(opening);
  });
  return button;
}

function renderVisualCurrentColorUnlinkButton(index, bind) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "icon-button is-danger visual-current-tag-unlink-button visual-icon-button";
  button.title = bind?.name ? `Unlink color tag ${bind.name}` : "Unlink color tag";
  button.setAttribute("aria-label", button.title);
  button.innerHTML = visualUnlinkIconSvg();
  button.addEventListener("click", (event) => {
    event.preventDefault();
    event.stopPropagation();
    visual.colorTagPickerOpen = false;
    clearVisualColorEditorState();
    toggleVisualPaletteEntryBinding(index);
  });
  return button;
}

function renderVisualAssetNamePicker({ className, names, value, placeholder, ariaLabel, emptyText, optionMeta, onCommit, onCancel }) {
  const picker = document.createElement("form");
  picker.className = ["visual-tag-picker", className || ""].filter(Boolean).join(" ");
  picker.noValidate = true;
  const input = document.createElement("input");
  input.type = "text";
  input.className = "visual-tag-picker-input";
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
  options.className = "visual-tag-options";
  options.setAttribute("role", "listbox");
  if (names.length) {
    for (const name of names) {
      const option = document.createElement("button");
      option.type = "button";
      option.className = "visual-tag-option";
      option.setAttribute("role", "option");
      const meta = typeof optionMeta === "function" ? optionMeta(name) : null;
      if (meta && Object.prototype.hasOwnProperty.call(meta, "color")) {
        const color = parseVisualHexColor(meta.color);
        if (color) {
          option.classList.add("has-color");
          option.style.setProperty("--visual-tag-option-color", color);
          option.style.setProperty("--visual-tag-option-ink", readableInkForColor(color));
          option.title = `${name} ${color}`;
          option.setAttribute("aria-label", `Use color tag ${name} ${color}`);
          const swatch = document.createElement("span");
          swatch.className = "visual-tag-option-swatch";
          swatch.setAttribute("aria-hidden", "true");
          const label = document.createElement("span");
          label.className = "visual-tag-option-name";
          label.textContent = name;
          const hexLabel = document.createElement("span");
          hexLabel.className = "visual-tag-option-value";
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
    empty.className = "visual-tag-empty";
    empty.textContent = emptyText;
    options.append(empty);
  }
  picker.append(input, options);
  return picker;
}

function focusVisualTagPickerInput(tagPicker) {
  const input = tagPicker.querySelector(".visual-tag-picker-input");
  if (!input) {
    return;
  }
  input.focus();
  input.select();
}

function visualColorAssetNames() {
  return [...visualSourceColorAssets().keys()].sort((a, b) => a.localeCompare(b));
}

function visualShapeAssetNames() {
  return [...visualSourceShapeAssets().keys()].sort((a, b) => a.localeCompare(b));
}

function activeVisualSourceContract() {
  return visual.sourceVisualContract && typeof visual.sourceVisualContract === "object"
    ? visual.sourceVisualContract
    : null;
}

function visualSourceColorAssets() {
  const assets = new Map();
  const contract = activeVisualSourceContract();
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

function visualSourceShapeAssets() {
  const assets = new Map();
  const contract = activeVisualSourceContract();
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
  if (!validVisualColorIndex(index)) {
    return false;
  }
  const entry = visual.palette[index];
  const name = sanitizeVisualColorAssetRef(rawName);
  if (!name) {
    if (options.reportError) {
      setVisualActionStatus("Enter a color name", "is-error");
    }
    return false;
  }
  const colorAssets = visualSourceColorAssets();
  let status = `Using color ${name}`;
  if (colorAssets.has(name)) {
    const resolved = colorAssets.get(name);
    if (!resolved) {
      if (options.reportError) {
        setVisualActionStatus(`Cannot resolve color ${name}`, "is-error");
      }
      return false;
    }
    entry.color = resolved;
  } else {
    if (name.includes(":")) {
      if (options.reportError) {
        setVisualActionStatus(`Cannot resolve color ${name}`, "is-error");
      }
      return false;
    }
    const staged = visual.palette.find((candidate, candidateIndex) => {
      const bind = visualPaletteEntryBindInfo(candidate);
      return candidateIndex !== index && bind.linked && bind.name === name;
    });
    if (staged) {
      entry.color = normalizeVisualColor(staged.color);
    }
    status = `Tagged color ${name}`;
  }
  entry.bind = { type: "color", name, linked: true };
  entry.editMode = "name";
  syncVisualPaletteEntriesForColorName(name, entry.color);
  setVisualActionStatus(status, "is-ok");
  renderVisualBuilder();
  return true;
}

function renderVisualBindToggle(entry, index, options = {}) {
  const bind = visualPaletteEntryBindInfo(entry);
  if (!bind.available) {
    return null;
  }
  const button = document.createElement("button");
  button.type = "button";
  button.className = ["icon-button", "visual-bind-toggle", options.className || ""].filter(Boolean).join(" ");
  button.classList.toggle("is-linked", bind.linked);
  button.classList.toggle("is-unlinked", !bind.linked);
  button.dataset.colorIndex = String(index);
  button.title = bind.linked ? `Unlink ${bind.label}` : (bind.name ? `Relink ${bind.label}` : "Link color");
  button.setAttribute("aria-label", bind.linked ? `Unlink color ${index}` : (bind.name ? `Relink color ${index}` : `Link color ${index}`));
  button.innerHTML = visualLinkIconSvg();
  button.addEventListener("click", (event) => {
    event.preventDefault();
    event.stopPropagation();
    toggleVisualPaletteEntryBinding(index);
  });
  return button;
}

function renderVisualBindMarker(entry) {
  const bind = visualPaletteEntryBindInfo(entry);
  if (!bind.available || !bind.linked) {
    return null;
  }
  const marker = document.createElement("span");
  marker.className = "visual-bind-marker is-linked";
  marker.title = bind.label;
  marker.setAttribute("aria-label", bind.label);
  marker.innerHTML = visualTagIconSvg();
  return marker;
}

function renderVisualAssetBindToggle({ bind, className, label, linkedTitle, unlinkedTitle, onClick }) {
  const info = visualAssetBindInfo(bind, label);
  const button = document.createElement("button");
  button.type = "button";
  button.className = ["icon-button", "visual-bind-toggle", "visual-asset-bind-toggle", className || ""].filter(Boolean).join(" ");
  button.classList.toggle("is-linked", info.linked);
  button.classList.toggle("is-unlinked", !info.linked);
  button.title = info.linked ? `${linkedTitle}: ${info.name}` : unlinkedTitle;
  button.setAttribute("aria-label", info.linked ? `${linkedTitle} ${label}` : unlinkedTitle);
  button.innerHTML = visualLinkIconSvg();
  button.addEventListener("click", (event) => {
    event.preventDefault();
    event.stopPropagation();
    onClick();
  });
  return button;
}

function toggleVisualPaletteEntryBinding(index) {
  if (!validVisualColorIndex(index)) {
    return;
  }
  const entry = visual.palette[index];
  const rawBind = entry?.bind ?? entry?.bound ?? entry?.sourceRef ?? null;
  if (!rawBind) {
    linkVisualPaletteEntryToNewColor(index);
    return;
  }
  if (typeof rawBind === "string") {
    entry.bind = { type: "color", name: rawBind, linked: false };
  } else {
    rawBind.linked = rawBind.linked === false || rawBind.unlinked === true || rawBind.detached === true;
    delete rawBind.unlinked;
    delete rawBind.detached;
  }
  visual.selectedColorIndex = index;
  const bind = visualPaletteEntryBindInfo(entry);
  void syncCurrentVisualDefinitionFromBuilder(bind.linked ? "Linked color" : "Unlinked color");
  renderVisualPalette();
  renderVisualColorSurfaces();
}

async function linkVisualPaletteEntryToNewColor(index) {
  const entry = visual.palette[index];
  if (!entry) {
    return;
  }
  const name = promptVisualAssetName("Color name", defaultVisualAssetName("color", index));
  if (!name) {
    return;
  }
  const previousBind = entry.bind ?? null;
  entry.bind = { type: "color", name, linked: true };
  if (!await syncCurrentVisualDefinitionFromBuilder(`Linked color ${name}`)) {
    entry.bind = previousBind;
    renderVisualPalette();
    return;
  }
  visual.selectedColorIndex = index;
  renderVisualBuilder();
}

async function syncCurrentVisualDefinitionFromBuilder(status = "") {
  try {
    await commitVisualEditorMutation({
      state: visual,
      request: () => visualEditMutationRequest("update"),
    });
  } catch (error) {
    setVisualActionStatus(userFacingRuntimeError(error), "is-error");
    setStatus(userFacingRuntimeError(error), "is-error");
    return false;
  }
  if (status) {
    setVisualActionStatus(status, "is-ok");
    setStatus(status, "is-ok");
  }
  syncVisualSourceActionButtons();
  return true;
}

function updateVisualBoundColorDefinition(entry, color) {
  const bind = visualPaletteEntryBindInfo(entry);
  if (!bind.linked || !bind.name) {
    return false;
  }
  void syncCurrentVisualDefinitionFromBuilder().then((applied) => {
    if (!applied) return;
    syncVisualPaletteEntriesForColorName(bind.name, color);
  });
  return true;
}

function toggleVisualShapeBinding() {
  const info = visualAssetBindInfo(visual.shapeBind, "shape");
  if (!info.name) {
    linkVisualShapeToNewShape();
    return;
  }
  visual.shapeBind = { type: "shape", name: info.name, linked: !info.linked };
  void syncCurrentVisualDefinitionFromBuilder(visual.shapeBind.linked ? "Linked shape" : "Unlinked shape");
  renderVisualBuilder();
}

async function linkVisualShapeToNewShape() {
  const name = promptVisualShapeAssetName("Shape name", defaultVisualAssetName("shape"));
  if (!name) {
    return;
  }
  const previousBind = visual.shapeBind;
  visual.shapeBind = { type: "shape", name, linked: true };
  if (!await syncCurrentVisualDefinitionFromBuilder(`Linked shape ${name}`)) {
    visual.shapeBind = previousBind;
    renderVisualBuilder();
    return;
  }
  renderVisualBuilder();
}

function updateVisualBoundShapeDefinition() {
  const info = visualAssetBindInfo(visual.shapeBind, "shape");
  if (!info.linked || !info.name) {
    return false;
  }
  void syncCurrentVisualDefinitionFromBuilder();
  return true;
}

function promptVisualAssetName(label, defaultValue) {
  let raw = defaultValue;
  try {
    raw = window.prompt(label, defaultValue);
  } catch {
    raw = defaultValue;
  }
  if (raw === null) {
    return null;
  }
  const name = sanitizeVisualAssetName(raw);
  if (!name) {
    setVisualActionStatus("Use an asset name like wall_color", "is-error");
    return null;
  }
  return name;
}

function promptVisualShapeAssetName(label, defaultValue) {
  let raw = defaultValue;
  try {
    raw = window.prompt(label, defaultValue);
  } catch {
    raw = defaultValue;
  }
  if (raw === null) {
    return null;
  }
  const name = sanitizeVisualShapeRef(raw);
  if (!name) {
    setVisualActionStatus("Use a shape name like wall-shape or shape:tag", "is-error");
    return null;
  }
  return name;
}

function sanitizeVisualAssetName(value) {
  const cleaned = String(value || "")
    .trim()
    .replace(/[^\w]+/g, "_")
    .replace(/^_+|_+$/g, "");
  if (!cleaned) {
    return "";
  }
  return /^[A-Za-z_]/.test(cleaned) ? cleaned : `color_${cleaned}`;
}

function sanitizeVisualColorAssetRef(value) {
  const raw = String(value || "").trim();
  if (!raw.includes(":")) {
    return sanitizeVisualAssetName(raw);
  }
  const parts = raw.split(":");
  if (parts.length !== 2) {
    return "";
  }
  const tableName = sanitizeVisualAssetName(parts[0]);
  const rowName = sanitizeVisualAssetName(parts[1]);
  return tableName && rowName ? `${tableName}:${rowName}` : "";
}

function sanitizeVisualShapeRef(value) {
  const raw = String(value || "").trim();
  if (!raw || /[\s{}#]/.test(raw)) {
    return "";
  }
  if (!raw.includes(":")) {
    return isVisualPlainShapeName(raw) ? raw : "";
  }
  const parts = raw.split(":");
  if (parts.length !== 2) {
    return "";
  }
  return isVisualShapeTableRef(parts[0], parts[1]) ? raw : "";
}

function isVisualPlainShapeName(value) {
  return /^[A-Za-z_][A-Za-z0-9_+*()/-]*$/.test(String(value || ""));
}

function isVisualShapeTableRef(tableName, valueName) {
  return /^[A-Za-z_]\w*$/.test(String(tableName || ""))
    && /^[A-Za-z0-9_+*()/-]+$/.test(String(valueName || ""));
}

function defaultVisualAssetName(kind, index = 0) {
  if (kind === "color") {
    const objectName = String(visualObjectName()).split(":")[0];
    const base = sanitizeVisualAssetName(objectName) || "visual";
    return `${base}_${Number(index) + 1}`;
  }
  const base = sanitizeVisualAssetName(visualObjectName()).replace(new RegExp(`_${kind}$`), "") || "visual";
  return `${base}_${kind}_${Number(index) + 1}`;
}

function syncVisualPaletteEntriesForColorName(name, color) {
  const normalized = normalizeVisualColor(color);
  for (const entry of visual.palette) {
    const bind = visualPaletteEntryBindInfo(entry);
    if (bind.linked && bind.name === name) {
      entry.color = normalized;
    }
  }
}

function activeVisualEditDocument() {
  return visualEditorOwnedDocument(visual, { allowActive: true }) || activePreviewDocument();
}

function setVisualEditSource(entry, document = activeDocument()) {
  const normalized = Number.isInteger(entry?.start) ? entry : { ...entry, start: entry?.openIndex };
  setVisualEditorSourceTarget(visual, normalized, document);
}

function clearVisualEditSource() {
  clearVisualEditorSourceTarget(visual);
}

function invalidateVisualEditSourceForDocument(document = activeDocument()) {
  if (!document || !visual.editDocumentId || document.id !== visual.editDocumentId) {
    return false;
  }
  return invalidateVisualEditorSourceTarget(visual, document);
}

function activeVisualEditSource() {
  return visualEditorSourceSnapshot(visual, { allowActive: true }).source;
}

function visualLinkIconSvg() {
  return `
    ${editorIconSvg("link")}
  `;
}

function visualUnlinkIconSvg() {
  return `
    ${editorIconSvg("tag-x")}
  `;
}

function visualTagIconSvg() {
  return `
    ${editorIconSvg("tag")}
  `;
}

function positionVisualColorMenu(menu, anchor, options = {}) {
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

function renderVisualColorMenu({
  mode,
  customValue,
  inline = false,
  onPreset = null,
  onChange = null,
  onDiscard = cancelVisualColorAdd,
  renderPalette = renderVisualPalette,
}) {
  const presetList = document.createElement("span");
  presetList.className = [
    "visual-color-menu",
    "is-adjuster",
    inline ? "is-inline-custom" : "",
  ].filter(Boolean).join(" ");

  const presetGrid = document.createElement("span");
  presetGrid.className = "visual-preset-grid";
  for (const color of VISUAL_COLOR_PRESETS) {
    const preset = document.createElement("button");
    preset.type = "button";
    preset.className = "visual-color-preset visual-color-swatch";
    preset.classList.toggle("is-selected", normalizeVisualColor(color) === normalizeVisualColor(customValue));
    preset.style.setProperty("--visual-swatch-color", normalizeVisualColor(color));
    preset.title = mode === "add" ? `Start from ${color}` : `Use ${color}`;
    preset.setAttribute("aria-label", mode === "add" ? `Start from color ${color}` : `Use color ${color}`);
    preset.addEventListener("click", () => {
      if (onPreset) {
        onPreset(color, { deferHistory: true });
      } else if (mode === "add") {
        previewNewVisualColor(color, { deferHistory: true });
      } else {
        updateSelectedVisualColor(color, { deferHistory: true });
      }
      renderPalette();
    });
    presetGrid.append(preset);
  }
  presetList.append(presetGrid);
  presetList.append(renderVisualColorAdjuster({
    color: customValue,
    ariaLabel: mode === "add" ? "New color" : "Selected color",
    onChange: (color) => {
      if (onChange) {
        onChange(color, { deferHistory: true });
      } else if (mode === "add") {
        previewNewVisualColor(color, { deferHistory: true });
      } else {
        updateSelectedVisualColor(color, { deferHistory: true });
      }
    },
  }));
  const actionRow = document.createElement("span");
  actionRow.className = "visual-color-actions";
  if (mode === "add") {
    actionRow.classList.add("is-floating");
    const discardButton = document.createElement("button");
    discardButton.type = "button";
    discardButton.className = "icon-button is-danger visual-color-action-button visual-color-trash-button";
    discardButton.title = "Discard new color";
    discardButton.setAttribute("aria-label", "Discard new color");
    discardButton.innerHTML = visualTrashIconSvg();
    discardButton.addEventListener("click", onDiscard);
    actionRow.append(discardButton);
  } else {
    actionRow.hidden = true;
  }
  presetList.append(actionRow);
  return presetList;
}

function visualTrashIconSvg() {
  return `
    ${editorIconSvg("trash-2")}
  `;
}

function visualLucideIconSvg(name) {
  return editorIconSvg(name);
}

function toggleVisualTranslateMode() {
  if (visualTranslateActive) {
    deactivateVisualTranslateMode();
    return;
  }
  visualBucketActive = false;
  deactivateVisualClipMode({ render: false });
  visualTranslateActive = true;
  visualTranslateDrag = null;
  renderVisualBuilder();
  setVisualActionStatus("Translate: drag the visual", "is-ok");
}

function deactivateVisualTranslateMode(options = {}) {
  const wasActive = visualTranslateActive || visualTranslateDrag;
  if (visualTranslateDrag && visualBoard.hasPointerCapture?.(visualTranslateDrag.pointerId)) {
    visualBoard.releasePointerCapture(visualTranslateDrag.pointerId);
  }
  visualTranslateActive = false;
  visualTranslateDrag = null;
  if (options.render === false || !wasActive) {
    return;
  }
  renderVisualBuilder();
  setVisualActionStatus(visualPaintToolStatusText(), "is-ok");
}

function visualPositiveModulo(value, size) {
  return ((value % size) + size) % size;
}

function translatedVisualCells(cells, dx, dy) {
  const next = Array.from({ length: visualFrameCellCount() }, () => null);
  for (let y = 0; y < visual.height; y += 1) {
    for (let x = 0; x < visual.width; x += 1) {
      const targetX = visualPositiveModulo(x + dx, visual.width);
      const targetY = visualPositiveModulo(y + dy, visual.height);
      next[targetY * visual.width + targetX] = cells[y * visual.width + x];
    }
  }
  return next;
}

function visualCellsEqual(left, right) {
  return left.length === right.length && left.every((value, index) => value === right[index]);
}

function startVisualTranslate(event, geometry) {
  event.preventDefault();
  visualTranslateDrag = {
    pointerId: event.pointerId,
    startClientX: event.clientX,
    startClientY: event.clientY,
    geometry,
    originCells: [...visual.cells],
    beforeSnapshot: visualEditSnapshot("visual"),
  };
  visualBoard.setPointerCapture?.(event.pointerId);
  visualBoard.classList.add("is-translating");
}

function continueVisualTranslate(event) {
  if (!visualTranslateDrag || visualTranslateDrag.pointerId !== event.pointerId) {
    return false;
  }
  event.preventDefault();
  const cellWidth = visualTranslateDrag.geometry.width / visual.width;
  const cellHeight = visualTranslateDrag.geometry.height / visual.height;
  const dx = Math.round((event.clientX - visualTranslateDrag.startClientX) / cellWidth);
  const dy = Math.round((event.clientY - visualTranslateDrag.startClientY) / cellHeight);
  visual.cells = translatedVisualCells(visualTranslateDrag.originCells, dx, dy);
  if (visual.animationMode) {
    visual.animationFrames[visual.animationFrameIndex] = visual.cells;
  }
  renderVisualBoard();
  visualBoard.classList.add("is-translating");
  return true;
}

function stopVisualTranslate(event) {
  if (!visualTranslateDrag || visualTranslateDrag.pointerId !== event.pointerId) {
    return false;
  }
  if (visualBoard.hasPointerCapture?.(event.pointerId)) {
    visualBoard.releasePointerCapture(event.pointerId);
  }
  const drag = visualTranslateDrag;
  visualTranslateDrag = null;
  visualBoard.classList.remove("is-translating");
  if (!visualCellsEqual(visual.cells, drag.originCells)) {
    updateVisualBoundShapeDefinition();
    syncVisualSourceActionButtons();
    pushVisualEditUndoSnapshot("visual", drag.beforeSnapshot);
  }
  return true;
}

function renderVisualClipButton({ title, ariaLabel, icon, active = false, disabled = false, danger = false, onClick }) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "icon-button visual-icon-button visual-clip-button";
  button.classList.toggle("is-active", active);
  button.classList.toggle("is-danger", danger);
  button.disabled = Boolean(disabled);
  button.title = title;
  button.setAttribute("aria-label", ariaLabel);
  button.setAttribute("aria-pressed", String(active));
  button.innerHTML = icon;
  if (typeof onClick === "function") {
    button.addEventListener("click", onClick);
  }
  return button;
}

function toggleVisualClipMode() {
  if (visualClipActive) {
    deactivateVisualClipMode();
    setVisualActionStatus(visualPaintToolStatusText(), "is-ok");
    return;
  }
  visualBucketActive = false;
  visualClipActive = true;
  visualClipSelection = normalizeVisualClipRect(visualClipSelection);
  visualClipDrag = null;
  renderVisualBuilder();
  setVisualActionStatus(
    visualClipSelection ? "Clip: drag selection to move it" : "Clip: drag to select visual area",
    "is-ok",
  );
}

function deactivateVisualClipMode(options = {}) {
  const wasActive = visualClipActive || visualClipSelection || visualClipDrag || visualClipFloating;
  const clearSelection = options.clearSelection !== false;
  visualClipActive = false;
  if (clearSelection) {
    visualClipSelection = null;
  } else {
    visualClipSelection = normalizeVisualClipRect(visualClipSelection);
  }
  visualClipDrag = null;
  visualClipFloating = null;
  if (options.render === false || !wasActive) {
    return;
  }
  renderVisualBuilder();
}

function normalizeVisualClipRect(rect) {
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
  if (x < 0 || y < 0 || x + width > visual.width || y + height > visual.height) {
    return null;
  }
  return { x, y, width, height };
}

function visualClipRectFromCells(start, end) {
  return normalizeVisualClipRect({
    x: Math.min(start.x, end.x),
    y: Math.min(start.y, end.y),
    width: Math.abs(end.x - start.x) + 1,
    height: Math.abs(end.y - start.y) + 1,
  });
}

function visualClipSelectionContainsCell(cell, rect = visualClipSelection) {
  const normalized = normalizeVisualClipRect(rect);
  return Boolean(
    normalized
    && cell
    && cell.x >= normalized.x
    && cell.x < normalized.x + normalized.width
    && cell.y >= normalized.y
    && cell.y < normalized.y + normalized.height
  );
}

function visualClipRectContainsIndex(rect, index) {
  const normalized = normalizeVisualClipRect(rect);
  if (!normalized || !Number.isInteger(index) || index < 0) {
    return false;
  }
  const x = index % visual.width;
  const y = Math.floor(index / visual.width);
  return x >= normalized.x
    && x < normalized.x + normalized.width
    && y >= normalized.y
    && y < normalized.y + normalized.height;
}

function visualClipCellFromClient(clientX, clientY, geometry = visualBoardGeometry()) {
  if (geometry.width <= 0 || geometry.height <= 0) {
    return null;
  }
  return {
    x: Math.max(0, Math.min(visual.width - 1, Math.floor(((clientX - geometry.left) / geometry.width) * geometry.columns))),
    y: Math.max(0, Math.min(visual.height - 1, Math.floor(((clientY - geometry.top) / geometry.height) * geometry.rows))),
  };
}

function visualClipRectCells(rect) {
  const normalized = normalizeVisualClipRect(rect);
  if (!normalized) {
    return [];
  }
  const cells = [];
  for (let y = 0; y < normalized.height; y += 1) {
    for (let x = 0; x < normalized.width; x += 1) {
      const index = (normalized.y + y) * visual.width + normalized.x + x;
      const colorIndex = visual.cells[index];
      cells.push(validVisualColorIndex(colorIndex) ? colorIndex : null);
    }
  }
  return cells;
}

function pasteVisualClipCell(index, clipboardValue) {
  if (clipboardValue === null) {
    return false;
  }
  if (!validVisualColorIndex(clipboardValue)) {
    throw new Error(`Invalid visual clip palette index ${clipboardValue}`);
  }
  return setVisualCellColorAtIndex(index, clipboardValue);
}

function visualClipCellsForCurrentPalette(clipboard) {
  if (!Array.isArray(clipboard?.colors)) return clipboard?.cells;
  const colorToIndex = new Map(visual.palette.map((entry, index) => [normalizeVisualColor(entry.color), index]));
  const sourceToTarget = clipboard.colors.map((rawColor) => {
    const color = normalizeVisualColor(rawColor);
    if (color === "#00000000") return null;
    if (!colorToIndex.has(color)) {
      if (visual.palette.length >= VISUAL_COLOR_TOKENS.length) {
        throw new Error("Paste needs more colors than the visual palette can hold");
      }
      colorToIndex.set(color, visual.palette.length);
      visual.palette.push({ color });
    }
    return colorToIndex.get(color);
  });
  return clipboard.cells.map((value) => value === null ? null : sourceToTarget[value]);
}

function setVisualClipRectCells(rect, cells) {
  const normalized = normalizeVisualClipRect(rect);
  if (!normalized || !Array.isArray(cells) || cells.length !== normalized.width * normalized.height) {
    return [];
  }
  const changedIndices = [];
  for (let y = 0; y < normalized.height; y += 1) {
    for (let x = 0; x < normalized.width; x += 1) {
      const index = (normalized.y + y) * visual.width + normalized.x + x;
      const next = cells[y * normalized.width + x];
      if (pasteVisualClipCell(index, next)) {
        changedIndices.push(index);
      }
    }
  }
  return changedIndices;
}

function clearVisualClipRect(rect) {
  const normalized = normalizeVisualClipRect(rect);
  if (!normalized) {
    return [];
  }
  const changedIndices = [];
  for (let y = normalized.y; y < normalized.y + normalized.height; y += 1) {
    for (let x = normalized.x; x < normalized.x + normalized.width; x += 1) {
      const index = y * visual.width + x;
      if (setVisualCellColorAtIndex(index, null)) {
        changedIndices.push(index);
      }
    }
  }
  return changedIndices;
}

function commitVisualClipMutation(before, changedIndices, message) {
  if (!changedIndices.length) {
    setVisualActionStatus("Clip did not change visual", "is-ok");
    renderVisualBuilder();
    return false;
  }
  visual.solidSource = false;
  updateVisualBoundShapeDefinition();
  renderVisualBuilder();
  syncVisualSourceActionButtons();
  setVisualActionStatus(message, "is-ok");
  setStatus(message, "is-ok");
  pushVisualEditUndoSnapshot("visual", before);
  return true;
}

function deleteVisualClipSelection() {
  if (visualClipFloating) {
    visualClipFloating = null;
    visualClipSelection = null;
    visualClipDrag = null;
    renderVisualBuilder();
    setVisualActionStatus("Clip preview discarded", "is-ok");
    return true;
  }
  const rect = normalizeVisualClipRect(visualClipSelection);
  if (!rect) {
    setVisualActionStatus("No clip selection", "is-error");
    return false;
  }
  const before = visualEditSnapshot("visual");
  const changedIndices = clearVisualClipRect(rect);
  return commitVisualClipMutation(before, changedIndices, "Deleted selected area");
}

function pasteVisualClipClipboard() {
  if (!visualClipClipboard) {
    setVisualActionStatus("No copied clip", "is-error");
    return false;
  }
  if (visualClipClipboard.width > visual.width || visualClipClipboard.height > visual.height) {
    setVisualActionStatus("Copied clip is larger than visual", "is-error");
    return false;
  }
  const base = normalizeVisualClipRect(visualClipSelection) || { x: 0, y: 0, width: 1, height: 1 };
  const rect = normalizeVisualClipRect({
    x: base.x,
    y: base.y,
    width: visualClipClipboard.width,
    height: visualClipClipboard.height,
  });
  if (!rect) {
    setVisualActionStatus("Copied clip does not fit at selection", "is-error");
    return false;
  }
  const before = visualEditSnapshot("visual");
  let cells;
  try {
    cells = visualClipCellsForCurrentPalette(visualClipClipboard);
  } catch (error) {
    setVisualActionStatus(error?.message || String(error), "is-error");
    return false;
  }
  const changedIndices = setVisualClipRectCells(rect, cells);
  visualClipActive = true;
  visualClipSelection = rect;
  visualClipFloating = null;
  commitVisualClipMutation(before, changedIndices, `Pasted ${rect.width}x${rect.height} clip`);
  setVisualActionStatus(`Pasted ${rect.width}x${rect.height} clip`, "is-ok");
  return true;
}

function visualWholeEditRect() {
  return { x: 0, y: 0, width: visual.width, height: visual.height };
}

function visualEditRect() {
  return visualClipActive ? normalizeVisualClipRect(visualClipSelection) : visualWholeEditRect();
}

function visualClipboardTextForRect(rect) {
  const cells = visualClipRectCells(rect);
  const rows = [];
  for (let y = 0; y < rect.height; y += 1) {
    rows.push(cells.slice(y * rect.width, (y + 1) * rect.width).map(visualExportCharForColorIndex).join(""));
  }
  return [
    `colors = ${visualPaletteSourceTokens().join(" ")}`,
    "shape = {",
    ...rows,
    "}",
  ].join("\n");
}

async function copyVisualEditRegion() {
  const rect = visualEditRect();
  if (!rect) return false;
  visualClipClipboard = {
    dimension: "2d",
    width: rect.width,
    height: rect.height,
    cells: visualClipRectCells(rect),
    colors: visual.palette.map((entry) => normalizeVisualColor(entry.color)),
  };
  try {
    await copyTextToClipboard(visualClipboardTextForRect(rect));
  } catch (error) {
    setVisualActionStatus(`Copy failed: ${error?.message || error}`, "is-error");
    return false;
  }
  renderVisualBuilder();
  setVisualActionStatus(`Copied ${rect.width}x${rect.height} edit region`, "is-ok");
  return true;
}

async function cutVisualEditRegion() {
  const rect = visualEditRect();
  if (!rect) return false;
  try {
    if (!await copyVisualEditRegion()) return false;
  } catch (error) {
    setVisualActionStatus(`Copy failed; visual was not cut: ${error?.message || error}`, "is-error");
    return false;
  }
  const before = visualEditSnapshot("visual");
  return commitVisualClipMutation(before, clearVisualClipRect(rect), `Cut ${rect.width}x${rect.height} edit region`);
}

function pasteVisualEditRegion() {
  if (!visualClipClipboard) {
    setVisualActionStatus("No copied visual region", "is-error");
    return false;
  }
  const wasClipActive = visualClipActive;
  if (!wasClipActive) visualClipSelection = { x: 0, y: 0, width: 1, height: 1 };
  const result = pasteVisualClipClipboard();
  if (!wasClipActive) {
    visualClipActive = false;
    visualClipSelection = null;
    visualClipFloating = null;
    renderVisualBuilder();
  }
  return result;
}

function deleteVisualEditRegion() {
  if (!visualClipActive) {
    deleteWholeVisualRegion();
    return true;
  }
  return deleteVisualClipSelection();
}

function moveVisualClipRange(target, message = "Moved clip range") {
  visualClipSelection = target;
  renderVisualBuilder();
  setVisualActionStatus(message, "is-ok");
}

function visualClipFloatingRectAtCell(cell) {
  if (!cell || !visualClipClipboard) {
    return null;
  }
  const width = Math.min(visual.width, visualClipClipboard.width);
  const height = Math.min(visual.height, visualClipClipboard.height);
  return normalizeVisualClipRect({
    x: Math.max(0, Math.min(visual.width - width, cell.x)),
    y: Math.max(0, Math.min(visual.height - height, cell.y)),
    width,
    height,
  });
}

function visualClipFloatingCellsForSelection(rect) {
  const normalized = normalizeVisualClipRect(rect);
  if (!normalized || !visualClipFloating || !visualClipClipboard) {
    return null;
  }
  if (visualClipClipboard.width !== normalized.width || visualClipClipboard.height !== normalized.height) {
    return null;
  }
  if (!Array.isArray(visualClipClipboard.cells) || visualClipClipboard.cells.length !== normalized.width * normalized.height) {
    return null;
  }
  return visualClipClipboard.cells;
}

function renderVisualBoard() {
  withVisual2dPaneScrollPreserved(() => renderVisualBoardContent());
}

function renderVisualBoardContent() {
  visualBoard.style.setProperty("--visual-size", Math.max(visual.width, visual.height));
  visualBoard.style.setProperty("--visual-cols", visual.width);
  visualBoard.style.setProperty("--visual-rows", visual.height);
  visualBoard.classList.toggle("is-translate-active", visualTranslateActive);
  syncVisualGridVisibility();
  visualBoard.classList.toggle("is-clip-active", visualClipActive);
  visualBoard.classList.toggle("is-clip-floating", Boolean(visualClipActive && visualClipFloating && visualClipClipboard));
  visualClipSelection = normalizeVisualClipRect(visualClipSelection);
  const nextBoard = document.createDocumentFragment();
  for (let index = 0; index < visual.cells.length; index += 1) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "visual-cell visual-color-swatch";
    syncVisualCellElement(button, index);
    nextBoard.append(button);
  }
  renderVisualClipSelectionFrame(nextBoard);
  visualBoard.replaceChildren(nextBoard);
  renderVisualAnimationSurfaces();
}

function syncVisualGridVisibility() {
  if (!visualBoard) {
    return;
  }
  visualBoard.classList.toggle("is-grid-hidden", !visualGridVisible);
  syncVisualGridButton();
}

function syncVisualGridButton() {
  if (!visualGridButton) {
    return;
  }
  visualGridButton.classList.toggle("is-active", visualGridVisible);
  visualGridButton.setAttribute("aria-pressed", visualGridVisible ? "true" : "false");
  visualGridButton.title = "Toggle grid";
  visualGridButton.setAttribute("aria-label", "Toggle visual grid");
}

function toggleVisualGrid() {
  visualGridVisible = !visualGridVisible;
  syncVisualGridVisibility();
  setVisualActionStatus(visualGridVisible ? "Visual grid visible" : "Visual grid hidden", "is-ok");
}

function renderVisualClipSelectionFrame(target = visualBoard) {
  const rect = normalizeVisualClipRect(visualClipSelection);
  if (!rect) {
    return;
  }
  renderVisualClipFloatingPreview(rect, target);
  const frame = document.createElement("div");
  frame.className = "visual-clip-selection-frame";
  frame.style.setProperty("--visual-clip-x", String(rect.x));
  frame.style.setProperty("--visual-clip-y", String(rect.y));
  frame.style.setProperty("--visual-clip-width", String(rect.width));
  frame.style.setProperty("--visual-clip-height", String(rect.height));
  frame.setAttribute("aria-hidden", "true");
  if (!visualClipFloating) {
    for (const edge of ["n", "e", "s", "w"]) {
      const node = document.createElement("span");
      node.className = `visual-clip-selection-edge visual-clip-selection-edge-${edge}`;
      node.dataset.visualClipResize = edge;
      frame.append(node);
    }
  }
  for (const handle of ["nw", "ne", "sw", "se"]) {
    const node = document.createElement("span");
    node.className = `visual-clip-selection-handle visual-clip-selection-handle-${handle}`;
    if (!visualClipFloating) {
      node.dataset.visualClipResize = handle;
    }
    frame.append(node);
  }
  target.append(frame);
}

function renderVisualClipFloatingPreview(rect, target = visualBoard) {
  const cells = visualClipFloatingCellsForSelection(rect);
  if (!cells) {
    return;
  }
  const preview = document.createElement("div");
  preview.className = `visual-clip-floating-preview is-${visualClipFloating.kind || "copy"}`;
  preview.style.setProperty("--visual-clip-x", String(rect.x));
  preview.style.setProperty("--visual-clip-y", String(rect.y));
  preview.style.setProperty("--visual-clip-width", String(rect.width));
  preview.style.setProperty("--visual-clip-height", String(rect.height));
  preview.style.setProperty("--visual-clip-preview-cols", String(rect.width));
  preview.setAttribute("aria-hidden", "true");
  for (const colorIndex of cells) {
    const cell = document.createElement("span");
    const validIndex = validVisualColorIndex(colorIndex) ? colorIndex : null;
    cell.className = "visual-clip-preview-cell visual-color-swatch";
    cell.dataset.colorIndex = validIndex === null ? "erase" : String(validIndex);
    cell.style.setProperty("--visual-swatch-color", visualColorForColorIndex(validIndex));
    cell.style.setProperty("--visual-cell-ink", visualInkForColorIndex(validIndex));
    cell.style.setProperty("--visual-puzzle-line", visualGridLineForColorIndex(validIndex));
    preview.append(cell);
  }
  target.append(preview);
}

function syncVisualCellElement(button, index) {
  const colorIndex = validVisualColorIndex(visual.cells[index]) ? visual.cells[index] : null;
  const char = visualExportCharForColorIndex(colorIndex);
  const isClipSelected = visualClipRectContainsIndex(visualClipSelection, index);
  button.dataset.index = String(index);
  button.dataset.colorIndex = colorIndex === null ? "erase" : String(colorIndex);
  button.classList.toggle("is-clip-selected", isClipSelected);
  button.style.setProperty("--visual-swatch-color", visualColorForColorIndex(colorIndex));
  button.style.setProperty("--visual-cell-ink", visualInkForColorIndex(colorIndex));
  button.style.setProperty("--visual-puzzle-line", visualGridLineForColorIndex(colorIndex));
  button.setAttribute("aria-label", `Visual cell ${index + 1}: ${char}`);
}

function renderVisualCellsAtIndices(indices) {
  for (const index of new Set(indices)) {
    const cell = visualBoard.children[index];
    if (!cell || !cell.classList.contains("visual-cell") || cell.dataset.index !== String(index)) {
      throw new Error(`Visual cell element missing for changed cell ${index}`);
    }
    syncVisualCellElement(cell, index);
  }
}

function selectVisualColor(index) {
  commitVisualColorEditHistory("visual");
  const wasClipActive = visualClipActive || visualClipSelection;
  deactivateVisualClipMode({ render: false });
  visual.selectedColorIndex = validVisualColorIndex(index) ? index : null;
  if (validVisualColorIndex(visual.selectedColorIndex)) {
    visualLastPaintColorIndex = visual.selectedColorIndex;
  }
  visual.addPaletteOpen = false;
  visual.editPaletteOpen = false;
  visual.customColorOpen = false;
  visual.addDraftColorIndex = null;
  renderVisualControls();
  renderVisualPalette();
  if (wasClipActive) {
    renderVisualBoard();
  }
}

function updateSelectedVisualColor(value, options = {}) {
  const before = options.deferHistory || options.commitHistory ? null : visualEditSnapshot("visual");
  if (options.deferHistory || options.commitHistory) {
    beginVisualColorEditHistory("visual");
  }
  if (!validVisualColorIndex(visual.selectedColorIndex)) {
    visual.selectedColorIndex = 0;
  }
  const selected = visual.palette[visual.selectedColorIndex];
  if (!selected) {
    return;
  }
  const normalized = normalizeVisualColor(value);
  selected.color = normalized;
  updateVisualBoundColorDefinition(selected, normalized);
  if (options.closeMenu) {
    visual.editPaletteOpen = false;
    visual.customColorOpen = false;
    visual.addDraftColorIndex = null;
    renderVisualBuilder();
    if (options.deferHistory || options.commitHistory) {
      commitVisualColorEditHistory("visual");
    } else {
      pushVisualEditUndoSnapshot("visual", before);
    }
    return;
  }
  renderVisualColorSurfaces();
  if (options.deferHistory) {
    return;
  }
  if (options.commitHistory) {
    commitVisualColorEditHistory("visual");
    return;
  }
  pushVisualEditUndoSnapshot("visual", before);
}

function toggleVisualAddPalette() {
  commitVisualColorEditHistory("visual");
  const before = visualEditSnapshot("visual");
  const opening = !visual.addPaletteOpen;
  if (opening && visual.palette.length >= VISUAL_COLOR_TOKENS.length) {
    setVisualActionStatus(`Palette limit is ${VISUAL_COLOR_TOKENS.length} colors`, "is-error");
    return;
  }
  visual.addPaletteOpen = opening;
  visual.editPaletteOpen = false;
  visual.customColorOpen = opening;
  if (opening) {
    if (!validVisualColorIndex(visual.addDraftColorIndex)) {
      visual.palette.push({ color: normalizeVisualColor(nextVisualPresetColor()) });
      visual.addDraftColorIndex = visual.palette.length - 1;
    }
    visual.selectedColorIndex = visual.addDraftColorIndex;
    renderVisualBuilder();
    pushVisualEditUndoSnapshot("visual", before);
    return;
  }
  visual.addDraftColorIndex = null;
  renderVisualBuilder();
  pushVisualEditUndoSnapshot("visual", before);
}

function addVisualColor(color = nextVisualPresetColor()) {
  const before = visualEditSnapshot("visual");
  const draftIndex = validVisualColorIndex(visual.addDraftColorIndex) ? visual.addDraftColorIndex : null;
  if (draftIndex === null && visual.palette.length >= VISUAL_COLOR_TOKENS.length) {
    setVisualActionStatus(`Palette limit is ${VISUAL_COLOR_TOKENS.length} colors`, "is-error");
    return;
  }
  if (draftIndex === null) {
    visual.palette.push({ color: normalizeVisualColor(color) });
    visual.selectedColorIndex = visual.palette.length - 1;
  } else {
    visual.palette[draftIndex].color = normalizeVisualColor(color);
    visual.selectedColorIndex = draftIndex;
  }
  visual.addPaletteOpen = false;
  visual.editPaletteOpen = false;
  visual.customColorOpen = false;
  visual.addDraftColorIndex = null;
  renderVisualBuilder();
  pushVisualEditUndoSnapshot("visual", before);
}

function previewNewVisualColor(color, options = {}) {
  const before = options.deferHistory ? null : visualEditSnapshot("visual");
  if (options.deferHistory) {
    beginVisualColorEditHistory("visual");
  }
  if (!validVisualColorIndex(visual.addDraftColorIndex) && visual.palette.length >= VISUAL_COLOR_TOKENS.length) {
    return;
  }
  if (!validVisualColorIndex(visual.addDraftColorIndex)) {
    visual.palette.push({ color: normalizeVisualColor(color) });
    visual.addDraftColorIndex = visual.palette.length - 1;
    visual.selectedColorIndex = visual.addDraftColorIndex;
    renderVisualBuilder();
  } else {
    visual.palette[visual.addDraftColorIndex].color = normalizeVisualColor(color);
    visual.selectedColorIndex = visual.addDraftColorIndex;
    renderVisualColorSurfaces();
  }
  if (options.closeMenu) {
    visual.addPaletteOpen = false;
    visual.editPaletteOpen = false;
    visual.customColorOpen = false;
    visual.addDraftColorIndex = null;
    renderVisualBuilder();
  }
  if (options.deferHistory) {
    return;
  }
  pushVisualEditUndoSnapshot("visual", before);
}

function closeVisualColorEditor() {
  clearVisualColorEditorState();
  renderVisualPalette();
}

function confirmVisualColorAdd() {
  if (!validVisualColorIndex(visual.addDraftColorIndex)) {
    return;
  }
  commitVisualColorEditHistory("visual");
  visual.selectedColorIndex = visual.addDraftColorIndex;
  visual.addPaletteOpen = false;
  visual.editPaletteOpen = false;
  visual.customColorOpen = false;
  visual.addDraftColorIndex = null;
  renderVisualBuilder();
}

function cancelVisualColorAdd() {
  discardVisualColorEditHistory("visual");
  const before = visualEditSnapshot("visual");
  if (validVisualColorIndex(visual.addDraftColorIndex)) {
    removeVisualPaletteColor(visual.addDraftColorIndex);
  }
  visual.addPaletteOpen = false;
  visual.editPaletteOpen = false;
  visual.customColorOpen = false;
  visual.addDraftColorIndex = null;
  renderVisualBuilder();
  pushVisualEditUndoSnapshot("visual", before);
}

function closeVisualColorEditorFromOutside(event) {
  const target = event.target;
  const visualPopupOpen = Boolean(
    visual.addPaletteOpen
    || visual.editPaletteOpen
    || visual.colorTagPickerOpen
    || visual.shapeTagPickerOpen
  );
  const visual3dPopupOpen = Boolean(visual3d.addPaletteOpen || visual3d.editPaletteOpen);
  if (visualPopupOpen && !visualPalette.contains(target) && !visualShapeField?.contains(target)) {
    clearVisualColorEditorState();
    clearVisualTagPickerState();
    renderVisualControls();
    renderVisualPalette();
  }
  if (
    visual3dPopupOpen
    && !visual3dPalette?.contains(target)
    && typeof closeVisual3dColorEditor === "function"
  ) {
    closeVisual3dColorEditor();
  }
}

function nextVisualPresetColor(palette = visual.palette) {
  const used = new Set(palette.map((entry) => normalizeVisualColor(entry.color)));
  return VISUAL_COLOR_PRESETS.find((color) => !used.has(color)) || "#e94f64";
}

function deleteSelectedVisualColor() {
  commitVisualColorEditHistory("visual");
  const before = visualEditSnapshot("visual");
  if (!validVisualColorIndex(visual.selectedColorIndex) || visual.palette.length <= 1) {
    return;
  }
  visual.addPaletteOpen = false;
  visual.editPaletteOpen = false;
  visual.customColorOpen = false;
  visual.addDraftColorIndex = null;
  removeVisualPaletteColor(visual.selectedColorIndex);
  updateVisualBoundShapeDefinition();
  renderVisualBuilder();
  pushVisualEditUndoSnapshot("visual", before);
}

function removeVisualPaletteColor(deletedIndex) {
  if (!validVisualColorIndex(deletedIndex) || visual.palette.length <= 1) {
    return;
  }
  const oldPaletteLength = visual.palette.length;
  visual.palette.splice(deletedIndex, 1);
  const normalizeCell = (colorIndex) => {
    if (!Number.isInteger(colorIndex) || colorIndex < 0 || colorIndex >= oldPaletteLength) {
      return null;
    }
    if (colorIndex === deletedIndex) {
      return null;
    }
    return colorIndex > deletedIndex ? colorIndex - 1 : colorIndex;
  };
  visual.cells = visual.cells.map(normalizeCell);
  if (Array.isArray(visual.animationFrames)) {
    visual.animationFrames = visual.animationFrames.map((frame) => (
      Array.isArray(frame) ? frame.map(normalizeCell) : frame
    ));
    if (visual.animationMode) {
      ensureVisualAnimationFrames();
      visual.animationFrames[visual.animationFrameIndex] = visual.cells;
    }
  }
  visual.selectedColorIndex = Math.min(deletedIndex, visual.palette.length - 1);
}

function normalizeVisualColor(value) {
  return parseVisualHexColor(value) || "#e94f64";
}

function parseVisualHexColor(value) {
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

function visualRgbHex(value) {
  return normalizeVisualColor(value).slice(0, 7);
}

function visualAlphaPercent(value) {
  const normalized = normalizeVisualColor(value);
  if (normalized.length !== 9) {
    return 100;
  }
  return Math.round((Number.parseInt(normalized.slice(7, 9), 16) / 255) * 100);
}

function visualColorWithAlpha(rgb, alphaPercent) {
  const base = visualRgbHex(rgb);
  const percent = Math.max(0, Math.min(100, Math.round(Number(alphaPercent) || 0)));
  if (percent >= 100) {
    return base;
  }
  const alpha = Math.round((percent / 100) * 255).toString(16).padStart(2, "0");
  return `${base}${alpha}`;
}

function renderVisualColorSurfaces() {
  syncVisualPaletteSwatches();
  syncVisualColorAdjusters();
  renderVisualBoard();
  syncVisualSourceActionButtons();
}

function syncVisualPaletteSwatches() {
  for (const [index, entry] of visual.palette.entries()) {
    const color = normalizeVisualColor(entry.color);
    const displayName = visualPaletteEntryDisplayName(entry);
    for (const token of visualPalette.querySelectorAll(`[data-color-index="${index}"]`)) {
      token.style.setProperty("--visual-swatch-color", color);
      token.style.setProperty("--visual-token-ink", readableInkForColor(color));
      token.title = displayName ? `Paint ${displayName} (${color})` : `Paint ${color}`;
      token.setAttribute("aria-label", displayName ? `Paint color ${index}: ${displayName}` : `Paint color ${index}`);
    }
  }
  const selected = visual.palette[visual.selectedColorIndex];
  const currentButton = visualPalette.querySelector(".visual-current-color-button");
  if (currentButton && selected) {
    const normalized = normalizeVisualColor(selected.color);
    const displayName = visualPaletteEntryDisplayName(selected);
    currentButton.style.setProperty("--visual-current-color", normalized);
    currentButton.setAttribute("aria-label", displayName ? `Edit selected color ${displayName}` : `Edit selected color ${normalized}`);
    const currentHexInput = visualPalette.querySelector(".visual-current-hex-input");
    if (currentHexInput && !currentHexInput.classList.contains("is-name-mode") && document.activeElement !== currentHexInput) {
      currentHexInput.value = normalized;
    }
  }
}

function syncVisualColorAdjusters() {
  const selected = validVisualColorIndex(visual.selectedColorIndex)
    ? visual.palette[visual.selectedColorIndex]
    : null;
  if (!selected) {
    return;
  }
  const normalized = normalizeVisualColor(selected.color);
  for (const adjuster of visualPalette.querySelectorAll(".visual-color-adjuster")) {
    if (adjuster.contains(document.activeElement)) {
      continue;
    }
    adjuster.syncColor?.(normalized);
  }
}

function validVisualColorIndex(index) {
  return Number.isInteger(index) && index >= 0 && index < visual.palette.length;
}

function visualExportCharForColorIndex(index) {
  if (!validVisualColorIndex(index)) {
    return ".";
  }
  return VISUAL_COLOR_TOKENS[index] || ".";
}

function visualColorForColorIndex(index) {
  return validVisualColorIndex(index) ? normalizeVisualColor(visual.palette[index].color) : "#00000000";
}

function visualInkForColorIndex(index) {
  return validVisualColorIndex(index) ? readableInkForColor(visual.palette[index].color) : "#8d969f";
}

function visualGridLineForColorIndex(index) {
  return validVisualColorIndex(index) ? readableInkForColor(visual.palette[index].color) : "#1d242b";
}

function readableInkForColor(color) {
  const normalized = normalizeVisualColor(color).slice(1);
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

function updateVisualDimension(axis, value) {
  const before = visualEditSnapshot("visual");
  const nextValue = clampVisualSize(value);
  const next = visual.sizeBound
    ? { width: nextValue, height: nextValue }
    : {
        width: axis === "width" ? nextValue : visual.width,
        height: axis === "height" ? nextValue : visual.height,
      };
  if (next.width === visual.width && next.height === visual.height) {
    renderVisualControls();
    return;
  }
  remapVisualFrames(next, (x, y) => ({ x, y }));
  updateVisualBoundShapeDefinition();
  renderVisualBuilder();
  pushVisualEditUndoSnapshot("visual", before);
}

function remapVisualFrames(next, sourceCoordinates) {
  const previous = { width: visual.width, height: visual.height };
  const remap = (cells) => {
    const nextCells = Array.from({ length: next.width * next.height }, () => null);
    for (let y = 0; y < next.height; y += 1) {
      for (let x = 0; x < next.width; x += 1) {
        const source = sourceCoordinates(x, y, previous, next);
        if (!source || source.x < 0 || source.x >= previous.width
          || source.y < 0 || source.y >= previous.height) {
          continue;
        }
        const colorIndex = cells[source.y * previous.width + source.x];
        nextCells[y * next.width + x] = validVisualColorIndex(colorIndex) ? colorIndex : null;
      }
    }
    return nextCells;
  };
  const frames = visual.animationMode && visual.animationFrames.length
    ? visual.animationFrames
    : [visual.cells];
  visual.animationFrames = frames.map(remap);
  visual.width = next.width;
  visual.height = next.height;
  visual.animationFrameCount = visual.animationFrames.length;
  visual.animationFrameIndex = Math.min(visual.animationFrameIndex, visual.animationFrames.length - 1);
  visual.animationPlaybackIndex = Math.min(visual.animationPlaybackIndex, visual.animationFrames.length - 1);
  visual.cells = visual.animationFrames[visual.animationFrameIndex];
}

function visualScaleFactor() {
  return visualEditorScaleFactor(visualScaleInput, VISUAL_EDITOR_MAX_SIZE);
}

function canScaleDownVisual(factor = visualScaleFactor()) {
  return factor > 1
    && visual.width >= factor
    && visual.height >= factor
    && visual.width % factor === 0
    && visual.height % factor === 0;
}

function scaleUpVisual() {
  const before = visualEditSnapshot("visual");
  const factor = visualScaleFactor();
  const next = {
    width: visual.width * factor,
    height: visual.height * factor,
  };
  if (Math.max(next.width, next.height) > VISUAL_EDITOR_MAX_SIZE) {
    setVisualActionStatus(`Visual size limit is ${VISUAL_EDITOR_MAX_SIZE}`, "is-error");
    renderVisualControls();
    return;
  }

  remapVisualFrames(next, (x, y) => ({
    x: Math.floor(x / factor),
    y: Math.floor(y / factor),
  }));
  updateVisualBoundShapeDefinition();
  renderVisualBuilder();
  const message = `Scaled ${factor}x to ${next.width}x${next.height}`;
  setVisualActionStatus(message, "is-ok");
  setStatus(`Scaled visual ${factor}x to ${next.width}x${next.height}`, "is-ok");
  pushVisualEditUndoSnapshot("visual", before);
}

function scaleDownVisual() {
  const before = visualEditSnapshot("visual");
  const factor = visualScaleFactor();
  if (!canScaleDownVisual(factor)) {
    setVisualActionStatus(`Dimensions ${visual.width}x${visual.height} are not divisible by ${factor}`, "is-error");
    renderVisualControls();
    return;
  }

  const next = {
    width: visual.width / factor,
    height: visual.height / factor,
  };
  remapVisualFrames(next, (x, y) => ({ x: x * factor, y: y * factor }));
  updateVisualBoundShapeDefinition();
  renderVisualBuilder();
  const message = `Scaled down ${factor}x to ${next.width}x${next.height}`;
  setVisualActionStatus(message, "is-ok");
  setStatus(`Scaled visual down ${factor}x to ${next.width}x${next.height}`, "is-ok");
  pushVisualEditUndoSnapshot("visual", before);
}

function transformVisualCells(next, sourceCoordinates, message) {
  const before = visualEditSnapshot("visual");
  remapVisualFrames(next, sourceCoordinates);
  visual.addPaletteOpen = false;
  visual.editPaletteOpen = false;
  visual.customColorOpen = false;
  visual.addDraftColorIndex = null;
  updateVisualBoundShapeDefinition();
  renderVisualBuilder();
  setVisualActionStatus(message, "is-ok");
  setStatus(message, "is-ok");
  pushVisualEditUndoSnapshot("visual", before);
}

function rotateVisualLeft() {
  transformVisualCells(
    { width: visual.height, height: visual.width },
    (x, y, previous) => ({ x: previous.width - 1 - y, y: x }),
    "Rotated left",
  );
}

function rotateVisualRight() {
  transformVisualCells(
    { width: visual.height, height: visual.width },
    (x, y, previous) => ({ x: y, y: previous.height - 1 - x }),
    "Rotated right",
  );
}

function flipVisualHorizontal() {
  transformVisualCells(
    { width: visual.width, height: visual.height },
    (x, y, previous) => ({ x: previous.width - 1 - x, y }),
    "Flipped horizontal",
  );
}

function flipVisualVertical() {
  transformVisualCells(
    { width: visual.width, height: visual.height },
    (x, y, previous) => ({ x, y: previous.height - 1 - y }),
    "Flipped vertical",
  );
}

function normalizedVisualCellColorIndex(index) {
  const colorIndex = visual.cells[index];
  return validVisualColorIndex(colorIndex) ? colorIndex : null;
}

function floodFillVisualComponentAtIndex(index, colorIndex) {
  if (!Number.isInteger(index) || index < 0 || index >= visual.cells.length) {
    return 0;
  }
  const nextColorIndex = validVisualColorIndex(colorIndex) ? colorIndex : null;
  const targetColorIndex = normalizedVisualCellColorIndex(index);
  if (targetColorIndex === nextColorIndex) {
    return 0;
  }
  const visited = new Uint8Array(visual.cells.length);
  const region = visualClipActive ? normalizeVisualClipRect(visualClipSelection) : visualWholeEditRect();
  if (!region || !visualClipRectContainsIndex(region, index)) {
    return 0;
  }
  const stack = [index];
  let changed = 0;
  while (stack.length) {
    const current = stack.pop();
    if (visited[current] || !visualClipRectContainsIndex(region, current)
      || normalizedVisualCellColorIndex(current) !== targetColorIndex) {
      continue;
    }
    visited[current] = 1;
    visual.cells[current] = nextColorIndex;
    changed += 1;
    const x = current % visual.width;
    const y = Math.floor(current / visual.width);
    if (x > 0) {
      stack.push(current - 1);
    }
    if (x < visual.width - 1) {
      stack.push(current + 1);
    }
    if (y > 0) {
      stack.push(current - visual.width);
    }
    if (y < visual.height - 1) {
      stack.push(current + visual.width);
    }
  }
  if (!changed) {
    return 0;
  }
  visual.solidSource = false;
  visual.addPaletteOpen = false;
  visual.editPaletteOpen = false;
  visual.customColorOpen = false;
  visual.addDraftColorIndex = null;
  updateVisualBoundShapeDefinition();
  renderVisualBoard();
  syncVisualSourceActionButtons();
  return changed;
}

function bucketFillVisualFromIndex(index) {
  if (visualClipActive && !normalizeVisualClipRect(visualClipSelection)) {
    setVisualActionStatus("Select a clip region before bucket fill", "is-error");
    return false;
  }
  if (visualClipActive && !visualClipRectContainsIndex(visualClipSelection, index)) {
    setVisualActionStatus("Bucket fill start must be inside the clip region", "is-error");
    return false;
  }
  const count = floodFillVisualComponentAtIndex(index, visual.selectedColorIndex);
  if (!count) {
    setVisualActionStatus("Connected area already has that color", "is-ok");
    deactivateVisualBucketModeAfterUse();
    return false;
  }
  const colorIndex = validVisualColorIndex(visual.selectedColorIndex) ? visual.selectedColorIndex : null;
  const message = colorIndex === null ? "Filled connected area with transparent" : "Filled connected area";
  deactivateVisualBucketModeAfterUse();
  setVisualActionStatus(message, "is-ok");
  setStatus(message, "is-ok");
  return true;
}

function bucketFillVisualFromElement(element) {
  return bucketFillVisualFromIndex(visualCellIndexFromElement(element));
}

function paintVisualCellFromElement(element) {
  const index = visualCellIndexFromElement(element);
  return paintVisualAtPoint(visualPointForCellIndex(index), visual.selectedColorIndex);
}

function visualCellIndexFromElement(element) {
  const cell = element?.closest?.(".visual-cell");
  if (!cell || !visualBoard.contains(cell)) {
    return -1;
  }
  const index = Number(cell.dataset.index);
  return Number.isInteger(index) && index >= 0 && index < visual.cells.length ? index : -1;
}

function visualPointForCellIndex(index) {
  if (!Number.isInteger(index) || index < 0 || index >= visual.cells.length) {
    return null;
  }
  return {
    x: (index % visual.width) + 0.5,
    y: Math.floor(index / visual.width) + 0.5,
  };
}

function paintVisualAtPoint(point, colorIndex) {
  const indices = visualPaintIndicesForPoint(point);
  const changedIndices = paintVisualCellsAtIndices(indices, colorIndex);
  if (!changedIndices.length) {
    return false;
  }
  finishVisualPaintMutation(changedIndices);
  return true;
}

function paintVisualCellsAtIndices(indices, colorIndex) {
  const changedIndices = [];
  for (const index of indices) {
    if (setVisualCellColorAtIndex(index, colorIndex)) {
      changedIndices.push(index);
    }
  }
  return changedIndices;
}

function setVisualCellColorAtIndex(index, colorIndex) {
  if (!Number.isInteger(index) || index < 0 || index >= visual.cells.length) {
    return false;
  }
  const nextColorIndex = validVisualColorIndex(colorIndex) ? colorIndex : null;
  if (visual.cells[index] === nextColorIndex) {
    return false;
  }
  visual.cells[index] = nextColorIndex;
  return true;
}

function finishVisualPaintMutation(changedIndices, options = {}) {
  visual.solidSource = false;
  if (!options.deferSourceSync) {
    updateVisualBoundShapeDefinition();
  }
  renderVisualCellsAtIndices(changedIndices);
  renderVisualAnimationSurfaces();
  if (!options.deferSourceSync) {
    syncVisualSourceActionButtons();
  }
}

function paintVisualCellFromPoint(clientX, clientY, colorIndex) {
  return paintVisualAtPoint(visualBoardPointFromClient(clientX, clientY), colorIndex);
}

function visualBoardGeometry() {
  const rect = visualBoard.getBoundingClientRect();
  return {
    left: rect.left,
    top: rect.top,
    right: rect.right,
    bottom: rect.bottom,
    width: rect.width,
    height: rect.height,
    columns: visual.width,
    rows: visual.height,
  };
}

function visualBoardPointFromClient(clientX, clientY, geometry = visualBoardGeometry()) {
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
    x: ((clientX - geometry.left) / geometry.width) * geometry.columns,
    y: ((clientY - geometry.top) / geometry.height) * geometry.rows,
  };
}

function visualCellIndexFromPoint(point) {
  if (!point || !Number.isFinite(point.x) || !Number.isFinite(point.y)) {
    return -1;
  }
  const x = Math.floor(point.x);
  const y = Math.floor(point.y);
  if (x < 0 || x >= visual.width || y < 0 || y >= visual.height) {
    return -1;
  }
  return y * visual.width + x;
}

function visualBrushDiameterCells() {
  return Math.min(Math.max(visual.width, visual.height), visualBrushSizePx);
}

function visualBrushDiameterForSize(size) {
  return Math.min(size, visualBrushSizePx);
}

function visualPaintIndicesForPoint(point) {
  if (!point || !Number.isFinite(point.x) || !Number.isFinite(point.y)) {
    return [];
  }
  if (visualBrushSizePx === 1) {
    const index = visualCellIndexFromPoint(point);
    return index >= 0 ? [index] : [];
  }
  const diameter = visualBrushDiameterCells();
  const radius = diameter / 2;
  const minX = Math.max(0, Math.floor(point.x - radius - 0.5));
  const maxX = Math.min(visual.width - 1, Math.ceil(point.x + radius - 0.5));
  const minY = Math.max(0, Math.floor(point.y - radius - 0.5));
  const maxY = Math.min(visual.height - 1, Math.ceil(point.y + radius - 0.5));
  const indices = [];
  for (let y = minY; y <= maxY; y += 1) {
    for (let x = minX; x <= maxX; x += 1) {
      const dx = x + 0.5 - point.x;
      const dy = y + 0.5 - point.y;
      if ((dx * dx) + (dy * dy) <= radius * radius) {
        indices.push(y * visual.width + x);
      }
    }
  }
  if (!indices.length) {
    const index = visualCellIndexFromPoint(point);
    if (index >= 0) {
      indices.push(index);
    }
  }
  return indices;
}

function visualPaintDragIndices(point) {
  if (!visualPaintDrag || !point) {
    return [];
  }
  const lastPoint = visualPaintDrag.lastPoint;
  if (!lastPoint) {
    return visualPaintIndicesForPoint(point);
  }
  const points = visualInterpolatedBrushPoints(lastPoint, point);
  const indices = new Set();
  for (const brushPoint of points) {
    for (const cellIndex of visualPaintIndicesForPoint(brushPoint)) {
      indices.add(cellIndex);
    }
  }
  return [...indices];
}

function visualInterpolatedBrushPoints(fromPoint, toPoint) {
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

function startVisualClip(event, geometry, cell) {
  event.preventDefault();
  visualClipActive = true;
  const resizeHandle = !visualClipFloating && visualClipSelection
    ? event.target.closest("[data-visual-clip-resize]")
    : null;
  if (resizeHandle) {
    visualClipDrag = {
      mode: "resize",
      pointerId: event.pointerId,
      geometry,
      startCell: cell,
      origin: visualClipSelection,
      preview: visualClipSelection,
      edge: resizeHandle.dataset.visualClipResize,
    };
  } else if (visualClipSelectionContainsCell(cell)) {
    visualClipDrag = {
      mode: "move",
      pointerId: event.pointerId,
      geometry,
      startCell: cell,
      origin: visualClipSelection,
      preview: visualClipSelection,
    };
  } else if (visualClipFloating && visualClipClipboard) {
    const target = visualClipFloatingRectAtCell(cell);
    if (!target) {
      return;
    }
    visualClipSelection = target;
    visualClipDrag = {
      mode: "move",
      pointerId: event.pointerId,
      geometry,
      startCell: cell,
      origin: target,
      preview: target,
    };
  } else {
    visualClipSelection = visualClipRectFromCells(cell, cell);
    visualClipDrag = {
      mode: "select",
      pointerId: event.pointerId,
      geometry,
      startCell: cell,
    };
  }
  if (visualBoard.setPointerCapture) {
    visualBoard.setPointerCapture(event.pointerId);
  }
  renderVisualBoard();
}

function continueVisualClip(event) {
  if (!visualClipDrag || visualClipDrag.pointerId !== event.pointerId) {
    return false;
  }
  const cell = visualClipCellFromClient(event.clientX, event.clientY, visualClipDrag.geometry);
  if (!cell) {
    return true;
  }
  event.preventDefault();
  if (visualClipDrag.mode === "select") {
    visualClipSelection = visualClipRectFromCells(visualClipDrag.startCell, cell);
    renderVisualBoard();
    return true;
  }
  if (visualClipDrag.mode === "move") {
    const origin = visualClipDrag.origin;
    const dx = cell.x - visualClipDrag.startCell.x;
    const dy = cell.y - visualClipDrag.startCell.y;
    const nextX = Math.max(0, Math.min(visual.width - origin.width, origin.x + dx));
    const nextY = Math.max(0, Math.min(visual.height - origin.height, origin.y + dy));
    const next = normalizeVisualClipRect({ ...origin, x: nextX, y: nextY });
    if (next && (!visualClipDrag.preview || next.x !== visualClipDrag.preview.x || next.y !== visualClipDrag.preview.y)) {
      visualClipSelection = next;
      visualClipDrag.preview = next;
      renderVisualBoard();
    }
    return true;
  }
  if (visualClipDrag.mode === "resize") {
    const next = visualClipResizeRect(visualClipDrag.origin, visualClipDrag.edge, cell);
    if (next && (!visualClipDrag.preview
      || next.x !== visualClipDrag.preview.x
      || next.y !== visualClipDrag.preview.y
      || next.width !== visualClipDrag.preview.width
      || next.height !== visualClipDrag.preview.height)) {
      visualClipSelection = next;
      visualClipDrag.preview = next;
      renderVisualBoard();
    }
    return true;
  }
  return true;
}

function stopVisualClip(event) {
  if (!visualClipDrag || visualClipDrag.pointerId !== event.pointerId) {
    return false;
  }
  if (visualBoard.hasPointerCapture?.(event.pointerId)) {
    visualBoard.releasePointerCapture(event.pointerId);
  }
  event.preventDefault();
  const drag = visualClipDrag;
  visualClipDrag = null;
  visualClipSelection = normalizeVisualClipRect(visualClipSelection);
  renderVisualBuilder();
  if (!visualClipSelection) {
    return true;
  }
  const verb = drag.mode === "move"
    ? "Clip range moved"
    : drag.mode === "resize"
      ? "Clip range resized"
      : "Clip range selected";
  setVisualActionStatus(`${verb} ${visualClipSelection.width}x${visualClipSelection.height}`, "is-ok");
  return true;
}

function visualClipResizeRect(origin, edge, cell) {
  const rect = normalizeVisualClipRect(origin);
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
    right = Math.min(visual.width - 1, Math.max(cell.x, left));
  }
  if (edge.includes("n")) {
    top = Math.max(0, Math.min(cell.y, bottom));
  }
  if (edge.includes("s")) {
    bottom = Math.min(visual.height - 1, Math.max(cell.y, top));
  }
  return normalizeVisualClipRect({
    x: left,
    y: top,
    width: right - left + 1,
    height: bottom - top + 1,
  });
}

function visualClipShortcutTargetIsText(target) {
  const tagName = target?.tagName || "";
  return target?.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(tagName);
}

function visualClipShortcutsAreActive() {
  return currentPreviewMode === "visual" && !visualBuilder.hidden;
}

function moveVisualClipRangeBy(dx, dy) {
  const origin = normalizeVisualClipRect(visualClipSelection);
  if (!origin) {
    return false;
  }
  const target = normalizeVisualClipRect({
    ...origin,
    x: origin.x + dx,
    y: origin.y + dy,
  });
  if (!target) {
    setVisualActionStatus("Clip must stay inside visual", "is-error");
    return true;
  }
  moveVisualClipRange(target);
  return true;
}

function handleVisualClipKeyboard(event) {
  if (!visualClipShortcutsAreActive() || !visualClipActive || visualClipShortcutTargetIsText(event.target)) {
    return false;
  }
  const key = event.key.length === 1 ? event.key.toLowerCase() : event.key;
  const modifier = (event.metaKey && !event.ctrlKey) || (event.ctrlKey && !event.metaKey);
  let handled = false;
  if (!modifier && !event.altKey && key === "ArrowLeft") {
    handled = moveVisualClipRangeBy(-1, 0);
  } else if (!modifier && !event.altKey && key === "ArrowRight") {
    handled = moveVisualClipRangeBy(1, 0);
  } else if (!modifier && !event.altKey && key === "ArrowUp") {
    handled = moveVisualClipRangeBy(0, -1);
  } else if (!modifier && !event.altKey && key === "ArrowDown") {
    handled = moveVisualClipRangeBy(0, 1);
  }
  if (!handled) {
    return false;
  }
  event.preventDefault();
  event.stopPropagation();
  return true;
}

function visualPaneShortcutDimension() {
  if (currentPreviewMode === "visual" && !visualBuilder.hidden) return "2d";
  if (currentPreviewMode === "visual3d" && !visual3dBuilder.hidden) return "3d";
  return "";
}

function cancelVisualPaneToolShortcut(dimension) {
  if (dimension === "3d") {
    if (visual3dClipActive) deactivateVisual3dClipMode();
    else if (visual3dTranslateActive) deactivateVisual3dTranslateMode();
    else if (visual3dBucketActive) toggleVisual3dBucketMode();
    else return false;
    return true;
  }
  if (visualClipActive) deactivateVisualClipMode();
  else if (visualTranslateActive) deactivateVisualTranslateMode();
  else if (visualBucketActive) toggleVisualBucketMode();
  else return false;
  return true;
}

function startVisualPaint(event) {
  if (event.button !== 0) {
    return;
  }
  const geometry = visualBoardGeometry();
  if (visualTranslateActive) {
    event.preventDefault();
    startVisualTranslate(event, geometry);
    return;
  }
  const point = visualBoardPointFromClient(event.clientX, event.clientY, geometry);
  const index = visualCellIndexFromPoint(point);
  if (visualBucketActive) {
    if (!point || index < 0) {
      return;
    }
    event.preventDefault();
    const before = visualEditSnapshot("visual");
    if (bucketFillVisualFromIndex(index)) {
      pushVisualEditUndoSnapshot("visual", before);
    }
    return;
  }
  if (visualClipActive) {
    const cell = visualClipCellFromClient(event.clientX, event.clientY, geometry);
    if (!cell) {
      return;
    }
    startVisualClip(event, geometry, cell);
    return;
  }
  if (!point || index < 0) {
    return;
  }
  event.preventDefault();
  visualPaintDrag = {
    pointerId: event.pointerId,
    colorIndex: visual.selectedColorIndex,
    lastPoint: null,
    geometry,
    beforeSnapshot: visualEditSnapshot("visual"),
    changed: false,
  };
  if (visualBoard.setPointerCapture) {
    visualBoard.setPointerCapture(event.pointerId);
  }
  paintVisualDragPoint(point);
}

function continueVisualPaint(event) {
  if (continueVisualTranslate(event)) {
    return;
  }
  if (continueVisualClip(event)) {
    return;
  }
  const geometry = visualPaintDrag?.geometry || visualBoardGeometry();
  const point = visualBoardPointFromClient(event.clientX, event.clientY, geometry);
  if (!visualPaintDrag || visualPaintDrag.pointerId !== event.pointerId) {
    return;
  }
  event.preventDefault();
  paintVisualDragPoint(point);
}

function stopVisualPaint(event) {
  if (stopVisualTranslate(event)) {
    return;
  }
  if (stopVisualClip(event)) {
    return;
  }
  if (!visualPaintDrag || visualPaintDrag.pointerId !== event.pointerId) {
    return;
  }
  if (visualBoard.hasPointerCapture?.(event.pointerId)) {
    visualBoard.releasePointerCapture(visualPaintDrag.pointerId);
  }
  if (visualPaintDrag.changed) {
    updateVisualBoundShapeDefinition();
    syncVisualSourceActionButtons();
    pushVisualEditUndoSnapshot("visual", visualPaintDrag.beforeSnapshot);
  }
  visualPaintDrag = null;
}

function paintVisualDragPoint(point) {
  if (!visualPaintDrag || !point) {
    return;
  }
  const indices = visualPaintDragIndices(point);
  visualPaintDrag.lastPoint = point;
  const changedIndices = paintVisualCellsAtIndices(indices, visualPaintDrag.colorIndex);
  if (changedIndices.length) {
    finishVisualPaintMutation(changedIndices, { deferSourceSync: true });
    visualPaintDrag.changed = true;
  }
}

function visualAscii() {
  const rows = [];
  for (let y = 0; y < visual.height; y += 1) {
    const row = [];
    for (let x = 0; x < visual.width; x += 1) {
      row.push(visualExportCharForColorIndex(visual.cells[y * visual.width + x]));
    }
    rows.push(row.join(""));
  }
  return rows.join("\n");
}

function visualObjectName() {
  const raw = String(visualNameInput.value || "").trim();
  const explicitAnimation = raw.startsWith("!");
  const cleaned = raw
    .replace(/^!+/, "")
    .replace(/[^\w:@]+/g, "_")
    .replace(/(?!^)@/g, "_")
    .replace(/^_+|_+$/g, "");
  const name = cleaned || "Visual";
  return explicitAnimation ? `!${name}` : name;
}

function renderVisualShapeBindRow(target) {
  if (!target) {
    return;
  }
  target.replaceChildren();
  const info = visualAssetBindInfo(visual.shapeBind, "shape");
  const row = document.createElement("div");
  row.className = "visual-shape-bind-row";
  row.classList.toggle("has-unlink", info.linked && info.name);
  const input = document.createElement("input");
  input.type = "text";
  input.className = "visual-shape-name-input";
  input.value = info.name || "";
  input.placeholder = "shape";
  input.spellcheck = false;
  input.autocomplete = "off";
  input.setAttribute("aria-label", "Shape name");
  const tagButton = document.createElement("button");
  tagButton.type = "button";
  tagButton.className = "icon-button visual-shape-tag-button visual-icon-button";
  tagButton.classList.toggle("is-active", info.linked);
  tagButton.innerHTML = visualTagIconSvg();
  tagButton.setAttribute("aria-pressed", String(info.linked));
  tagButton.setAttribute("aria-haspopup", "listbox");
  tagButton.setAttribute("aria-expanded", String(Boolean(visual.shapeTagPickerOpen)));
  tagButton.title = info.name ? `Shape tag: ${info.name}` : "Tag shape by name";
  tagButton.setAttribute("aria-label", tagButton.title);
  const commitName = (options = {}) => {
    commitVisualShapeName(input.value, {
      sync: Boolean(visual.shapeTagPickerOpen) || visualAssetBindInfo(visual.shapeBind, "shape").linked,
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
    const opening = !visual.shapeTagPickerOpen;
    if (opening) {
      clearVisualColorEditorState();
      visual.colorTagPickerOpen = false;
      renderVisualPalette();
    }
    visual.shapeTagPickerOpen = opening;
    renderVisualControls();
  });
  row.append(input, tagButton);
  if (info.linked && info.name) {
    row.append(renderVisualShapeUnlinkButton(info));
  }
  if (visual.shapeTagPickerOpen) {
    const tagPicker = renderVisualAssetNamePicker({
      className: "visual-shape-tag-picker",
      names: visualShapeAssetNames(),
      value: info.name || "",
      placeholder: "",
      ariaLabel: "Shape tag name",
      emptyText: "No named shapes yet",
      onCommit: (name) => {
        const wasOpen = visual.shapeTagPickerOpen;
        visual.shapeTagPickerOpen = false;
        const ok = setVisualShapeSync(true, name);
        if (!ok) {
          visual.shapeTagPickerOpen = wasOpen;
          return false;
        }
        clearVisualColorEditorState();
        renderVisualBuilder();
        return true;
      },
      onCancel: () => {
        visual.shapeTagPickerOpen = false;
        renderVisualControls();
      },
    });
    row.append(tagPicker);
    requestAnimationFrame(() => {
      focusVisualTagPickerInput(tagPicker);
    });
  }
  target.append(row);
}

function renderVisualShapeBindControl(target, options) {
  if (!target) {
    return;
  }
  const state = options.state;
  const info = visualAssetBindInfo(state.shapeBind, "shape");
  target.replaceChildren();
  const row = document.createElement("div");
  row.className = "visual-shape-bind-row";
  row.classList.toggle("has-unlink", info.linked && info.name);
  const input = document.createElement("input");
  input.type = "text";
  input.className = "visual-shape-name-input";
  input.value = info.name || "";
  input.placeholder = "shape";
  input.spellcheck = false;
  input.autocomplete = "off";
  input.setAttribute("aria-label", "Shape name");
  const tagButton = document.createElement("button");
  tagButton.type = "button";
  tagButton.className = "icon-button visual-shape-tag-button visual-icon-button";
  tagButton.classList.toggle("is-active", info.linked);
  tagButton.innerHTML = visualTagIconSvg();
  tagButton.setAttribute("aria-pressed", String(info.linked));
  tagButton.setAttribute("aria-haspopup", "listbox");
  tagButton.setAttribute("aria-expanded", String(Boolean(state.shapeTagPickerOpen)));
  tagButton.title = info.name ? `Shape tag: ${info.name}` : "Tag shape by name";
  tagButton.setAttribute("aria-label", tagButton.title);
  const commit = (linked = info.linked) => {
    const name = sanitizeVisualShapeRef(input.value);
    state.shapeBind = name ? { type: "shape", name, linked: Boolean(linked) } : null;
    options.onChange();
  };
  input.addEventListener("change", () => commit());
  input.addEventListener("keydown", (event) => {
    event.stopPropagation();
    if (event.key === "Enter") {
      event.preventDefault();
      commit();
    }
  });
  tagButton.addEventListener("click", () => {
    state.shapeTagPickerOpen = !state.shapeTagPickerOpen;
    if (state.shapeTagPickerOpen && input.value) {
      commit(true);
      return;
    }
    options.render();
  });
  row.append(input, tagButton);
  if (info.linked && info.name) {
    const unlink = document.createElement("button");
    unlink.type = "button";
    unlink.className = "icon-button is-danger visual-shape-tag-unlink-button visual-icon-button";
    unlink.title = `Unlink shape tag ${info.name}`;
    unlink.setAttribute("aria-label", unlink.title);
    unlink.innerHTML = visualUnlinkIconSvg();
    unlink.addEventListener("click", () => {
      state.shapeTagPickerOpen = false;
      state.shapeBind = { type: "shape", name: info.name, linked: false };
      options.onChange();
    });
    row.append(unlink);
  }
  if (state.shapeTagPickerOpen) {
    const picker = renderVisualAssetNamePicker({
      className: "visual-shape-tag-picker",
      names: visualShapeAssetNames(),
      value: info.name || "",
      placeholder: "",
      ariaLabel: "Shape tag name",
      emptyText: "No named shapes yet",
      onCommit: (name) => {
        state.shapeTagPickerOpen = false;
        state.shapeBind = { type: "shape", name, linked: true };
        options.onChange();
        return true;
      },
      onCancel: () => {
        state.shapeTagPickerOpen = false;
        options.render();
      },
    });
    row.append(picker);
  }
  target.append(row);
}

function renderVisualShapeUnlinkButton(info) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "icon-button is-danger visual-shape-tag-unlink-button visual-icon-button";
  button.title = info?.name ? `Unlink shape tag ${info.name}` : "Unlink shape tag";
  button.setAttribute("aria-label", button.title);
  button.innerHTML = visualUnlinkIconSvg();
  button.addEventListener("click", (event) => {
    event.preventDefault();
    event.stopPropagation();
    visual.shapeTagPickerOpen = false;
    clearVisualColorEditorState();
    toggleVisualShapeBinding();
  });
  return button;
}

function commitVisualShapeName(rawName, options = {}) {
  const name = sanitizeVisualShapeRef(rawName);
  const info = visualAssetBindInfo(visual.shapeBind, "shape");
  if (!name) {
    if (info.name) {
      visual.shapeBind = null;
      void syncCurrentVisualDefinitionFromBuilder("Shape sync off");
      renderVisualBuilder();
    } else if (options.reportError && options.sync) {
      setVisualActionStatus("Enter a shape name", "is-error");
    }
    return false;
  }
  visual.shapeBind = { type: "shape", name, linked: Boolean(options.sync) };
  if (options.sync) {
    return setVisualShapeSync(true, name);
  }
  syncVisualSourceActionButtons();
  return true;
}

function setVisualShapeSync(sync, rawName) {
  const name = sanitizeVisualShapeRef(rawName || visualAssetBindInfo(visual.shapeBind, "shape").name);
  if (!sync) {
    visual.shapeBind = name ? { type: "shape", name, linked: false } : null;
    renderVisualBuilder();
    return true;
  }
  if (!name) {
    setVisualActionStatus("Enter a shape name", "is-error");
    return false;
  }
  const shapes = visualSourceShapeAssets();
  let status = `Using shape ${name}`;
  if (shapes.has(name)) {
    const parsed = visualCellsFromAsciiRows(shapes.get(name), visual.palette.length);
    if (!parsed) {
      setVisualActionStatus(`Cannot use shape ${name}`, "is-error");
      return false;
    }
    visual.width = parsed.width;
    visual.height = parsed.height;
    visual.cells = parsed.cells;
  } else {
    status = `Tagged shape ${name}`;
  }
  visual.shapeBind = { type: "shape", name, linked: true };
  setVisualActionStatus(status, "is-ok");
  renderVisualBuilder();
  return true;
}

function visualCellsFromAsciiRows(rows, paletteLength) {
  if (!Array.isArray(rows) || rows.length === 0) {
    return null;
  }
  const width = Math.max(...rows.map((row) => row.length));
  const height = rows.length;
  const clampedWidth = clampVisualSize(width);
  const clampedHeight = clampVisualSize(height);
  const cells = Array.from({ length: clampedWidth * clampedHeight }, () => null);
  for (let y = 0; y < clampedHeight; y += 1) {
    for (let x = 0; x < Math.min(rows[y].length, clampedWidth); x += 1) {
      const colorIndex = visualColorIndexForPaletteChar(rows[y][x], paletteLength);
      if (colorIndex === undefined) {
        return null;
      }
      cells[y * clampedWidth + x] = colorIndex;
    }
  }
  return { width: clampedWidth, height: clampedHeight, cells };
}

function loadVisualSourceTarget(target, options = {}) {
  if (!isPuzzleDocument(activeDocument()) || !isTextDocument(activeDocument())) {
    return null;
  }
  const source = sourceEditorDocumentValue();
  if (!Number.isInteger(target?.bodyStart) || !Number.isInteger(target?.bodyEnd)) {
    return null;
  }
  if (options.recordHistory && typeof pushSourceNavigationHistory === "function") {
    pushSourceNavigationHistory();
  }
  if (options.switchMode && currentPreviewMode !== "visual") {
    setPreviewMode("visual");
  }
  const targetName = target.name || visualObjectName();
  const loaded = parseVisualDefinitionSource(target.sourceVisual, targetName);
  if (!loaded) {
    const contractError = visualSourceContractError(target.sourceVisual);
    if (target?.sourceVisual?.dimension === "2d") {
      applyIncompleteVisualSourceTarget(targetName, target);
      if (!options.silent) {
        const message = contractError
          || `Loaded unfinished visual ${visualNameInput.value || ""}`.trim();
        const status = contractError ? "is-error" : "is-ok";
        setVisualActionStatus(message, status);
        setStatus(message, status);
      }
      return `visual:${targetName}:${target.start ?? target.bodyStart}`;
    } else if (!options.silent) {
      setVisualActionStatus("No editable visual here", "is-error");
    }
    return null;
  }
  visualNameInput.value = targetName || "Visual";
  setVisualEditSource(target, activeDocument());
  visual.width = loaded.width;
  visual.height = loaded.height;
  visual.palette = loaded.palette;
  visual.shapeBind = loaded.shapeBind || null;
  visual.solidSource = Boolean(loaded.solid);
  visual.sourcePreludeRows = Array.isArray(loaded.sourcePreludeRows) ? loaded.sourcePreludeRows : [];
  visual.sourceSpatialOps = Array.isArray(loaded.sourceSpatialOps) ? loaded.sourceSpatialOps : [];
  visual.cells = loaded.cells;
  if (loaded.animationMode) {
    visual.animationMode = true;
    visual.animationDurationMs = normalizedVisualAnimationDuration(loaded.animationDurationMs);
    visual.animationFrameCount = normalizedVisualAnimationFrameCount(loaded.animationFrameCount);
    visual.animationFrameIndex = 0;
    visual.animationPlaybackIndex = 0;
    visual.animationPlaying = false;
    visual.animationFrames = Array.isArray(loaded.animationFrames)
      ? loaded.animationFrames.map((frame) => cloneVisualCells(frame))
      : [cloneVisualCells(visual.cells)];
    ensureVisualAnimationFrames();
  } else {
    visual.animationMode = false;
    resetVisualAnimationFramesFromCurrentCells();
  }
  visual.selectedColorIndex = visual.palette.length ? 0 : null;
  visual.addPaletteOpen = false;
  visual.editPaletteOpen = false;
  visual.customColorOpen = false;
  visual.addDraftColorIndex = null;
  renderVisualBuilder();
  syncPreviewModeButtonState();
  if (!options.silent) {
    setVisualActionStatus(`Loaded ${visualNameInput.value}`, "is-ok");
    setStatus(`Loaded visual ${visualNameInput.value}`, "is-ok");
  }
  return `visual:${targetName}:${target.start ?? target.bodyStart}`;
}

function applyIncompleteVisualSourceTarget(name, target) {
  if (target && typeof target === "object") {
    setVisualEditSource(target, activeDocument());
  }
  visualNameInput.value = name || "";
  visual.width = clampVisualSize(visual.width);
  visual.height = clampVisualSize(visual.height);
  visual.palette = [];
  visual.shapeBind = null;
  visual.solidSource = false;
  visual.sourcePreludeRows = [];
  visual.sourceSpatialOps = [];
  visual.animationMode = false;
  visual.cells = Array.from({ length: visualFrameCellCount() }, () => null);
  resetVisualAnimationFramesFromCurrentCells();
  visual.selectedColorIndex = null;
  visual.addPaletteOpen = false;
  visual.editPaletteOpen = false;
  visual.customColorOpen = false;
  visual.addDraftColorIndex = null;
  renderVisualBuilder();
  syncPreviewModeButtonState();
}

function parseVisualDefinitionSource(contract, selectorName = "") {
  const documentContract = projectVisualDocumentContract(contract);
  if (!documentContract || documentContract.dimension !== "2d") {
    return null;
  }
  const sourcePreludeRows = Array.isArray(contract.preludeRows)
    ? contract.preludeRows.map((row) => String(row || "").trim()).filter(Boolean)
    : [];
  const sourceSpatialOps = Array.isArray(contract.spatialOps) ? contract.spatialOps : [];
  const paletteTokens = Array.isArray(contract.paletteTokens)
    ? contract.paletteTokens.map((token) => String(token || "").trim()).filter(Boolean)
    : [];
  const shapeName = typeof contract.shapeRef === "string" ? contract.shapeRef.trim() : "";
  const resolvedPalette = documentContract.resolvedPalette;
  if (contract.status !== "complete" || !paletteTokens.length || !resolvedPalette.length) {
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
    shapeBind = { type: "shape", name: shapeName, linked: true };
  }
  const width = Number(contract?.extent?.width);
  const height = Number(contract?.extent?.height);
  const depth = Number(contract?.extent?.depth);
  const semanticFrames = Array.isArray(contract.frames) ? contract.frames : [];
  if (!Number.isInteger(width) || !Number.isInteger(height) || width <= 0 || height <= 0 || depth !== 1 || !semanticFrames.length) {
    return null;
  }
  const parsedWidth = clampVisualSize(width);
  const parsedHeight = clampVisualSize(height);
  const parsedFrames = semanticFrames.map((frame) => {
    const cells = frame?.layers?.[0]?.cells;
    if (!Array.isArray(cells) || cells.length !== width * height) return null;
    const parsed = Array.from({ length: parsedWidth * parsedHeight }, () => null);
    for (let y = 0; y < parsedHeight; y += 1) {
      for (let x = 0; x < parsedWidth; x += 1) {
        const cell = cells[y * width + x];
        if (cell !== null && (!Number.isInteger(cell) || cell < 0 || cell >= palette.length)) return null;
        parsed[y * parsedWidth + x] = cell;
      }
    }
    return parsed;
  });
  if (parsedFrames.some((frame) => !frame)) return null;
  if (parsedFrames.length >= 2) {
    const frameDurationMs = Number.isFinite(Number(contract.frameDurationMs))
      ? Number(contract.frameDurationMs)
      : null;
    const durationMs = Number.isFinite(Number(contract.durationMs))
      ? normalizedVisualAnimationDuration(contract.durationMs)
      : normalizedVisualAnimationDuration(frameDurationMs === null ? undefined : frameDurationMs * parsedFrames.length);
    return {
      width: parsedWidth,
      height: parsedHeight,
      palette,
      shapeBind: null,
      sourcePreludeRows,
      sourceSpatialOps,
      animationMode: true,
      animationDurationMs: durationMs,
      animationFrameCount: parsedFrames.length,
      animationFrames: parsedFrames,
      cells: parsedFrames[0],
    };
  }
  return {
    width: parsedWidth,
    height: parsedHeight,
    palette,
    shapeBind,
    sourcePreludeRows,
    sourceSpatialOps,
    solid: width === 1 && height === 1 && parsedFrames[0][0] === 0,
    cells: parsedFrames[0],
  };
}

function visualSourceContractError(contract) {
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

function visualPaletteEntrySourceToken(entry) {
  const bind = visualPaletteEntryBindInfo(entry);
  if (bind.linked && bind.name) {
    return bind.name;
  }
  return normalizeVisualColor(entry.color);
}

function visualColorIndexForPaletteChar(char, paletteLength) {
  if (char === ".") {
    return null;
  }
  const index = VISUAL_COLOR_TOKENS.indexOf(char);
  return index >= 0 && index < paletteLength ? index : undefined;
}

async function mutateVisualSourceFromRust(source, request) {
  if (typeof window.PuzzleStudioRuntime?.mutateVisualSource !== "function") {
    throw new Error("Editor WASM visual mutation API is unavailable.");
  }
  const result = await window.PuzzleStudioRuntime.mutateVisualSource(source, request);
  if (!result || typeof result.source !== "string" || !Number.isInteger(result.start) || !Number.isInteger(result.end)) {
    throw new Error("Editor WASM returned an invalid visual mutation result.");
  }
  return result;
}

function visualEditFrames() {
  const frames = visual.animationMode
    ? visual.animationFrames.slice(0, visual.animationFrameCount)
    : [visual.cells];
  return frames.map((cells) => [Array.from({ length: visual.height }, (_, y) => (
    Array.from({ length: visual.width }, (_, x) => {
      const cell = Array.isArray(cells) ? cells[y * visual.width + x] : null;
      return Number.isInteger(cell) ? cell : null;
    })
  ))]);
}

function visualEditMutationRequest(operation, options = {}) {
  const shape = visualAssetBindInfo(visual.shapeBind, "shape");
  const colorBindings = visual.palette
    .filter((entry) => entry?.bind?.type === "color" && entry.bind.linked && entry.bind.name)
    .map((entry) => ({ name: entry.bind.name, color: normalizeVisualColor(entry.color) }));
  return {
    operation,
    dimension: "2d",
    name: options.name ?? visualObjectName(),
    originalName: options.originalName ?? visual.editSourceName ?? visualObjectName(),
    cursor: options.cursor,
    palette: visualPaletteSourceTokens(),
    frames: visualEditFrames(),
    durationMs: visual.animationMode ? visual.animationDurationMs : null,
    shapeRef: shape.linked ? shape.name : null,
    preludeRows: visual.sourcePreludeRows || [],
    spatialOps: visual.sourceSpatialOps || [],
    colorBindings,
  };
}

async function addVisualToSource() {
  let result;
  try {
    ({ result } = await commitVisualEditorMutation({
      state: visual,
      allowActiveDocument: true,
      request: (source, document) => visualEditMutationRequest(
        canReplaceCurrentVisualDefinition(source) ? "duplicate" : "insert",
        { cursor: visualSourceCursorPosition(source, document) },
      ),
    }));
  } catch (error) {
    setVisualActionStatus(userFacingRuntimeError(error), "is-error");
    return;
  }
  visualNameInput.value = result.name;
  setVisualActionStatus("Added visual", "is-ok");
  setStatus("Added visual", "is-ok");
  syncVisualSourceActionButtons();
}

function newVisualDraft() {
  const before = visualEditSnapshot("visual");
  clearVisualEditSource();
  visualNameInput.value = "Visual";
  visual.palette = [{ color: "#ff004d" }];
  visual.selectedColorIndex = 0;
  visual.animationMode = false;
  resetVisualBuilder(5, 5);
  setVisualActionStatus("Started new visual", "is-ok");
  pushVisualEditUndoSnapshot("visual", before);
}

async function updateVisualInSource() {
  let result;
  try {
    ({ result } = await commitVisualEditorMutation({
      state: visual,
      request: () => visualEditMutationRequest("update"),
    }));
  } catch (error) {
    setVisualActionStatus("No selected visual source range", "is-error");
    setStatus("No selected visual source range", "is-error");
    setVisualActionStatus(userFacingRuntimeError(error), "is-error");
    return;
  }
  setVisualActionStatus("Updated visual", "is-ok");
  setStatus("Updated visual", "is-ok");
  syncVisualSourceActionButtons();
}

function deleteWholeVisualRegion() {
  const before = visualEditSnapshot("visual");
  if (visual.animationMode) {
    ensureVisualAnimationFrames();
    visual.cells = Array.from({ length: visualFrameCellCount() }, () => null);
    visual.animationFrames[visual.animationFrameIndex] = visual.cells;
    updateVisualBoundShapeDefinition();
    renderVisualBuilder();
    setVisualActionStatus(`Deleted frame ${visual.animationFrameIndex + 1} contents`, "is-ok");
    pushVisualEditUndoSnapshot("visual", before);
    return;
  }
  resetVisualBuilder();
  setVisualActionStatus("Deleted whole visual contents", "is-ok");
  pushVisualEditUndoSnapshot("visual", before);
}

function setVisualActionStatus(text, className = "") {
  if (!visualActionStatus) {
    return;
  }
  window.clearTimeout(visualActionClearTimer);
  visualActionStatus.className = `visual-action-status tool-feedback-bar ${className}`.trim();
  visualActionStatus.textContent = text;
  setPaneStatus("visual", text, className);
  if (text && className === "is-ok") {
    visualActionClearTimer = window.setTimeout(() => {
      if (visualActionStatus.textContent === text && visualActionStatus.classList.contains("is-ok")) {
        visualActionStatus.className = "visual-action-status tool-feedback-bar";
        visualActionStatus.textContent = "";
      }
    }, 1800);
  }
}

function clearVisualActionError() {
  if (!visualActionStatus?.classList.contains("is-error")) {
    return;
  }
  setVisualActionStatus("");
}

function visualSourceCursorPosition(source, document = activeDocument()) {
  if (document?.id === activeDocument()?.id && sourceEditor) {
    return Math.max(
      0,
      Math.min(String(source || "").length, Math.max(sourceEditor.selectionStart || 0, sourceEditor.selectionEnd || 0)),
    );
  }
  return String(source || "").length;
}

function canReplaceCurrentVisualDefinition(source) {
  return Boolean(currentVisualEditSourceRange(source));
}

function syncVisualSourceActionButtons() {
  const hasEditableSource = canReplaceCurrentVisualDefinition(activeVisualEditSource());
  if (visualUpdateButton) {
    visualUpdateButton.disabled = !hasEditableSource;
  }
  if (visualInsertButton) {
    visualInsertButton.disabled = false;
  }
}

function currentVisualEditSourceRange(source) {
  return visualEditorSourceRange(visual, source, () => "");
}

function revealVisualSourceResult(document, result) {
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

function visualPaletteSourceTokens() {
  return visual.palette.map((entry) => visualPaletteEntrySourceToken(entry));
}

for (const input of [
  visualNameInput,
  visualWidthInput,
  visualHeightInput,
  visualScaleInput,
  visualAnimationDurationInput,
  visualAnimationFrameInput,
]) {
  if (input) {
    installSelectAllOnFocus(input);
  }
}
function bindVisualDimensionInput(input, axis) {
  input.addEventListener("input", () => {
    if (input.validity.valid && input.value !== "") {
      updateVisualDimension(axis, input.value);
    }
  });
  input.addEventListener("change", () => updateVisualDimension(axis, input.value));
  input.addEventListener("keydown", (event) => {
    if (event.key !== "Enter") return;
    event.preventDefault();
    updateVisualDimension(axis, input.value);
  });
}
bindVisualDimensionInput(visualWidthInput, "width");
bindVisualDimensionInput(visualHeightInput, "height");
visualScaleInput.addEventListener("input", () => {
  clearVisualActionError();
  renderVisualControls();
});
visualScaleInput.addEventListener("keydown", (event) => {
  if (event.key !== "Enter") {
    return;
  }
  event.preventDefault();
});
visualAnimationDurationInput?.addEventListener("input", () => updateVisualAnimationDuration(visualAnimationDurationInput.value, { preserveInput: true, recordHistory: false }));
visualAnimationDurationInput?.addEventListener("change", () => updateVisualAnimationDuration(visualAnimationDurationInput.value));
visualAnimationDurationInput?.addEventListener("keydown", (event) => {
  if (event.key !== "Enter") {
    return;
  }
  event.preventDefault();
  updateVisualAnimationDuration(visualAnimationDurationInput.value);
});
visualAnimationFrameInput?.addEventListener("change", () => {
  if (currentVisualPaneMode === "visual3d") return setVisual3dAnimationFrame(Number(visualAnimationFrameInput.value) - 1);
  return setVisualAnimationFrame(Number(visualAnimationFrameInput.value) - 1);
});
visualAnimationFrameInput?.addEventListener("keydown", (event) => {
  if (event.key !== "Enter") {
    return;
  }
  event.preventDefault();
  if (currentVisualPaneMode === "visual3d") setVisual3dAnimationFrame(Number(visualAnimationFrameInput.value) - 1);
  else setVisualAnimationFrame(Number(visualAnimationFrameInput.value) - 1);
});
visualAnimationInsertFrameButton?.addEventListener("click", () => insertSharedVisualAnimationFrameAfterCurrent());
visualAnimationRemoveFrameButton?.addEventListener("click", () => removeSharedVisualAnimationCurrentFrame());
visualBrushSizeInput.addEventListener("change", () => {
  if (currentVisualPaneMode === "visual3d" && typeof selectVisual3dBrushSize === "function") {
    selectVisual3dBrushSize(visualBrushSizeInput.value);
    return;
  }
  selectVisualBrushSize(visualBrushSizeInput.value);
});
visualNameInput.addEventListener("input", syncVisualSourceActionButtons);
sourceEditor.addEventListener("input", () => {
  invalidateVisualEditSourceForDocument(activeDocument());
  syncVisualSourceActionButtons();
});
visualPalette.addEventListener("mousedown", (event) => {
  const button = event.target.closest("button");
  if (!button || !visualPalette.contains(button)) {
    return;
  }
  event.preventDefault();
});
visualPalette.addEventListener("keydown", (event) => {
  const token = event.target.closest(".visual-token");
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
      visualBucketActive = false;
    }
    selectVisualColor(nextIndex);
  }
});
visualBoard.addEventListener("pointerdown", startVisualPaint);
visualBoard.addEventListener("pointermove", continueVisualPaint);
visualBoard.addEventListener("pointerup", stopVisualPaint);
visualBoard.addEventListener("pointercancel", stopVisualPaint);
visualBoard.addEventListener("keydown", (event) => {
  if (visualTranslateActive) {
    event.preventDefault();
    return;
  }
  if (handleVisualClipKeyboard(event)) {
    return;
  }
  if (event.key === "Enter" || event.key === " ") {
    const mutate = visualBucketActive ? bucketFillVisualFromElement : paintVisualCellFromElement;
    if (withVisualEditHistory("visual", () => mutate(event.target))) {
      event.preventDefault();
      event.stopPropagation();
    }
  }
});
document.addEventListener("click", (event) => {
  if (!visualTranslateActive || visualBoard.contains(event.target)) {
    return;
  }
  const translateButton = event.target.closest?.(".visual-translate-button");
  if (translateButton) {
    return;
  }
  deactivateVisualTranslateMode();
});
document.addEventListener("keydown", handleVisualClipKeyboard);
document.addEventListener("pointerdown", closeVisualColorEditorFromOutside);
visualScaleDownButton.addEventListener("click", scaleDownVisual);
visualScaleUpButton.addEventListener("click", scaleUpVisual);
visualRotateLeftButton.addEventListener("click", rotateVisualLeft);
visualRotateRightButton.addEventListener("click", rotateVisualRight);
visualFlipHorizontalButton.addEventListener("click", flipVisualHorizontal);
visualFlipVerticalButton.addEventListener("click", flipVisualVertical);
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
visualGridButton?.addEventListener("click", () => {
  if (currentVisualPaneMode === "visual3d" && typeof toggleVisual3dGrid === "function") {
    toggleVisual3dGrid();
    return;
  }
  toggleVisualGrid();
});
=======
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
resetVisualBuilder();
