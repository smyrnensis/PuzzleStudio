(() => {
  const wasmCompilerAssetVersion = Date.now().toString(36);
  let wasmCompiler = null;
  let wasmCompilerPromise = null;
  let gameRuntimeAssetsPromise = null;
  let playerRuntimeAssetsPromise = null;
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
  let editorAudioPromise = null;
=======
  let wasmPlayerModulePromise = null;
  let workspaceSession = null;
  let workspaceSessionKey = "";
  let activeSourceAnalysis = null;
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
  let analysisWorker = null;
  let analysisWorkerFailure = null;
  let nextAnalysisWorkerRequestId = 1;
  const analysisWorkerRequests = new Map();
  let analysisWorkerSource = null;
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

  function resetAnalysisWorkerSource(source) {
    const text = asString(source);
    analysisWorkerSource = text;
    analysisWorkerMutation = analysisWorkerMutation.then(() => postAnalysisWorker("reset", {
      source: text,
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
    if (analysisWorkerSource !== expected) {
      throw runtimeUnavailable("Editor source analysis is not synchronized with CodeMirror.");
    }
    const synchronizedMutation = analysisWorkerMutation;
    await synchronizedMutation;
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

  async function loadWasmPlayerModule() {
    if (!wasmPlayerModulePromise) {
      wasmPlayerModulePromise = import(wasmModuleUrl("./wasm_player/puzzle_wasm_player.js"))
        .then(async (module) => {
          if (typeof module.default !== "function") {
            throw runtimeUnavailable("Player WASM loader is missing its default initializer.");
          }
          await module.default({
            module_or_path: wasmModuleUrl("./wasm_player/puzzle_wasm_player_bg.wasm"),
          });
          return module;
        })
        .catch((error) => {
          wasmPlayerModulePromise = null;
          throw error;
        });
    }
    return wasmPlayerModulePromise;
  }

  function runtimeExportValue(value) {
    const runtimeExport = JSON.parse(JSON.stringify(value || {}));
    delete runtimeExport.__kind;
    return runtimeExport;
  }

  async function requireWasmFunction(name) {
    const module = await loadWasmCompiler();
    const fn = module?.[name];
    if (typeof fn !== "function") {
      throw runtimeUnavailable(`Editor WASM function is missing: ${name}`);
    }
    return fn;
  }

  async function workspaceSessionFor(documents) {
    const module = await loadWasmCompiler();
    const WorkspaceSession = module?.WasmWorkspaceSession;
    if (typeof WorkspaceSession !== "function") {
      throw runtimeUnavailable("Editor WASM workspace session is missing.");
    }
    const key = JSON.stringify(documents);
    if (workspaceSession && workspaceSessionKey === key) {
      return workspaceSession;
    }
    if (workspaceSession) {
      workspaceSession.replace_documents(documents);
    } else {
      workspaceSession = new WorkspaceSession(documents);
    }
    workspaceSessionKey = key;
    return workspaceSession;
  }

  function asString(value) {
    return typeof value === "string" ? value : "";
  }

  function parseSourceAnalysisJson(raw) {
    const payload = JSON.parse(raw || "{}");
    return payload && typeof payload === "object" ? payload : {};
  }

  window.PuzzleStudioRuntime = {
    async projectRendererState(payload = {}) {
      const module = await loadWasmPlayerModule();
      if (typeof module.project_renderer_state !== "function") {
        throw runtimeUnavailable("Player WASM renderer-state projection is missing.");
      }
      return JSON.parse(module.project_renderer_state(
        JSON.stringify(runtimeExportValue(payload.runtimeExport)),
        JSON.stringify(payload.state),
        Number(payload.levelIndex),
      ));
    },

    async prepareRenderScene(renderScene) {
      const module = await loadWasmPlayerModule();
      if (!window.PuzzleRenderAssetDecoder?.hydrateRenderSceneImages) {
        throw runtimeUnavailable("Render asset decoder is unavailable.");
      }
      return window.PuzzleRenderAssetDecoder.hydrateRenderSceneImages(module, renderScene);
    },

    async resolveRenderMoment(renderScene, moment) {
      const module = await loadWasmPlayerModule();
      if (typeof module.resolve_render_moment !== "function") {
        throw runtimeUnavailable("Player WASM render-moment resolver is missing.");
      }
      return JSON.parse(module.resolve_render_moment(
        JSON.stringify(renderScene),
        JSON.stringify(moment),
      ));
    },

    async compilePreview(payload = {}) {
      const session = await workspaceSessionFor(payload.workspaceDocuments);
      return session.compile_preview(
        asString(payload.puzzlePath),
        asString(payload.gameCss),
        asString(payload.gameVisualsJs),
      );
    },

    async workspacePresentationManifest(payload = {}) {
      const session = await workspaceSessionFor(payload.workspaceDocuments);
      return session.presentation_manifest(asString(payload.puzzlePath));
    },

    async workspaceIndex(payload = {}) {
      const session = await workspaceSessionFor(payload.workspaceDocuments);
      return JSON.parse(session.index_json() || "{}");
    },

    async exportHtml(payload = {}) {
      const session = await workspaceSessionFor(payload.workspaceDocuments);
      const runtimeAssets = await window.PuzzleStudioRuntime.playerRuntimeAssets();
      return session.export_html(
        asString(payload.puzzlePath),
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
      });
    },

    async sourceOutline(payload = {}) {
      const source = asString(payload.source);
      return querySynchronizedAnalysisWorker("outline", source);
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

    async mutateVisualSource(source, visual) {
      const raw = await querySynchronizedAnalysisWorker("mutateVisual", asString(source), { visual });
      return JSON.parse(raw || "null");
    },

    async soundSourceRequest(source, soundRequest) {
      const raw = await querySynchronizedAnalysisWorker("soundSource", asString(source), { soundRequest });
      return JSON.parse(raw || "null");
    },

    async levelSourceRequest(source, levelRequest) {
      const raw = await querySynchronizedAnalysisWorker("levelSource", asString(source), { levelRequest });
      return JSON.parse(raw || "null");
    },

    async sourceImportReference(source, documentPath, cursorOffset) {
      const raw = await querySynchronizedAnalysisWorker("importReference", asString(source), {
        documentPath: asString(documentPath),
        cursorOffset: Number(cursorOffset) || 0,
      });
      return parseSourceAnalysisJson(raw)?.reference || null;
    },

    async sourceEntries(source) {
      const payload = await window.PuzzleStudioRuntime.sourceEntryInfo(source);
      return payload.entries;
    },

    async sourceEntryInfo(source) {
      const raw = await querySynchronizedAnalysisWorker("entries", asString(source));
      const payload = parseSourceAnalysisJson(raw);
      return {
        entries: Array.isArray(payload.entries) ? payload.entries : [],
      };
    },

    async levelEditorSourceSession(source) {
      const bundle = await querySynchronizedAnalysisWorker(
        "levelEditorBundle",
        asString(source),
      );
      const revision = Number(bundle?.revision);
      if (!Number.isInteger(revision) || revision <= 0) {
        throw runtimeUnavailable(`Editor analysis worker returned an invalid revision: ${revision}`);
      }
      return {
        revision,
        manifest() {
          return bundle.manifest;
        },
        levelSlots(levelIndex, authoredLayer = -1) {
          const level = bundle.slots?.[Number(levelIndex)];
          const slots = Number(authoredLayer) < 0
            ? level?.integrated
            : level?.authored?.[Number(authoredLayer)];
          if (!(slots instanceof Uint32Array)) {
            throw runtimeUnavailable("Editor analysis worker returned level slots in an invalid format.");
          }
          return slots;
        },
        visual(objectId) {
          const index = bundle.manifest?.objects?.findIndex(
            (object) => Number(object?.id) === Number(objectId),
          );
          return Number.isInteger(index) && index >= 0 ? bundle.visuals?.[index] ?? null : null;
        },
      };
    },

<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
    resetSourceAnalysis(source, sourceProfile) {
      return resetAnalysisWorkerSource(source, sourceProfile);
=======
    sourceAnalysisPayload(source) {
      const analysis = sourceAnalysisForLoadedSource(source);
      if (!analysis.payload) {
        const raw = querySourceAnalysis(wasmCompiler, analysis.revision, "active_source_analysis_json");
        analysis.payload = parseSourceAnalysisJson(raw);
      }
      return analysis.payload;
    },

    resetSourceAnalysis(source) {
      return resetAnalysisWorkerSource(source);
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
    },

    applySourceAnalysisEdits(changes, source) {
      try {
        return applyAnalysisWorkerEdits(changes, source);
      } catch (error) {
        return Promise.reject(error);
      }
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

    async editorAudio() {
      if (!editorAudioPromise) {
        editorAudioPromise = loadWasmCompiler()
          .then((module) => {
            if (typeof module.WasmEditorAudio !== "function") {
              throw runtimeUnavailable("Editor WASM audio service is missing.");
            }
            if (typeof module.editor_audio_sfx_types !== "function") {
              throw runtimeUnavailable("Editor WASM audio authoring contract is missing.");
            }
            const session = new module.WasmEditorAudio();
            const recipe = {
              sfx: { seed: "123456", type: "random", volume: 1 },
              music: { seed: "123456", height: 0.5, bars: 8, bpm: 110, volume: 1 },
            };
            let musicPlaying = false;
            let feedbackHandler = null;
            const pendingFeedback = [];
            const deliverFeedback = (diagnostic) => {
              const message = String(diagnostic || "Editor audio output failed.");
              if (!feedbackHandler) {
                pendingFeedback.push(message);
                return;
              }
              try {
                feedbackHandler(message);
              } catch (error) {
                console.error(`Editor audio feedback handler: ${String(error?.message || error)}`);
              }
            };
            session.set_audio_feedback_wakeup(() => {
              try {
                const diagnostics = JSON.parse(session.audio_feedback_event(performance.now()));
                if (!Array.isArray(diagnostics) || diagnostics.some((item) => typeof item !== "string")) {
                  throw new Error("Editor audio feedback returned an invalid diagnostic contract.");
                }
                for (const diagnostic of diagnostics) {
                  deliverFeedback(diagnostic);
                }
              } catch (error) {
                deliverFeedback(`Editor audio feedback failed: ${error?.message || error}`);
              }
            });
            const configure = () => session.configure(
              recipe.sfx.seed,
              recipe.sfx.type,
              recipe.sfx.volume,
              recipe.music.seed,
              recipe.music.height,
              recipe.music.bars,
              recipe.music.bpm,
              recipe.music.volume,
              performance.now(),
            );
            return Object.freeze({
              async sfxTypes() {
                return ["random", ...module.editor_audio_sfx_types()];
              },
              randomSfxPreset(seed, type) {
                return module.editor_audio_random_sfx_preset(String(seed), String(type));
              },
              randomMusicPreset(seed) {
                return module.editor_audio_random_music_preset(String(seed));
              },
              unlock() {
                return session.unlock(performance.now());
              },
              async playSfx(next) {
                const now = performance.now();
                const musicProgress = musicPlaying ? session.music_progress(now) : null;
                recipe.sfx = {
                  seed: String(next.seed),
                  type: String(next.type),
                  volume: Number(next.volume),
                };
                configure();
                session.play_sfx(now);
                if (musicProgress !== null) {
                  session.play_music(musicProgress, now);
                }
              },
              async playMusic(next) {
                recipe.music = {
                  seed: String(next.seed),
                  height: Number(next.height),
                  bars: Number(next.bars),
                  bpm: Number(next.bpm),
                  volume: Number(next.volume),
                };
                configure();
                session.play_music(Number(next.progress), performance.now());
                musicPlaying = true;
              },
              async pauseMusic() {
                session.pause_music(performance.now());
                musicPlaying = false;
              },
              async stop() {
                session.stop(performance.now());
                musicPlaying = false;
              },
              async musicProgress() {
                return session.music_progress(performance.now());
              },
              setFeedbackHandler(callback) {
                if (typeof callback !== "function") {
                  throw new TypeError("Editor audio feedback handler must be a function.");
                }
                feedbackHandler = callback;
                for (const diagnostic of pendingFeedback.splice(0)) {
                  deliverFeedback(diagnostic);
                }
              },
              async setVisible(visible) {
                session.set_visible(Boolean(visible), performance.now());
              },
              exportSfxWav(next) {
                if (next) {
                  recipe.sfx = {
                    seed: String(next.seed),
                    type: String(next.type),
                    volume: Number(next.volume),
                  };
                  configure();
                }
                return session.export_sfx_wav();
              },
              exportMusicWav(next) {
                if (next) {
                  recipe.music = {
                    seed: String(next.seed),
                    height: Number(next.height),
                    bars: Number(next.bars),
                    bpm: Number(next.bpm),
                    volume: Number(next.volume),
                  };
                  configure();
                }
                return session.export_music_wav();
              },
            });
          })
          .catch((error) => {
            editorAudioPromise = null;
            throw error;
          });
      }
      return editorAudioPromise;
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

    async playerRuntimeAssets() {
      if (!playerRuntimeAssetsPromise) {
        playerRuntimeAssetsPromise = Promise.all([
          fetchRequiredText(wasmModuleUrl("./wasm_player/puzzle_wasm_player.js"), "Puzzle player runtime module"),
          fetchRequiredBytes(wasmModuleUrl("./wasm_player/puzzle_wasm_player_bg.wasm"), "Puzzle player runtime WASM"),
        ])
          .then(([moduleSource, wasmBytes]) => ({
            moduleSource,
            wasmBase64: bytesToBase64(wasmBytes),
          }))
          .catch((error) => {
            playerRuntimeAssetsPromise = null;
            throw error;
          });
      }
      return playerRuntimeAssetsPromise;
    },

    loadWasmCompiler,
  };
})();
