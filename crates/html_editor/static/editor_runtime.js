(() => {
  const wasmCompilerAssetVersion = Date.now().toString(36);
  let wasmCompiler = null;
  let wasmCompilerPromise = null;
  let gameRuntimeAssetsPromise = null;
  let activeSourceAnalysis = null;
  let analysisWorker = null;
  let analysisWorkerFailure = null;
  let nextAnalysisWorkerRequestId = 1;
  const analysisWorkerRequests = new Map();
  let analysisWorkerSource = null;
  let analysisWorkerSourceProfile = null;
  let analysisWorkerMutation = Promise.resolve();

  function runtimeUnavailable(message) {
    const error = new Error(message);
    error.status = 500;
    return error;
  }

  function wasmModuleUrl(path, version = wasmCompilerAssetVersion) {
    return `${path}?v=${encodeURIComponent(version)}`;
  }

  function rejectAnalysisWorkerRequests(error) {
    for (const request of analysisWorkerRequests.values()) {
      request.reject(error);
    }
    analysisWorkerRequests.clear();
  }

  function requireAnalysisWorker() {
    if (analysisWorkerFailure) {
      throw analysisWorkerFailure;
    }
    if (analysisWorker) {
      return analysisWorker;
    }
    if (typeof Worker !== "function") {
      throw runtimeUnavailable("Editor source analysis requires Web Worker support.");
    }
    const worker = new Worker(wasmModuleUrl("./editor_analysis_worker.js"), { type: "module" });
    worker.addEventListener("message", (event) => {
      const response = event.data || {};
      const request = analysisWorkerRequests.get(Number(response.id));
      if (!request) {
        return;
      }
      analysisWorkerRequests.delete(Number(response.id));
      if (typeof response.error === "string" && response.error) {
        request.reject(runtimeUnavailable(response.error));
      } else {
        request.resolve(response.value);
      }
    });
    worker.addEventListener("error", (event) => {
      const error = runtimeUnavailable(event.message || "Editor source analysis worker failed.");
      analysisWorkerFailure = error;
      analysisWorker = null;
      worker.terminate();
      rejectAnalysisWorkerRequests(error);
    });
    analysisWorker = worker;
    return worker;
  }

  function postAnalysisWorker(method, payload = {}) {
    let worker;
    try {
      worker = requireAnalysisWorker();
    } catch (error) {
      return Promise.reject(error);
    }
    const id = nextAnalysisWorkerRequestId++;
    return new Promise((resolve, reject) => {
      analysisWorkerRequests.set(id, { resolve, reject });
      worker.postMessage({ id, method, ...payload });
    });
  }

  function resetAnalysisWorkerSource(source, sourceProfile) {
    const text = asString(source);
    const profile = asString(sourceProfile);
    if (profile !== "puzzle2d" && profile !== "puzzle3d") {
      throw runtimeUnavailable("Editor source analysis requires a puzzle2d or puzzle3d source profile.");
    }
    analysisWorkerSource = text;
    analysisWorkerSourceProfile = profile;
    analysisWorkerMutation = analysisWorkerMutation.then(() => postAnalysisWorker("reset", {
      source: text,
      sourceProfile: profile,
    }));
    return analysisWorkerMutation;
  }

  function applyAnalysisWorkerEdits(changes, source) {
    if (analysisWorkerSource === null) {
      throw runtimeUnavailable("Editor source analysis document is not initialized.");
    }
    const normalized = (Array.isArray(changes) ? changes : []).map((change) => ({
      from: Number(change?.from),
      to: Number(change?.to),
      insert: asString(change?.insert),
    })).sort((left, right) => right.from - left.from);
    let next = analysisWorkerSource;
    for (const change of normalized) {
      if (
        !Number.isInteger(change.from)
        || !Number.isInteger(change.to)
        || change.from < 0
        || change.from > change.to
        || change.to > next.length
      ) {
        throw runtimeUnavailable("Editor source analysis edit has an invalid UTF-16 range.");
      }
      next = `${next.slice(0, change.from)}${change.insert}${next.slice(change.to)}`;
    }
    const expected = asString(source);
    if (next !== expected) {
      throw runtimeUnavailable("Editor source analysis edits do not match the active CodeMirror document.");
    }
    analysisWorkerSource = next;
    analysisWorkerMutation = analysisWorkerMutation.then(() => postAnalysisWorker("edit", {
      changes: normalized,
      sourceLength: next.length,
    }));
    return analysisWorkerMutation;
  }

  async function querySynchronizedAnalysisWorker(method, source, payload = {}) {
    const expected = asString(source);
    const expectedProfile = asString(payload.sourceProfile);
    if (analysisWorkerSource !== expected) {
      throw runtimeUnavailable("Editor source analysis is not synchronized with CodeMirror.");
    }
    if (expectedProfile && analysisWorkerSourceProfile !== expectedProfile) {
      throw runtimeUnavailable("Editor source analysis profile is not synchronized with the active document.");
    }
    await analysisWorkerMutation;
    if (analysisWorkerSource !== expected) {
      throw runtimeUnavailable("Editor source analysis changed before the query started.");
    }
    return postAnalysisWorker(method, {
      ...payload,
      sourceLength: expected.length,
    });
  }

  function bytesToBase64(bytes) {
    let binary = "";
    const chunkSize = 0x8000;
    for (let offset = 0; offset < bytes.length; offset += chunkSize) {
      const chunk = bytes.subarray(offset, offset + chunkSize);
      binary += String.fromCharCode(...chunk);
    }
    return btoa(binary);
  }

  async function fetchRequiredText(url, label) {
    const response = await fetch(url);
    if (!response.ok) {
      throw runtimeUnavailable(`${label} is unavailable: ${response.status} ${response.statusText}`);
    }
    return response.text();
  }

  async function fetchRequiredBytes(url, label) {
    const response = await fetch(url);
    if (!response.ok) {
      throw runtimeUnavailable(`${label} is unavailable: ${response.status} ${response.statusText}`);
    }
    return new Uint8Array(await response.arrayBuffer());
  }

  async function loadWasmCompiler() {
    if (!wasmCompilerPromise) {
      wasmCompilerPromise = import(wasmModuleUrl("./wasm/puzzle_wasm.js"))
        .then(async (module) => {
          if (typeof module.default !== "function") {
            throw runtimeUnavailable("Editor WASM loader is missing its default initializer.");
          }
          await module.default({ module_or_path: wasmModuleUrl("./wasm/puzzle_wasm_bg.wasm") });
          wasmCompiler = module;
          return module;
        })
        .catch((error) => {
          wasmCompiler = null;
          wasmCompilerPromise = null;
          throw error;
        });
    }
    return wasmCompilerPromise;
  }

  async function requireWasmFunction(name) {
    const module = await loadWasmCompiler();
    const fn = module?.[name];
    if (typeof fn !== "function") {
      throw runtimeUnavailable(`Editor WASM function is missing: ${name}`);
    }
    return fn;
  }

  function asString(value) {
    return typeof value === "string" ? value : "";
  }

  function requireSourceAnalysisFunction(module, name) {
    const fn = module?.[name];
    if (typeof fn !== "function") {
      throw runtimeUnavailable(`Editor WASM function is missing: ${name}`);
    }
    return fn;
  }

  function parseSourceAnalysisJson(raw) {
    const payload = JSON.parse(raw || "{}");
    return payload && typeof payload === "object" ? payload : {};
  }

  function querySourceAnalysis(module, revision, name, ...args) {
    const query = requireSourceAnalysisFunction(module, name);
    return query(revision, ...args);
  }

  function activateSourceAnalysis(module, source) {
    const text = asString(source);
    if (activeSourceAnalysis?.source === text) {
      return activeSourceAnalysis;
    }
    const activate = requireSourceAnalysisFunction(module, "activate_source_analysis");
    const revision = activate(text);
    if (!Number.isInteger(revision) || revision <= 0) {
      throw runtimeUnavailable(`Editor WASM source analysis revision is invalid: ${revision}`);
    }
    activeSourceAnalysis = {
      source: text,
      revision,
      payload: null,
    };
    return activeSourceAnalysis;
  }

  function sourceAnalysisForLoadedSource(source) {
    if (!wasmCompiler) {
      throw runtimeUnavailable("Editor WASM parser is not loaded.");
    }
    return activateSourceAnalysis(wasmCompiler, source);
  }

  window.PuzzleStudioRuntime = {
    async compilePreview(payload = {}) {
      const compile = await requireWasmFunction("compile_workspace_preview");
      return compile(
        asString(payload.puzzlePath) || "game.puzzle",
        JSON.stringify(payload.workspaceDocuments || []),
        asString(payload.gameCss),
        asString(payload.gameVisualsJs),
      );
    },

    async exportHtml(payload = {}) {
      const exportHtml = await requireWasmFunction("export_workspace_html");
      const runtimeAssets = await window.PuzzleStudioRuntime.gameRuntimeAssets();
      return exportHtml(
        asString(payload.puzzlePath) || "game.puzzle",
        JSON.stringify(payload.workspaceDocuments || []),
        asString(payload.gameCss),
        asString(payload.gameVisualsJs),
        asString(runtimeAssets.moduleSource),
        asString(runtimeAssets.wasmBase64),
      );
    },

    async highlightSource(payload = {}) {
      const source = asString(payload.source);
      const rangeStart = Number(payload.rangeStart);
      const rangeEnd = Number(payload.rangeEnd);
      if (
        !Number.isInteger(rangeStart)
        || !Number.isInteger(rangeEnd)
        || rangeStart < 0
        || rangeStart > rangeEnd
        || rangeEnd > source.length
      ) {
        throw runtimeUnavailable("Editor source highlighting requires a valid UTF-16 viewport range.");
      }
      return querySynchronizedAnalysisWorker("highlightRange", source, {
        rangeStart,
        rangeEnd,
        includeOutline: Boolean(payload.includeOutline),
        sourceProfile: asString(payload.sourceProfile),
      });
    },

    async sourceOutline(payload = {}) {
      const source = asString(payload.source);
      return querySynchronizedAnalysisWorker("outline", source, {
        sourceProfile: asString(payload.sourceProfile),
      });
    },

    async translatePuzzleScript(source) {
      const translate = await requireWasmFunction("translate_puzzlescript");
      return translate(asString(source));
    },

    async suggestSourceCompletions(source, cursorOffset) {
      return querySynchronizedAnalysisWorker("completion", source, {
        cursorOffset: Number(cursorOffset) || 0,
      });
    },

    async resolveSourceTarget(source, cursorOffset) {
      return querySynchronizedAnalysisWorker("target", source, {
        cursorOffset: Number(cursorOffset) || 0,
      });
    },

    async mutateSpriteSource(source, sprite) {
      const raw = await querySynchronizedAnalysisWorker("mutateSprite", asString(source), { sprite });
      return JSON.parse(raw || "null");
    },

    sourceImportReference(source, documentPath, cursorOffset) {
      const analysis = sourceAnalysisForLoadedSource(source);
      const raw = querySourceAnalysis(
        wasmCompiler,
        analysis.revision,
        "active_source_analysis_import_at_json",
        asString(documentPath),
        Number(cursorOffset) || 0,
      );
      return parseSourceAnalysisJson(raw)?.reference || null;
    },

    async sourceEntries(source) {
      const raw = await querySynchronizedAnalysisWorker("entries", asString(source));
      const payload = parseSourceAnalysisJson(raw);
      return Array.isArray(payload.entries) ? payload.entries : [];
    },

    workspaceSourceEntries(source) {
      const analysis = sourceAnalysisForLoadedSource(source);
      const raw = querySourceAnalysis(wasmCompiler, analysis.revision, "active_source_analysis_entries_json");
      const payload = parseSourceAnalysisJson(raw);
      return Array.isArray(payload.entries) ? payload.entries : [];
    },

    levelEditorSourceSession(source) {
      const analysis = sourceAnalysisForLoadedSource(source);
      return {
        revision: analysis.revision,
        manifest() {
          const raw = querySourceAnalysis(
            wasmCompiler,
            analysis.revision,
            "active_source_analysis_level_editor_manifest_json",
          );
          return parseSourceAnalysisJson(raw);
        },
        levelSlots(levelIndex, authoredLayer = -1) {
          const slots = querySourceAnalysis(
            wasmCompiler,
            analysis.revision,
            "active_source_analysis_level_editor_level_slots",
            Number(levelIndex),
            Number(authoredLayer),
          );
          if (!(slots instanceof Uint32Array)) {
            throw runtimeUnavailable("Editor WASM returned level slots in an invalid format.");
          }
          return slots;
        },
        sprite(objectId) {
          const raw = querySourceAnalysis(
            wasmCompiler,
            analysis.revision,
            "active_source_analysis_level_editor_sprite_json",
            Number(objectId),
          );
          return JSON.parse(raw || "null");
        },
      };
    },

    sourceAnalysisPayload(source) {
      const analysis = sourceAnalysisForLoadedSource(source);
      if (!analysis.payload) {
        const raw = querySourceAnalysis(wasmCompiler, analysis.revision, "active_source_analysis_json");
        analysis.payload = parseSourceAnalysisJson(raw);
      }
      return analysis.payload;
    },

    resetSourceAnalysis(source, sourceProfile) {
      return resetAnalysisWorkerSource(source, sourceProfile);
    },

    applySourceAnalysisEdits(changes, source) {
      try {
        return applyAnalysisWorkerEdits(changes, source);
      } catch (error) {
        return Promise.reject(error);
      }
    },

    async solveState(source, puzzlePath, stateJson, maxDepth, maxNodes, maxMs) {
      const solve = await requireWasmFunction("solve_state");
      return solve(
        asString(source),
        asString(puzzlePath) || "game.puzzle",
        asString(stateJson),
        Number(maxDepth) || 0,
        Number(maxNodes) || 0,
        Number(maxMs) || 0,
      );
    },

    async solverTaskInitialDisplayState(requestJson) {
      const materialize = await requireWasmFunction("solver_task_initial_display_state_json");
      return materialize(asString(requestJson));
    },

    wasmCompilerConfig() {
      return {
        moduleUrl: new URL(wasmModuleUrl("./wasm/puzzle_wasm.js"), document.baseURI).href,
        wasmUrl: new URL(wasmModuleUrl("./wasm/puzzle_wasm_bg.wasm"), document.baseURI).href,
      };
    },

    cachedWasmCompiler() {
      return wasmCompiler;
    },

    async gameRuntimeAssets() {
      if (!gameRuntimeAssetsPromise) {
        gameRuntimeAssetsPromise = Promise.all([
          fetchRequiredText(wasmModuleUrl("./wasm_game/puzzle_wasm_game.js"), "Puzzle game runtime module"),
          fetchRequiredBytes(wasmModuleUrl("./wasm_game/puzzle_wasm_game_bg.wasm"), "Puzzle game runtime WASM"),
        ])
          .then(([moduleSource, wasmBytes]) => ({
            moduleSource,
            wasmBase64: bytesToBase64(wasmBytes),
          }))
          .catch((error) => {
            gameRuntimeAssetsPromise = null;
            throw error;
          });
      }
      return gameRuntimeAssetsPromise;
    },

    loadWasmCompiler,
  };
})();
