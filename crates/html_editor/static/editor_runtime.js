(() => {
  const wasmCompilerAssetVersion = Date.now().toString(36);
  let wasmCompiler = null;
  let wasmCompilerPromise = null;
  let gameRuntimeAssetsPromise = null;

  function runtimeUnavailable(message) {
    const error = new Error(message);
    error.status = 500;
    return error;
  }

  function wasmModuleUrl(path, version = wasmCompilerAssetVersion) {
    return `${path}?v=${encodeURIComponent(version)}`;
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

  window.PuzzleStudioRuntime = {
    async compilePreview(payload = {}) {
      const compile = await requireWasmFunction("compile_preview");
      return compile(
        asString(payload.source),
        asString(payload.puzzlePath) || "game.puzzle",
        asString(payload.gameCss),
        asString(payload.gameVisualsJs),
      );
    },

    async exportHtml(payload = {}) {
      const exportHtml = await requireWasmFunction("export_html");
      const runtimeAssets = await window.PuzzleStudioRuntime.gameRuntimeAssets();
      return exportHtml(
        asString(payload.source),
        asString(payload.puzzlePath) || "game.puzzle",
        asString(payload.gameCss),
        asString(payload.gameVisualsJs),
        asString(runtimeAssets.moduleSource),
        asString(runtimeAssets.wasmBase64),
      );
    },

    async highlightSource(payload = {}) {
      const highlight = await requireWasmFunction("highlight_source_json");
      return highlight(asString(payload.source), Boolean(payload.includeOutline));
    },

    async sourceOutline(payload = {}) {
      const outline = await requireWasmFunction("source_outline_json");
      return outline(asString(payload.source));
    },

    async translatePuzzleScript(source) {
      const translate = await requireWasmFunction("translate_puzzlescript");
      return translate(asString(source));
    },

    async suggestSourceCompletions(source, cursorOffset) {
      const suggest = await requireWasmFunction("suggest_source_completions");
      return suggest(asString(source), Number(cursorOffset) || 0);
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
