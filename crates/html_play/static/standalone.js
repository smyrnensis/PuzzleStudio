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
      const next = JSON.parse(raw);
      if (method === "POST" && this.writeSessionProgressSave()) {
        if (next && typeof next.snapshot === "object") {
          next.snapshot.has_progress_save = true;
        } else {
          next.has_progress_save = true;
        }
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

    async unlockAudio() {
      await this.ensureInitialized();
      if (typeof this.sessionRuntime?.unlock_audio !== "function") {
        throw new Error("Rust browser audio backend is unavailable.");
      }
      this.reportAudioDiagnostics(
        await this.sessionRuntime.unlock_audio(this.audioClockSeconds()),
      );
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
      return JSON.parse(
        this.sessionRuntime.apply_debug_input_name(String(inputName || "")),
      );
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
    }

    restoreSessionProgressSave() {
      if (!this.sessionRuntime || !this.sessionProgressEnabled()) {
        return;
      }
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

    writeSessionProgressSave() {
      if (!this.sessionRuntime || !this.sessionProgressEnabled()) {
        return false;
      }
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
      this.sessionRuntime.confirm_progress_save_written(request.requestId);
      return true;
    }

    clearSessionProgressSave() {
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
    }

    sessionProgressEnabled() {
      return this.data?.editorPreview !== true;
    }
  }

  window.PuzzleStandaloneRuntime = PuzzleStandaloneRuntime;
}());
