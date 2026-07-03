(() => {
  try {
    const theme = window.localStorage.getItem("PuzzleStudioEditorTheme:v1");
    document.documentElement.dataset.theme = theme === "light" ? "light" : "dark";
  } catch {
    document.documentElement.dataset.theme = "dark";
  }
})();

(() => {
  function tauriInvoke() {
    return window.__TAURI__?.core?.invoke || window.__TAURI__?.tauri?.invoke || null;
  }

  function tauriListen(eventName, handler) {
    if (window.__TAURI__?.event?.listen) {
      return window.__TAURI__.event.listen(eventName, handler);
    }
    if (window.__TAURI__?.core?.listen) {
      return window.__TAURI__.core.listen(eventName, handler);
    }
    return null;
  }

  function serverBackendAvailable() {
    if (window.PuzzleStudioStaticSite || document.documentElement.dataset.staticSite === "true") {
      return false;
    }
    return window.location.protocol === "http:" || window.location.protocol === "https:";
  }

  function backendUnavailableError() {
    const error = new Error("Editor server backend is unavailable.");
    error.status = 404;
    return error;
  }

  function editorRuntime() {
    const runtime = window.PuzzleStudioRuntime;
    if (!runtime) {
      throw new Error("PuzzleStudio browser runtime is unavailable.");
    }
    return runtime;
  }

  function diagnosticSummary(diagnostics) {
    if (!Array.isArray(diagnostics) || diagnostics.length === 0) {
      return "";
    }
    if (diagnostics.length === 1) {
      return diagnostics[0]?.message || "Compile error";
    }
    return `${diagnostics.length} compile errors`;
  }

  function hostErrorFromPayload(payload, fallbackMessage) {
    if (payload && typeof payload === "object") {
      const diagnostics = Array.isArray(payload.diagnostics) ? payload.diagnostics : null;
      const message = payload.error || payload.message || diagnosticSummary(diagnostics) || fallbackMessage;
      const error = new Error(message);
      if (diagnostics) {
        error.diagnostics = diagnostics;
      }
      return error;
    }
    return new Error(String(payload || fallbackMessage));
  }

  async function fetchText(url, options = {}) {
    const response = await fetch(url, options);
    const contentType = response.headers.get("content-type") || "";
    if (!response.ok) {
      let error;
      if (contentType.includes("application/json")) {
        const body = await response.json();
        error = hostErrorFromPayload(body, response.statusText);
      } else {
        error = new Error(await response.text());
      }
      error.status = response.status;
      throw error;
    }
    return response.text();
  }

  async function fetchJson(url, options = {}) {
    const response = await fetch(url, options);
    const body = await response.json();
    if (!response.ok) {
      const error = new Error(body.error || response.statusText);
      error.status = response.status;
      throw error;
    }
    return body;
  }

  window.PuzzleStudioHost = {
    mode() {
      return tauriInvoke() ? "tauri" : "server";
    },
    async loadSource() {
      const invoke = tauriInvoke();
      if (invoke) {
        return invoke("load_source");
      }
      if (!serverBackendAvailable()) {
        throw backendUnavailableError();
      }
      return fetchJson("/api/source");
    },
    async openWorkspace(payload = {}) {
      const invoke = tauriInvoke();
      if (!invoke) {
        throw new Error("Open workspace is only available in the desktop app.");
      }
      return invoke("open_workspace", { request: payload });
    },
    async openProject() {
      return this.openWorkspace({ kind: "folder" });
    },
    async recentWorkspaces() {
      const invoke = tauriInvoke();
      if (!invoke) {
        return [];
      }
      return invoke("recent_workspaces");
    },
    async openRecentWorkspace(payload) {
      const invoke = tauriInvoke();
      if (!invoke) {
        throw new Error("Open recent is only available in the desktop app.");
      }
      return invoke("open_recent_workspace", { request: payload });
    },
    async removeWorkspace(payload) {
      const invoke = tauriInvoke();
      if (!invoke) {
        throw new Error("Remove workspace is only available in the desktop app.");
      }
      return invoke("remove_workspace", { request: payload });
    },
    async listenWorkspaceChanged(handler) {
      const listen = tauriListen("puzzlestudio-workspace-changed", (event) => {
        handler(event?.payload || {});
      });
      if (!listen) {
        return () => {};
      }
      return listen;
    },
    async preview(payload, options = {}) {
      if (options.signal?.aborted) {
        throw new DOMException("Preview request was aborted.", "AbortError");
      }
      try {
        return await editorRuntime().compilePreview(payload);
      } catch (error) {
        throw hostErrorFromPayload(error, "Preview compile failed");
      }
    },
    async exportStandaloneHtml(payload, options = {}) {
      if (options.signal?.aborted) {
        throw new DOMException("Export request was aborted.", "AbortError");
      }
      try {
        return await editorRuntime().exportHtml(payload);
      } catch (error) {
        throw hostErrorFromPayload(error, "HTML export failed");
      }
    },
    async highlight(payload, options = {}) {
      if (options.signal?.aborted) {
        throw new DOMException("Highlight request was aborted.", "AbortError");
      }
      return editorRuntime().highlightSource(payload);
    },
    async soundTools() {
      const invoke = tauriInvoke();
      if (invoke) {
        return invoke("sound_tools");
      }
      if (!serverBackendAvailable()) {
        throw backendUnavailableError();
      }
      return fetchText("/sound-tools.js");
    },
    async editorDocsHtml() {
      const invoke = tauriInvoke();
      if (invoke) {
        return invoke("editor_docs");
      }
      throw new Error("Editor documents must be embedded in the editor HTML outside desktop mode.");
    },
    async newPuzzleSource(payload) {
      throw new Error("New puzzle source is browser-runtime owned, not host-owned.");
    },
    async save(payload) {
      const invoke = tauriInvoke();
      if (invoke) {
        return invoke("save_source", { request: payload });
      }
      if (!serverBackendAvailable()) {
        throw backendUnavailableError();
      }
      return fetchText("/api/save", {
        method: "POST",
        headers: { "Content-Type": "application/json; charset=utf-8" },
        body: JSON.stringify(payload),
      });
    },
    async loadWorkspaceDocument(payload) {
      const invoke = tauriInvoke();
      if (invoke) {
        return invoke("load_workspace_document", { request: payload });
      }
      if (!serverBackendAvailable()) {
        throw backendUnavailableError();
      }
      return fetchJson("/api/load-workspace-document", {
        method: "POST",
        headers: { "Content-Type": "application/json; charset=utf-8" },
        body: JSON.stringify(payload),
      });
    },
    async exportHtml(payload) {
      const invoke = tauriInvoke();
      if (invoke) {
        return invoke("export_html", { request: payload });
      }
      return { handled: false };
    },
    async createSourceFile(payload) {
      const invoke = tauriInvoke();
      if (invoke) {
        return invoke("create_source_file", { request: payload });
      }
      if (!serverBackendAvailable()) {
        throw backendUnavailableError();
      }
      return fetchJson("/api/create-source-file", {
        method: "POST",
        headers: { "Content-Type": "application/json; charset=utf-8" },
        body: JSON.stringify(payload),
      });
    },
    async createSourceFolder(payload) {
      const invoke = tauriInvoke();
      if (invoke) {
        return invoke("create_source_folder", { request: payload });
      }
      if (!serverBackendAvailable()) {
        throw backendUnavailableError();
      }
      return fetchJson("/api/create-source-folder", {
        method: "POST",
        headers: { "Content-Type": "application/json; charset=utf-8" },
        body: JSON.stringify(payload),
      });
    },
    async renameWorkspaceEntry(payload) {
      const invoke = tauriInvoke();
      if (invoke) {
        return invoke("rename_workspace_entry", { request: payload });
      }
      if (!serverBackendAvailable()) {
        throw backendUnavailableError();
      }
      return fetchJson("/api/rename-workspace-entry", {
        method: "POST",
        headers: { "Content-Type": "application/json; charset=utf-8" },
        body: JSON.stringify(payload),
      });
    },
    async deleteWorkspaceEntry(payload) {
      const invoke = tauriInvoke();
      if (invoke) {
        return invoke("delete_workspace_entry", { request: payload });
      }
      if (!serverBackendAvailable()) {
        throw backendUnavailableError();
      }
      return fetchJson("/api/delete-workspace-entry", {
        method: "POST",
        headers: { "Content-Type": "application/json; charset=utf-8" },
        body: JSON.stringify(payload),
      });
    },
  };
})();
