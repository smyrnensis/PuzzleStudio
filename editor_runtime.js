(() => {
  const wasmCompilerAssetVersion = Date.now().toString(36);
  let wasmCompiler = null;
  let wasmCompilerPromise = null;
  let gameRuntimeAssetsPromise = null;
  let visualAuthoringRuntimeAssetsPromise = null;
  let runtimeCapabilitiesPromise = null;
  const playerRuntimeAssetPromises = new Map();
  let playerAudioWorkletPromise = null;
  let editorAudioPromise = null;
  let workspaceSession = null;
  let workspaceSessionKey = "";
  let workspaceSessionOperations = Promise.resolve();
  let editorWorkspaceSession = null;
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

  async function runtimeCapabilities() {
    if (!runtimeCapabilitiesPromise) {
      runtimeCapabilitiesPromise = fetch(
        wasmModuleUrl("./runtime_capabilities.json"),
        { cache: "no-store" },
      )
        .then(async (response) => {
          if (!response.ok) {
            throw runtimeUnavailable(
              `Runtime capability manifest request failed (${response.status}).`,
            );
          }
          const manifest = await response.json();
          if (manifest?.schema !== 4 || !manifest.runtimes || !manifest.launchProfiles) {
            throw runtimeUnavailable("Runtime capability manifest is invalid.");
          }
          return manifest;
        })
        .catch((error) => {
          runtimeCapabilitiesPromise = null;
          throw error;
        });
    }
    return runtimeCapabilitiesPromise;
  }

  function requiredRuntimeFile(files, name, runtimeName) {
    const file = (files || []).find((candidate) => candidate?.name === name);
    if (!file?.path) {
      throw runtimeUnavailable(`Runtime ${runtimeName} is missing its ${name} asset.`);
    }
    return file.path;
  }

  function runtimeAssetUrl(path) {
    return new URL(wasmModuleUrl(`./${path}`), document.baseURI).href;
  }

  function launchProfileRuntime(manifest, profileName) {
    const profile = manifest.launchProfiles?.[profileName];
    const runtime = manifest.runtimes?.[profile?.runtime];
    if (!profile || !runtime) {
      throw runtimeUnavailable(`Runtime launch profile is missing: ${profileName}.`);
    }
    return { profile, runtime };
  }

  function standalonePlayerRuntimeUrls(manifest, artifact) {
    const { runtime } = launchProfileRuntime(manifest, "standalonePlayer");
    const descriptor = Object.values(runtime.artifacts || {})
      .find((candidate) => candidate.id === artifact && !candidate.editorCapability);
    if (!descriptor) {
      throw runtimeUnavailable(`Unknown standalone player artifact: ${artifact}`);
    }
    return {
      moduleUrl: runtimeAssetUrl(requiredRuntimeFile(descriptor.files, "module", "player")),
      wasmUrl: runtimeAssetUrl(requiredRuntimeFile(descriptor.files, "wasm", "player")),
      rendererModuleUrl: runtimeAssetUrl(
        requiredRuntimeFile(runtime.sharedFiles, "rendererModule", "player"),
      ),
      audioWorkletUrl: runtimeAssetUrl(
        requiredRuntimeFile(runtime.sharedFiles, "audioWorklet", "player"),
      ),
    };
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

  function withWorkspaceSession(documents, operation) {
    const result = workspaceSessionOperations.then(async () => {
      const session = await workspaceSessionFor(documents);
      return operation(session);
    });
    workspaceSessionOperations = result.then(
      () => undefined,
      () => undefined,
    );
    return result;
  }

  function asString(value) {
    return typeof value === "string" ? value : "";
  }

  function consumeRustPanicDiagnostic() {
    const key = "__PuzzleStudioLastRustPanic";
    const message = asString(globalThis[key]).trim();
    Reflect.deleteProperty(globalThis, key);
    return message;
  }

  function parseSourceAnalysisJson(raw) {
    const payload = JSON.parse(raw || "{}");
    return payload && typeof payload === "object" ? payload : {};
  }

  window.PuzzleStudioRuntime = {
    initializeEditorWorkspace(documentIds = []) {
      if (!wasmCompiler) {
        throw runtimeUnavailable("Editor WASM must load before workspace initialization.");
      }
      if (!editorWorkspaceSession) {
        const Session = wasmCompiler.WasmEditorWorkspaceSession;
        if (typeof Session !== "function") {
          throw runtimeUnavailable("Editor WASM workspace state session is missing.");
        }
        editorWorkspaceSession = new Session(documentIds);
      }
    },

    dispatchEditorWorkspace(command) {
      if (!editorWorkspaceSession) {
        throw runtimeUnavailable("Editor workspace state is not initialized.");
      }
      return editorWorkspaceSession.dispatch(command);
    },

    async compilePreview(payload = {}) {
      return withWorkspaceSession(payload.workspaceDocuments, (session) => {
        let rawBuild;
        try {
          rawBuild = session.compile_preview(
            asString(payload.puzzlePath),
            JSON.stringify(payload.audioFileDocuments || []),
          );
        } catch (error) {
          const panic = consumeRustPanicDiagnostic();
          if (panic) {
            throw runtimeUnavailable(`Editor preview compiler panicked: ${panic}`);
          }
          throw error;
        }
        const build = JSON.parse(rawBuild || "{}");
        if (
          !build
          || typeof build !== "object"
          || typeof build.runtime?.runtimeExportJson !== "string"
          || !build.runtime.runtimeExportJson
          || typeof build.runtime?.progressIdentityKey !== "string"
          || !build.runtime.progressIdentityKey
          || typeof build.runtime?.playerArtifact !== "string"
          || !build.runtime.playerArtifact
        ) {
          throw runtimeUnavailable("Editor preview compiler returned an invalid typed build.");
        }
        return build;
      });
    },

    async workspacePresentationManifest(payload = {}) {
      return withWorkspaceSession(
        payload.workspaceDocuments,
        (session) => session.presentation_manifest(asString(payload.puzzlePath)),
      );
    },

    async workspaceIndex(payload = {}) {
      return withWorkspaceSession(
        payload.workspaceDocuments,
        (session) => JSON.parse(session.index_json() || "{}"),
      );
    },

    async exportWebBundle(payload = {}) {
      const prepared = await withWorkspaceSession(
        payload.workspaceDocuments,
        (session) => session.prepare_web_export(
            asString(payload.puzzlePath),
            JSON.stringify(payload.audioFileDocuments || []),
          ),
      );
      try {
        const runtimeAsset = await window.PuzzleStudioRuntime.playerRuntimeAsset(
          prepared.renderer_artifact(),
        );
        return JSON.parse(
          prepared.export_web_bundle(
            runtimeAsset.moduleSource,
            runtimeAsset.wasmBase64,
            runtimeAsset.rendererModuleSource,
            runtimeAsset.audioWorkletSource,
          ) || "{}",
        );
      } finally {
        prepared.free();
      }
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

    async levelEditorSourceSession(source, modelName) {
      const bundle = await querySynchronizedAnalysisWorker(
        "levelEditorBundle",
        asString(source),
        { modelName: asString(modelName) },
      );
      const revision = Number(bundle?.revision);
      if (!Number.isInteger(revision) || revision <= 0) {
        throw runtimeUnavailable(`Editor analysis worker returned an invalid revision: ${revision}`);
      }
      const levelSnapshots = new Map((Array.isArray(bundle.snapshots) ? bundle.snapshots : []).map(
        (record) => [Number(record.levelIndex), {
          snapshot: record.snapshot,
        }],
      ));
      const owner = (levelIndex) => {
        const index = Number(levelIndex);
        if (!Number.isInteger(index) || index < 0) {
          throw runtimeUnavailable(`Editor level session requires a valid level index: ${levelIndex}`);
        }
        const record = levelSnapshots.get(index);
        if (!record) {
          throw runtimeUnavailable(`Editor analysis worker did not open level ${index}.`);
        }
        return record;
      };
      const call = async (levelIndex, request) => {
        const record = owner(levelIndex);
        const value = await querySynchronizedAnalysisWorker(
          "levelEditorRequest",
          asString(source),
          { applicationRequest: { ...request, levelIndex: Number(levelIndex) } },
        );
        if (request.type === "dispatch" && value?.snapshot) {
          record.snapshot = value.snapshot;
        }
        return value;
      };
      return {
        revision,
        manifest() {
          return bundle.manifest;
        },
        authoringModelProjection() {
          const projection = bundle.authoringModelProjection;
          if (!projection || typeof projection !== "object") {
            throw runtimeUnavailable("Level editor source analysis is missing its typed authoring model projection.");
          }
          return projection;
        },
        visual(objectId) {
          const index = bundle.manifest?.objects?.findIndex(
            (object) => Number(object?.id) === Number(objectId),
          );
          return Number.isInteger(index) && index >= 0 ? bundle.visuals?.[index] ?? null : null;
        },
        levelSnapshot(levelIndex) {
          return owner(levelIndex).snapshot;
        },
        dispatchLevel(levelIndex, command) {
          return call(levelIndex, { type: "dispatch", command });
        },
        formatLevelSource(levelIndex, name) {
          return call(levelIndex, { type: "formatSource", name: asString(name) });
        },
        insertLevelSource(levelIndex, request) {
          return call(levelIndex, {
            type: "insertSource",
            expectedSource: asString(request?.expectedSource),
            name: asString(request?.name),
            namespace: asString(request?.namespace),
            cursor: Number.isInteger(request?.cursor) ? request.cursor : -1,
            createContainer: request?.createContainer === true,
          });
        },
        draftProjection(levelIndex) {
          return call(levelIndex, { type: "draftProjection" });
        },
        async draftState(levelIndex) {
          const projection = await this.draftProjection(levelIndex);
          return projection?.state;
        },
        async authoringState(levelIndex) {
          const projection = await call(levelIndex, { type: "authoringStateProjection" });
          return projection?.state;
        },
      };
    },

    resetSourceAnalysis(source) {
      return resetAnalysisWorkerSource(source);
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
            if (typeof module.editor_audio_music_backends !== "function") {
              throw runtimeUnavailable("Editor WASM music backend contract is missing.");
            }
            const musicBackends = Object.freeze(module.editor_audio_music_backends());
            const declaredMusicBackend = musicBackends.find((backend) => backend.declarable);
            if (!declaredMusicBackend) {
              throw runtimeUnavailable("Editor WASM music backends declare no default.");
            }
            const session = new module.WasmEditorAudio();
            const recipe = {
              sfx: { seed: "123456", type: "random", volume: 1 },
              music: { backend: declaredMusicBackend.id, seed: "123456", instruments: { rootSeed: null, slotSeeds: Array(6).fill(null) }, height: 0.5, bars: 8, bpm: 110, volume: 1 },
            };
            const setMusicRecipe = (next) => {
              recipe.music = {
                backend: String(next.backend || declaredMusicBackend.id),
                seed: String(next.seed),
                instruments: next.instruments,
                height: Number(next.height),
                bars: Number(next.bars),
                bpm: Number(next.bpm),
                volume: Number(next.volume),
              };
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
              recipe.music.backend,
              recipe.music.seed,
              JSON.stringify(recipe.music.instruments),
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
              async musicBackends() {
                return musicBackends;
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
                await session.prepare_music();
                setMusicRecipe(next);
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
                  setMusicRecipe(next);
                  configure();
                }
                return session.export_music_wav();
              },
              exportMusicPcm(next) {
                if (next) {
                  setMusicRecipe(next);
                  configure();
                }
                return session.export_music_pcm();
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
        gameRuntimeAssetsPromise = runtimeCapabilities().then((manifest) => {
          const { profile, runtime } = launchProfileRuntime(manifest, "editorPlayer");
          const artifact = runtime.artifacts?.[profile.artifact];
          if (!artifact?.editorCapability) {
            throw runtimeUnavailable("Editor player launch profile has no editor-capable artifact.");
          }
          return {
            moduleUrl: runtimeAssetUrl(requiredRuntimeFile(artifact.files, "module", profile.runtime)),
            wasmUrl: runtimeAssetUrl(requiredRuntimeFile(artifact.files, "wasm", profile.runtime)),
            rendererModuleUrl: runtimeAssetUrl(
              requiredRuntimeFile(runtime.sharedFiles, "rendererModule", profile.runtime),
            ),
            audioWorkletUrl: runtimeAssetUrl(
              requiredRuntimeFile(runtime.sharedFiles, "audioWorklet", profile.runtime),
            ),
          };
        });
      }
      return gameRuntimeAssetsPromise;
    },

    async visualAuthoringRuntimeAssets() {
      if (!visualAuthoringRuntimeAssetsPromise) {
        visualAuthoringRuntimeAssetsPromise = runtimeCapabilities().then((manifest) => {
          const { profile, runtime } = launchProfileRuntime(manifest, "editorAuthoring");
          return {
            moduleUrl: runtimeAssetUrl(requiredRuntimeFile(runtime.files, "module", profile.runtime)),
            wasmUrl: runtimeAssetUrl(requiredRuntimeFile(runtime.files, "wasm", profile.runtime)),
            rendererModuleUrl: runtimeAssetUrl(requiredRuntimeFile(runtime.sharedFiles, "rendererModule", profile.runtime)),
          };
        });
      }
      return visualAuthoringRuntimeAssetsPromise;
    },

    async playerRuntimeAsset(artifact) {
      if (!playerRuntimeAssetPromises.has(artifact)) {
        const loading = runtimeCapabilities()
          .then((manifest) => {
            const urls = standalonePlayerRuntimeUrls(manifest, artifact);
            if (!playerAudioWorkletPromise) {
              playerAudioWorkletPromise = fetchRequiredText(
                urls.audioWorkletUrl,
                "Standalone player AudioWorklet",
              ).catch((error) => {
                playerAudioWorkletPromise = null;
                throw error;
              });
            }
            return Promise.all([
              fetchRequiredText(urls.moduleUrl, `${artifact} player module`),
              fetchRequiredBytes(urls.wasmUrl, `${artifact} player WASM`),
              fetchRequiredText(urls.rendererModuleUrl, "Standalone browser renderer"),
              playerAudioWorkletPromise,
            ]);
          })
          .then(([moduleSource, wasmBytes, rendererModuleSource, audioWorkletSource]) => ({
            moduleSource,
            wasmBase64: bytesToBase64(wasmBytes),
            rendererModuleSource,
            audioWorkletSource,
          }))
          .catch((error) => {
            playerRuntimeAssetPromises.delete(artifact);
            throw error;
          });
        playerRuntimeAssetPromises.set(artifact, loading);
      }
      return playerRuntimeAssetPromises.get(artifact);
    },

    async playerRuntimeAssetUrls(artifact) {
      return runtimeCapabilities().then((manifest) => (
        standalonePlayerRuntimeUrls(manifest, artifact)
      ));
    },

    loadWasmCompiler,
  };
})();
