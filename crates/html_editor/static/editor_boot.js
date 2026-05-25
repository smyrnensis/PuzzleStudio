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

  function serverBackendAvailable() {
    return window.location.protocol === "http:" || window.location.protocol === "https:";
  }

  function backendUnavailableError() {
    const error = new Error("Editor server backend is unavailable.");
    error.status = 404;
    return error;
  }

  async function fetchText(url, options = {}) {
    const response = await fetch(url, options);
    const contentType = response.headers.get("content-type") || "";
    if (!response.ok) {
      let message = response.statusText;
      if (contentType.includes("application/json")) {
        const body = await response.json();
        message = body.error || response.statusText;
      } else {
        message = await response.text();
      }
      const error = new Error(message);
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
    async openProject() {
      const invoke = tauriInvoke();
      if (!invoke) {
        throw new Error("Open project is only available in the desktop app.");
      }
      return invoke("open_project", { request: {} });
    },
    async preview(payload, options = {}) {
      const invoke = tauriInvoke();
      if (invoke) {
        return invoke("compile_preview", { request: payload });
      }
      if (!serverBackendAvailable()) {
        throw backendUnavailableError();
      }
      return fetchText("/api/preview", {
        method: "POST",
        headers: { "Content-Type": "application/json; charset=utf-8" },
        body: JSON.stringify(payload),
        signal: options.signal,
      });
    },
    async highlight(payload, options = {}) {
      const invoke = tauriInvoke();
      if (invoke) {
        return invoke("highlight_source", { request: payload });
      }
      if (!serverBackendAvailable()) {
        throw backendUnavailableError();
      }
      return fetchText("/api/highlight", {
        method: "POST",
        headers: { "Content-Type": "application/json; charset=utf-8" },
        body: JSON.stringify(payload),
        signal: options.signal,
      });
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
  };
})();
