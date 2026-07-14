// Editor command ownership boundary.
// Change a command's shortcut here to update both keyboard dispatch and hover hints.

function editorCommandTextEntryTarget(target) {
  const element = target instanceof Element ? target : null;
  const tagName = element?.tagName || "";
  return Boolean(element?.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(tagName));
}

function editorCommandElements(selector) {
  return () => Array.from(document.querySelectorAll(selector));
}

function editorCommandIndexedElements(selector, index) {
  return () => {
    const element = document.querySelectorAll(selector)[index];
    return element ? [element] : [];
  };
}

function editorCommandClick(selector) {
  const button = document.querySelector(selector);
  if (!button || button.disabled) return false;
  button.click();
  return true;
}

function levelCommandActive() {
  return currentPreviewMode === "edit" && levelBuilder && !levelBuilder.hidden;
}

function level3dCommandActive() {
  return currentPreviewMode === "level3d" && level3dBuilder && !level3dBuilder.hidden;
}

function solverCommandActive() {
  return currentPreviewMode === "solver" && solverPanel && !solverPanel.hidden && Boolean(levelSolutionPreview);
}

function soundCommandKind(event) {
  const target = event?.target instanceof Element ? event.target : null;
  if (target?.closest("#soundsMusicPanel")) return "music";
  if (target?.closest("#soundsSfxPanel")) return "sfx";
  const active = document.activeElement instanceof Element ? document.activeElement : null;
  if (active?.closest("#soundsMusicPanel")) return "music";
  if (active?.closest("#soundsSfxPanel")) return "sfx";
  return sounds.mode === "music" ? "music" : "sfx";
}

function activateLevelBrush() {
  levelBucketActive = false;
  setLevelResizeMode(null);
  syncLevelBucketButton();
  setStatus("Brush: paint individual cells", "is-ok");
  return true;
}

function activateLevel3dBrush() {
  setLevel3dStageResizeMode(null);
  level3d.layerFillActive = false;
  renderLevel3dPalette();
  renderLevel3dLayerPalette();
  setLevel3dActionStatus("Brush: paint individual cells", "is-ok");
  return true;
}

const editorCommandDatabase = [
  {
    id: "workspace.save",
    group: "workspace",
    label: "Save",
    shortcut: { key: "s", modifiers: ["primary"] },
    elements: editorCommandElements("#saveButton"),
    dispatch: false,
  },
  {
    id: "level.brush",
    group: "level",
    label: "Brush",
    shortcut: { key: "b" },
    available: () => levelCommandActive() && !levelPlaytestActive,
    run: activateLevelBrush,
  },
  {
    id: "level.fill",
    group: "level",
    label: "Fill",
    shortcut: { key: "f" },
    elements: editorCommandElements("#levelFillButton"),
    available: () => levelCommandActive() && !levelPlaytestActive,
    run: () => (toggleLevelBucketMode(), true),
  },
  {
    id: "level.grid",
    group: "level",
    label: "Grid",
    shortcut: { key: "g" },
    elements: editorCommandElements("#levelGridButton"),
    available: () => levelCommandActive() && !levelPlaytestActive,
    run: () => (toggleLevelGrid(), true),
  },
  {
    id: "level.play",
    group: "level",
    label: () => levelPlaytestActive ? "Stop" : "Play",
    shortcut: () => ({ key: levelPlaytestActive ? "Escape" : "p" }),
    elements: editorCommandElements("#levelPlaytestButton"),
    available: () => levelCommandActive(),
    run: () => (toggleLevelPlaytest(), true),
  },
  {
    id: "level3d.brush",
    group: "level3d",
    label: "Brush",
    shortcut: { key: "b" },
    available: () => level3dCommandActive() && !level3dPlaytestActive,
    run: activateLevel3dBrush,
  },
  {
    id: "level3d.fill",
    group: "level3d",
    label: "Fill",
    shortcut: { key: "f" },
    elements: editorCommandElements("#level3dLayerPalette .sprite-fill-button"),
    available: () => level3dCommandActive() && !level3dPlaytestActive && level3dViewMode === "layer",
    run: () => editorCommandClick("#level3dLayerPalette .sprite-fill-button"),
  },
  {
    id: "level3d.grid",
    group: "level3d",
    label: "Grid",
    shortcut: { key: "g" },
    elements: editorCommandElements("#level3dPalette .level3d-frame-toggle-button, #level3dLayerPalette .level-grid-button"),
    available: () => level3dCommandActive() && !level3dPlaytestActive,
    run: () => editorCommandClick(level3dViewMode === "layer"
      ? "#level3dLayerPalette .level-grid-button"
      : "#level3dPalette .level3d-frame-toggle-button"),
  },
  {
    id: "level3d.play",
    group: "level3d",
    label: () => level3dPlaytestActive ? "Stop" : "Play",
    shortcut: () => ({ key: level3dPlaytestActive ? "Escape" : "p" }),
    elements: editorCommandElements("#level3dPlaytestButton"),
    available: () => level3dCommandActive(),
    run: () => (toggleLevel3dPlaytest(), true),
  },
  {
    id: "level3d.slice.previous",
    group: "level3d",
    label: "Previous slice",
    shortcut: { key: "[" },
    elements: editorCommandElements("#level3dPreviousLayerButton"),
    available: () => level3dCommandActive() && !level3dPlaytestActive,
    run: () => (moveLevel3dLayer(-1), true),
  },
  {
    id: "level3d.slice.next",
    group: "level3d",
    label: "Next slice",
    shortcut: { key: "]" },
    elements: editorCommandElements("#level3dNextLayerButton"),
    available: () => level3dCommandActive() && !level3dPlaytestActive,
    run: () => (moveLevel3dLayer(1), true),
  },
  {
    id: "level3d.view.toggle",
    group: "level3d",
    label: (element) => element.id === "level3dStageViewButton" ? "3D View" : "Layer View",
    shortcut: { key: "v" },
    elements: editorCommandElements("#level3dStageViewButton, #level3dLayerViewButton"),
    available: () => level3dCommandActive() && !level3dPlaytestActive,
    run: () => (setLevel3dViewMode(level3dViewMode === "layer" ? "stage" : "layer"), true),
  },
  {
    id: "solver.previous",
    group: "solver",
    label: "Previous step",
    shortcut: { key: "ArrowLeft" },
    elements: editorCommandElements("#solutionPrevButton"),
    available: solverCommandActive,
    run: () => (setSolutionStep(levelSolutionPreview.index - 1), true),
  },
  {
    id: "solver.next",
    group: "solver",
    label: "Next step",
    shortcut: { key: "ArrowRight" },
    elements: editorCommandElements("#solutionNextButton"),
    available: solverCommandActive,
    run: () => (setSolutionStep(levelSolutionPreview.index + 1), true),
  },
  {
    id: "solver.first",
    group: "solver",
    label: "First step",
    shortcut: { key: "Home" },
    available: solverCommandActive,
    run: () => (setSolutionStep(0), true),
  },
  {
    id: "solver.last",
    group: "solver",
    label: "Last step",
    shortcut: { key: "End" },
    available: solverCommandActive,
    run: () => (setSolutionStep(levelSolutionPreview.steps.length - 1), true),
  },
  {
    id: "solver.play",
    group: "solver",
    label: "Play / Pause",
    shortcut: { key: " " },
    elements: editorCommandElements("#solutionPlayButton"),
    available: solverCommandActive,
    run: () => (toggleSolutionPlayback(), true),
  },
  {
    id: "solver.reset",
    group: "solver",
    label: "Reset",
    shortcut: { key: "r" },
    elements: editorCommandElements("#solutionResetButton"),
    available: solverCommandActive,
    run: () => (resetSolutionPreview(), true),
  },
  {
    id: "solver.copy",
    group: "solver",
    label: "Copy",
    shortcut: { key: "c", modifiers: ["primary"] },
    elements: editorCommandElements("#solutionExportButton"),
    available: solverCommandActive,
    run: () => (exportSolution(), true),
  },
  {
    id: "sounds.sfx.play",
    group: "sounds",
    label: "Play SFX",
    shortcut: { key: " " },
    elements: editorCommandElements("#soundsSfxPlayButton"),
    available: (event) => currentPreviewMode === "sounds" && soundCommandKind(event) === "sfx",
    run: () => (playSoundSfx().catch((error) => setStatus(`Sounds failed: ${error?.message || error}`, "is-error")), true),
  },
  {
    id: "sounds.sfx.random",
    group: "sounds",
    label: "Randomize SFX",
    shortcut: { key: "r" },
    elements: editorCommandElements("#soundsSfxRandomButton"),
    available: (event) => currentPreviewMode === "sounds" && soundCommandKind(event) === "sfx",
    run: () => (randomizeSoundSfx(), true),
  },
  {
    id: "sounds.music.play",
    group: "sounds",
    label: "Play / Pause music",
    shortcut: { key: " " },
    elements: editorCommandElements("#soundsMusicPlayButton"),
    available: (event) => currentPreviewMode === "sounds" && soundCommandKind(event) === "music",
    run: () => (toggleSoundMusic().catch((error) => setStatus(`Sounds failed: ${error?.message || error}`, "is-error")), true),
  },
  {
    id: "sounds.music.random",
    group: "sounds",
    label: "Randomize music",
    shortcut: { key: "r" },
    elements: editorCommandElements("#soundsMusicRandomButton"),
    available: (event) => currentPreviewMode === "sounds" && soundCommandKind(event) === "music",
    run: () => (randomizeSoundMusic(), true),
  },
  {
    id: "import.copy",
    group: "import",
    label: "Copy",
    shortcut: { key: "c", modifiers: ["primary"] },
    elements: editorCommandElements("#psImportCopyButton"),
    available: () => currentPreviewMode === "psimport" && !psImportCopyButton.disabled,
    run: () => (psImportCopyButton.click(), true),
  },
  {
    id: "import.add",
    group: "import",
    label: "Add file",
    shortcut: { key: "Enter", modifiers: ["primary"] },
    elements: editorCommandElements("#psImportAddFileButton"),
    allowTextEntry: true,
    available: () => currentPreviewMode === "psimport" && !psImportAddFileButton.disabled,
    run: () => (psImportAddFileButton.click(), true),
  },
];

for (let index = 0; index < 9; index += 1) {
  editorCommandDatabase.push({
    id: `level.palette.${index + 1}`,
    group: "level",
    label: (element) => `Paint ${element.dataset.label || `palette item ${index + 1}`}`,
    shortcut: { key: String(index + 1) },
    elements: editorCommandIndexedElements("#levelPalette .level-token", index),
    available: () => levelCommandActive() && !levelPlaytestActive,
    run: () => {
      const button = document.querySelectorAll("#levelPalette .level-token")[index];
      if (!button || button.disabled) return false;
      button.click();
      return true;
    },
  });
  editorCommandDatabase.push({
    id: `level3d.palette.${index + 1}`,
    group: "level3d",
    label: (element) => `Paint ${element.dataset.label || `palette item ${index + 1}`}`,
    shortcut: { key: String(index + 1) },
    elements: () => [
      document.querySelectorAll("#level3dPalette .level3d-token")[index],
      document.querySelectorAll("#level3dLayerPalette .level3d-layer-token")[index],
    ].filter(Boolean),
    available: () => level3dCommandActive() && !level3dPlaytestActive,
    run: () => {
      const selector = level3dViewMode === "layer"
        ? "#level3dLayerPalette .level3d-layer-token"
        : "#level3dPalette .level3d-token";
      const button = document.querySelectorAll(selector)[index];
      if (!button || button.disabled) return false;
      button.click();
      return true;
    },
  });
}

const editorCommandById = new Map(editorCommandDatabase.map((command) => [command.id, command]));

function resolvedEditorCommandValue(command, key, element = null) {
  const value = command?.[key];
  return typeof value === "function" ? value(element) : value;
}

function editorCommandShortcut(id) {
  const command = editorCommandById.get(id);
  if (!command) throw new Error(`Unknown editor command ${id}`);
  return resolvedEditorCommandValue(command, "shortcut");
}

function editorCommandMatches(id, event) {
  return editorShortcutMatches(event, editorCommandShortcut(id));
}

function editorCommandAriaShortcut(shortcut) {
  const normalized = normalizeEditorShortcut(shortcut);
  const keys = normalized.keys.map((key) => key === " " ? "Space" : key);
  if (!normalized.modifiers.includes("primary")) return keys.join(" ");
  return keys.flatMap((key) => [`Control+${key}`, `Meta+${key}`]).join(" ");
}

function bindEditorCommandElement(element, command) {
  const label = resolvedEditorCommandValue(command, "label", element);
  const shortcut = resolvedEditorCommandValue(command, "shortcut", element);
  element.dataset.editorCommand = command.id;
  element.dataset.tooltip = label;
  setEditorShortcutHint(element, shortcut);
  element.setAttribute("aria-keyshortcuts", editorCommandAriaShortcut(shortcut));
}

function refreshEditorCommandBindings() {
  for (const command of editorCommandDatabase) {
    if (typeof command.elements !== "function") continue;
    for (const element of command.elements()) {
      bindEditorCommandElement(element, command);
    }
  }
}

function dispatchEditorCommandEvent(event, options = {}) {
  if (event.defaultPrevented || event.repeat) return false;
  for (const command of editorCommandDatabase) {
    if (command.dispatch === false || typeof command.run !== "function") continue;
    if (options.group && command.group !== options.group) continue;
    const shortcut = resolvedEditorCommandValue(command, "shortcut");
    if (!editorShortcutMatches(event, shortcut)) continue;
    if (!command.allowTextEntry && editorCommandTextEntryTarget(event.target)) continue;
    if (typeof command.available === "function" && !command.available(event)) continue;
    if (command.run(event) === false) continue;
    event.preventDefault();
    event.stopImmediatePropagation();
    refreshEditorCommandBindings();
    return true;
  }
  return false;
}

const editorCommandObserver = new MutationObserver((records) => {
  if (records.some((record) => (
    record.type === "attributes" || record.addedNodes.length || record.removedNodes.length
  ))) {
    refreshEditorCommandBindings();
  }
});

refreshEditorCommandBindings();
editorCommandObserver.observe(document.body, {
  attributes: true,
  attributeFilter: ["aria-pressed", "disabled"],
  childList: true,
  subtree: true,
});
document.addEventListener("keydown", (event) => dispatchEditorCommandEvent(event), true);
