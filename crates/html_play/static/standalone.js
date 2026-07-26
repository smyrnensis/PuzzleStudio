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
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
      this.audioFeedbackWakeup = () => {
        try {
          this.reportAudioDiagnostics(
            this.sessionRuntime.audio_feedback_event(this.audioClockSeconds()),
          );
        } catch (error) {
          console.error(`Audio consumer: ${String(error?.message || error)}`);
        }
      };
      this.sessionRuntime.set_audio_feedback_wakeup(this.audioFeedbackWakeup);
=======
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
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

<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
    async unlockAudio() {
      await this.ensureInitialized();
      if (typeof this.sessionRuntime?.unlock_audio !== "function") {
        throw new Error("Rust browser audio backend is unavailable.");
      }
      this.reportAudioDiagnostics(
        await this.sessionRuntime.unlock_audio(this.audioClockSeconds()),
      );
=======
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
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
    }

    async setAudioVisible(visible) {
      await this.ensureInitialized();
      if (typeof this.sessionRuntime?.set_audio_visible !== "function") {
        throw new Error("Rust browser audio backend is unavailable.");
      }
      this.reportAudioDiagnostics(
        this.sessionRuntime.set_audio_visible(visible, this.audioClockSeconds()),
      );
    }

    presentationEventConsumed() {
      if (typeof this.sessionRuntime?.presentation_event_consumed !== "function") {
        throw new Error("Rust presentation timeline is unavailable.");
      }
      this.reportAudioDiagnostics(
        this.sessionRuntime.presentation_event_consumed(this.audioClockSeconds()),
      );
    }

    presentationFrame() {
      if (typeof this.sessionRuntime?.presentation_frame !== "function") {
        throw new Error("Rust presentation timeline is unavailable.");
      }
      this.reportAudioDiagnostics(
        this.sessionRuntime.presentation_frame(this.audioClockSeconds()),
      );
    }

    audioClockSeconds() {
      return performance.now() / 1000;
    }

    reportAudioDiagnostics(raw) {
      const diagnostics = JSON.parse(raw || "[]");
      for (const diagnostic of diagnostics) {
        console.error(`Audio consumer: ${diagnostic}`);
      }
    }

    /* puzzle-host:optional:editor-preview:start */
    applyDebugInputName(inputName) {
      if (!this.sessionRuntime || !this.editorPreviewDebugAvailable) {
        throw new Error("Debug input is unavailable in this standalone runtime.");
      }
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
      return JSON.parse(
        this.sessionRuntime.apply_debug_input_name(String(inputName || "")),
      );
=======
      return JSON.parse(this.sessionRuntime.dispatch(JSON.stringify({
        kind: "debug_input",
        name: String(inputName || ""),
      })));
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
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
    /* puzzle-host:optional:editor-preview:end */

<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
    progressSaveVersion() {
      const version = Number(this.sessionRuntime?.progress_storage_save_version?.());
      if (!Number.isInteger(version) || version < 1) {
        throw new Error("Rust player export requires a positive progress storage save version.");
      }
      return version;
    }

    progressSaveStorageKey() {
      const key = this.sessionRuntime?.progress_storage_key?.();
      if (typeof key !== "string" || key.length === 0) {
        throw new Error("Rust player export requires a progress storage key.");
      }
      return `PuzzleStudio.progress.v${this.progressSaveVersion()}:${key}`;
    }

    editorPreviewProgressSave() {
      if (window.parent === window) {
        return "";
      }
      const saves = window.PuzzleStudioEditorPreviewProgressSaves;
      const value = saves && saves[this.progressSaveStorageKey()];
      return typeof value === "string" ? value : "";
    }

    notifyEditorPreviewProgressSave(type, saveJson = "") {
      if (window.parent === window) {
        return;
      }
      window.parent.postMessage({
        type,
        storageKey: this.progressSaveStorageKey(),
        saveJson,
      }, "*");
=======
    progressSaveStorageKey() {
      if (typeof this.data.progressStorageKey !== "string" || this.data.progressStorageKey.length === 0) {
        throw new Error("Standalone runtime requires progressStorageKey for progress persistence.");
      }
      return this.data.progressStorageKey;
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
    }

    restoreSessionProgressSave() {
      if (!this.sessionRuntime || !this.sessionProgressEnabled()) {
        return;
      }
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
      let raw = this.editorPreviewProgressSave();
      if (!raw) {
        try {
          if (!window.localStorage) {
            throw new Error("localStorage is unavailable");
          }
          raw = window.localStorage.getItem(this.progressSaveStorageKey());
        } catch (error) {
          throw new Error(
            `Progress save could not be read for ${this.progressSaveStorageKey()}. The saved progress was not modified. ${error?.message || error}`,
          );
        }
=======
      let raw;
      try {
        raw = window.localStorage?.getItem(this.progressSaveStorageKey());
      } catch (_error) {
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
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
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
      const request = JSON.parse(this.sessionRuntime.progress_save_request());
      if (!request) {
        return false;
      }
      if (!Number.isInteger(request.requestId) || request.requestId < 1
          || typeof request.saveJson !== "string" || request.saveJson.length === 0) {
        throw new Error("Runtime returned an invalid progress save request.");
      }
      try {
        if (!window.localStorage) {
          throw new Error("localStorage is unavailable");
        }
        window.localStorage.setItem(this.progressSaveStorageKey(), request.saveJson);
      } catch (error) {
        console.warn(`Progress save remains pending because storage failed: ${error?.message || error}`);
        return false;
      }
      this.notifyEditorPreviewProgressSave("PuzzleStudioPreviewProgressSave", request.saveJson);
=======
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
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
      this.sessionRuntime.confirm_progress_save_written(request.requestId);
      return true;
    }

    clearSessionProgressSave() {
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
      if (!this.sessionRuntime || !this.sessionProgressEnabled()) {
        return;
      }
      try {
        if (!window.localStorage) {
          throw new Error("localStorage is unavailable");
        }
        window.localStorage.removeItem(this.progressSaveStorageKey());
      } catch (error) {
        console.warn(`Progress clear was not confirmed because storage failed: ${error?.message || error}`);
        return;
      }
      this.notifyEditorPreviewProgressSave("PuzzleStudioPreviewProgressSaveClear");
      this.sessionRuntime.confirm_progress_save_cleared();
=======
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
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
    }

    sessionProgressEnabled() {
      return this.data?.editorPreview !== true;
    }
  }

  window.PuzzleStandaloneRuntime = PuzzleStandaloneRuntime;
}());
