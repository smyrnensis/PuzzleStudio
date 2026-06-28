(function () {
  class PuzzleStandaloneRuntime {
    constructor(exportData) {
      this.data = exportData;
      this.wasmModule = null;
      this.sessionRuntime = null;
      this.usesRustSession = false;
      this.initialized = false;
      this.editorPreviewSceneEnabled = false;
      this.editorPreviewInputEnabled = false;
      this.inputIdsByName = new Map((exportData.inputs || []).map((input) => [input.name, input.id]));
      this.initializationPromise = this.initializeRuntime();
    }

    async requestJson(url, options = {}) {
      await this.ensureInitialized();
      const method = options.method || "GET";
      return this.sessionRequestJson(method, url);
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
      this.sessionRuntime = this.wasmModule.WasmStandaloneSession.fromExport(JSON.stringify(this.data || {}));
      this.restoreSessionProgressSave();
      this.usesRustSession = true;
      return true;
    }

    sessionRequestJson(method, url) {
      const raw = this.sessionRuntime.request_json(method, url);
      const next = JSON.parse(raw);
      if (method === "POST") {
        this.writeSessionProgressSave();
      }
      return next;
    }

    snapshot() {
      if (!this.sessionRuntime) {
        throw new Error("Puzzle game WASM runtime is unavailable.");
      }
      return JSON.parse(this.sessionRuntime.snapshot());
    }

    applyInputName(inputName) {
      if (!this.sessionRuntime) {
        throw new Error("Puzzle game WASM runtime is unavailable.");
      }
      this.sessionRuntime.apply_input_name(inputName);
      this.writeSessionProgressSave();
      window.dispatchEvent(new CustomEvent("PuzzleStandaloneStateChanged"));
    }

    applyCommandName(commandName) {
      if (!this.sessionRuntime) {
        throw new Error("Puzzle game WASM runtime is unavailable.");
      }
      this.sessionRuntime.apply_command_name(commandName);
      this.writeSessionProgressSave();
      window.dispatchEvent(new CustomEvent("PuzzleStandaloneStateChanged"));
    }

    setCurrentState(state, options = {}) {
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

    progressSaveVersion() {
      return Number(this.data.progressSaveVersion || 1);
    }

    progressSaveStorageKey() {
      const key = this.data.saveKey || this.data.puzzlePath || this.data.title || "untitled";
      return `PuzzleStudio.progress.v${this.progressSaveVersion()}:${key}`;
    }

    restoreSessionProgressSave() {
      if (!this.sessionRuntime) {
        return;
      }
      let raw;
      try {
        raw = window.localStorage?.getItem(this.progressSaveStorageKey());
      } catch (_error) {
        return;
      }
      if (!raw) {
        return;
      }
      try {
        this.sessionRuntime.restore_progress_save(raw);
        this.sessionRuntime.mark_progress_save_written();
      } catch (error) {
        console.warn(
          `Progress save could not be restored for ${this.progressSaveStorageKey()}; starting from defaults.`,
          error,
        );
      }
    }

    writeSessionProgressSave() {
      if (!this.sessionRuntime) {
        return;
      }
      try {
        window.localStorage?.setItem(this.progressSaveStorageKey(), this.sessionRuntime.progress_save());
        this.sessionRuntime.mark_progress_save_written();
      } catch (_error) {
        // Browsers can deny storage for local files, private windows, or quota limits.
      }
    }

    clearSessionProgressSave() {
      if (this.sessionRuntime) {
        this.sessionRuntime.clear_progress_save();
      }
      try {
        window.localStorage?.removeItem(this.progressSaveStorageKey());
      } catch (_error) {
        // Ignore storage failures; the in-memory progress was already cleared.
      }
    }
  }

  window.PuzzleStandaloneRuntime = PuzzleStandaloneRuntime;
}());
