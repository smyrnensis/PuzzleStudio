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
const fixture2d = path.resolve(repoRoot, args.fixture || "games/spec_2d.puzzle");
const fixture3d = path.resolve(repoRoot, args.fixture3d || "games/spec_3d.puzzle3");
const chromePath = resolveChrome(args.chrome);
const headless = !args.headed;
const sourceInputOnly = Boolean(args.sourceInputOnly);

const failures = [];

async function main() {
  const browser = new Browser(chromePath, { headless });
  await browser.start();
  const page = await browser.newPage();
  try {
    await withEditorServer(fixture2d, async (server) => {
      await page.navigate(server.url);
      await editorLoads(page);
      await sourceEditorReflectsInputBeforeKeyup(page);
      await sourceEditorReflectsCompositionBeforeCommit(page);
      if (sourceInputOnly) {
        return;
      }
      await runPreviewStartsRuntime(page);
      await levelPlaytestKeyboardChangesBoardWithoutSavingSource(page);
    });

    if (!sourceInputOnly) {
      await withEditorServer(fixture3d, async (server) => {
        await page.navigate(server.url);
        await editorLoads(page);
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
    await page.send("Input.dispatchKeyEvent", {
      type: "keyDown",
      key: "a",
      code: "KeyA",
      windowsVirtualKeyCode: 65,
      nativeVirtualKeyCode: 65,
      unmodifiedText: "a",
      text: "a",
    });
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
    await page.send("Input.dispatchKeyEvent", {
      type: "keyUp",
      key: "a",
      code: "KeyA",
      windowsVirtualKeyCode: 65,
      nativeVirtualKeyCode: 65,
      unmodifiedText: "",
      text: "",
    }).catch(() => {});
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
          return Boolean(
            editor?.value === ""
            && highlight?.textContent === "a"
            && documents[currentDocumentIndex]?.source === ""
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

async function levelPlaytestKeyboardChangesBoardWithoutSavingSource(page) {
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

async function pressKey(page, { key, code, keyCode }) {
  const event = {
    key,
    code,
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
    }, `html-editor server for ${path.relative(repoRoot, this.fixture)}`, { timeoutMs: 20_000 });
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
    throw new Error(`unknown argument: ${arg}`);
  }
  return parsed;
}

main().catch((error) => {
  console.error(error.stack || error.message || String(error));
  process.exit(1);
});
