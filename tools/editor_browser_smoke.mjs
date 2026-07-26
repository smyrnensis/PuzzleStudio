#!/usr/bin/env node
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import fs from "node:fs";
import net from "node:net";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

const args = parseArgs(process.argv.slice(2));
const editorBin = requiredPath(args.editorBin, "--editor-bin");
const fixture2d = path.resolve(repoRoot, args.fixture || "crates/lang/tests/fixtures/spec_2d_microban_basic.puzzle");
const fixture3d = path.resolve(repoRoot, args.fixture3d || "games/spec_3d.puzzle");
const importFileOnly = args.importFileOnly ? path.resolve(repoRoot, args.importFileOnly) : "";
const chromePath = resolveChrome(args.chrome);
const headless = !args.headed;
const sourceInputOnly = Boolean(args.sourceInputOnly);
const visualPaletteOnly = Boolean(args.visualPaletteOnly);
const visualClipFillOnly = Boolean(args.visualClipFillOnly);
const sourceEditingCommandsOnly = Boolean(args.sourceEditingCommandsOnly);
const sourceSelectionOnly = Boolean(args.sourceSelectionOnly);
const sourceOptionDragOnly = Boolean(args.sourceOptionDragOnly);
const sourceOccurrenceSelectionOnly = Boolean(args.sourceOccurrenceSelectionOnly);
const levelSelectionRevisionOnly = Boolean(args.levelSelectionRevisionOnly);
const indexControlLayoutOnly = Boolean(args.indexControlLayoutOnly);
const initialPreviewOnly = Boolean(args.initialPreviewOnly);

const failures = [];

async function main() {
  const browser = new Browser(chromePath, { headless });
  await browser.start();
  const page = await browser.newPage();
  if (initialPreviewOnly) {
    await page.send("Page.addScriptToEvaluateOnNewDocument", {
      source: `(() => {
        let runtime = null;
        Object.defineProperty(window, "PuzzleStudioRuntime", {
          configurable: true,
          get() {
            return runtime;
          },
          set(value) {
            runtime = value;
            runtime.sourceEntryInfo = () => new Promise(() => {});
          },
        });
      })();`,
    });
  }
  try {
    await withEditorServer(fixture2d, async (server) => {
      await page.navigate(server.url);
      await editorLoads(page);
      if (initialPreviewOnly) {
        await initialPreviewStartsRuntime(page);
        return;
      }
      if (indexControlLayoutOnly) {
        await visualAnimationIndexControlStaysCentered(page);
        return;
      }
      if (levelSelectionRevisionOnly) {
        await levelSelectionWaitsForCurrentEntries(page);
        return;
      }
      if (visualClipFillOnly) {
        await visualBucketFillRespectsActiveClip(page);
        return;
      }
      if (sourceOccurrenceSelectionOnly) {
        await sourceOccurrenceSelectionShortcutsMatchVsCode(page);
        return;
      }
      if (sourceOptionDragOnly) {
        await sourceOptionDragCreatesMultipleCursors(page);
        return;
      }
      if (sourceSelectionOnly) {
        await sourcePlainClickCollapsesSelection(page);
        await sourceMultiLineSelectionStartsAtTextColumn(page);
        await sourceOptionDragCreatesMultipleCursors(page);
        return;
      }
      if (importFileOnly) {
        await fileInputImportAddsExternalPuzzleDocument(page, importFileOnly);
        return;
      }
      await visualPaletteMouseClickPreservesPaneScroll(page);
      await visualColorEditorOffersPresetPalette(page);
      await visualAnimationIndexControlStaysCentered(page);
      if (visualPaletteOnly) {
        return;
      }
      if (sourceEditingCommandsOnly) {
        await sourceCodeMirrorEditingCommandsReachWorkflow(page);
        await sourceOutlineNavigationCentersCursor(page);
        await sourceAddControlOverlaysEmptyLineWithoutChangingEditorWidth(page);
        await sourceMultiLineSelectionStartsAtTextColumn(page);
        await sourceOptionDragCreatesMultipleCursors(page);
        await sourceOccurrenceSelectionShortcutsMatchVsCode(page);
        await sourceEntryRefreshIgnoresSupersededEdits(page);
        await sourceCompletionPopoverStaysInsideEditor(page);
        return;
      }
      await sourceEditorReflectsInputBeforeKeyup(page);
      await sourceEditorPairsDoubleQuote(page);
      await sourceUndoReturnsToEditedLocationAfterCursorMove(page);
      await sourceUndoSurvivesSameDocumentReload(page);
      await sourceEditorReflectsCompositionBeforeCommit(page);
      await sourceCompletionKeepsKeyboardSelectionAcrossRefresh(page);
      await sourceCompletionPopoverStaysInsideEditor(page);
      await sourceCodeMirrorEditingCommandsReachWorkflow(page);
      await sourceOutlineNavigationCentersCursor(page);
      await sourceAddControlOverlaysEmptyLineWithoutChangingEditorWidth(page);
      await sourcePlainClickCollapsesSelection(page);
      await sourceMultiLineSelectionStartsAtTextColumn(page);
      await sourceOptionDragCreatesMultipleCursors(page);
      await sourceOccurrenceSelectionShortcutsMatchVsCode(page);
      await sourceEntryRefreshIgnoresSupersededEdits(page);
      await sourceRewritePatternTabCopiesLhsToEmptyRhs(page);
      await fileInputImportAddsPuzzleDocument(page);
      if (sourceInputOnly) {
        return;
      }
      await runPreviewStartsRuntime(page);
      await sourceEditKeepsCompiledPreviewRunning(page);
      await levelPlaytestKeyboardChangesBoardWithoutSavingSource(page);
      await sourceLevelAsciiClickOpensLevelEditor(page);
    });

    if (visualClipFillOnly) {
      await withEditorServer(fixture3d, async (server) => {
        await page.navigate(server.url);
        await editorLoads(page);
        await visual3dBucketFillRespectsActiveClip(page);
      });
    } else if (!initialPreviewOnly && !sourceInputOnly && !sourceEditingCommandsOnly && !sourceSelectionOnly && !sourceOptionDragOnly && !sourceOccurrenceSelectionOnly && !levelSelectionRevisionOnly && !visualPaletteOnly && !importFileOnly) {
      await withEditorServer(fixture3d, async (server) => {
        await page.navigate(server.url);
        await editorLoads(page);
        if (indexControlLayoutOnly) {
          await visual3dIndexControlStaysCentered(page);
          await level3dIndexControlStaysCentered(page);
          return;
        }
        await visual3dIndexControlStaysCentered(page);
        await level3dPreviewUpdateReachesRuntime(page);
      });
    }
  } finally {
    await page.close().catch(() => {});
    await browser.close();
  }

  if (failures.length) {
    throw new Error(`browser smoke saw page errors:\n- ${failures.join("\n- ")}`);
  }
  console.log("editor browser smoke tests passed");
}

async function visualPaletteMouseClickPreservesPaneScroll(page) {
  await clickTop(page, "#visualModeButton");
  await page.waitForTop(
    `Boolean(document.querySelector("#visualBuilder") && !document.querySelector("#visualBuilder").hidden)`,
    "2D visual pane"
  );
  const clickPoint = await page.evaluateTop(`(() => {
    const palette = document.querySelector("#visualPalette");
    const button = palette?.querySelector('[data-visual-brush-preset="thick"]');
    const scroller = palette?.closest(".tool-pane-scroll");
    if (!palette || !button || !scroller) {
      throw new Error("missing visual palette scroll test target");
    }
    window.__visualPaletteSmokeScroller = scroller;
    window.__visualPaletteSmokeOriginalStyle = scroller.style.cssText;
    window.__visualPaletteSmokeClicked = false;
    scroller.style.height = "458px";
    scroller.style.maxHeight = "458px";
    scroller.style.overflow = "auto";
    const maxScroll = scroller.scrollHeight - scroller.clientHeight;
    scroller.scrollTop = Math.min(70, maxScroll);
    if (scroller.scrollTop <= 0) {
      throw new Error(
        "visual palette smoke requires overflow: " + JSON.stringify({
          clientHeight: scroller.clientHeight,
          scrollHeight: scroller.scrollHeight,
        })
      );
    }
    const buttonRect = button.getBoundingClientRect();
    const scrollerRect = scroller.getBoundingClientRect();
    if (buttonRect.bottom <= scrollerRect.top || buttonRect.top >= scrollerRect.bottom) {
      throw new Error("visual palette smoke button is outside the scroller viewport");
    }
    window.__visualPaletteSmokeBaseline = scroller.scrollTop;
    button.addEventListener("click", () => {
      window.__visualPaletteSmokeClicked = true;
    }, { once: true });
    return {
      x: Math.round(buttonRect.left + buttonRect.width / 2),
      y: Math.round(buttonRect.top + buttonRect.height / 2),
    };
  })()`);

  try {
    await clickViewport(page, clickPoint);
    const result = await page.evaluateTop(`(() => {
      const scroller = window.__visualPaletteSmokeScroller;
      const button = document.querySelector('#visualPalette [data-visual-brush-preset="thick"]');
      return {
        baseline: window.__visualPaletteSmokeBaseline,
        scrollTop: scroller?.scrollTop ?? null,
        clicked: window.__visualPaletteSmokeClicked === true,
        buttonFocused: document.activeElement === button,
      };
    })()`);
    assert.equal(result.clicked, true, "visual palette button click should still fire");
    assert.equal(
      result.scrollTop,
      result.baseline,
      `visual palette mouse click changed pane scroll: ${JSON.stringify(result)}`
    );
    assert.equal(result.buttonFocused, false, "visual palette mouse click should not take focus");
  } finally {
    await page.evaluateTop(`(() => {
      const scroller = window.__visualPaletteSmokeScroller;
      if (scroller) {
        scroller.style.cssText = window.__visualPaletteSmokeOriginalStyle || "";
        scroller.scrollTop = 0;
      }
      delete window.__visualPaletteSmokeScroller;
      delete window.__visualPaletteSmokeOriginalStyle;
      delete window.__visualPaletteSmokeBaseline;
      delete window.__visualPaletteSmokeClicked;
      return true;
    })()`).catch(() => {});
    await clickTop(page, "#playModeButton").catch(() => {});
  }
  await page.assertNoErrors("visual palette mouse focus");
}

async function visualBucketFillRespectsActiveClip(page) {
  await clickTop(page, "#visualModeButton");
  await page.waitForTop(
    `Boolean(document.querySelector("#visualBuilder") && !document.querySelector("#visualBuilder").hidden)`,
    "2D visual pane"
  );
  await page.evaluateTop(`(() => {
    visual.palette = [{ color: "#111111" }, { color: "#eeeeee" }];
    resetVisualBuilder(3, 3);
    visual.cells.fill(0);
    resetVisualAnimationFramesFromCurrentCells();
    visual.selectedColorIndex = 1;
    visualClipSelection = { x: 1, y: 1, width: 1, height: 1 };
    visualClipActive = true;
    visualBucketActive = false;
    renderVisualBuilder();
    return true;
  })()`);

  await clickTop(page, "#visualFillButton");
  const activeTools = await page.evaluateTop(`({
    clipActive: visualClipActive,
    clipSelection: visualClipSelection,
    bucketActive: visualBucketActive,
  })`);
  assert.deepEqual(activeTools, {
    clipActive: true,
    clipSelection: { x: 1, y: 1, width: 1, height: 1 },
    bucketActive: true,
  }, "enabling bucket fill must retain the active clip scope");

  const clickPoint = await page.evaluateTop(`(() => {
    const index = visual.width + 1;
    const cell = document.querySelector('#visualBoard .visual-cell[data-index="' + index + '"]');
    if (!cell) throw new Error("missing clipped bucket-fill target cell");
    cell.scrollIntoView({ block: "center", inline: "center" });
    const rect = cell.getBoundingClientRect();
    return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
  })()`);
  await clickViewport(page, clickPoint);

  const result = await page.evaluateTop(`(() => {
    const target = visual.width + 1;
    return {
      cells: visual.cells.map((color, index) => ({ index, color })).filter(({ color }) => color !== 0),
      allCells: visual.cells,
      target,
      clipActive: visualClipActive,
      clipSelection: visualClipSelection,
      bucketActive: visualBucketActive,
      selectedColorIndex: visual.selectedColorIndex,
      status: document.querySelector("#visualActionStatus")?.textContent || "",
    };
  })()`);
  assert.deepEqual(result.cells, [{ index: result.target, color: 1 }], JSON.stringify(result));
  assert.equal(result.clipActive, true);
  assert.deepEqual(result.clipSelection, { x: 1, y: 1, width: 1, height: 1 });
  assert.equal(result.bucketActive, false);
  await page.assertNoErrors("visual bucket fill clip scope");
}

async function visual3dBucketFillRespectsActiveClip(page) {
  await clickTop(page, "#visualDimension3dButton");
  await clickTop(page, "#visualModeButton");
  await page.waitForTop(
    `Boolean(document.querySelector("#visual3dBuilder") && !document.querySelector("#visual3dBuilder").hidden)`,
    "3D visual pane"
  );
  await page.evaluateTop(`(() => {
    visual3d.palette = [{ color: "#111111" }, { color: "#eeeeee" }];
    visual3d.axis = "z";
    visual3d.slice = 1;
    visual3d.editScope = "slice";
    resetVisual3dBuilder(3, 3, 3);
    visual3d.cells.fill(0);
    visual3d.frames = [visual3d.cells];
    visual3d.selectedColorIndex = 1;
    visual3dClipSelection = { minX: 1, maxX: 1, minY: 1, maxY: 1, minZ: 1, maxZ: 1 };
    visual3dClipActive = true;
    visual3dBucketActive = false;
    renderVisual3dBuilder();
    return true;
  })()`);

  await clickTop(page, "#visual3dFillButton");
  const activeTools = await page.evaluateTop(`({
    clipActive: visual3dClipActive,
    clipSelection: visual3dClipSelection,
    bucketActive: visual3dBucketActive,
  })`);
  assert.deepEqual(activeTools, {
    clipActive: true,
    clipSelection: { minX: 1, maxX: 1, minY: 1, maxY: 1, minZ: 1, maxZ: 1 },
    bucketActive: true,
  }, "enabling 3D bucket fill must retain the active clip scope");

  const clickPoint = await page.evaluateTop(`(() => {
    const cell = document.querySelector('#visual3dSliceBoard .visual-cell[data-index="4"]');
    if (!cell) throw new Error("missing clipped 3D bucket-fill target cell");
    cell.scrollIntoView({ block: "center", inline: "center" });
    const rect = cell.getBoundingClientRect();
    return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
  })()`);
  await clickViewport(page, clickPoint);

  const result = await page.evaluateTop(`(() => ({
    cells: visual3d.cells.map((color, index) => ({ index, color })).filter(({ color }) => color !== 0),
    target: visual3dCellIndex(1, 1, 1),
    clipActive: visual3dClipActive,
    clipSelection: visual3dClipSelection,
    bucketActive: visual3dBucketActive,
    status: document.querySelector("#visual3dActionStatus")?.textContent || "",
  }))()`);
  assert.deepEqual(result.cells, [{ index: result.target, color: 1 }], JSON.stringify(result));
  assert.equal(result.clipActive, true);
  assert.deepEqual(result.clipSelection, { minX: 1, maxX: 1, minY: 1, maxY: 1, minZ: 1, maxZ: 1 });
  assert.equal(result.bucketActive, false);
  await page.assertNoErrors("3D visual bucket fill clip scope");
}

async function visualColorEditorOffersPresetPalette(page) {
  await clickTop(page, "#visualModeButton");
  await page.waitForTop(
    `Boolean(document.querySelector("#visualBuilder") && !document.querySelector("#visualBuilder").hidden)`,
    "2D visual pane"
  );
  const before = await page.evaluateTop(`(() => {
    const button = document.querySelector("#visualPalette .visual-current-color-button");
    if (!button || button.getAttribute("aria-disabled") === "true") {
      throw new Error("missing editable current visual color");
    }
    return {
      title: button.title,
      ariaLabel: button.getAttribute("aria-label"),
      tooltip: compactEditorTooltipText(button.title),
    };
  })()`);
  assert.equal(before.title, "Edit selected color");
  assert.match(before.ariaLabel || "", /^Edit selected color /);
  assert.equal(before.tooltip, "Edit color");

  await clickTop(page, "#visualPalette .visual-current-color-button");
  const opened = await page.evaluateTop(`(() => {
    const menu = document.querySelector("#visualPalette .visual-current-editor-panel .visual-color-menu");
    const presets = menu?.querySelectorAll(".visual-preset-grid .visual-color-preset") || [];
    return {
      expanded: document.querySelector("#visualPalette .visual-current-color-button")?.getAttribute("aria-expanded"),
      menuVisible: Boolean(menu && !menu.hidden),
      presetCount: presets.length,
    };
  })()`);
  assert.deepEqual(opened, {
    expanded: "true",
    menuVisible: true,
    presetCount: 16,
  });
  await clickTop(page, '#visualPalette .visual-current-editor-panel [aria-label="Use color #00e436"]');
  const selectedPreset = await page.evaluateTop(`(() => {
    const preset = document.querySelector(
      '#visualPalette .visual-current-editor-panel [aria-label="Use color #00e436"]'
    );
    return {
      selected: Boolean(preset?.classList.contains("is-selected")),
      color: normalizeVisualColor(visual.palette[visual.selectedColorIndex]?.color),
    };
  })()`);
  assert.deepEqual(selectedPreset, {
    selected: true,
    color: "#00e436",
  });
  await clickTop(page, "#playModeButton");
  await page.assertNoErrors("visual color edit presets");
}

async function visualAnimationIndexControlStaysCentered(page) {
  await clickTop(page, "#visualModeButton");
  await page.waitForTop(
    `Boolean(document.querySelector("#visualBuilder") && !document.querySelector("#visualBuilder").hidden)`,
    "2D visual pane"
  );
  const animationMode = await page.evaluateTop(
    `document.querySelector("#visualBuilder")?.classList.contains("is-animation-mode") === true`
  );
  if (!animationMode) {
    await clickTop(page, "#visualAnimateModeButton");
  }
  await page.waitForTop(
    `document.querySelector("#visualBuilder .visual-animation-toolbar")?.getClientRects().length > 0`,
    "visual animation index control"
  );
  await assertIndexControlLayout(page, "#visualBuilder .visual-animation-toolbar", "visual animation frames");
  await clickTop(page, "#playModeButton");
  await page.assertNoErrors("visual animation index control layout");
}

async function visual3dIndexControlStaysCentered(page) {
  await clickTop(page, "#visualDimension3dButton");
  await clickTop(page, "#visualModeButton");
  await page.waitForTop(
    `Boolean(
      document.querySelector("#visual3dBuilder")
      && !document.querySelector("#visual3dBuilder").hidden
      && document.querySelector("#visual3dBuilder .visual3d-preview-wrap > .compact-control-strip")?.getClientRects().length
    )`,
    "3D visual index control"
  );
  await assertIndexControlLayout(
    page,
    "#visual3dBuilder .visual3d-preview-wrap > .compact-control-strip",
    "3D visual slices"
  );
  await clickTop(page, "#playModeButton");
  await page.assertNoErrors("3D visual index control layout");
}

async function level3dIndexControlStaysCentered(page) {
  await page.evaluateTop(`(() => {
    const source = document.querySelector(".level3d-layer-toolbar");
    if (!source) {
      throw new Error("missing 3D level layout fixture");
    }
    const fixture = source.cloneNode(true);
    fixture.dataset.indexControlLayoutFixture = "";
    fixture.querySelectorAll("[id]").forEach((element) => element.removeAttribute("id"));
    fixture.style.position = "fixed";
    fixture.style.top = "20px";
    fixture.style.left = "20px";
    document.body.append(fixture);
    return true;
  })()`);
  await page.waitForTop(
    `document.querySelector("[data-index-control-layout-fixture]")?.getClientRects().length > 0`,
    "3D level index control"
  );
  await assertIndexControlLayout(page, "[data-index-control-layout-fixture]", "3D level slices");
  await page.assertNoErrors("3D level index control layout");
}

async function assertIndexControlLayout(page, stripSelector, label) {
  const result = await page.evaluateTop(`(() => {
    const strip = document.querySelector(${JSON.stringify(stripSelector)});
    if (!strip || !strip.getClientRects().length) {
      throw new Error("missing visible compact control strip");
    }
    const indexControl = strip.querySelector(":scope > .index-control");
    const current = indexControl?.querySelector(":scope > .index-control__current");
    const separator = indexControl?.querySelector(":scope > .index-control__separator");
    const total = indexControl?.querySelector(":scope > .index-control__total");
    if (!indexControl || !current || !separator || !total) {
      throw new Error("incomplete index control structure");
    }
    const centerY = (element) => {
      const rect = element.getBoundingClientRect();
      return rect.top + rect.height / 2;
    };
    const stripCenter = centerY(strip);
    const deltas = {};
    for (const [name, element] of [
      ["index", indexControl],
      ["current", current],
      ["separator", separator],
      ["total", total],
    ]) {
      deltas[name] = Math.abs(centerY(element) - stripCenter);
    }
    for (const [index, element] of Array.from(strip.children).entries()) {
      if (element.matches(".compact-control-strip__label, .compact-control-strip__button")) {
        deltas["item-" + index] = Math.abs(centerY(element) - stripCenter);
      }
      if (element.matches(".compact-control-strip__button")) {
        const icon = element.querySelector(":scope > svg");
        if (!icon) {
          throw new Error("compact control strip button has no direct SVG icon");
        }
        deltas["icon-" + index] = Math.abs(centerY(icon) - centerY(element));
      }
    }
    return {
      deltas,
      indexControlCount: strip.querySelectorAll(":scope > .index-control").length,
    };
  })()`);
  assert.equal(result.indexControlCount, 1, `${label} should have one shared index control`);
  for (const [part, delta] of Object.entries(result.deltas)) {
    assert.ok(delta <= 0.5, `${label} ${part} is vertically misaligned by ${delta}px`);
  }
}

async function editorLoads(page) {
  await page.waitForTop(
    `Boolean(
      document.querySelector("#sourceEditor")
      && document.querySelector("#previewFrame")
      && document.querySelector("#runButton")
      && document.querySelector("#editModeButton")
    )`,
    "editor shell"
  );
  await page.waitForTop(
    `Boolean(document.querySelector("#sourceEditor")?.value.includes("puzzle"))`,
    "editor source load",
    { timeoutMs: 20_000 }
  );
  await page.assertNoErrors("editor load");
}

async function runPreviewStartsRuntime(page) {
  await clickTop(page, "#runButton");
  await page.waitForTop(
    `Boolean(typeof latestHtml !== "undefined" && latestHtml.includes("PuzzleExport"))`,
    "compiled preview HTML",
    { timeoutMs: 20_000 }
  );
  await waitForTopWithDiagnostics(
    page,
    `Boolean(
      typeof latestPreviewRuntimeStatus !== "undefined"
      && latestPreviewRuntimeStatus
      && latestPreviewRuntimeStatus.title === "PuzzleStudio HTML Export"
    )`,
    "compiled 2D preview runtime load",
    { timeoutMs: 20_000 }
  );
  await page.assertNoErrors("run preview");
}

async function initialPreviewStartsRuntime(page) {
  try {
    await page.waitForTop(
      `Boolean(
        document.querySelector("#previewFrame")?.srcdoc?.length > 1_000
        && document.querySelector("#runButton")?.getAttribute("aria-label") === "Refresh preview"
      )`,
      "initial compiled preview document",
      { timeoutMs: 20_000 }
    );
  } catch (error) {
    const state = await page.evaluateTop(`(() => ({
      editorStatus: document.querySelector("#editorStatusLabel")?.textContent || "",
      paneStatus: document.querySelector("#previewPaneStatus")?.textContent || "",
      previewLog: document.querySelector("#previewLog")?.textContent || "",
      previewFrameLength: document.querySelector("#previewFrame")?.srcdoc?.length || 0,
      runButtonLabel: document.querySelector("#runButton")?.getAttribute("aria-label") || "",
      runButtonDisabled: Boolean(document.querySelector("#runButton")?.disabled),
    }))()`);
    throw new Error(`${error.message}: ${JSON.stringify(state)}`);
  }
}

async function sourceEditKeepsCompiledPreviewRunning(page) {
  const before = await page.evaluateTop(`(() => {
    const editor = document.querySelector("#sourceEditor");
    const frame = document.querySelector("#previewFrame");
    if (!editor || !frame || !latestHtml) {
      throw new Error("missing compiled preview for source dirty smoke");
    }
    const source = editor.value || "";
    const insertAt = Math.max(0, source.indexOf("\\n"));
    window.__sourceDirtySmokeOriginal = source;
    window.__sourceDirtySmokeFrame = frame;
    window.__sourceDirtySmokeSrcdoc = frame.srcdoc || "";
    editor.focus();
    editor.setSelectionRange(insertAt, insertAt);
    return {
      source,
      srcdocLength: frame.srcdoc.length,
      latestHtmlLength: latestHtml.length,
    };
  })()`);
  assert.ok(before.srcdocLength > 0, "compiled preview frame should have srcdoc before source edit");
  assert.ok(before.latestHtmlLength > 0, "latest compiled preview HTML should exist before source edit");

  try {
    await page.evaluateTop(`(() => {
      if (typeof handleSourcePrintableKeydownInput !== "function") {
        throw new Error("missing handleSourcePrintableKeydownInput");
      }
      handleSourcePrintableKeydownInput({
        defaultPrevented: false,
        isComposing: false,
        altKey: false,
        ctrlKey: false,
        metaKey: false,
        key: " ",
        preventDefault() {},
        stopPropagation() {},
      });
      return true;
    })()`);
    await page.waitForTop(
      `(() => {
        const frame = document.querySelector("#previewFrame");
        return Boolean(
          frame
          && frame === window.__sourceDirtySmokeFrame
          && frame.srcdoc === window.__sourceDirtySmokeSrcdoc
          && latestHtml.length > 0
          && previewExport
          && compiledPreviewStale === true
          && previewDocumentLoaded === true
        );
      })()`,
      "source edit keeps compiled preview running",
      { timeoutMs: 5_000 }
    );
  } finally {
    await page.evaluateTop(`(() => {
      const original = window.__sourceDirtySmokeOriginal;
      if (typeof original === "string") {
        setSourceEditorValue(original, { resetUndo: true });
        if (documents[currentDocumentIndex]) {
          documents[currentDocumentIndex].source = original;
        }
        scheduleSourceHighlight(true);
        scheduleLocalSave();
      }
      delete window.__sourceDirtySmokeOriginal;
      delete window.__sourceDirtySmokeFrame;
      delete window.__sourceDirtySmokeSrcdoc;
      return true;
    })()`).catch(() => {});
  }

  await clickTop(page, "#runButton");
  await page.waitForTop(
    `Boolean(compiledPreviewStale === false && latestHtml.includes("PuzzleExport"))`,
    "recompiled preview after source dirty smoke",
    { timeoutMs: 20_000 }
  );
  await page.assertNoErrors("source edit keeps preview running");
}

async function sourceLevelAsciiClickOpensLevelEditor(page) {
  const source = `title "Ascii Click Smoke"

puzzle main {
  layers {
    actor = Player
  }

  rules {
  }
}

levels main of main {
  legend {
    . = empty
    P = Player
  }

  level ascii
  P
}
`;
  await page.evaluateTop(`(() => {
    const source = ${JSON.stringify(source)};
    setSourceEditorValue(source, { resetUndo: true });
    if (documents[currentDocumentIndex]) {
      documents[currentDocumentIndex].source = source;
    }
    sourceEditor.setSelectionRange(0, 0);
    scheduleSourceHighlight(true);
    scheduleLocalSave();
    return true;
  })()`);
  await clickTop(page, "#runButton");
  await waitForTopWithDiagnostics(
    page,
    `Boolean(previewExport?.levels?.some((level) => level.name === "main.ascii" || level.name === "ascii"))`,
    "level ascii preview export",
    { timeoutMs: 20_000 }
  );
  const clickPoint = await page.evaluateTop(`(() => {
    const editor = document.querySelector("#sourceEditor");
    const source = editor?.value || "";
    const offset = source.indexOf("level ascii");
    if (offset < 0) {
      throw new Error("missing level ascii source");
    }
    const linesBefore = source.slice(0, offset).split("\\n").length - 1;
    const lineHeight = sourceEditorLineHeight();
    editor.scrollTop = Math.max(0, (linesBefore - 4) * lineHeight);
    syncSourceHighlightScroll();
    const point = sourceVisualCaretPoint(offset);
    if (!point) {
      throw new Error("missing visual caret point for level ascii");
    }
    const rect = sourceEditorWrap.getBoundingClientRect();
    return {
      x: Math.round(rect.left + point.left + 8),
      y: Math.round(rect.top + point.top + lineHeight / 2),
    };
  })()`);
  await clickViewport(page, clickPoint);
  await waitForTopWithDiagnostics(
    page,
    `Boolean(
      currentPreviewMode === "edit"
      && document.querySelector("#levelBuilder")
      && !document.querySelector("#levelBuilder").hidden
      && document.querySelector("#levelNameInput")?.value === "ascii"
    )`,
    "level ascii source click opens 2D level editor",
    { timeoutMs: 10_000 }
  );
  await page.assertNoErrors("level ascii source click");
}

async function sourceEditorReflectsInputBeforeKeyup(page) {
  await page.evaluateTop(`(() => {
    const editor = document.querySelector("#sourceEditor");
    if (!editor) {
      throw new Error("missing #sourceEditor");
    }
    const original = editor.value || "";
    const insertAt = Math.max(0, original.indexOf("\\n"));
    const expected = \`\${original.slice(0, insertAt)}a\${original.slice(insertAt)}\`;
    window.__sourceRealtimeOriginal = original;
    window.__sourceRealtimeExpected = expected;
    window.__sourceRealtimeInputEvents = 0;
    editor.addEventListener("input", () => {
      window.__sourceRealtimeInputEvents += 1;
    }, { once: true });
    editor.focus();
    editor.setSelectionRange(insertAt, insertAt);
    return true;
  })()`);

  try {
    await page.evaluateTop(`(() => {
      if (typeof handleSourcePrintableKeydownInput !== "function") {
        throw new Error("missing handleSourcePrintableKeydownInput");
      }
      handleSourcePrintableKeydownInput({
        defaultPrevented: false,
        isComposing: false,
        altKey: false,
        ctrlKey: false,
        metaKey: false,
        key: "a",
        preventDefault() {},
        stopPropagation() {},
      });
      return true;
    })()`);
    try {
      await page.waitForTop(
        `(() => {
          const editor = document.querySelector("#sourceEditor");
          const highlight = document.querySelector("#sourceHighlight");
          const expected = window.__sourceRealtimeExpected;
          return Boolean(
            expected
            && editor?.value === expected
            && highlight?.textContent === expected
            && documents[currentDocumentIndex]?.source === expected
          );
        })()`,
        "source input reflection before keyup",
        { timeoutMs: 5_000 }
      );
    } catch (error) {
      const diagnostics = await page.evaluateTop(`(() => {
        const editor = document.querySelector("#sourceEditor");
        const highlight = document.querySelector("#sourceHighlight");
        const expected = window.__sourceRealtimeExpected || "";
        const value = editor?.value || "";
        const highlightText = highlight?.textContent || "";
        const doc = documents[currentDocumentIndex]?.source || "";
        return {
          expectedHead: expected.slice(0, 80),
          valueHead: value.slice(0, 80),
          highlightHead: highlightText.slice(0, 80),
          docHead: doc.slice(0, 80),
          expectedMatchesValue: value === expected,
          expectedMatchesHighlight: highlightText === expected,
          expectedMatchesDocument: doc === expected,
          inputEvents: window.__sourceRealtimeInputEvents || 0,
          selectionStart: editor?.selectionStart ?? null,
          selectionEnd: editor?.selectionEnd ?? null,
        };
      })()`).catch((diagnosticError) => ({ diagnosticError: diagnosticError.message }));
      throw new Error(`${error.message}\nSource diagnostics: ${JSON.stringify(diagnostics, null, 2)}`);
    }
  } finally {
    await page.evaluateTop(`true`).catch(() => {});
    await page.evaluateTop(`(() => {
      const original = window.__sourceRealtimeOriginal;
      if (typeof original !== "string") {
        return false;
      }
      setSourceEditorValue(original, { resetUndo: true });
      if (documents[currentDocumentIndex]) {
        documents[currentDocumentIndex].source = original;
      }
      scheduleSourceHighlight(true);
      delete window.__sourceRealtimeOriginal;
      delete window.__sourceRealtimeExpected;
      delete window.__sourceRealtimeInputEvents;
      return true;
    })()`).catch(() => {});
  }
  await page.assertNoErrors("source input reflection");
}

async function sourceEditorPairsDoubleQuote(page) {
  const result = await page.evaluateTop(`(() => {
    const editor = document.querySelector("#sourceEditor");
    if (!editor || typeof handleSourcePrintableKeydownInput !== "function") {
      throw new Error("missing source editor quote pair helpers");
    }
    const original = editor.value || "";
    const originalDocumentSource = documents[currentDocumentIndex]?.source || "";
    const result = {};
    const quoteEvent = () => ({
      defaultPrevented: false,
      isComposing: false,
      altKey: false,
      ctrlKey: false,
      metaKey: false,
      key: String.fromCharCode(34),
      preventDefault() {},
      stopPropagation() {},
    });
    try {
      setSourceEditorValue("", { resetUndo: true });
      if (documents[currentDocumentIndex]) {
        documents[currentDocumentIndex].source = "";
      }
      editor.focus();
      editor.setSelectionRange(0, 0);
      result.emptyHandled = handleSourcePrintableKeydownInput(quoteEvent());
      result.empty = {
        value: editor.value,
        selectionStart: editor.selectionStart,
        selectionEnd: editor.selectionEnd,
        documentSource: documents[currentDocumentIndex]?.source || "",
      };

      setSourceEditorValue("abc", { resetUndo: true });
      if (documents[currentDocumentIndex]) {
        documents[currentDocumentIndex].source = "abc";
      }
      editor.setSelectionRange(0, 3);
      result.selectionHandled = handleSourcePrintableKeydownInput(quoteEvent());
      result.selection = {
        value: editor.value,
        selectionStart: editor.selectionStart,
        selectionEnd: editor.selectionEnd,
        documentSource: documents[currentDocumentIndex]?.source || "",
      };
    } finally {
      setSourceEditorValue(original, { resetUndo: true });
      if (documents[currentDocumentIndex]) {
        documents[currentDocumentIndex].source = originalDocumentSource;
      }
      scheduleSourceHighlight(true);
    }
    return result;
  })()`);
  assert.equal(result.emptyHandled, true);
  assert.deepEqual(result.empty, {
    value: String.fromCharCode(34, 34),
    selectionStart: 1,
    selectionEnd: 1,
    documentSource: String.fromCharCode(34, 34),
  });
  assert.equal(result.selectionHandled, true);
  assert.deepEqual(result.selection, {
    value: `${String.fromCharCode(34)}abc${String.fromCharCode(34)}`,
    selectionStart: 1,
    selectionEnd: 4,
    documentSource: `${String.fromCharCode(34)}abc${String.fromCharCode(34)}`,
  });
  await page.assertNoErrors("source double quote pairing");
}

async function sourceUndoReturnsToEditedLocationAfterCursorMove(page) {
  await page.evaluateTop(`(() => {
    const editor = document.querySelector("#sourceEditor");
    if (
      !editor
      || typeof handleSourcePrintableKeydownInput !== "function"
      || typeof handleSourceUndoShortcut !== "function"
      || typeof resetSourceUndoHistory !== "function"
    ) {
      throw new Error("missing source undo cursor helpers");
    }
    const original = editor.value || "";
    const source = "first\\nsecond\\nthird";
    const editAt = source.indexOf("second") + "second".length;
    setSourceEditorValue(source, { resetUndo: true });
    if (documents[currentDocumentIndex]) {
      documents[currentDocumentIndex].source = source;
    }
    editor.focus();
    editor.setSelectionRange(0, 0);
    resetSourceUndoHistory();
    editor.setSelectionRange(editAt, editAt);
    const editHandled = handleSourcePrintableKeydownInput({
      defaultPrevented: false,
      isComposing: false,
      altKey: false,
      ctrlKey: false,
      metaKey: false,
      key: "!",
      preventDefault() {},
      stopPropagation() {},
    });
    const edited = "first\\nsecond!\\nthird";
    if (!editHandled || editor.value !== edited) {
      throw new Error("source edit did not create expected undo setup");
    }
    const undoHandled = handleSourceUndoShortcut({
      altKey: false,
      ctrlKey: false,
      metaKey: true,
      shiftKey: false,
      key: "z",
      preventDefault() {},
      stopPropagation() {},
    });
    const result = {
      undoHandled,
      value: editor.value,
      selectionStart: editor.selectionStart,
      selectionEnd: editor.selectionEnd,
    };
    setSourceEditorValue(original, { resetUndo: true });
    if (documents[currentDocumentIndex]) {
      documents[currentDocumentIndex].source = original;
    }
    scheduleSourceHighlight(true);
    if (
      !result.undoHandled
      || result.value !== source
      || result.selectionStart !== editAt
      || result.selectionEnd !== editAt
    ) {
      throw new Error(\`source undo did not return to edited location: \${JSON.stringify(result)}\`);
    }
    return true;
  })()`);
  await page.assertNoErrors("source undo cursor location");
}

async function sourceUndoSurvivesSameDocumentReload(page) {
  await page.evaluateTop(`(() => {
    const editor = document.querySelector("#sourceEditor");
    if (!editor || typeof loadEmbeddedDocument !== "function" || typeof handleSourceUndoShortcut !== "function") {
      throw new Error("missing source undo reload helpers");
    }
    const original = editor.value || "";
    const insertAt = Math.max(0, original.indexOf("\\n"));
    setSourceEditorValue(original, { resetUndo: true });
    editor.focus();
    editor.setSelectionRange(insertAt, insertAt);
    handleSourcePrintableKeydownInput({
      defaultPrevented: false,
      isComposing: false,
      altKey: false,
      ctrlKey: false,
      metaKey: false,
      key: "z",
      preventDefault() {},
      stopPropagation() {},
    });
    const edited = editor.value || "";
    if (edited === original) {
      throw new Error("source edit did not change source before reload");
    }
    if (documents[currentDocumentIndex]) {
      documents[currentDocumentIndex].source = edited;
    }
    loadEmbeddedDocument(currentDocumentIndex);
    const handled = handleSourceUndoShortcut({
      altKey: false,
      ctrlKey: false,
      metaKey: true,
      shiftKey: false,
      key: "z",
      preventDefault() {},
      stopPropagation() {},
    });
    if (!handled || editor.value !== original) {
      throw new Error("same-document reload cleared source undo history");
    }
    if (documents[currentDocumentIndex]) {
      documents[currentDocumentIndex].source = original;
    }
    setSourceEditorValue(original, { resetUndo: true });
    scheduleSourceHighlight(true);
    return true;
  })()`);
  await page.assertNoErrors("source undo after same-document reload");
}

async function sourceEditorReflectsCompositionBeforeCommit(page) {
  await page.evaluateTop(`(() => {
    const editor = document.querySelector("#sourceEditor");
    if (!editor) {
      throw new Error("missing #sourceEditor");
    }
    window.__sourceCompositionOriginal = editor.value || "";
    editor.value = "";
    if (documents[currentDocumentIndex]) {
      documents[currentDocumentIndex].source = "";
    }
    renderPlainSourceHighlight("");
    editor.focus();
    editor.setSelectionRange(0, 0);
    const event = new Event("beforeinput", {
      bubbles: true,
      cancelable: true,
    });
    Object.defineProperties(event, {
      data: { value: "a" },
      inputType: { value: "insertCompositionText" },
      isComposing: { value: true },
    });
    editor.dispatchEvent(event);
    return true;
  })()`);

  try {
    try {
      await page.waitForTop(
        `(() => {
        const editor = document.querySelector("#sourceEditor");
        const highlight = document.querySelector("#sourceHighlight");
        const wrap = document.querySelector("#sourceEditorWrap");
        return Boolean(
          editor?.value === ""
          && highlight?.textContent === "a"
          && documents[currentDocumentIndex]?.source === ""
          && !wrap?.classList.contains("is-native-input-active")
        );
      })()`,
        "source composition reflection before commit",
        { timeoutMs: 5_000 }
      );
    } catch (error) {
      const diagnostics = await page.evaluateTop(`(() => {
        const editor = document.querySelector("#sourceEditor");
        const highlight = document.querySelector("#sourceHighlight");
        return {
          value: editor?.value || "",
          highlight: highlight?.textContent || "",
          documentSource: documents[currentDocumentIndex]?.source || "",
          highlightMode: typeof sourceHighlightMode === "string" ? sourceHighlightMode : null,
          predicted: typeof sourcePredictedBeforeInputValue === "function"
            ? sourcePredictedBeforeInputValue({ inputType: "insertCompositionText", data: "a" })
            : null,
          isTextDocument: typeof isTextDocument === "function" ? isTextDocument(documents[currentDocumentIndex]) : null,
        };
      })()`).catch((diagnosticError) => ({ diagnosticError: diagnosticError.message }));
      throw new Error(`${error.message}\nComposition diagnostics: ${JSON.stringify(diagnostics, null, 2)}`);
    }
  } finally {
    await page.evaluateTop(`(() => {
      const original = window.__sourceCompositionOriginal;
      if (typeof original !== "string") {
        return false;
      }
      setSourceEditorValue(original, { resetUndo: true });
      if (documents[currentDocumentIndex]) {
        documents[currentDocumentIndex].source = original;
      }
      scheduleSourceHighlight(true);
      delete window.__sourceCompositionOriginal;
      return true;
    })()`).catch(() => {});
  }
  await page.assertNoErrors("source composition reflection");
}

async function sourceCompletionKeepsKeyboardSelectionAcrossRefresh(page) {
  await page.evaluateTop(`(() => {
    if (
      typeof sourceCompletionSelectedIndexForSession !== "function"
      || typeof sourceCompletionSessionMatches !== "function"
      || typeof sourceCompletionMatchesCurrentCursor !== "function"
    ) {
      throw new Error("missing source completion selection helpers");
    }
    const previous = {
      mode: "completion",
      source: "abc",
      cursor: 3,
      replaceStart: 1,
      replaceEnd: 3,
      items: [
        { label: "alpha", insertText: "alpha", kind: "keyword", detail: "" },
        { label: "beta", insertText: "beta", kind: "keyword", detail: "" },
      ],
      selectedIndex: 1,
      keyboardCommit: true,
    };
    const next = {
      mode: "completion",
      source: previous.source,
      cursor: previous.cursor,
      replaceStart: previous.replaceStart,
      replaceEnd: previous.replaceEnd,
      items: [
        { label: "alpha", insertText: "alpha", kind: "keyword", detail: "" },
        { label: "beta", insertText: "beta", kind: "keyword", detail: "" },
      ],
    };
    if (sourceCompletionSelectedIndexForSession(previous, next) !== 1) {
      throw new Error("completion selection reset during same-session refresh");
    }
    const editor = document.querySelector("#sourceEditor");
    if (!editor || !sourceCompletionPopover) {
      throw new Error("missing source editor completion UI");
    }
    const originalShow = showSourceCompletions;
    const originalState = sourceCompletionState;
    const originalValue = editor.value;
    const originalStart = editor.selectionStart;
    const originalEnd = editor.selectionEnd;
    const originalHidden = sourceCompletionPopover.hidden;
    let calls = 0;
    try {
      showSourceCompletions = () => {
        calls += 1;
        return false;
      };
      editor.value = previous.source;
      editor.focus();
      editor.setSelectionRange(previous.cursor, previous.cursor);
      sourceCompletionState = { ...previous };
      sourceCompletionPopover.hidden = false;
      editor.dispatchEvent(new KeyboardEvent("keyup", { key: "ArrowDown", bubbles: true }));
    } finally {
      showSourceCompletions = originalShow;
      sourceCompletionState = originalState;
      sourceCompletionPopover.hidden = originalHidden;
      editor.value = originalValue;
      editor.setSelectionRange(originalStart, originalEnd);
    }
    if (calls !== 0) {
      throw new Error("completion ArrowDown keyup reopened completions");
    }
    return true;
  })()`);
  await page.assertNoErrors("source completion keyboard selection");
}

async function sourceCodeMirrorEditingCommandsReachWorkflow(page) {
  const original = await page.evaluateTop(`(() => {
    const editor = document.querySelector("#sourceEditor");
    if (!editor?.sourceEditorPort || editor.sourceEditorPort.kind !== "codemirror") {
      throw new Error("missing CodeMirror source editor");
    }
    const value = editor.value || "";
    window.__sourceEditingCommandsOriginal = value;
    return value;
  })()`);
  try {
    await setSourceAndCursor(page, "puzzle command_smoke\nrules\n", "puzzle command_smoke\nrules\n".length);
    await pressKey(page, { key: "{", code: "BracketLeft", keyCode: 219 });
    assert.deepEqual(
      await sourceValueAndSelection(page),
      { value: "puzzle command_smoke\nrules\n{}", start: 28, end: 28 },
      "CodeMirror { should insert a brace pair and select its interior"
    );
    await pressKey(page, { key: "}", code: "BracketRight", keyCode: 221 });
    assert.deepEqual(
      await sourceValueAndSelection(page),
      { value: "puzzle command_smoke\nrules\n{}", start: 29, end: 29 },
      "CodeMirror } should move across an existing closing brace"
    );
    await page.evaluateTop(`document.querySelector("#sourceEditor").setSelectionRange(28, 28)`);
    await pressKey(page, { key: "Backspace", code: "Backspace", keyCode: 8 });
    assert.deepEqual(
      await sourceValueAndSelection(page),
      { value: "puzzle command_smoke\nrules\n", start: 27, end: 27 },
      "CodeMirror Backspace should remove an empty brace pair"
    );

    const emptyRuleSource = "puzzle command_smoke\nrules\n";
    await setSourceAndCursor(page, emptyRuleSource, emptyRuleSource.length);
    await pressKey(page, { key: "[", code: "BracketLeft", keyCode: 219 });
    assert.deepEqual(
      await sourceValueAndSelection(page),
      { value: `${emptyRuleSource}[  ]`, start: emptyRuleSource.length + 2, end: emptyRuleSource.length + 2 },
      "CodeMirror [ should insert an editable rule cell"
    );
    await pressKey(page, { key: "Tab", code: "Tab", keyCode: 9 });
    assert.deepEqual(
      await sourceValueAndSelection(page),
      { value: `${emptyRuleSource}[  ] `, start: emptyRuleSource.length + 5, end: emptyRuleSource.length + 5 },
      "CodeMirror Tab should exit an empty rule cell"
    );

    const bracketSource = "puzzle command_smoke\nrules\n[  |  ]";
    await setSourceAndCursor(page, bracketSource, bracketSource.indexOf("[  ") + 2);
    await pressKey(page, { key: "Tab", code: "Tab", keyCode: 9 });
    const secondSlot = bracketSource.indexOf("|  ") + 2;
    assert.deepEqual(
      await sourceValueAndSelection(page),
      { value: bracketSource, start: secondSlot, end: secondSlot },
      "CodeMirror Tab should move to the next rule-cell slot"
    );

    const ordinaryTabSource = "puzzle command_smoke\nrules\nplain";
    await setSourceAndCursor(page, ordinaryTabSource, ordinaryTabSource.length);
    await pressKey(page, { key: "Tab", code: "Tab", keyCode: 9 });
    assert.equal(
      (await sourceValueAndSelection(page)).value,
      "puzzle command_smoke\nrules\n  plain",
      "an unconsumed CodeMirror Tab should keep ordinary indentation"
    );

    const rewriteSource = "puzzle command_smoke\nrules\n[ A B ] -> ";
    await setSourceAndCursor(page, rewriteSource, rewriteSource.length);
    await pressKey(page, { key: "Tab", code: "Tab", keyCode: 9 });
    assert.equal(
      (await sourceValueAndSelection(page)).value,
      `${rewriteSource}[ A B ]`,
      "CodeMirror Tab should copy the rewrite LHS into an empty RHS"
    );
  } finally {
    await page.evaluateTop(`(() => {
      const value = window.__sourceEditingCommandsOriginal;
      setSourceEditorValue(typeof value === "string" ? value : ${JSON.stringify(original)}, { resetUndo: true });
      if (documents[currentDocumentIndex]) {
        documents[currentDocumentIndex].source = sourceEditor.value;
      }
      delete window.__sourceEditingCommandsOriginal;
      return true;
    })()`);
  }
  await page.assertNoErrors("CodeMirror source editing commands");
}

async function sourceAddControlOverlaysEmptyLineWithoutChangingEditorWidth(page) {
  await page.evaluateTop(`(() => {
    const editor = document.querySelector("#sourceEditor");
    const port = editor?.sourceEditorPort;
    if (!editor || port?.kind !== "codemirror" || typeof port.setAddLineOverlay !== "function") {
      throw new Error("missing CodeMirror source line add control");
    }
    const original = editor.value || "";
    const source = "puzzle add_overlay_smoke\\n\\nrules {\\n}\\n";
    const cursor = source.indexOf("\\n") + 1;
    try {
      setSourceEditorValue(source, { resetUndo: true });
      editor.setSelectionRange(cursor, cursor);
      port.setAddLineOverlay(source, cursor, false);
      const content = document.querySelector("#sourceEditorMount .cm-content");
      const gutters = document.querySelector("#sourceEditorMount .cm-gutters");
      if (!content || !gutters) {
        throw new Error("missing CodeMirror layout surfaces");
      }
      const before = {
        contentLeft: content.getBoundingClientRect().left,
        contentWidth: content.getBoundingClientRect().width,
        guttersWidth: gutters.getBoundingClientRect().width,
      };
      port.setAddLineOverlay(source, cursor, true);
      const marker = document.querySelector("#sourceEditorMount .cm-source-add-marker");
      const anchor = marker?.closest(".cm-source-add-anchor");
      const line = marker?.closest(".cm-line");
      if (!marker || !anchor || !line) {
        throw new Error("source add control was not rendered on its line");
      }
      const after = {
        contentLeft: content.getBoundingClientRect().left,
        contentWidth: content.getBoundingClientRect().width,
        guttersWidth: gutters.getBoundingClientRect().width,
      };
      const markerRect = marker.getBoundingClientRect();
      const iconRect = marker.querySelector("svg")?.getBoundingClientRect();
      const anchorRect = anchor.getBoundingClientRect();
      const lineRect = line.getBoundingClientRect();
      const lineHeight = parseFloat(getComputedStyle(line).lineHeight);
      const markerFontSize = parseFloat(getComputedStyle(marker).fontSize);
      if (
        before.contentLeft !== after.contentLeft
        || before.contentWidth !== after.contentWidth
        || before.guttersWidth !== after.guttersWidth
        || document.querySelector("#sourceEditorMount .cm-source-add-gutter")
        || getComputedStyle(marker).position !== "absolute"
        || anchorRect.width !== 0
        || Math.abs(anchorRect.height - lineHeight) > 1
        || Math.abs(markerRect.top - lineRect.top) > 1
        || Math.abs(markerRect.left - lineRect.left) > 1
        || Math.abs(markerRect.width - lineHeight) > 1
        || Math.abs(markerRect.height - lineHeight) > 1
        || !iconRect
        || Math.abs(iconRect.width - markerFontSize) > 1
        || Math.abs(iconRect.height - markerFontSize) > 1
      ) {
        throw new Error("source add control changed editor layout: " + JSON.stringify({
          before,
          after,
          markerLeft: markerRect.left,
          markerTop: markerRect.top,
          markerWidth: markerRect.width,
          iconWidth: iconRect?.width,
          iconHeight: iconRect?.height,
          anchorWidth: anchorRect.width,
          anchorHeight: anchorRect.height,
          lineLeft: lineRect.left,
          lineTop: lineRect.top,
          lineHeight,
          markerFontSize,
          position: getComputedStyle(marker).position,
        }));
      }
    } finally {
      port.setAddLineOverlay(source, cursor, false);
      setSourceEditorValue(original, { resetUndo: true });
      if (documents[currentDocumentIndex]) {
        documents[currentDocumentIndex].source = original;
      }
    }
    return true;
  })()`);
  await page.assertNoErrors("source add line overlay layout");
}

async function sourceMultiLineSelectionStartsAtTextColumn(page) {
  await page.evaluateTop(`(() => {
    const editor = document.querySelector("#sourceEditor");
    if (!editor?.sourceEditorPort || editor.sourceEditorPort.kind !== "codemirror") {
      throw new Error("missing CodeMirror source editor");
    }
    window.__sourceSelectionOriginal = {
      source: editor.value || "",
      start: editor.selectionStart,
      end: editor.selectionEnd,
    };
    const source = "first line\\nmiddle line\\nlast line";
    setSourceEditorValue(source, { resetUndo: true });
    editor.focus();
    editor.setSelectionRange(2, source.length - 2);
    return true;
  })()`);
  try {
    await page.waitForTop(
      `document.querySelectorAll("#sourceEditorMount .cm-selectionBackground").length >= 3`,
      "multiline source selection"
    );
    const result = await page.evaluateTop(`(() => {
      const lines = Array.from(document.querySelectorAll("#sourceEditorMount .cm-line"));
      const backgrounds = Array.from(document.querySelectorAll(
        "#sourceEditorMount .cm-selectionLayer .cm-selectionBackground"
      ));
      const middleLine = lines[1];
      const middleLineRect = middleLine?.getBoundingClientRect();
      const middleLineStyle = middleLine ? getComputedStyle(middleLine) : null;
      const middleY = middleLineRect ? middleLineRect.top + middleLineRect.height / 2 : undefined;
      const middleBackground = backgrounds.find((background) => {
        const rect = background.getBoundingClientRect();
        return middleY !== undefined && rect.top <= middleY && rect.bottom >= middleY;
      });
      if (!middleLineRect || !middleLineStyle || !middleBackground) {
        throw new Error("missing multiline selection layout surfaces");
      }
      const backgroundRect = middleBackground.getBoundingClientRect();
      return {
        backgroundLeft: backgroundRect.left,
        textLeft: middleLineRect.left + parseFloat(middleLineStyle.paddingLeft),
      };
    })()`);
    assert.ok(
      Math.abs(result.backgroundLeft - result.textLeft) <= 1,
      `multiline selection starts outside the text column: ${JSON.stringify(result)}`
    );
  } finally {
    await page.evaluateTop(`(() => {
      const editor = document.querySelector("#sourceEditor");
      const original = window.__sourceSelectionOriginal;
      setSourceEditorValue(original?.source || "", { resetUndo: true });
      editor.setSelectionRange(original?.start || 0, original?.end || 0);
      delete window.__sourceSelectionOriginal;
      return true;
    })()`);
  }
  await page.assertNoErrors("source multiline selection layout");
}

async function sourcePlainClickCollapsesSelection(page) {
  const click = await page.evaluateTop(`(async () => {
    const editor = document.querySelector("#sourceEditor");
    const view = editor?.sourceEditorPort?.view;
    if (!editor || !view) {
      throw new Error("missing CodeMirror source editor");
    }
    window.__sourcePlainClickOriginal = {
      source: editor.value || "",
      start: editor.selectionStart,
      end: editor.selectionEnd,
    };
    const source = "alpha beta\\ngamma delta\\nepsilon";
    setSourceEditorValue(source, { resetUndo: true });
    editor.focus();
    editor.setSelectionRange(1, 18);
    await new Promise((resolve) => requestAnimationFrame(resolve));
    const target = source.indexOf("epsilon") + 3;
    const rect = view.coordsAtPos(target);
    if (!rect) {
      throw new Error("missing plain click coordinates");
    }
    return {
      target,
      x: rect.left,
      y: (rect.top + rect.bottom) / 2,
    };
  })()`);
  try {
    await clickViewport(page, click);
    const selection = await page.evaluateTop(`(() => {
      const editor = document.querySelector("#sourceEditor");
      const main = editor?.sourceEditorPort?.view?.state.selection.main;
      return main ? { anchor: main.anchor, head: main.head, from: main.from, to: main.to } : null;
    })()`);
    assert.deepEqual(
      selection,
      { anchor: click.target, head: click.target, from: click.target, to: click.target },
      "plain source click should leave one cursor at the clicked position"
    );

    await page.evaluateTop(`document.querySelector("#sourceEditor").setSelectionRange(1, 18)`);
    await page.send("Input.dispatchMouseEvent", {
      type: "mousePressed",
      button: "left",
      buttons: 1,
      clickCount: 1,
      x: click.x,
      y: click.y,
    });
    await page.evaluateTop(`setSourceEditorValue("ALPHA BETA\\nGAMMA DELTA\\nEPSILON", { resetUndo: true })`);
    await page.evaluateTop(`document.querySelector("#sourceEditor").setSelectionRange(1, 18)`);
    await page.send("Input.dispatchMouseEvent", {
      type: "mouseReleased",
      button: "left",
      buttons: 0,
      clickCount: 1,
      x: click.x,
      y: click.y,
    });
    const loadedSelection = await page.evaluateTop(`(() => {
      const selection = document.querySelector("#sourceEditor")?.sourceEditorPort?.view?.state.selection;
      const main = selection?.main;
      return main ? { rangeCount: selection.ranges.length, from: main.from, to: main.to } : null;
    })()`);
    assert.deepEqual(
      loadedSelection,
      { rangeCount: 1, from: click.target, to: click.target },
      "source load finishing during a click should discard its stale selection anchor"
    );
  } finally {
    await page.evaluateTop(`(() => {
      const editor = document.querySelector("#sourceEditor");
      const original = window.__sourcePlainClickOriginal;
      setSourceEditorValue(original?.source || "", { resetUndo: true });
      editor.setSelectionRange(original?.start || 0, original?.end || 0);
      delete window.__sourcePlainClickOriginal;
      return true;
    })()`);
  }
  await page.assertNoErrors("source plain click selection");
}

async function sourceOptionDragCreatesMultipleCursors(page) {
  const drag = await page.evaluateTop(`(async () => {
    const editor = document.querySelector("#sourceEditor");
    const view = editor?.sourceEditorPort?.view;
    if (!editor || !view) {
      throw new Error("missing CodeMirror source editor");
    }
    window.__sourceOptionDragOriginal = {
      source: editor.value || "",
      start: editor.selectionStart,
      end: editor.selectionEnd,
    };
    const source = "alpha\\nbravo\\ncharl";
    setSourceEditorValue(source, { resetUndo: true });
    editor.focus();
    await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
    const start = view.coordsAtPos(1);
    const end = view.coordsAtPos(15);
    if (!start || !end) {
      throw new Error("missing CodeMirror drag coordinates");
    }
    return {
      start: { x: start.left, y: (start.top + start.bottom) / 2 },
      end: { x: end.left, y: (end.top + end.bottom) / 2 },
    };
  })()`);
  try {
    await page.send("Input.dispatchMouseEvent", {
      type: "mousePressed",
      button: "left",
      buttons: 1,
      clickCount: 1,
      modifiers: 1,
      ...drag.start,
    });
    await page.send("Input.dispatchMouseEvent", {
      type: "mouseMoved",
      button: "left",
      buttons: 1,
      modifiers: 1,
      ...drag.end,
    });
    await page.send("Input.dispatchMouseEvent", {
      type: "mouseReleased",
      button: "left",
      buttons: 0,
      clickCount: 1,
      modifiers: 1,
      ...drag.end,
    });
    const selections = await page.evaluateTop(`(() => {
      const view = document.querySelector("#sourceEditor")?.sourceEditorPort?.view;
      return view?.state.selection.ranges.map((range) => ({
        from: range.from,
        to: range.to,
        text: view.state.sliceDoc(range.from, range.to),
      })) || [];
    })()`);
    assert.deepEqual(
      selections.map((selection) => selection.text),
      ["lp", "ra", "ha"],
      "Option+drag should create one rectangular selection per line"
    );

    await page.send("Input.insertText", { text: "X" });
    assert.equal(
      await page.evaluateTop(`document.querySelector("#sourceEditor")?.value || ""`),
      "aXha\nbXvo\ncXrl",
      "typing should replace every Option+drag selection"
    );
  } finally {
    await page.evaluateTop(`(() => {
      const editor = document.querySelector("#sourceEditor");
      const original = window.__sourceOptionDragOriginal;
      setSourceEditorValue(original?.source || "", { resetUndo: true });
      editor.setSelectionRange(original?.start || 0, original?.end || 0);
      delete window.__sourceOptionDragOriginal;
      return true;
    })()`);
  }
  await page.assertNoErrors("source Option drag multiple cursors");
}

async function sourceOccurrenceSelectionShortcutsMatchVsCode(page) {
  await page.evaluateTop(`(() => {
    const editor = document.querySelector("#sourceEditor");
    if (!editor?.sourceEditorPort?.view) {
      throw new Error("missing CodeMirror source editor");
    }
    window.__sourceOccurrenceSelectionOriginal = {
      source: editor.value || "",
      start: editor.selectionStart,
      end: editor.selectionEnd,
    };
    setSourceEditorValue("foo bar foo foo", { resetUndo: true });
    editor.focus();
    editor.setSelectionRange(1, 1);
    return true;
  })()`);
  try {
    await pressKey(page, { key: "d", code: "KeyD", keyCode: 68, modifiers: 4 });
    assert.deepEqual(
      await sourceSelectionTexts(page),
      ["foo"],
      "Cmd+D should select the word at an empty cursor"
    );
    await pressKey(page, { key: "d", code: "KeyD", keyCode: 68, modifiers: 4 });
    assert.deepEqual(
      await sourceSelectionTexts(page),
      ["foo", "foo"],
      "repeated Cmd+D should add the next occurrence"
    );

    await page.evaluateTop(`document.querySelector("#sourceEditor").setSelectionRange(0, 3)`);
    await pressKey(page, { key: "l", code: "KeyL", keyCode: 76, modifiers: 12 });
    assert.deepEqual(
      await sourceSelectionTexts(page),
      ["foo", "foo", "foo"],
      "Shift+Cmd+L should select all occurrences"
    );
    await page.send("Input.insertText", { text: "X" });
    assert.equal(
      await page.evaluateTop(`document.querySelector("#sourceEditor")?.value || ""`),
      "X bar X X",
      "typing should replace every selected occurrence"
    );
  } finally {
    await page.evaluateTop(`(() => {
      const editor = document.querySelector("#sourceEditor");
      const original = window.__sourceOccurrenceSelectionOriginal;
      setSourceEditorValue(original?.source || "", { resetUndo: true });
      editor.setSelectionRange(original?.start || 0, original?.end || 0);
      delete window.__sourceOccurrenceSelectionOriginal;
      return true;
    })()`);
  }
  await page.assertNoErrors("source occurrence selection shortcuts");
}

async function sourceSelectionTexts(page) {
  return page.evaluateTop(`(() => {
    const view = document.querySelector("#sourceEditor")?.sourceEditorPort?.view;
    return view?.state.selection.ranges.map((range) => view.state.sliceDoc(range.from, range.to)) || [];
  })()`);
}

async function sourceOutlineNavigationCentersCursor(page) {
  const result = await page.evaluateTop(`(() => {
    const editor = document.querySelector("#sourceEditor");
    const originalItems = sourceOutlineItems;
    const originalScrollIntoView = editor?.sourceEditorPort?.scrollIntoView;
    const document = activeDocument();
    if (!editor || typeof originalScrollIntoView !== "function" || !document) {
      throw new Error("missing source outline navigation test surfaces");
    }
    const start = Math.min(editor.value.length, Math.max(0, editor.value.indexOf("\\n") + 1));
    const item = {
      id: "browser-smoke-outline-target",
      kind: "rules",
      label: "Browser smoke outline target",
      start,
      end: start,
      depth: 0,
      parent: "",
    };
    let scrollRequest = null;
    try {
      sourceOutlineItems = [item];
      renderSourceOutline();
      editor.sourceEditorPort.scrollIntoView = (offset, alignment) => {
        scrollRequest = { offset, alignment };
      };
      document.querySelector('[data-source-outline-id="browser-smoke-outline-target"]')?.click();
      return {
        cursor: editor.selectionStart,
        expectedCursor: start,
        scrollRequest,
      };
    } finally {
      editor.sourceEditorPort.scrollIntoView = originalScrollIntoView;
      sourceOutlineItems = originalItems;
      renderSourceOutline();
      syncSourceOutlineActiveItem();
    }
  })()`);
  assert.deepEqual(result, {
    cursor: result.expectedCursor,
    expectedCursor: result.expectedCursor,
    scrollRequest: {
      offset: result.expectedCursor,
      alignment: "center",
    },
  });
  await page.assertNoErrors("source outline centered navigation");
}

async function sourceEntryRefreshIgnoresSupersededEdits(page) {
  await page.evaluateTop(`(() => {
    const editor = document.querySelector("#sourceEditor");
    const status = document.querySelector("#editorStatusLabel");
    if (!editor || !status || typeof sourceEditorContentChanged !== "function") {
      throw new Error("missing source entry refresh test surfaces");
    }
    window.__sourceEntriesRaceOriginal = editor.value || "";
    window.__sourceEntriesRaceStatus = {
      text: status.textContent || "",
      className: status.className,
    };
    setEditorStatus("", "");
    const insertAt = Math.max(0, editor.value.indexOf("\\n"));
    editor.setSelectionRange(insertAt, insertAt);
    for (const text of ["a", "b"]) {
      const cursor = editor.selectionStart;
      editor.setRangeText(text, cursor, cursor, "end");
      sourceEditorContentChanged();
    }
    return editor.value;
  })()`);
  try {
    await page.waitForTop(
      `Boolean(surfaceEntriesCache?.source === document.querySelector("#sourceEditor")?.value)`,
      "latest source entries after superseded edits",
      { timeoutMs: 10_000 }
    );
    const status = await page.evaluateTop(`document.querySelector("#editorStatusLabel")?.textContent || ""`);
    assert.equal(
      status.includes("Editor source analysis changed before the query started."),
      false,
      `superseded source entry query leaked into editor status: ${status}`
    );
  } finally {
    await page.evaluateTop(`(() => {
      const original = window.__sourceEntriesRaceOriginal;
      setSourceEditorValue(typeof original === "string" ? original : "", { resetUndo: true });
      if (documents[currentDocumentIndex]) {
        documents[currentDocumentIndex].source = sourceEditor.value;
      }
      const status = window.__sourceEntriesRaceStatus;
      if (status) {
        const element = document.querySelector("#editorStatusLabel");
        element.textContent = status.text;
        element.className = status.className;
      }
      delete window.__sourceEntriesRaceOriginal;
      delete window.__sourceEntriesRaceStatus;
      return true;
    })()`);
  }
  await page.assertNoErrors("superseded source entry refresh");
}

async function levelSelectionWaitsForCurrentEntries(page) {
  await page.waitForTop(
    `Boolean(surfaceEntriesCache?.source === document.querySelector("#sourceEditor")?.value)`,
    "initial source entries",
    { timeoutMs: 10_000 }
  );
  await page.evaluateTop(`(() => {
    const editor = document.querySelector("#sourceEditor");
    const status = document.querySelector("#editorStatusLabel");
    const runtime = window.PuzzleStudioRuntime;
    if (!editor || !status || typeof runtime?.sourceEntryInfo !== "function") {
      throw new Error("missing source target revision test surfaces");
    }
    window.__sourceTargetRevisionOriginal = {
      source: editor.value || "",
      sourceEntryInfo: runtime.sourceEntryInfo,
      statusText: status.textContent || "",
      statusClassName: status.className,
      previewMode: currentPreviewMode,
    };
    window.__sourceTargetRevisionRelease = null;
    window.__sourceTargetRevisionBlocked = false;
    const levelEntry = surfaceEntriesCache.entries.find((entry) => entry?.kind === "level");
    if (!levelEntry) {
      throw new Error("source target revision fixture has no parsed level entry");
    }
    openPreviewModePane("play");
    runtime.sourceEntryInfo = async (...args) => {
      await new Promise((resolve) => {
        window.__sourceTargetRevisionRelease = resolve;
        window.__sourceTargetRevisionBlocked = true;
      });
      return window.__sourceTargetRevisionOriginal.sourceEntryInfo.apply(runtime, args);
    };
    setEditorStatus("", "");
    const insertAt = Math.max(0, editor.value.indexOf("\\n"));
    editor.setSelectionRange(insertAt, insertAt);
    editor.setRangeText(" ", insertAt, insertAt, "end");
    sourceEditorContentChanged();
    return true;
  })()`);
  try {
    await page.waitForTop(
      `window.__sourceTargetRevisionBlocked === true`,
      "blocked current-revision source entries",
      { timeoutMs: 10_000 }
    );
    await page.evaluateTop(`(() => {
      const button = document.querySelector("#editModeButton");
      if (!button) {
        throw new Error("source target revision fixture has no level editor button");
      }
      button.click();
      return true;
    })()`);
    await new Promise((resolve) => setTimeout(resolve, 250));
    const pending = await page.evaluateTop(`(() => ({
      status: document.querySelector("#editorStatusLabel")?.textContent || "",
      target: sourceCursorResolveRegion,
      cacheSource: surfaceEntriesCache?.source ?? null,
    }))()`);
    assert.equal(pending.cacheSource, null, `source entry cache unexpectedly became ready: ${JSON.stringify(pending)}`);
    assert.equal(
      pending.status.includes("Source entries are not ready for the active editor revision."),
      false,
      `source target consumed entries before its revision was ready: ${pending.status}`
    );
    await page.evaluateTop(`(() => {
      window.__sourceTargetRevisionRelease?.();
      return true;
    })()`);
    await page.waitForTop(
      `Boolean(surfaceEntriesCache?.source === document.querySelector("#sourceEditor")?.value)`,
      "current-revision source target entries",
      { timeoutMs: 10_000 }
    );
    await page.waitForTop(
      `currentPreviewMode === "edit"`,
      "level editor after current-revision entries",
      { timeoutMs: 10_000 }
    );
  } finally {
    await page.evaluateTop(`(() => {
      const original = window.__sourceTargetRevisionOriginal;
      const editor = document.querySelector("#sourceEditor");
      window.__sourceTargetRevisionRelease?.();
      if (original) {
        window.PuzzleStudioRuntime.sourceEntryInfo = original.sourceEntryInfo;
        setSourceEditorValue(original.source, { resetUndo: true });
        if (documents[currentDocumentIndex]) {
          documents[currentDocumentIndex].source = editor.value;
        }
        const status = document.querySelector("#editorStatusLabel");
        status.textContent = original.statusText;
        status.className = original.statusClassName;
        openPreviewModePane(original.previewMode);
      }
      delete window.__sourceTargetRevisionOriginal;
      delete window.__sourceTargetRevisionRelease;
      delete window.__sourceTargetRevisionBlocked;
      return true;
    })()`);
  }
  await page.assertNoErrors("source target revision synchronization");
}

async function sourceCompletionPopoverStaysInsideEditor(page) {
  await page.evaluateTop(`(() => {
    const editor = document.querySelector("#sourceEditor");
    if (!editor || !sourceCompletionPopover) {
      throw new Error("missing source completion positioning targets");
    }
    const original = editor.value || "";
    const source = Array.from({ length: 36 }, (_, index) => "line_" + index).join("\\n");
    try {
      setSourceEditorValue(source, { resetUndo: true });
      editor.setSelectionRange(source.length, source.length);
      editor.sourceEditorPort.scrollIntoView(source.length, "end");
      sourceCompletionState = {
        mode: "completion",
        source,
        cursor: source.length,
        replaceStart: source.length - "line_35".length,
        replaceEnd: source.length,
        selectedIndex: 0,
        items: Array.from({ length: 20 }, (_, index) => ({
          label: "candidate_" + index,
          insertText: "candidate_" + index,
          kind: "keyword",
          detail: "keyword",
        })),
      };
      renderSourceCompletionItems();
      sourceCompletionPopover.hidden = false;
      positionSourceCompletionPopover();
      const popoverRect = sourceCompletionPopover.getBoundingClientRect();
      const wrapRect = sourceEditorWrap.getBoundingClientRect();
      if (
        sourceCompletionPopover.dataset.placement !== "above"
        || popoverRect.top < wrapRect.top
        || popoverRect.bottom > wrapRect.bottom
      ) {
        throw new Error("completion popover escaped editor: " + JSON.stringify({
          placement: sourceCompletionPopover.dataset.placement,
          popover: { top: popoverRect.top, bottom: popoverRect.bottom },
          wrap: { top: wrapRect.top, bottom: wrapRect.bottom },
        }));
      }
    } finally {
      hideSourceCompletions();
      setSourceEditorValue(original, { resetUndo: true });
      if (documents[currentDocumentIndex]) {
        documents[currentDocumentIndex].source = original;
      }
    }
    return true;
  })()`);
  await page.assertNoErrors("source completion popover positioning");
}

async function setSourceAndCursor(page, source, cursor) {
  await page.evaluateTop(`(() => {
    const editor = document.querySelector("#sourceEditor");
    setSourceEditorValue(${JSON.stringify(source)}, { resetUndo: true });
    if (documents[currentDocumentIndex]) {
      documents[currentDocumentIndex].source = editor.value;
    }
    editor.focus();
    editor.setSelectionRange(${cursor}, ${cursor});
    hideSourceCompletions();
    return true;
  })()`);
}

async function sourceValueAndSelection(page) {
  return page.evaluateTop(`(() => {
    const editor = document.querySelector("#sourceEditor");
    return { value: editor.value, start: editor.selectionStart, end: editor.selectionEnd };
  })()`);
}

async function sourceRewritePatternTabCopiesLhsToEmptyRhs(page) {
  await page.evaluateTop(`(() => {
    const editor = document.querySelector("#sourceEditor");
    if (!editor || typeof handleSourceRewritePatternTab !== "function") {
      throw new Error("missing source rewrite pattern tab helper");
    }
    const original = editor.value || "";
    const originalSourceEditorContentChanged = sourceEditorContentChanged;
    sourceEditorContentChanged = () => {};
    const source = "puzzle tab_rhs\\nrules\\n[ A B C ] -> ";
    const cursor = source.length;
    setSourceEditorValue(source, { resetUndo: true });
    if (documents[currentDocumentIndex]) {
      documents[currentDocumentIndex].source = source;
    }
    editor.focus();
    editor.setSelectionRange(cursor, cursor);
    const event = {
      key: "Tab",
      altKey: false,
      ctrlKey: false,
      metaKey: false,
      shiftKey: false,
      defaultPrevented: false,
      propagationStopped: false,
      preventDefault() {
        this.defaultPrevented = true;
      },
      stopPropagation() {
        this.propagationStopped = true;
      },
    };
    const handled = handleSourceRewritePatternTab(event);
    const expected = "puzzle tab_rhs\\nrules\\n[ A B C ] -> [ A B C ]";
    const result = {
      handled,
      value: editor.value,
      selectionStart: editor.selectionStart,
      selectionEnd: editor.selectionEnd,
      defaultPrevented: event.defaultPrevented,
      propagationStopped: event.propagationStopped,
    };
    if (documents[currentDocumentIndex]) {
      documents[currentDocumentIndex].source = original;
    }
    sourceEditorContentChanged = originalSourceEditorContentChanged;
    setSourceEditorValue(original, { resetUndo: true });
    loadEmbeddedDocument(currentDocumentIndex);
    scheduleSourceHighlight(true);
    scheduleLocalSave();
    invalidateCompiledPreview?.(activePreviewDocument?.());
    if (
      !handled
      || result.value !== expected
      || result.selectionStart !== expected.length
      || result.selectionEnd !== expected.length
      || !result.defaultPrevented
      || !result.propagationStopped
    ) {
      throw new Error(\`rewrite RHS Tab did not copy LHS pattern: \${JSON.stringify(result)}\`);
    }
    return true;
  })()`);
  await page.evaluateTop(`(() => {
    const editor = document.querySelector("#sourceEditor");
    if (
      !editor
      || typeof handleSourceRewritePatternTab !== "function"
      || typeof handleSourceRewriteRhsPatternAssist !== "function"
    ) {
      throw new Error("missing source rewrite pattern helpers");
    }
    const original = editor.value || "";
    const originalSourceEditorContentChanged = sourceEditorContentChanged;
    sourceEditorContentChanged = () => {};
    const source = "puzzle semicolon_rhs\\nrules\\n[ X ] -> [ X ]; [ A B C ] -> ;";
    const cursor = source.lastIndexOf(";");
    setSourceEditorValue(source, { resetUndo: true });
    if (documents[currentDocumentIndex]) {
      documents[currentDocumentIndex].source = source;
    }
    editor.focus();
    editor.setSelectionRange(cursor, cursor);
    const bracketEvent = {
      key: "[",
      altKey: false,
      ctrlKey: false,
      metaKey: false,
      isComposing: false,
      defaultPrevented: false,
      propagationStopped: false,
      preventDefault() {
        this.defaultPrevented = true;
      },
      stopPropagation() {
        this.propagationStopped = true;
      },
    };
    const bracketHandled = handleSourceRewriteRhsPatternAssist(bracketEvent);
    const bracketExpected = "puzzle semicolon_rhs\\nrules\\n[ X ] -> [ X ]; [ A B C ] -> [  ];";
    const bracketResult = {
      handled: bracketHandled,
      value: editor.value,
      selectionStart: editor.selectionStart,
      selectionEnd: editor.selectionEnd,
      defaultPrevented: bracketEvent.defaultPrevented,
      propagationStopped: bracketEvent.propagationStopped,
    };
    setSourceEditorValue(source, { resetUndo: true });
    if (documents[currentDocumentIndex]) {
      documents[currentDocumentIndex].source = source;
    }
    editor.setSelectionRange(cursor, cursor);
    const tabEvent = {
      key: "Tab",
      altKey: false,
      ctrlKey: false,
      metaKey: false,
      shiftKey: false,
      defaultPrevented: false,
      propagationStopped: false,
      preventDefault() {
        this.defaultPrevented = true;
      },
      stopPropagation() {
        this.propagationStopped = true;
      },
    };
    const tabHandled = handleSourceRewritePatternTab(tabEvent);
    const tabExpected = "puzzle semicolon_rhs\\nrules\\n[ X ] -> [ X ]; [ A B C ] -> [ A B C ];";
    const tabResult = {
      handled: tabHandled,
      value: editor.value,
      selectionStart: editor.selectionStart,
      selectionEnd: editor.selectionEnd,
      defaultPrevented: tabEvent.defaultPrevented,
      propagationStopped: tabEvent.propagationStopped,
    };
    if (documents[currentDocumentIndex]) {
      documents[currentDocumentIndex].source = original;
    }
    sourceEditorContentChanged = originalSourceEditorContentChanged;
    setSourceEditorValue(original, { resetUndo: true });
    loadEmbeddedDocument(currentDocumentIndex);
    scheduleSourceHighlight(true);
    scheduleLocalSave();
    invalidateCompiledPreview?.(activePreviewDocument?.());
    if (
      !bracketHandled
      || bracketResult.value !== bracketExpected
      || bracketResult.selectionStart <= cursor
      || bracketResult.selectionEnd !== bracketResult.selectionStart
      || !bracketResult.defaultPrevented
      || !bracketResult.propagationStopped
    ) {
      throw new Error(\`rewrite RHS [ did not respect semicolon boundary: \${JSON.stringify(bracketResult)}\`);
    }
    if (
      !tabHandled
      || tabResult.value !== tabExpected
      || tabResult.selectionStart !== tabExpected.lastIndexOf(";")
      || tabResult.selectionEnd !== tabExpected.lastIndexOf(";")
      || !tabResult.defaultPrevented
      || !tabResult.propagationStopped
    ) {
      throw new Error(\`rewrite RHS Tab did not respect semicolon boundary: \${JSON.stringify(tabResult)}\`);
    }
    return true;
  })()`);
  await page.evaluateTop(`(() => {
    const editor = document.querySelector("#sourceEditor");
    if (
      !editor
      || typeof handleSourceRewritePatternTab !== "function"
      || typeof sourceEmptyRewritePattern !== "function"
      || typeof sourceRewritePatternSlotOffsets !== "function"
    ) {
      throw new Error("missing source rewrite pattern semicolon helpers");
    }
    const original = editor.value || "";
    const originalSourceEditorContentChanged = sourceEditorContentChanged;
    sourceEditorContentChanged = () => {};
    const emptySingleRow = sourceEmptyRewritePattern("[ ; ]");
    const emptyMixedRow = sourceEmptyRewritePattern("[ A | ; | B ]");
    const singleRowSlots = sourceRewritePatternSlotOffsets(emptySingleRow);
    const mixedRowSlots = sourceRewritePatternSlotOffsets(emptyMixedRow);
    const cases = [
      {
        name: "semicolon row",
        lhs: "[ ; ]",
        expected: "[ ; ]",
      },
      {
        name: "pipe and semicolon row",
        lhs: "[ A | ; | B ]",
        expected: "[ A | ; | B ]",
      },
    ];
    try {
      if (emptySingleRow !== "[ ; ]" || singleRowSlots.length !== 2) {
        throw new Error(\`single-row semicolon shape was not preserved: \${JSON.stringify({ emptySingleRow, singleRowSlots })}\`);
      }
      if (emptyMixedRow !== "[  | ; |  ]" || mixedRowSlots.length !== 4) {
        throw new Error(\`pipe/semicolon shape was not preserved: \${JSON.stringify({ emptyMixedRow, mixedRowSlots })}\`);
      }
      for (const testCase of cases) {
        const source = \`puzzle semicolon_cell_rhs\\nrules\\n\${testCase.lhs} -> \`;
        const cursor = source.length;
        setSourceEditorValue(source, { resetUndo: true });
        if (documents[currentDocumentIndex]) {
          documents[currentDocumentIndex].source = source;
        }
        editor.focus();
        editor.setSelectionRange(cursor, cursor);
        const event = {
          key: "Tab",
          altKey: false,
          ctrlKey: false,
          metaKey: false,
          shiftKey: false,
          defaultPrevented: false,
          propagationStopped: false,
          preventDefault() {
            this.defaultPrevented = true;
          },
          stopPropagation() {
            this.propagationStopped = true;
          },
        };
        const handled = handleSourceRewritePatternTab(event);
        const expected = \`\${source}\${testCase.expected}\`;
        const result = {
          handled,
          value: editor.value,
          selectionStart: editor.selectionStart,
          selectionEnd: editor.selectionEnd,
          defaultPrevented: event.defaultPrevented,
          propagationStopped: event.propagationStopped,
        };
        if (
          !handled
          || result.value !== expected
          || result.selectionStart !== expected.length
          || result.selectionEnd !== expected.length
          || !result.defaultPrevented
          || !result.propagationStopped
        ) {
          throw new Error(\`rewrite RHS Tab did not copy \${testCase.name}: \${JSON.stringify(result)}\`);
        }
      }
    } finally {
      if (documents[currentDocumentIndex]) {
        documents[currentDocumentIndex].source = original;
      }
      sourceEditorContentChanged = originalSourceEditorContentChanged;
      setSourceEditorValue(original, { resetUndo: true });
      loadEmbeddedDocument(currentDocumentIndex);
      scheduleSourceHighlight(true);
      scheduleLocalSave();
      invalidateCompiledPreview?.(activePreviewDocument?.());
    }
    return true;
  })()`);
  await page.assertNoErrors("source rewrite RHS Tab copy");
}

async function fileInputImportAddsPuzzleDocument(page) {
  const source = `title "File Import Smoke"

puzzle main {
  layers {
    actor = Player
  }

  rules {
  }
}

levels main of main {
  legend {
    . = empty
    P = Player
  }

  level imported
  P
}
`;
  const tempDir = await fs.promises.mkdtemp(path.join(os.tmpdir(), "puzzlestudio-import-smoke-"));
  const importPath = path.join(tempDir, "browser_import_smoke.puzzle");
  try {
    await fs.promises.writeFile(importPath, source, "utf8");
    const beforeCount = await page.evaluateTop(`documents.length`);
    await page.setFileInputFiles("#importFileInput", [importPath]);
    await page.evaluateTop(`document.querySelector("#importFileInput")?.dispatchEvent(new Event("change", { bubbles: true }))`);
    await waitForTopWithDiagnostics(
      page,
      `(() => {
        const imported = documents.find((document) => document.name === "browser_import_smoke.puzzle");
        return Boolean(
          imported
          && documents.length === ${beforeCount + 1}
          && activeDocument()?.id === imported.id
          && document.querySelector("#sourceEditor")?.value.includes("File Import Smoke")
          && !document.querySelector("#editorStatusLabel")?.textContent.includes("Import failed")
          && window.localStorage.getItem("PuzzleStudioFileTree:v4")?.includes("browser_import_smoke.puzzle")
        );
      })()`,
      "file input import adds puzzle document",
      { timeoutMs: 20_000 }
    );
    await page.assertNoErrors("file input import");
  } finally {
    await fs.promises.rm(tempDir, { recursive: true, force: true });
  }
}

async function fileInputImportAddsExternalPuzzleDocument(page, importPath) {
  const name = path.basename(importPath);
  if (!fs.existsSync(importPath)) {
    throw new Error(`import file does not exist: ${importPath}`);
  }
  const beforeCount = await page.evaluateTop(`documents.length`);
  await page.setFileInputFiles("#importFileInput", [importPath]);
  await page.evaluateTop(`document.querySelector("#importFileInput")?.dispatchEvent(new Event("change", { bubbles: true }))`);
  await waitForTopWithDiagnostics(
    page,
    `(() => {
      const imported = documents.find((document) => document.name === ${JSON.stringify(name)});
      return Boolean(imported);
    })()`,
    `file input import adds ${name}`,
    { timeoutMs: 20_000 }
  );
  const result = await page.evaluateTop(`(() => {
    const imported = documents.find((document) => document.name === ${JSON.stringify(name)});
    const status = document.querySelector("#editorStatusLabel")?.textContent || "";
    return {
      documentCount: documents.length,
      importedName: imported?.name || "",
      activeName: activeDocument()?.name || "",
      sourceHead: (document.querySelector("#sourceEditor")?.value || "").slice(0, 160),
      status,
      previewLogTail: Array.from(document.querySelectorAll("#previewLog [data-preview-log-message], #previewLog li, #previewLog .preview-log-entry"), (item) => item.textContent.trim()).filter(Boolean).slice(-5),
      localStorageHasFile: window.localStorage.getItem("PuzzleStudioFileTree:v4")?.includes(${JSON.stringify(name)}) || false,
    };
  })()`);
  if (result.documentCount !== beforeCount + 1) {
    throw new Error(`expected document count ${beforeCount + 1}, got ${result.documentCount}`);
  }
  const activeBaseName = result.activeName.replace(/-\d+(?=\.puzzle$)/, "");
  if (activeBaseName !== name) {
    throw new Error(`expected active imported file based on ${name}, got ${result.activeName}`);
  }
  if (result.status.includes("Import failed")) {
    throw new Error(`import reported failure: ${result.status}`);
  }
  if (!result.localStorageHasFile) {
    throw new Error(`${name} was not persisted in localStorage`);
  }
  await page.assertNoErrors(`file input import ${name}`);
  console.log(JSON.stringify(result, null, 2));
}

async function levelPlaytestKeyboardChangesBoardWithoutSavingSource(page) {
  const source = `title "Level Playtest Smoke"

puzzle main {
  layers {
    actor = Player
  }

  rules {
    input [ Player ] -> [ > Player ]
    move
  }
}

levels main of main {
  legend {
    . = empty
    P = Player
  }

  level start
  P.
}
`;
  await page.evaluateTop(`(() => {
    const source = ${JSON.stringify(source)};
    setSourceEditorValue(source, { resetUndo: true });
    if (documents[currentDocumentIndex]) {
      documents[currentDocumentIndex].source = source;
    }
    sourceEditor.setSelectionRange(0, 0);
    scheduleSourceHighlight(true);
    scheduleLocalSave();
    invalidateCompiledPreview?.(activePreviewDocument?.());
    return true;
  })()`);
  await clickTop(page, "#editModeButton");
  await page.waitForTop(
    `(() => {
      const builder = document.querySelector("#levelBuilder");
      return Boolean(builder && !builder.hidden && document.querySelectorAll("#levelBoard .cell").length > 0);
    })()`,
    "2D level editor board"
  );
  const before = await page.evaluateTop(`(() => ({
    labels: Array.from(document.querySelectorAll("#levelBoard .cell"), (cell) => cell.getAttribute("aria-label") || ""),
    source: document.querySelector("#sourceEditor")?.value || "",
  }))()`);

  await clickTop(page, "#levelPlaytestButton");
  await page.waitForTop(
    `Boolean(
      document.querySelector("#levelBuilder")?.classList.contains("is-playtesting")
      && document.querySelector("#levelPlaytestButton")?.classList.contains("is-playing")
    )`,
    "2D level playtest start",
    { timeoutMs: 15_000 }
  );

  await page.evaluateTop(`(() => {
    const board = document.querySelector("#levelBoard");
    if (!board) {
      throw new Error("missing #levelBoard");
    }
    window.__puzzleStudioSmokeKeys = [];
    board.addEventListener("keydown", (event) => {
      window.__puzzleStudioSmokeKeys.push({
        key: event.key,
        code: event.code,
        target: event.target?.id || event.target?.className || "",
      });
    }, { capture: true });
    board.focus();
    return true;
  })()`);
  await pressKey(page, { key: "ArrowRight", code: "ArrowRight", keyCode: 39 });
  await waitForTopWithDiagnostics(
    page,
    `Array.isArray(window.__puzzleStudioSmokeKeys) && window.__puzzleStudioSmokeKeys.length > 0`,
    "2D level playtest browser keydown delivery",
    { timeoutMs: 5_000 }
  );

  const labelsJson = JSON.stringify(before.labels);
  await waitForTopWithDiagnostics(
    page,
    `JSON.stringify(Array.from(document.querySelectorAll("#levelBoard .cell"), (cell) => cell.getAttribute("aria-label") || "")) !== ${JSON.stringify(labelsJson)}`,
    "2D level playtest board state change",
    { timeoutMs: 20_000 }
  );

  const afterSource = await page.evaluateTop(`document.querySelector("#sourceEditor")?.value || ""`);
  assert.equal(afterSource, before.source, "2D level playtest must not commit draft board state to source");
  await page.assertNoErrors("2D level playtest");
}

async function level3dPreviewUpdateReachesRuntime(page) {
  await clickTop(page, "#runButton");
  await waitForTopWithDiagnostics(
    page,
    `Boolean(
      typeof latestHtml !== "undefined"
      && latestHtml.includes("PuzzleExport")
      && typeof latestPreviewRuntimeStatus !== "undefined"
      && latestPreviewRuntimeStatus
      && latestPreviewRuntimeStatus.title === "PuzzleStudio HTML Export"
    )`,
    "compiled 3D preview runtime load",
    { timeoutMs: 20_000 }
  );

  await clickTop(page, "#editModeButton");
  await page.waitForTop(
    `Boolean(
      document.querySelector("#level3dBuilder")
      && !document.querySelector("#level3dBuilder").hidden
      && document.querySelector("#level3dRuntimeFrame")
      && document.querySelector("#level3dResetPreviewButton")
    )`,
    "3D level editor runtime frame",
    { timeoutMs: 20_000 }
  );

  await assertIndexControlLayout(page, ".level3d-layer-toolbar", "3D level slices");

  await page.waitForTop(
    `Boolean(
      typeof level3dStageRendererView !== "undefined"
      && level3dStageRendererView
      && level3dStageRendererView.coordinateSpace === "canvas-css-px"
      && level3dStageRendererView.width > 0
      && level3dStageRendererView.height > 0
      && level3dStageRendererView.viewport?.width > 0
      && level3dStageRendererView.viewport?.height > 0
    )`,
    "3D preview runtime view message",
    { timeoutMs: 20_000 }
  );

  await clickTop(page, "#level3dResetPreviewButton");
  await waitForTopWithDiagnostics(
    page,
    `Boolean(
      document.querySelector("#level3dActionStatus")?.textContent.includes("Reset 3D preview view")
    )`,
    "3D preview reset contract",
    { timeoutMs: 15_000 }
  );
  await page.assertNoErrors("3D preview contract");
}

async function clickTop(page, selector) {
  await page.waitForTop(
    `Boolean(document.querySelector(${JSON.stringify(selector)}))`,
    `click target ${selector}`
  );
  await page.evaluateTop(`(() => {
    const element = document.querySelector(${JSON.stringify(selector)});
    if (!element) {
      throw new Error("missing ${selector}");
    }
    element.click();
    return true;
  })()`);
}

async function clickViewport(page, { x, y }) {
  await page.send("Input.dispatchMouseEvent", {
    type: "mousePressed",
    button: "left",
    buttons: 1,
    clickCount: 1,
    x,
    y,
  });
  await page.send("Input.dispatchMouseEvent", {
    type: "mouseReleased",
    button: "left",
    buttons: 0,
    clickCount: 1,
    x,
    y,
  });
}

async function pressKey(page, { key, code, keyCode, modifiers = 0 }) {
  const event = {
    key,
    code,
    modifiers,
    windowsVirtualKeyCode: keyCode,
    nativeVirtualKeyCode: keyCode,
    unmodifiedText: "",
    text: "",
  };
  await page.send("Input.dispatchKeyEvent", { ...event, type: "keyDown" });
  await page.send("Input.dispatchKeyEvent", { ...event, type: "keyUp" });
}

async function waitForTopWithDiagnostics(page, expression, label, options = {}) {
  try {
    return await page.waitForTop(expression, label, options);
  } catch (error) {
    const diagnostics = await page.evaluateTop(`(() => {
      const preview = document.querySelector("#previewFrame");
      const logItems = Array.from(document.querySelectorAll("#previewLog [data-preview-log-message], #previewLog li, #previewLog .preview-log-entry"), (item) => item.textContent.trim()).filter(Boolean);
      return {
        status: document.querySelector("#statusText, #status")?.textContent.trim() || "",
        latestHtmlLength: typeof latestHtml === "string" ? latestHtml.length : null,
        latestHtmlHead: typeof latestHtml === "string" ? latestHtml.slice(0, 500) : "",
        latestHtmlTail: typeof latestHtml === "string" ? latestHtml.slice(-500) : "",
        latestHtmlContainsPreviewState: typeof latestHtml === "string" && latestHtml.includes("PuzzleStudioPreviewState"),
        latestHtmlContainsNotifyPreviewState: typeof latestHtml === "string" && latestHtml.includes("notifyPreviewState"),
        latestHtmlContainsParentPostMessage: typeof latestHtml === "string" && latestHtml.includes("window.parent.postMessage"),
        latestHtmlScripts: typeof latestHtml === "string"
          ? Array.from(latestHtml.matchAll(/<script\\b[^>]*>/gi), (match) => match[0]).slice(0, 10)
          : [],
        latestPreviewStateType: typeof latestPreviewState,
        latestPreviewState,
        latestPreviewRuntimeStatusType: typeof latestPreviewRuntimeStatus,
        latestPreviewRuntimeStatus,
        previewFramePresent: Boolean(preview),
        previewFrameSrcdocLength: preview?.srcdoc?.length || 0,
        previewFrameTitle: preview?.title || "",
        contextCount: document.querySelectorAll("iframe").length,
        previewLogTail: logItems.slice(-5),
        levelPlaytestActive: typeof levelPlaytestActive === "boolean" ? levelPlaytestActive : null,
        levelPlaytestTransitionBusy: typeof levelPlaytestTransitionBusy === "boolean" ? levelPlaytestTransitionBusy : null,
        levelPlaytestStateDataPresent: Boolean(typeof levelPlaytestStateData !== "undefined" && levelPlaytestStateData),
        levelPlaytestStateDataHead: typeof levelPlaytestStateData !== "undefined" && levelPlaytestStateData
          ? JSON.stringify(levelPlaytestStateData).slice(0, 500)
          : "",
        previewInputSummaries: Array.isArray(previewExport?.inputs)
          ? previewExport.inputs.map((input) => ({ id: input.id, name: input.name, key: input.key, arrow: input.arrow, keys: input.keys })).slice(0, 20)
          : [],
        levelBoardLabels: Array.from(document.querySelectorAll("#levelBoard .cell"), (cell) => cell.getAttribute("aria-label") || "").slice(0, 40),
        smokeKeys: Array.isArray(window.__puzzleStudioSmokeKeys) ? window.__puzzleStudioSmokeKeys : [],
        activeElement: {
          id: document.activeElement?.id || "",
          tagName: document.activeElement?.tagName || "",
          className: String(document.activeElement?.className || ""),
        },
        levelBoardIsActiveElement: document.activeElement === document.querySelector("#levelBoard"),
        level3d: {
          builderVisible: Boolean(document.querySelector("#level3dBuilder") && !document.querySelector("#level3dBuilder").hidden),
          actionStatus: document.querySelector("#level3dActionStatus")?.textContent.trim() || "",
          resetButtonPresent: Boolean(document.querySelector("#level3dResetPreviewButton")),
          runtimeFramePresent: Boolean(document.querySelector("#level3dRuntimeFrame")),
          stageRendererView: typeof level3dStageRendererView !== "undefined" ? level3dStageRendererView : null,
        },
      };
    })()`).catch((diagnosticError) => ({ diagnosticError: diagnosticError.message }));
    const contexts = await page.evaluateAllContexts(`(() => ({
      href: location.href,
      title: document.title,
      readyState: document.readyState,
      bodyText: document.body?.textContent?.slice(0, 160) || "",
      hasPuzzleCurrentState: typeof window.__PuzzleCurrentState !== "undefined",
      currentStateScreen: window.__PuzzleCurrentState?.currentScene || window.__PuzzleCurrentState?.screen || "",
      hasPreviewStateNotifier: typeof window.notifyPreviewState === "function",
      hasScreenFrame: Boolean(document.querySelector("#screenFrame")),
      hasBoard: Boolean(document.querySelector("#board")),
      hasView: Boolean(document.querySelector("#view")),
    }))()`);
    throw new Error(`${error.message}\nDiagnostics: ${JSON.stringify({ ...diagnostics, contexts }, null, 2)}`);
  }
}

async function withEditorServer(fixture, callback) {
  const server = new EditorServer(editorBin, fixture);
  await server.start();
  try {
    await callback(server);
  } finally {
    await server.stop();
  }
}

class EditorServer {
  constructor(bin, fixture) {
    this.bin = bin;
    this.fixture = fixture;
    this.child = null;
    this.output = "";
    this.url = "";
  }

  async start() {
    const port = await freePort();
    this.child = spawn(this.bin, [this.fixture, "--serve", "--port", String(port)], {
      cwd: repoRoot,
      stdio: ["ignore", "pipe", "pipe"],
    });
    this.child.stdout.on("data", (chunk) => this.recordOutput(chunk));
    this.child.stderr.on("data", (chunk) => this.recordOutput(chunk));
    this.url = await waitFor(async () => {
      if (this.child.exitCode !== null) {
        throw new Error(`html-editor exited before serving:\n${this.output}`);
      }
      const match = this.output.match(/html-editor serving (http:\/\/127\.0\.0\.1:\d+\/editor)/);
      return match?.[1] || null;
    }, `html-editor server for ${path.relative(repoRoot, this.fixture)}`, { timeoutMs: 60_000 });
    await waitForFetch(this.url, `html-editor response for ${this.url}`);
  }

  recordOutput(chunk) {
    this.output += chunk.toString("utf8");
  }

  async stop() {
    if (!this.child || this.child.exitCode !== null) {
      return;
    }
    this.child.kill("SIGTERM");
    const exited = await waitForChildExit(this.child, 2000);
    if (!exited && this.child.exitCode === null) {
      this.child.kill("SIGKILL");
      await waitForChildExit(this.child, 2000);
    }
  }
}

class Browser {
  constructor(bin, options) {
    this.bin = bin;
    this.options = options;
    this.child = null;
    this.devtoolsPort = 0;
    this.profileDir = "";
    this.output = "";
  }

  async start() {
    this.devtoolsPort = await freePort();
    this.profileDir = await fs.promises.mkdtemp(path.join(os.tmpdir(), "puzzlestudio-browser-smoke-"));
    const launchArgs = [
      `--remote-debugging-port=${this.devtoolsPort}`,
      `--user-data-dir=${this.profileDir}`,
      "--no-first-run",
      "--no-default-browser-check",
      "--disable-background-networking",
      "--disable-popup-blocking",
      "about:blank",
    ];
    if (this.options.headless) {
      launchArgs.unshift("--headless=new", "--disable-gpu");
    }
    this.child = spawn(this.bin, launchArgs, {
      cwd: repoRoot,
      stdio: ["ignore", "pipe", "pipe"],
    });
    this.child.stdout.on("data", (chunk) => {
      this.output += chunk.toString("utf8");
    });
    this.child.stderr.on("data", (chunk) => {
      this.output += chunk.toString("utf8");
    });
    await waitForFetch(`http://127.0.0.1:${this.devtoolsPort}/json/version`, "Chrome DevTools");
  }

  async newPage() {
    const target = await fetchJson(
      `http://127.0.0.1:${this.devtoolsPort}/json/new?${encodeURIComponent("about:blank")}`,
      { method: "PUT" }
    );
    if (!target.webSocketDebuggerUrl) {
      throw new Error(`Chrome target did not expose a WebSocket debugger URL: ${JSON.stringify(target)}`);
    }
    const page = new CdpPage(target.webSocketDebuggerUrl);
    await page.open();
    return page;
  }

  async close() {
    if (this.child && this.child.exitCode === null) {
      this.child.kill("SIGTERM");
      const exited = await waitForChildExit(this.child, 2000);
      if (!exited && this.child.exitCode === null) {
        this.child.kill("SIGKILL");
        await waitForChildExit(this.child, 2000);
      }
    }
    if (this.profileDir) {
      await fs.promises.rm(this.profileDir, { recursive: true, force: true });
    }
  }
}

class CdpPage {
  constructor(webSocketUrl) {
    this.webSocketUrl = webSocketUrl;
    this.ws = null;
    this.nextId = 1;
    this.pending = new Map();
    this.waiters = [];
    this.contexts = new Map();
    this.pageErrors = [];
  }

  async open() {
    this.ws = new WebSocket(this.webSocketUrl);
    await new Promise((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error("timed out opening DevTools WebSocket")), 10_000);
      this.ws.addEventListener("open", () => {
        clearTimeout(timer);
        resolve();
      }, { once: true });
      this.ws.addEventListener("error", (event) => {
        clearTimeout(timer);
        reject(new Error(`DevTools WebSocket failed: ${event.message || "unknown error"}`));
      }, { once: true });
    });
    this.ws.addEventListener("message", (event) => this.handleMessage(event.data));
    await this.send("Page.enable");
    await this.send("DOM.enable");
    await this.send("Runtime.enable");
    await this.send("Log.enable");
  }

  async close() {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.close();
    }
  }

  async navigate(url) {
    this.pageErrors = [];
    this.contexts.clear();
    await this.send("Page.navigate", { url });
    await this.waitForEvent("Page.loadEventFired", () => true, { timeoutMs: 20_000 });
    await this.waitForTop(`document.readyState === "complete"`, "document ready", { timeoutMs: 20_000 });
  }

  async evaluateTop(expression, options = {}) {
    return this.evaluate(expression, options);
  }

  async evaluate(expression, options = {}) {
    const params = {
      expression,
      awaitPromise: true,
      returnByValue: true,
      userGesture: true,
    };
    if (options.contextId) {
      params.contextId = options.contextId;
    }
    const result = await this.send("Runtime.evaluate", params);
    if (result.exceptionDetails) {
      throw new Error(`browser evaluation failed: ${exceptionText(result.exceptionDetails)}\n${expression}`);
    }
    return result.result?.value;
  }

  async waitForTop(expression, label, options = {}) {
    return waitFor(async () => {
      try {
        return await this.evaluateTop(expression) ? true : null;
      } catch (_error) {
        return null;
      }
    }, label, options);
  }

  async waitForContext(contextId, expression, label, options = {}) {
    return waitFor(async () => {
      try {
        return await this.evaluate(expression, { contextId }) ? true : null;
      } catch (_error) {
        return null;
      }
    }, label, options);
  }

  async waitForAnyContext(expression, label, options = {}) {
    return waitFor(async () => {
      const contexts = Array.from(this.contexts.values()).filter((context) => context.isDefault);
      for (const context of contexts) {
        try {
          const value = await this.evaluate(expression, { contextId: context.id });
          if (value) {
            return { context, value };
          }
        } catch (_error) {
          // Frames can disappear during iframe swaps.
        }
      }
      return null;
    }, label, options);
  }

  async evaluateAllContexts(expression) {
    const values = [];
    const contexts = Array.from(this.contexts.values()).filter((context) => context.isDefault);
    for (const context of contexts) {
      try {
        values.push({
          context,
          value: await this.evaluate(expression, { contextId: context.id }),
        });
      } catch (error) {
        values.push({
          context,
          error: error.message,
        });
      }
    }
    return values;
  }

  async setFileInputFiles(selector, files) {
    const root = await this.send("DOM.getDocument", { depth: 1, pierce: true });
    const node = await this.send("DOM.querySelector", {
      nodeId: root.root.nodeId,
      selector,
    });
    if (!node.nodeId) {
      throw new Error(`missing file input: ${selector}`);
    }
    await this.send("DOM.setFileInputFiles", {
      nodeId: node.nodeId,
      files,
    });
  }

  async assertNoErrors(label) {
    if (!this.pageErrors.length) {
      return;
    }
    const messages = this.pageErrors.splice(0);
    failures.push(`${label}: ${messages.join(" | ")}`);
  }

  send(method, params = {}) {
    const id = this.nextId++;
    const payload = JSON.stringify({ id, method, params });
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      this.ws.send(payload);
      setTimeout(() => {
        if (!this.pending.has(id)) {
          return;
        }
        this.pending.delete(id);
        reject(new Error(`DevTools command timed out: ${method}`));
      }, 20_000);
    });
  }

  waitForEvent(method, predicate, options = {}) {
    return new Promise((resolve, reject) => {
      const timeoutMs = options.timeoutMs || 10_000;
      const timer = setTimeout(() => {
        this.waiters = this.waiters.filter((waiter) => waiter !== waiterRecord);
        reject(new Error(`timed out waiting for ${method}`));
      }, timeoutMs);
      const waiterRecord = {
        method,
        predicate,
        resolve: (params) => {
          clearTimeout(timer);
          resolve(params);
        },
      };
      this.waiters.push(waiterRecord);
    });
  }

  handleMessage(raw) {
    const message = JSON.parse(raw);
    if (message.id) {
      const pending = this.pending.get(message.id);
      if (!pending) {
        return;
      }
      this.pending.delete(message.id);
      if (message.error) {
        pending.reject(new Error(`DevTools error: ${message.error.message}`));
      } else {
        pending.resolve(message.result || {});
      }
      return;
    }

    this.handleEvent(message.method, message.params || {});
    for (const waiter of [...this.waiters]) {
      if (waiter.method === message.method && waiter.predicate(message.params || {})) {
        this.waiters = this.waiters.filter((candidate) => candidate !== waiter);
        waiter.resolve(message.params || {});
      }
    }
  }

  handleEvent(method, params) {
    if (method === "Runtime.executionContextsCleared") {
      this.contexts.clear();
      return;
    }
    if (method === "Runtime.executionContextDestroyed") {
      this.contexts.delete(params.executionContextId);
      return;
    }
    if (method === "Runtime.executionContextCreated") {
      const context = params.context;
      this.contexts.set(context.id, {
        id: context.id,
        frameId: context.auxData?.frameId || "",
        isDefault: context.auxData?.isDefault !== false,
        name: context.name || "",
      });
      return;
    }
    if (method === "Runtime.exceptionThrown") {
      this.pageErrors.push(exceptionText(params.exceptionDetails));
      return;
    }
    if (method === "Runtime.consoleAPICalled" && params.type === "error") {
      const text = (params.args || [])
        .map((arg) => arg.value ?? arg.description ?? arg.unserializableValue ?? "")
        .join(" ");
      this.pageErrors.push(`console.error: ${text}`);
      return;
    }
    if (method === "Log.entryAdded" && ["error", "violation"].includes(params.entry?.level)) {
      this.pageErrors.push(`${params.entry.level}: ${params.entry.text}`);
    }
  }
}

async function waitFor(callback, label, options = {}) {
  const timeoutMs = options.timeoutMs || 10_000;
  const intervalMs = options.intervalMs || 100;
  const deadline = Date.now() + timeoutMs;
  let lastError = null;
  while (Date.now() <= deadline) {
    try {
      const value = await callback();
      if (value) {
        return value;
      }
    } catch (error) {
      lastError = error;
    }
    await sleep(intervalMs);
  }
  const detail = lastError ? `: ${lastError.message}` : "";
  throw new Error(`timed out waiting for ${label}${detail}`);
}

async function waitForFetch(url, label) {
  await waitFor(async () => {
    const response = await fetchWithTimeout(url, {}, 1500).catch(() => null);
    return response?.ok ? true : null;
  }, label, { timeoutMs: 20_000, intervalMs: 200 });
}

async function fetchJson(url, options = {}) {
  const response = await fetchWithTimeout(url, options, 5000);
  if (!response.ok) {
    throw new Error(`${url} returned ${response.status}`);
  }
  return response.json();
}

async function fetchWithTimeout(url, options, timeoutMs) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    return await fetch(url, { ...options, signal: controller.signal });
  } finally {
    clearTimeout(timer);
  }
}

async function freePort() {
  const server = net.createServer();
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
  const address = server.address();
  const port = address.port;
  await new Promise((resolve) => server.close(resolve));
  return port;
}

function waitForChildExit(child, timeoutMs) {
  if (child.exitCode !== null) {
    return Promise.resolve(true);
  }
  return new Promise((resolve) => {
    const timer = setTimeout(() => {
      child.off("exit", onExit);
      resolve(false);
    }, timeoutMs);
    const onExit = () => {
      clearTimeout(timer);
      resolve(true);
    };
    child.once("exit", onExit);
  });
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function exceptionText(details) {
  if (!details) {
    return "unknown exception";
  }
  return details.exception?.description
    || details.exception?.value
    || details.text
    || JSON.stringify(details);
}

function requiredPath(value, label) {
  if (!value) {
    throw new Error(`${label} is required`);
  }
  const resolved = path.resolve(repoRoot, value);
  if (!fs.existsSync(resolved)) {
    throw new Error(`${label} does not exist: ${resolved}`);
  }
  return resolved;
}

function resolveChrome(explicit) {
  if (explicit) {
    return requiredPath(explicit, "--chrome");
  }
  if (process.env.PUZZLESTUDIO_CHROME) {
    return requiredPath(process.env.PUZZLESTUDIO_CHROME, "PUZZLESTUDIO_CHROME");
  }
  for (const candidate of [
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    "/Applications/Chromium.app/Contents/MacOS/Chromium",
  ]) {
    if (fs.existsSync(candidate)) {
      return candidate;
    }
  }
  for (const command of ["google-chrome", "chromium", "chromium-browser", "chrome"]) {
    const found = findCommandOnPath(command);
    if (found) {
      return found;
    }
  }
  throw new Error("Chrome or Chromium is required for editor browser smoke tests. Set PUZZLESTUDIO_CHROME or pass --chrome.");
}

function findCommandOnPath(command) {
  for (const entry of (process.env.PATH || "").split(path.delimiter)) {
    const candidate = path.join(entry, command);
    if (fs.existsSync(candidate)) {
      return candidate;
    }
  }
  return "";
}

function parseArgs(argv) {
  const parsed = {};
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--headed") {
      parsed.headed = true;
      continue;
    }
    if (arg === "--source-input-only") {
      parsed.sourceInputOnly = true;
      continue;
    }
    if (arg === "--source-editing-commands-only") {
      parsed.sourceEditingCommandsOnly = true;
      continue;
    }
    if (arg === "--source-selection-only") {
      parsed.sourceSelectionOnly = true;
      continue;
    }
    if (arg === "--source-option-drag-only") {
      parsed.sourceOptionDragOnly = true;
      continue;
    }
    if (arg === "--source-occurrence-selection-only") {
      parsed.sourceOccurrenceSelectionOnly = true;
      continue;
    }
    if (arg === "--level-selection-revision-only") {
      parsed.levelSelectionRevisionOnly = true;
      continue;
    }
    if (arg === "--index-control-layout-only") {
      parsed.indexControlLayoutOnly = true;
      continue;
    }
    if (arg === "--initial-preview-only") {
      parsed.initialPreviewOnly = true;
      continue;
    }
    if (arg === "--visual-palette-only") {
      parsed.visualPaletteOnly = true;
      continue;
    }
    if (arg === "--visual-clip-fill-only") {
      parsed.visualClipFillOnly = true;
      continue;
    }
    if (arg === "--editor-bin") {
      parsed.editorBin = argv[++index];
      continue;
    }
    if (arg === "--chrome") {
      parsed.chrome = argv[++index];
      continue;
    }
    if (arg === "--fixture") {
      parsed.fixture = argv[++index];
      continue;
    }
    if (arg === "--fixture3d") {
      parsed.fixture3d = argv[++index];
      continue;
    }
    if (arg === "--import-file-only") {
      parsed.importFileOnly = argv[++index];
      continue;
    }
    throw new Error(`unknown argument: ${arg}`);
  }
  return parsed;
}

main().catch((error) => {
  console.error(error.stack || error.message || String(error));
  process.exit(1);
});
