(function () {
  class PuzzleStandaloneRuntime {
    constructor(bootData, runtimeExportJson) {
      if (typeof runtimeExportJson !== "string" || runtimeExportJson.trim() === "") {
        throw new Error("Standalone runtime requires embedded raw export JSON.");
      }
      this.data = bootData;
      this.exportJson = runtimeExportJson;
      this.wasmModule = null;
      this.sessionRuntime = null;
      this.usesRustSession = false;
      this.initialized = false;
      this.editorPreviewSceneEnabled = false;
      this.editorPreviewInputEnabled = false;
      this.editorPreviewDebugAvailable = bootData.editorPreview === true;
      this.inputIdsByName = new Map((bootData.inputs || []).map((input) => [input.name, input.id]));
      this.initializationPromise = this.initializeRuntime();
    }

    async requestJson(url, options = {}) {
      await this.ensureInitialized();
      const method = options.method || "GET";
      return this.sessionRequestJson(method, url, options);
    }

    async initializeRuntime() {
      await this.loadRuntimeModule();
      if (!this.initializeSessionRuntime()) {
        throw new Error("Puzzle game WASM runtime is unavailable.");
      }
      this.initialized = true;
    }

    async ensureInitialized() {
      if (!this.initialized) {
        await this.initializationPromise;
      }
    }

    async loadRuntimeModule() {
      const version = String(this.data?.engineVersion || Date.now());
      this.wasmModule = await window.PuzzleRuntimeWasmLoader.load(version);
    }

    initializeSessionRuntime() {
      if (typeof this.wasmModule?.WasmStandaloneSession?.fromExport !== "function") {
        return false;
      }
      this.sessionRuntime = this.wasmModule.WasmStandaloneSession.fromExport(this.exportJson);
      this.sessionRuntime.set_progress_persistence_enabled(this.sessionProgressEnabled());
      this.releaseWasmOwnedExportPayload();
      this.restoreSessionProgressSave();
      this.usesRustSession = true;
      return true;
    }

    releaseWasmOwnedExportPayload() {
      if (window.PuzzleRuntimeExportJson === this.exportJson) {
        window.PuzzleRuntimeExportJson = "";
      }
      this.exportJson = "";
    }

    sessionRequestJson(method, url, options = {}) {
      const action = this.sessionAction(method, url, options);
      const raw = this.sessionRuntime.dispatch(JSON.stringify(action));
      let next = JSON.parse(raw);
      if (method === "POST" && this.flushProgressSaveRequest()) {
        const presentationEvents = Array.isArray(next?.presentationEvents)
          ? next.presentationEvents
          : [];
        next = this.snapshot();
        next.presentationEvents = presentationEvents;
      }
      return next;
    }

    sessionAction(method, url, options = {}) {
      if (method === "GET" && url === "/api/state") {
        return { kind: "snapshot" };
      }
      if (method !== "POST") {
        throw new Error(`Unsupported standalone session request: ${method} ${url}`);
      }
      if (url === "/api/action") {
        if (typeof options.body !== "string" || options.body.trim() === "") {
          throw new Error("Standalone session action requires a JSON request body.");
        }
        return JSON.parse(options.body);
      }
      throw new Error(`Unsupported standalone session request: ${method} ${url}`);
    }

    snapshot() {
      if (!this.sessionRuntime) {
        throw new Error("Puzzle game WASM runtime is unavailable.");
      }
      return JSON.parse(this.sessionRuntime.snapshot());
    }

    async resolveScenePresentation(sceneName, state = {}) {
      await this.ensureInitialized();
      if (!this.sessionRuntime) {
        throw new Error("Puzzle game WASM runtime is unavailable.");
      }
      if (typeof this.sessionRuntime.resolve_scene_presentation !== "function") {
        throw new Error("Puzzle game WASM runtime does not expose scene presentation resolution.");
      }
      return JSON.parse(this.sessionRuntime.resolve_scene_presentation(
        String(sceneName || ""),
        JSON.stringify(state || {}),
      ));
    }

    resolveRenderFrame(renderScene, elapsedMs) {
      if (typeof this.wasmModule?.resolve_render_frame !== "function") {
        throw new Error("Puzzle game WASM runtime does not expose render-frame resolution.");
      }
      if (!renderScene || typeof renderScene !== "object") {
        throw new Error("Render-frame resolution requires a typed render scene.");
      }
      const time = Math.max(0, Math.floor(Number(elapsedMs)));
      if (!Number.isSafeInteger(time) || time > 0xffffffff) {
        throw new Error("Render-frame elapsed time must fit an unsigned 32-bit millisecond value.");
      }
      return JSON.parse(this.wasmModule.resolve_render_frame(
        JSON.stringify(renderScene),
        time,
      ));
    }

    resolveRenderMoment(renderScene, moment) {
      if (typeof this.wasmModule?.resolve_render_moment !== "function") {
        throw new Error("Puzzle game WASM runtime does not expose animation-aware render resolution.");
      }
      return JSON.parse(this.wasmModule.resolve_render_moment(
        JSON.stringify(renderScene),
        JSON.stringify(moment),
      ));
    }

    async hydrateRenderSceneImages(renderScene) {
      return window.PuzzleRenderAssetDecoder.hydrateRenderSceneImages(this.wasmModule, renderScene);
    }

    applyDebugInputName(inputName) {
      if (!this.sessionRuntime || !this.editorPreviewDebugAvailable) {
        throw new Error("Debug input is unavailable in this standalone runtime.");
      }
      return JSON.parse(this.sessionRuntime.dispatch(JSON.stringify({
        kind: "debug_input",
        name: String(inputName || ""),
      })));
    }

    async setCurrentState(state, options = {}) {
      await this.ensureInitialized();
      if (!this.sessionRuntime) {
        throw new Error("Puzzle game WASM runtime is unavailable.");
      }
      if (typeof this.sessionRuntime.set_current_state !== "function") {
        throw new Error("Puzzle game WASM runtime does not support editor preview state.");
      }
      const levelIndex = Number(options.levelIndex);
      if (!Number.isInteger(levelIndex) || levelIndex < 0) {
        throw new Error("Editor preview state requires a valid level index.");
      }
      this.sessionRuntime.set_current_state(
        JSON.stringify(state),
        levelIndex,
        options.materializeLevelStart === true,
      );
      this.editorPreviewInputEnabled = options.acceptModelInput === true;
      this.editorPreviewSceneEnabled = options.acceptSceneInput === true;
    }

    progressSaveStorageKey() {
      if (typeof this.data.progressStorageKey !== "string" || this.data.progressStorageKey.length === 0) {
        throw new Error("Standalone runtime requires progressStorageKey for progress persistence.");
      }
      return this.data.progressStorageKey;
    }

    restoreSessionProgressSave() {
      if (!this.sessionRuntime || !this.sessionProgressEnabled()) {
        return;
      }
      let raw;
      try {
        raw = window.localStorage?.getItem(this.progressSaveStorageKey());
      } catch (_error) {
      }
      if (!raw) {
        return;
      }
      try {
        this.sessionRuntime.restore_progress_save(raw);
      } catch (error) {
        throw new Error(
          `Progress save could not be restored for ${this.progressSaveStorageKey()}. The saved progress was kept and was not overwritten. Clear progress to start a fresh save. ${error?.message || error}`,
        );
      }
    }

    flushProgressSaveRequest() {
      if (!this.sessionRuntime || !this.sessionProgressEnabled()) {
        return false;
      }
      const rawRequest = this.sessionRuntime.progress_save_request();
      if (!rawRequest) {
        return false;
      }
      const request = JSON.parse(rawRequest);
      if (!Number.isSafeInteger(request.requestId) || request.requestId < 1) {
        throw new Error("Progress save request is missing a valid requestId.");
      }
      if (typeof request.saveJson !== "string" || request.saveJson.length === 0) {
        throw new Error("Progress save request is missing saveJson.");
      }
      try {
        window.localStorage?.setItem(this.progressSaveStorageKey(), request.saveJson);
      } catch (error) {
        window.dispatchEvent(new CustomEvent("PuzzleProgressSaveError", {
          detail: {
            message: `Progress save could not be written for ${this.progressSaveStorageKey()}. ${error?.message || error}`,
          },
        }));
        return false;
      }
      this.sessionRuntime.confirm_progress_save_written(request.requestId);
      return true;
    }

    clearSessionProgressSave() {
      if (!this.sessionProgressEnabled()) {
        return;
      }
      try {
        window.localStorage?.removeItem(this.progressSaveStorageKey());
      } catch (error) {
        throw new Error(
          `Progress save could not be cleared for ${this.progressSaveStorageKey()}. ${error?.message || error}`,
        );
      }
      if (this.sessionRuntime) {
        this.sessionRuntime.confirm_progress_save_cleared();
      }
    }

    sessionProgressEnabled() {
      return this.data?.editorPreview !== true;
    }
  }

  window.PuzzleStandaloneRuntime = PuzzleStandaloneRuntime;
}());
