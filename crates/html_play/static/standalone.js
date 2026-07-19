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
      this.inputIdsByName = new Map((bootData.inputs || []).map((input) => [input.name, input.id]));
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
      this.sessionRuntime = this.wasmModule.WasmStandaloneSession.fromExport(this.exportJson);
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

    sessionRequestJson(method, url) {
      const action = this.sessionAction(method, url);
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

    sessionAction(method, url) {
      if (method === "GET" && url === "/api/state") {
        return { kind: "snapshot" };
      }
      if (method !== "POST") {
        throw new Error(`Unsupported standalone session request: ${method} ${url}`);
      }
      if (url === "/api/resume") {
        return { kind: "resume" };
      }
      const inputPrefix = "/api/input/";
      if (url.startsWith(inputPrefix)) {
        return { kind: "input", name: decodeURIComponent(url.slice(inputPrefix.length)) };
      }
      const commandPrefix = "/api/command/";
      if (!url.startsWith(commandPrefix)) {
        throw new Error(`Unsupported standalone session request: ${method} ${url}`);
      }
      const name = decodeURIComponent(url.slice(commandPrefix.length));
      const directActions = {
        undo: "undo",
        redo: "redo",
        restart: "restart",
        next: "next_level",
        next_level: "next_level",
        previous_level: "previous_level",
      };
      return directActions[name] ? { kind: directActions[name] } : { kind: "command", name };
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

    applyDebugInputName(inputName) {
      if (!this.sessionRuntime || !this.editorPreviewInputEnabled) {
        throw new Error("Debug input is unavailable in this standalone runtime.");
      }
      return JSON.parse(this.sessionRuntime.dispatch(JSON.stringify({
        kind: "debug_input",
        name: String(inputName || ""),
      })));
    }

    applyCommandName(commandName) {
      if (!this.sessionRuntime) {
        throw new Error("Puzzle game WASM runtime is unavailable.");
      }
      this.sessionRuntime.apply_command_name(commandName);
      this.writeSessionProgressSave();
      window.dispatchEvent(new CustomEvent("PuzzleStandaloneStateChanged"));
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

    progressSaveVersion() {
      return Number(this.data.progressSaveVersion || 1);
    }

    progressSaveStorageKey() {
      const key = this.data.saveKey || this.data.puzzlePath || this.data.title || "untitled";
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
      try {
        raw = raw || window.localStorage?.getItem(this.progressSaveStorageKey());
      } catch (_error) {
      }
      if (!raw) {
        return;
      }
      try {
        this.sessionRuntime.restore_progress_save(raw);
        this.sessionRuntime.mark_progress_save_written();
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
      const saveJson = this.sessionRuntime.progress_save();
      this.notifyEditorPreviewProgressSave("PuzzleStudioPreviewProgressSave", saveJson);
      try {
        window.localStorage?.setItem(this.progressSaveStorageKey(), saveJson);
      } catch (_error) {
        // Browsers can deny storage for local files, private windows, or quota limits.
      }
      this.sessionRuntime.mark_progress_save_written();
      return true;
    }

    clearSessionProgressSave() {
      if (this.sessionRuntime) {
        this.sessionRuntime.clear_progress_save();
      }
      if (!this.sessionProgressEnabled()) {
        return;
      }
      this.notifyEditorPreviewProgressSave("PuzzleStudioPreviewProgressSaveClear");
      try {
        window.localStorage?.removeItem(this.progressSaveStorageKey());
      } catch (_error) {
        // Ignore storage failures; the in-memory progress was already cleared.
      }
    }

    sessionProgressEnabled() {
      return this.data?.editorPreview !== true;
    }
  }

  window.PuzzleStandaloneRuntime = PuzzleStandaloneRuntime;
}());
