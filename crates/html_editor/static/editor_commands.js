// Editor command ownership boundary.
// Buttons and keyboard shortcuts invoke the same command through this registry.

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

function editorCommandContext(event = null, element = null, source = "keyboard") {
  const target = element || (event?.target instanceof Element ? event.target : null);
  const route = workbenchCommandContext(source, target);
  return Object.freeze({ event, element, target, source, route });
}

function editorCommandRouteIs(context, pane, modes = []) {
  if (context?.route?.pane !== pane) return false;
  return !modes.length || modes.includes(context.route.mode);
}

function editorCommandTargetInside(context, container) {
  const target = context?.target;
  return !target || target === document.body || Boolean(container?.contains(target));
}

function levelCommandActive(context) {
  return editorCommandRouteIs(context, "level", ["edit"]) && levelBuilder && !levelBuilder.hidden;
}

function level3dCommandActive(context) {
  return editorCommandRouteIs(context, "level", ["level3d"]) && level3dBuilder && !level3dBuilder.hidden;
}

function levelSourceCommandDimension(context) {
  if (levelCommandActive(context)) return "2d";
  if (level3dCommandActive(context)) return "3d";
  return null;
}

function levelLayerCommandActive(context) {
  return levelCommandActive(context)
    && editorCommandTargetInside(context, levelBuilder)
    && !levelPlaytestActive
    && level.layerMode
    && levelLayerCount2d() > 1
    && !levelLayerInsertMode
    && !levelLayerRemoveMode;
}

function level3dLayerCommandActive(context) {
  return level3dCommandActive(context) && !level3dPlaytestActive && level3dViewMode === "layer";
}

function level3dSliceCommandAvailable(context) {
  if (!level3dCommandActive(context) || level3dPlaytestActive) return false;
  const target = context?.target instanceof Element ? context.target : null;
  if (target && target !== document.body && !level3dBuilder.contains(target)) return false;
  return !target?.closest("[data-level3d-preview], [data-level3d-slice-scrub]");
}

function solverCommandActive(context) {
  return editorCommandRouteIs(context, "solver") && solverPanel && !solverPanel.hidden && Boolean(levelSolutionPreview);
}

function visualCommandMode(context) {
  return editorCommandRouteIs(context, "visual", ["visual", "visual3d"])
    ? context.route.mode
    : "";
}

function visualCommandDimension(context) {
  const mode = visualCommandMode(context);
  return mode === "visual3d" ? "3d" : mode === "visual" ? "2d" : "";
}

function visualCommandActive(context) {
  return Boolean(visualCommandDimension(context));
}

function visual3dCommandActive(context) {
  return visualCommandDimension(context) === "3d";
}

function visualNavigationCommandAvailable(context) {
  const dimension = visualCommandDimension(context);
  if (!dimension) return false;
  const target = context?.target instanceof Element ? context.target : null;
  if (target?.closest("[data-visual3d-camera], [data-visual3d-slice-scrub]")) return false;
  if (dimension === "3d") return !visual3dClipActive && !visual3dTranslateActive;
  return !visualClipActive && !visualTranslateActive;
}

function visualAnimationCommandAvailable(context) {
  if (!visualNavigationCommandAvailable(context)) return false;
  const dimension = visualCommandDimension(context);
  const state = dimension === "3d" ? visual3d : visual;
  return Boolean(state.animationMode && state.animationFrameCount > 1);
}

function moveVisualAnimationFrameFromCommand(context, delta) {
  const dimension = visualCommandDimension(context);
  if (!dimension) return false;
  moveSharedVisualAnimationFrame(dimension === "3d" ? "visual3d" : "visual", delta);
  return true;
}

function selectVisualCommandColor(context, index) {
  const dimension = visualCommandDimension(context);
  const entries = dimension === "3d" ? visual3dPaletteEntries() : visual.palette;
  if (!dimension || (index !== null && (index < 0 || index >= entries.length))) return false;
  if (dimension === "3d") selectVisual3dColor(index);
  else selectVisualColor(index);
  return true;
}

function toggleVisualFillCommand(context) {
  const dimension = visualCommandDimension(context);
  if (dimension === "3d") return (toggleVisual3dBucketMode(), true);
  if (dimension === "2d") return (toggleVisualBucketMode(), true);
  return false;
}

function toggleVisualMoveCommand(context) {
  const dimension = visualCommandDimension(context);
  if (dimension === "3d") return (toggleVisual3dTranslateMode(), true);
  if (dimension === "2d") return (toggleVisualTranslateMode(), true);
  return false;
}

function toggleVisualClipCommand(context) {
  const dimension = visualCommandDimension(context);
  if (dimension === "3d") return (toggleVisual3dClipMode(), true);
  if (dimension === "2d") return (toggleVisualClipMode(), true);
  return false;
}

function runVisualEditCommandFromContext(context, command) {
  return runVisualEditCommand(visualCommandDimension(context), command);
}

function soundCommandKind(context) {
  const target = context?.target instanceof Element ? context.target : null;
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

function runWorkspaceSaveCommand() {
  saveCurrentDocument(true).catch((error) => {
    console.error(error);
    setEditorStatus("Save failed", "is-error");
    saveButton.disabled = false;
  });
  return true;
}

function runLevelSaveCommand(context) {
  if (context.route.mode === "edit") {
    updateLevelInSource();
    return true;
  }
  if (context.route.mode === "level3d") {
    updateLevel3dInSource();
    return true;
  }
  return false;
}

function runVisualSaveCommand(context) {
  if (context.route.mode === "visual") {
    const update = updateVisualInSource();
    if (update && typeof update.catch === "function") {
      update.catch((error) => {
        console.error(error);
        setPaneStatus("visual", "Visual source update failed", "is-error");
      });
    }
    return true;
  }
  if (context.route.mode === "visual3d") {
    updateVisual3dInSource();
    return true;
  }
  return false;
}

function runSoundsSaveCommand(context) {
  updateSoundsDefinition(soundCommandKind(context));
  return true;
}

function runLevelAddCommand(context) {
  const mode = context.route.mode;
  if (mode === "edit") return (addLevelToSource(), true);
  if (mode === "level3d") return (addLevel3dToSource(), true);
  return false;
}

function runVisualAddCommand(context) {
  const mode = context.route.mode;
  if (mode === "visual") return (addVisualToSource(), true);
  if (mode === "visual3d") {
    addVisual3dToSource().catch((error) => {
      console.error(error);
      setVisual3dActionStatus("Could not add 3D visual", "is-error");
    });
    return true;
  }
  return false;
}

function runVisualNewCommand(context) {
  const mode = context.route.mode;
  if (mode === "visual3d") return (newVisual3dDraft(), true);
  if (mode === "visual") return (newVisualDraft(), true);
  return false;
}

function runLevelCopyCommand(context) {
  const mode = context.route.mode;
  if (mode === "level3d") {
    copyLevel3dToClipboard().catch((error) => setLevel3dActionStatus(error?.message || String(error), "is-error"));
    return true;
  }
  if (mode === "edit") return (copyLevelToClipboard(), true);
  return false;
}

function runEditedLevelSolveCommand() {
  solveEditedLevelFromEditor().catch((error) => {
    setLevelSolveStatus(`Solve failed: ${userFacingRuntimeError(error)}`, "is-error");
  });
  return true;
}

function visualEditHistoryCommandActive(context) {
  return !isTextEntryTarget(context?.target) && Boolean(editorCommandEditKind(context));
}

function editorCommandEditKind(context) {
  const mode = context?.route?.mode;
  return ["edit", "level3d", "visual", "visual3d"].includes(mode) ? mode : "";
}

function runVisualEditHistoryCommand(context, redo) {
  const kind = editorCommandEditKind(context);
  return kind ? (redo ? redoVisualEdit(kind) : undoVisualEdit(kind)) : false;
}

const editorCommandDatabase = [
  {
    id: "workspace.save",
    group: "workspace",
    label: "Save",
    shortcuts: [{ key: "s", modifiers: ["primary"] }],
    elements: editorCommandElements("#saveButton"),
    allowTextEntry: true,
    available: (context) => ["explorer", "source", "preview", "solver", "psimport", "docs"].includes(context.route.pane),
    run: runWorkspaceSaveCommand,
  },
  {
    id: "level.save",
    group: "level",
    label: "Update level in source",
    shortcuts: [{ key: "s", modifiers: ["primary"] }],
    elements: editorCommandElements("#updateLevelButton, #updateLevel3dButton"),
    allowTextEntry: true,
    available: (context) => editorCommandRouteIs(context, "level", ["edit", "level3d"]),
    run: runLevelSaveCommand,
  },
  {
    id: "visual.save",
    group: "visual",
    label: "Update visual in source",
    shortcuts: [{ key: "s", modifiers: ["primary"] }],
    elements: editorCommandElements("#visualUpdateButton, #visual3dUpdateButton"),
    allowTextEntry: true,
    available: (context) => editorCommandRouteIs(context, "visual", ["visual", "visual3d"]),
    run: runVisualSaveCommand,
  },
  {
    id: "sounds.save",
    group: "sounds",
    label: "Update sound in source",
    shortcuts: [{ key: "s", modifiers: ["primary"] }],
    elements: editorCommandElements("#soundsSfxUpdateButton, #soundsMusicUpdateButton"),
    allowTextEntry: true,
    available: (context) => editorCommandRouteIs(context, "sounds"),
    run: runSoundsSaveCommand,
  },
  {
    id: "editor.undo",
    group: "workspace",
    label: "Undo",
    shortcuts: [{ key: "z", modifiers: ["primary"] }],
    available: visualEditHistoryCommandActive,
    run: (context) => runVisualEditHistoryCommand(context, false),
  },
  {
    id: "editor.redo",
    group: "workspace",
    label: "Redo",
    shortcuts: [
      { key: "z", modifiers: ["primary", "shift"] },
      { key: "y", modifiers: ["primary"] },
    ],
    available: visualEditHistoryCommandActive,
    run: (context) => runVisualEditHistoryCommand(context, true),
  },
  {
    id: "workspace.explorer.toggle",
    group: "workspace",
    label: "Toggle explorer",
    shortcuts: [{ key: "b", modifiers: ["primary"] }],
    run: () => (togglePaneVisibility("explorer"), true),
  },
  {
    id: "visual.new",
    group: "visual",
    label: "New visual",
    shortcuts: [{ key: "n", modifiers: ["primary"] }],
    elements: editorCommandElements("#newVisualButton, #newVisual3dButton"),
    available: visualCommandActive,
    run: runVisualNewCommand,
  },
  {
    id: "level.add",
    group: "level",
    label: "Add level",
    shortcuts: [{ key: "a", modifiers: ["primary"] }],
    elements: editorCommandElements("#addLevelButton, #addLevel3dButton"),
    available: (context) => Boolean(levelSourceCommandDimension(context)),
    run: runLevelAddCommand,
  },
  {
    id: "visual.add",
    group: "visual",
    label: "Add visual",
    shortcuts: [{ key: "a", modifiers: ["primary"] }],
    elements: editorCommandElements("#visualInsertButton, #visual3dInsertButton"),
    available: visualCommandActive,
    run: runVisualAddCommand,
  },
  {
    id: "level.source.copy",
    group: "level",
    label: "Copy level",
    shortcuts: [{ key: "c", modifiers: ["primary"] }],
    elements: editorCommandElements("#copyLevelButton, #copyLevel3dButton"),
    available: (context) => Boolean(levelSourceCommandDimension(context)),
    run: runLevelCopyCommand,
  },
  {
    id: "level.solve",
    group: "level",
    label: "Solve",
    shortcuts: [{ key: "Enter", modifiers: ["primary"] }],
    elements: editorCommandElements("#levelSolveShortcutButton, #level3dSolveShortcutButton"),
    available: (context) => Boolean(levelSourceCommandDimension(context)),
    run: runEditedLevelSolveCommand,
  },
  {
    id: "level.brush",
    group: "level",
    label: "Brush",
    shortcuts: [{ key: "b" }],
    available: (context) => levelCommandActive(context) && !levelPlaytestActive,
    run: activateLevelBrush,
  },
  {
    id: "level.eraser",
    group: "level",
    label: "Eraser",
    shortcuts: [{ key: "." }],
    elements: editorCommandElements("#levelPalette .level-eraser-button"),
    available: (context) => levelCommandActive(context) && !levelPlaytestActive,
    run: selectLevelEraser,
  },
  {
    id: "level.fill",
    group: "level",
    label: "Fill",
    shortcuts: [{ key: "f" }],
    elements: editorCommandElements("#levelFillButton"),
    available: (context) => levelCommandActive(context) && !levelPlaytestActive,
    run: () => (toggleLevelBucketMode(), true),
  },
  {
    id: "level.grid",
    group: "level",
    label: "Grid",
    shortcuts: [{ key: "g" }],
    elements: editorCommandElements("#levelGridButton"),
    available: (context) => levelCommandActive(context) && !levelPlaytestActive,
    run: () => (toggleLevelGrid(), true),
  },
  {
    id: "level.layer.previous",
    group: "level",
    label: "Previous layer",
    shortcuts: [{ key: "ArrowLeft" }],
    available: levelLayerCommandActive,
    run: () => (setLevelLayer(level.activeLayer - 1), true),
  },
  {
    id: "level.layer.next",
    group: "level",
    label: "Next layer",
    shortcuts: [{ key: "ArrowRight" }],
    available: levelLayerCommandActive,
    run: () => (setLevelLayer(level.activeLayer + 1), true),
  },
  {
    id: "level.play",
    group: "level",
    label: () => levelPlaytestActive ? "Stop" : "Play",
    shortcuts: () => [{ key: levelPlaytestActive ? "Escape" : "p" }],
    elements: editorCommandElements("#levelPlaytestButton"),
    available: levelCommandActive,
    run: () => (toggleLevelPlaytest(), true),
  },
  {
    id: "level3d.brush",
    group: "level3d",
    label: "Brush",
    shortcuts: [{ key: "b" }],
    available: (context) => level3dCommandActive(context) && !level3dPlaytestActive,
    run: activateLevel3dBrush,
  },
  {
    id: "level3d.eraser",
    group: "level3d",
    label: "Eraser",
    shortcuts: [{ key: "." }],
    elements: editorCommandElements("#level3dLayerPalette .level-eraser-button"),
    available: level3dLayerCommandActive,
    run: () => (selectLevel3dEraser(), true),
  },
  {
    id: "level3d.fill",
    group: "level3d",
    label: "Fill",
    shortcuts: [{ key: "f" }],
    elements: editorCommandElements("#level3dLayerPalette .visual-fill-button"),
    available: (context) => level3dCommandActive(context) && !level3dPlaytestActive && level3dViewMode === "layer",
    run: () => (toggleLevel3dLayerFill(), true),
  },
  {
    id: "level3d.grid",
    group: "level3d",
    label: "Grid",
    shortcuts: [{ key: "g" }],
    elements: editorCommandElements("#level3dPalette .level3d-frame-toggle-button, #level3dLayerPalette .level-grid-button"),
    available: (context) => level3dCommandActive(context) && !level3dPlaytestActive,
    run: () => (level3dViewMode === "layer" ? toggleLevel3dLayerGrid() : toggleLevel3dFrameVisibility(), true),
  },
  {
    id: "level3d.play",
    group: "level3d",
    label: () => level3dPlaytestActive ? "Stop" : "Play",
    shortcuts: () => [{ key: level3dPlaytestActive ? "Escape" : "p" }],
    elements: editorCommandElements("#level3dPlaytestButton"),
    available: level3dCommandActive,
    run: () => (toggleLevel3dPlaytest(), true),
  },
  {
    id: "level3d.slice.previous",
    group: "level3d",
    label: "Previous slice",
    shortcuts: [{ key: "ArrowUp" }],
    elements: editorCommandElements("#level3dPreviousLayerButton"),
    available: level3dSliceCommandAvailable,
    run: () => (moveLevel3dLayer(-1), true),
  },
  {
    id: "level3d.slice.next",
    group: "level3d",
    label: "Next slice",
    shortcuts: [{ key: "ArrowDown" }],
    elements: editorCommandElements("#level3dNextLayerButton"),
    available: level3dSliceCommandAvailable,
    run: () => (moveLevel3dLayer(1), true),
  },
  {
    id: "level3d.slice.add-above",
    group: "level3d",
    label: "Add slice above",
    shortcuts: [{ key: "[" }],
    elements: editorCommandElements("#level3dAddSliceAboveButton"),
    available: level3dSliceCommandAvailable,
    run: () => insertLevel3dSlice("above"),
  },
  {
    id: "level3d.slice.add-below",
    group: "level3d",
    label: "Add slice below",
    shortcuts: [{ key: "]" }],
    elements: editorCommandElements("#level3dAddSliceBelowButton"),
    available: level3dSliceCommandAvailable,
    run: () => insertLevel3dSlice("below"),
  },
  {
    id: "level3d.view.toggle",
    group: "level3d",
    label: (element) => element.id === "level3dStageViewButton" ? "3D View" : "Layer View",
    shortcuts: [{ key: "v" }],
    elements: editorCommandElements("#level3dStageViewButton, #level3dLayerViewButton"),
    available: (context) => level3dCommandActive(context) && !level3dPlaytestActive,
    run: () => (setLevel3dViewMode(level3dViewMode === "layer" ? "stage" : "layer"), true),
  },
  {
    id: "solver.previous",
    group: "solver",
    label: "Previous step",
    shortcuts: [{ key: "ArrowLeft" }],
    elements: editorCommandElements("#solutionPrevButton"),
    available: solverCommandActive,
    run: () => (setSolutionStep(levelSolutionPreview.index - 1), true),
  },
  {
    id: "visual.palette.eraser",
    group: "visual",
    label: "Eraser",
    shortcuts: [{ key: visualExportCharForColorIndex(null) }],
    elements: editorCommandElements(
      "#visualPalette .visual-token-erase, #visual3dPalette .visual-token-erase",
    ),
    available: visualCommandActive,
    run: (context) => selectVisualCommandColor(context, null),
  },
  {
    id: "visual.fill",
    group: "visual",
    label: "Fill",
    shortcuts: [{ key: "f" }],
    elements: editorCommandElements("#visualFillButton, #visual3dFillButton"),
    available: visualCommandActive,
    run: toggleVisualFillCommand,
  },
  {
    id: "visual.move",
    group: "visual",
    label: "Move",
    shortcuts: [{ key: "m" }],
    elements: editorCommandElements(".visual-translate-button, #visual3dTranslateButton"),
    available: visualCommandActive,
    run: toggleVisualMoveCommand,
  },
  {
    id: "visual.clip",
    group: "visual",
    label: "Clip",
    shortcuts: [{ key: "c" }],
    elements: editorCommandElements(".visual-context-actions .visual-clip-actions > .visual-clip-button"),
    available: visualCommandActive,
    run: toggleVisualClipCommand,
  },
  {
    id: "visual.edit.copy",
    group: "visual",
    label: "Copy",
    shortcuts: [{ key: "c", modifiers: ["primary"] }],
    elements: editorCommandElements('[data-visual-edit-command="copy"]'),
    available: visualCommandActive,
    run: (context) => runVisualEditCommandFromContext(context, "copy"),
  },
  {
    id: "visual.edit.cut",
    group: "visual",
    label: "Cut",
    shortcuts: [{ key: "x", modifiers: ["primary"] }],
    elements: editorCommandElements('[data-visual-edit-command="cut"]'),
    available: visualCommandActive,
    run: (context) => runVisualEditCommandFromContext(context, "cut"),
  },
  {
    id: "visual.edit.paste",
    group: "visual",
    label: "Paste",
    shortcuts: [{ key: "v", modifiers: ["primary"] }],
    elements: editorCommandElements('[data-visual-edit-command="paste"]'),
    available: visualCommandActive,
    run: (context) => runVisualEditCommandFromContext(context, "paste"),
  },
  {
    id: "visual.edit.delete",
    group: "visual",
    label: "Delete",
    shortcuts: [{ key: "Delete" }, { key: "Backspace" }],
    elements: editorCommandElements('[data-visual-edit-command="delete"]'),
    available: visualCommandActive,
    run: (context) => runVisualEditCommandFromContext(context, "delete"),
  },
  ...["x", "y", "z"].map((axis) => ({
    id: `visual3d.axis.${axis}`,
    group: "visual",
    label: `${axis.toUpperCase()} axis`,
    shortcuts: [{ key: axis }],
    elements: editorCommandElements(`[data-visual3d-axis="${axis}"]`),
    available: visual3dCommandActive,
    run: () => (setVisual3dAxis(axis), true),
  })),
  {
    id: "visual.cancel-tool",
    group: "visual",
    label: "Cancel tool",
    shortcuts: [{ key: "Escape" }],
    available: visualCommandActive,
    run: (context) => cancelVisualPaneToolShortcut(visualCommandDimension(context)),
  },
  {
    id: "visual3d.slice.previous",
    group: "visual",
    label: "Previous slice",
    shortcuts: [{ key: "ArrowUp" }],
    elements: editorCommandElements("#visual3dPreviousSliceButton"),
    available: (context) => visual3dCommandActive(context) && visualNavigationCommandAvailable(context),
    run: () => (moveVisual3dSlice(-1), true),
  },
  {
    id: "visual3d.slice.next",
    group: "visual",
    label: "Next slice",
    shortcuts: [{ key: "ArrowDown" }],
    elements: editorCommandElements("#visual3dNextSliceButton"),
    available: (context) => visual3dCommandActive(context) && visualNavigationCommandAvailable(context),
    run: () => (moveVisual3dSlice(1), true),
  },
  {
    id: "visual.frame.previous",
    group: "visual",
    label: "Previous frame",
    shortcuts: [{ key: "ArrowLeft" }],
    elements: editorCommandElements("#visualAnimationPreviousFrameButton"),
    available: visualAnimationCommandAvailable,
    run: (context) => moveVisualAnimationFrameFromCommand(context, -1),
  },
  {
    id: "visual.frame.next",
    group: "visual",
    label: "Next frame",
    shortcuts: [{ key: "ArrowRight" }],
    elements: editorCommandElements("#visualAnimationNextFrameButton"),
    available: visualAnimationCommandAvailable,
    run: (context) => moveVisualAnimationFrameFromCommand(context, 1),
  },
  {
    id: "visual3d.scope.slice",
    group: "visual",
    label: "Scope 2D",
    shortcuts: [{ key: "2", modifiers: ["primary"] }],
    elements: editorCommandElements("#visual3dScopeSliceButton"),
    available: visual3dCommandActive,
    run: () => (setVisual3dEditScope("slice"), true),
  },
  {
    id: "visual3d.scope.all",
    group: "visual",
    label: "Scope 3D",
    shortcuts: [{ key: "3", modifiers: ["primary"] }],
    elements: editorCommandElements("#visual3dScopeAllButton"),
    available: visual3dCommandActive,
    run: () => (setVisual3dEditScope("all"), true),
  },
  {
    id: "solver.next",
    group: "solver",
    label: "Next step",
    shortcuts: [{ key: "ArrowRight" }],
    elements: editorCommandElements("#solutionNextButton"),
    available: solverCommandActive,
    run: () => (setSolutionStep(levelSolutionPreview.index + 1), true),
  },
  {
    id: "solver.first",
    group: "solver",
    label: "First step",
    shortcuts: [{ key: "Home" }],
    available: solverCommandActive,
    run: () => (setSolutionStep(0), true),
  },
  {
    id: "solver.last",
    group: "solver",
    label: "Last step",
    shortcuts: [{ key: "End" }],
    available: solverCommandActive,
    run: () => (setSolutionStep(levelSolutionPreview.steps.length - 1), true),
  },
  {
    id: "solver.play",
    group: "solver",
    label: "Play / Pause",
    shortcuts: [{ key: " " }],
    elements: editorCommandElements("#solutionPlayButton"),
    available: solverCommandActive,
    run: () => (toggleSolutionPlayback(), true),
  },
  {
    id: "solver.reset",
    group: "solver",
    label: "Reset",
    shortcuts: [{ key: "r" }],
    elements: editorCommandElements("#solutionResetButton"),
    available: solverCommandActive,
    run: () => (resetSolutionPreview(), true),
  },
  {
    id: "solver.copy",
    group: "solver",
    label: "Copy",
    shortcuts: [{ key: "c", modifiers: ["primary"] }],
    elements: editorCommandElements("#solutionExportButton"),
    available: solverCommandActive,
    run: () => (exportSolution(), true),
  },
  {
    id: "sounds.sfx.play",
    group: "sounds",
    label: "Play SFX",
    shortcuts: [{ key: " " }],
    elements: editorCommandElements("#soundsSfxPlayButton"),
    available: (context) => editorCommandRouteIs(context, "sounds") && soundCommandKind(context) === "sfx",
    run: () => (playSoundSfx().catch((error) => setStatus(`Sounds failed: ${error?.message || error}`, "is-error")), true),
  },
  {
    id: "sounds.sfx.random",
    group: "sounds",
    label: "Randomize SFX",
    shortcuts: [{ key: "r" }],
    elements: editorCommandElements("#soundsSfxRandomButton"),
    available: (context) => editorCommandRouteIs(context, "sounds") && soundCommandKind(context) === "sfx",
    run: () => (randomizeSoundSfx(), true),
  },
  {
    id: "sounds.music.play",
    group: "sounds",
    label: "Play / Pause music",
    shortcuts: [{ key: " " }],
    elements: editorCommandElements("#soundsMusicPlayButton"),
    available: (context) => editorCommandRouteIs(context, "sounds") && soundCommandKind(context) === "music",
    run: () => (toggleSoundMusic().catch((error) => setStatus(`Sounds failed: ${error?.message || error}`, "is-error")), true),
  },
  {
    id: "sounds.music.random",
    group: "sounds",
    label: "Randomize music",
    shortcuts: [{ key: "r" }],
    elements: editorCommandElements("#soundsMusicRandomButton"),
    available: (context) => editorCommandRouteIs(context, "sounds") && soundCommandKind(context) === "music",
    run: () => (randomizeSoundMusic(), true),
  },
  {
    id: "import.copy",
    group: "import",
    label: "Copy",
    shortcuts: [{ key: "c", modifiers: ["primary"] }],
    elements: editorCommandElements("#psImportCopyButton"),
    available: (context) => editorCommandRouteIs(context, "psimport") && !psImportCopyButton.disabled,
    run: () => {
      const api = window.PuzzleStudioImportExport;
      if (typeof api?.copyPuzzleScriptImportOutput !== "function") {
        setEditorStatus("PuzzleScript import is unavailable", "is-error");
        return true;
      }
      api.copyPuzzleScriptImportOutput().catch((error) => {
        console.error(error);
        api.setPuzzleScriptImportStatus?.("Copy failed", "is-error");
      });
      return true;
    },
  },
  {
    id: "import.add",
    group: "import",
    label: "Add file",
    shortcuts: [{ key: "Enter", modifiers: ["primary"] }],
    elements: editorCommandElements("#psImportAddFileButton"),
    allowTextEntry: true,
    available: (context) => editorCommandRouteIs(context, "psimport") && !psImportAddFileButton.disabled,
    run: () => {
      const api = window.PuzzleStudioImportExport;
      if (typeof api?.addPuzzleScriptImportFile !== "function") {
        setEditorStatus("PuzzleScript import is unavailable", "is-error");
        return true;
      }
      api.addPuzzleScriptImportFile().catch((error) => {
        console.error(error);
        api.setPuzzleScriptImportStatus?.(error.message || String(error), "is-error");
      });
      return true;
    },
  },
];

for (let index = 0; index < 9; index += 1) {
  editorCommandDatabase.push({
    id: `level.palette.${index + 1}`,
    group: "level",
    label: (element) => `Paint ${element.dataset.label || `palette item ${index + 1}`}`,
    shortcuts: [{ key: String(index + 1) }],
    elements: editorCommandIndexedElements("#levelPalette .level-token", index),
    available: (context) => levelCommandActive(context) && !levelPlaytestActive,
    run: () => selectLevelPaletteIndex(index),
  });
  editorCommandDatabase.push({
    id: `level3d.palette.${index + 1}`,
    group: "level3d",
    label: (element) => `Paint ${element.dataset.label || `palette item ${index + 1}`}`,
    shortcuts: [{ key: String(index + 1) }],
    elements: () => [
      document.querySelectorAll("#level3dPalette .level3d-token")[index],
      document.querySelectorAll("#level3dLayerPalette .level3d-layer-token")[index],
    ].filter(Boolean),
    available: (context) => level3dCommandActive(context) && !level3dPlaytestActive,
    run: () => selectLevel3dPaletteIndex(index),
  });
}

for (let index = 0; index < 10; index += 1) {
  editorCommandDatabase.push({
    id: `visual.palette.${VISUAL_COLOR_TOKENS[index]}`,
    group: "visual",
    label: `Paint color ${VISUAL_COLOR_TOKENS[index]}`,
    shortcuts: [{ key: VISUAL_COLOR_TOKENS[index] }],
    shortcutOnly: true,
    elements: editorCommandElements(
      `#visualPalette .visual-color-swatch[data-color-index="${index}"], #visual3dPalette .visual-color-swatch[data-color-index="${index}"]`,
    ),
    available: visualCommandActive,
    run: (context) => selectVisualCommandColor(context, index),
  });
}

const editorCommandById = new Map(editorCommandDatabase.map((command) => [command.id, command]));

function resolvedEditorCommandValue(command, key, element = null) {
  const value = command?.[key];
  return typeof value === "function" ? value(element) : value;
}

function editorCommandShortcuts(id, element = null) {
  const command = editorCommandById.get(id);
  if (!command) throw new Error(`Unknown editor command ${id}`);
  const shortcuts = resolvedEditorCommandValue(command, "shortcuts", element);
  if (!Array.isArray(shortcuts) || !shortcuts.length) {
    throw new Error(`Editor command ${id} requires at least one shortcut.`);
  }
  return shortcuts;
}

function editorCommandMatches(id, event) {
  return editorCommandShortcuts(id).some((shortcut) => editorShortcutMatches(event, shortcut));
}

function editorCommandAriaShortcuts(shortcuts) {
  return shortcuts.flatMap((shortcut) => {
    const normalized = normalizeEditorShortcut(shortcut);
    const keys = normalized.keys.map((key) => key === " " ? "Space" : key);
    const shift = normalized.modifiers.includes("shift") ? ["Shift"] : [];
    const primaryVariants = normalized.modifiers.includes("primary")
      ? [["Control"], ["Meta"]]
      : [[]];
    return keys.flatMap((key) => primaryVariants.map((primary) => [...primary, ...shift, key].join("+")));
  }).join(" ");
}

function bindEditorCommandElement(element, command) {
  const label = resolvedEditorCommandValue(command, "label", element);
  const shortcuts = editorCommandShortcuts(command.id, element);
  element.dataset.editorCommand = command.id;
  element.dataset.tooltip = label;
  if (command.shortcutOnly) element.dataset.shortcutOnly = "true";
  else delete element.dataset.shortcutOnly;
  setEditorShortcutHints(element, shortcuts);
  element.setAttribute("aria-keyshortcuts", editorCommandAriaShortcuts(shortcuts));
}

function refreshEditorCommandBindings() {
  for (const command of editorCommandDatabase) {
    if (typeof command.elements !== "function") continue;
    for (const element of command.elements()) {
      bindEditorCommandElement(element, command);
    }
  }
}

function invokeEditorCommand(id, context) {
  const command = editorCommandById.get(id);
  if (!command) throw new Error(`Unknown editor command ${id}`);
  if (typeof command.run !== "function") return false;
  if (typeof command.available === "function" && !command.available(context)) return false;
  if (command.run(context) === false) return false;
  refreshEditorCommandBindings();
  return true;
}

function dispatchEditorCommandEvent(event, options = {}) {
  if (event.defaultPrevented || event.repeat) return false;
  const context = editorCommandContext(event);
  const matches = editorCommandDatabase.filter((command) => {
    if (typeof command.run !== "function") return false;
    if (options.group && command.group !== options.group) return false;
    if (!editorCommandShortcuts(command.id).some((shortcut) => editorShortcutMatches(event, shortcut))) return false;
    if (!command.allowTextEntry && editorCommandTextEntryTarget(event.target)) return false;
    return typeof command.available !== "function" || command.available(context);
  });
  if (matches.length > 1) {
    throw new Error(`Ambiguous editor shortcut: ${matches.map((command) => command.id).join(", ")}`);
  }
  for (const command of matches) {
    if (!invokeEditorCommand(command.id, context)) continue;
    event.preventDefault();
    event.stopImmediatePropagation();
    return true;
  }
  return false;
}

function dispatchEditorCommandClick(event) {
  const element = event.target instanceof Element
    ? event.target.closest("[data-editor-command]")
    : null;
  if (!element || element.disabled) return false;
  const context = editorCommandContext(event, element, "button");
  if (!invokeEditorCommand(element.dataset.editorCommand, context)) return false;
  event.preventDefault();
  event.stopImmediatePropagation();
  return true;
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
document.addEventListener("click", dispatchEditorCommandClick, true);
