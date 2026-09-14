"use strict";
(() => {
    try {
        const colorScheme = window.localStorage.getItem("PuzzleStudioEditorColorScheme");
        document.documentElement.dataset.colorScheme = colorScheme === "light" ? "light" : "dark";
    }
    catch {
        document.documentElement.dataset.colorScheme = "dark";
    }
})();
(() => {
    let desktopExitRequestHandler = null;
    function asJsonRecord(value) {
        return value !== null && typeof value === "object" && !Array.isArray(value)
            ? value
            : null;
    }
    function requireJsonRecord(value, boundary) {
        const record = asJsonRecord(value);
        if (!record) {
            throw new Error(`${boundary} returned an invalid object.`);
        }
        return record;
    }
    function requireString(value, boundary) {
        if (typeof value !== "string") {
            throw new Error(`${boundary} returned an invalid string.`);
        }
        return value;
    }
    function requireOkResult(value, boundary) {
        const record = requireJsonRecord(value, boundary);
        if (record.ok !== true) {
            throw new Error(`${boundary} returned an invalid success result.`);
        }
        return record;
    }
    function requireRecentWorkspaces(value) {
        if (!Array.isArray(value) || value.some((entry) => {
            const record = asJsonRecord(entry);
            return !record || typeof record.workspaceRoot !== "string" || typeof record.name !== "string";
        })) {
            throw new Error("recent_workspaces returned an invalid workspace list.");
        }
        return value;
    }
    function requireEditorDocument(value, boundary) {
        const document = requireJsonRecord(value, boundary);
        if (typeof document.puzzlePath !== "string"
            || typeof document.workspaceRoot !== "string"
            || typeof document.encoding !== "string"
            || typeof document.mimeType !== "string"
            || typeof document.source !== "string"
            || typeof document.dataUrl !== "string"
            || typeof document.contentLoaded !== "boolean"
            || typeof document.previewHtml !== "string"
            || typeof document.previewError !== "string") {
            throw new Error(`${boundary} returned an invalid document.`);
        }
        return document;
    }
    function requireWorkspaceSource(value, boundary) {
        const workspace = requireJsonRecord(value, boundary);
        if (typeof workspace.puzzlePath !== "string"
            || typeof workspace.workspaceRoot !== "string"
            || typeof workspace.source !== "string"
            || !Array.isArray(workspace.folders)
            || workspace.folders.some((folder) => typeof folder !== "string")
            || !Array.isArray(workspace.documents)) {
            throw new Error(`${boundary} returned an invalid workspace source.`);
        }
        workspace.documents.forEach((document) => requireEditorDocument(document, boundary));
        return workspace;
    }
    function requireOpenedWorkspace(value, boundary) {
        const workspace = requireWorkspaceSource(value, boundary);
        requireRecentWorkspaces(workspace.recentWorkspaces);
        return workspace;
    }
    function requireOpenWorkspaceResult(value) {
        const result = requireJsonRecord(value, "open_workspace");
        if (result.canceled === true) {
            return result;
        }
        return requireOpenedWorkspace(result, "open_workspace");
    }
    function requireLoadedWorkspaces(value) {
        const payload = requireJsonRecord(value, "load_source");
        if (typeof payload.puzzlePath !== "string"
            || typeof payload.workspaceRoot !== "string"
            || typeof payload.source !== "string"
            || !Array.isArray(payload.documents)
            || typeof payload.empty !== "boolean"
            || !Array.isArray(payload.workspaces)) {
            throw new Error("load_source returned an invalid workspace collection.");
        }
        payload.documents.forEach((document) => requireEditorDocument(document, "load_source"));
        payload.workspaces.forEach((workspace) => requireWorkspaceSource(workspace, "load_source"));
        requireRecentWorkspaces(payload.recentWorkspaces);
        if (payload.restoreErrors !== undefined) {
            if (!Array.isArray(payload.restoreErrors) || payload.restoreErrors.some((entry) => {
                const error = asJsonRecord(entry);
                return !error
                    || typeof error.workspaceRoot !== "string"
                    || typeof error.message !== "string";
            })) {
                throw new Error("load_source returned invalid workspace restore errors.");
            }
        }
        return payload;
    }
    function requireRemovedWorkspaceResult(value) {
        const record = requireOkResult(value, "remove_workspace");
        if (typeof record.removed !== "boolean") {
            throw new Error("remove_workspace returned an invalid removal result.");
        }
        return record;
    }
    function requireCreatedSourceResult(value, boundary) {
        const record = requireOkResult(value, boundary);
        if (typeof record.puzzlePath !== "string") {
            throw new Error(`${boundary} returned an invalid source result.`);
        }
        return record;
    }
    function requireCreatedFolderResult(value) {
        const record = requireOkResult(value, "create_source_folder");
        if (typeof record.folderPath !== "string") {
            throw new Error("create_source_folder returned an invalid folder result.");
        }
        return record;
    }
    function requireRenamedEntryResult(value) {
        const record = requireOkResult(value, "rename_workspace_entry");
        if (typeof record.path !== "string") {
            throw new Error("rename_workspace_entry returned an invalid rename result.");
        }
        return record;
    }
    function requireExportWebBundleResult(value) {
        const record = requireJsonRecord(value, "export_web_bundle");
        if (record.canceled === true) {
            return record;
        }
        if (record.ok === true && typeof record.path === "string") {
            return record;
        }
        throw new Error("export_web_bundle returned an invalid export result.");
    }
    function requireStandaloneWebBundle(value) {
        const record = requireJsonRecord(value, "browser runtime exportWebBundle");
        if (!Array.isArray(record.files) || record.files.some((file) => {
            const entry = asJsonRecord(file);
            return !entry
                || typeof entry.path !== "string"
                || (entry.encoding !== "utf8" && entry.encoding !== "base64")
                || typeof entry.content !== "string";
        })) {
            throw new Error("browser runtime exportWebBundle returned an invalid file list.");
        }
        return record;
    }
    function validateTauriCommandResult(command, value) {
        switch (command) {
            case "load_source":
                return requireLoadedWorkspaces(value);
            case "open_workspace":
                return requireOpenWorkspaceResult(value);
            case "open_recent_workspace":
            case "select_workspace_entry":
                return requireOpenedWorkspace(value, command);
            case "load_workspace_document":
                return requireEditorDocument(value, command);
            case "recent_workspaces":
                return requireRecentWorkspaces(value);
            case "remove_workspace":
                return requireRemovedWorkspaceResult(value);
            case "complete_desktop_exit":
                if (value !== null && value !== undefined) {
                    throw new Error("complete_desktop_exit returned an invalid completion result.");
                }
                return undefined;
            case "editor_docs":
            case "save_source":
                return requireString(value, command);
            case "export_web_bundle":
                return requireExportWebBundleResult(value);
            case "open_exported_file":
            case "delete_workspace_entry":
                return requireOkResult(value, command);
            case "create_source_file":
            case "create_binary_file":
                return requireCreatedSourceResult(value, command);
            case "create_source_folder":
                return requireCreatedFolderResult(value);
            case "rename_workspace_entry":
                return requireRenamedEntryResult(value);
        }
    }
    function rawTauriInvoke() {
        const bridge = asJsonRecord(window.__TAURI__);
        const core = asJsonRecord(bridge?.core);
        const legacy = asJsonRecord(bridge?.tauri);
        const invoke = core?.invoke || legacy?.invoke;
        return typeof invoke === "function" ? invoke : null;
    }
    function tauriInvoke() {
        const rawInvoke = rawTauriInvoke();
        if (!rawInvoke) {
            return null;
        }
        const invoke = async (command, payload) => validateTauriCommandResult(command, await rawInvoke(command, payload));
        // The command map above is the checked projection of the dynamic Tauri bridge.
        return invoke;
    }
    function rawTauriListen() {
        const bridge = asJsonRecord(window.__TAURI__);
        const event = asJsonRecord(bridge?.event);
        if (typeof event?.listen === "function") {
            return { owner: event, listen: event.listen };
        }
        const core = asJsonRecord(bridge?.core);
        if (typeof core?.listen === "function") {
            return { owner: core, listen: core.listen };
        }
        return null;
    }
    function tauriListen(eventName, handler) {
        const target = rawTauriListen();
        if (!target) {
            return null;
        }
        return Promise.resolve(target.listen.call(target.owner, eventName, handler))
            .then((unlisten) => {
            if (typeof unlisten !== "function") {
                throw new Error(`Tauri listener ${eventName} returned an invalid unlisten function.`);
            }
            return unlisten;
        });
    }
    function tauriEventPayload(event, eventName) {
        const envelope = asJsonRecord(event);
        if (!envelope || !("payload" in envelope)) {
            throw new Error(`Tauri event ${eventName} returned an invalid envelope.`);
        }
        return envelope.payload;
    }
    function requireDesktopExitRequest(event) {
        const payload = requireJsonRecord(tauriEventPayload(event, "puzzlestudio-exit-requested"), "puzzlestudio-exit-requested");
        if ((payload.kind !== "window" && payload.kind !== "app")
            || typeof payload.requestId !== "string"
            || !payload.requestId) {
            throw new Error("puzzlestudio-exit-requested returned an invalid exit request.");
        }
        return payload;
    }
    function requireDesktopExitCompletion(value) {
        const payload = requireJsonRecord(value, "desktop exit completion");
        if ((payload.kind !== "window" && payload.kind !== "app")
            || typeof payload.requestId !== "string"
            || !payload.requestId
            || typeof payload.accepted !== "boolean") {
            throw new Error("Desktop exit completion is invalid.");
        }
        return payload;
    }
    function completeDesktopExit(payload) {
        const invoke = tauriInvoke();
        if (!invoke) {
            throw new Error("Desktop exit is only available in the desktop app.");
        }
        return invoke("complete_desktop_exit", { request: requireDesktopExitCompletion(payload) });
    }
    function configuredWebHost() {
        const bootstrap = asJsonRecord(window.PuzzleStudioBootstrap);
        if (!bootstrap || bootstrap.version !== 1
            || (bootstrap.host !== "server" && bootstrap.host !== "pages")) {
            throw new Error("PuzzleStudio web host bootstrap is missing or invalid.");
        }
        return bootstrap.host;
    }
    function serverBackendAvailable() {
        return configuredWebHost() === "server";
    }
    function backendUnavailableError() {
        const error = new Error("Editor server backend is unavailable.");
        error.status = 404;
        return error;
    }
    async function callEditorRuntime(methodName, payload) {
        const runtime = asJsonRecord(window.PuzzleStudioRuntime);
        const method = runtime?.[methodName];
        if (!runtime || typeof method !== "function") {
            throw new Error(`PuzzleStudio browser runtime ${methodName} is unavailable.`);
        }
        return method.call(runtime, payload);
    }
    function diagnosticSummary(diagnostics) {
        if (!Array.isArray(diagnostics) || diagnostics.length === 0) {
            return "";
        }
        if (diagnostics.length === 1) {
            const diagnostic = asJsonRecord(diagnostics[0]);
            return typeof diagnostic?.message === "string" && diagnostic.message
                ? diagnostic.message
                : "Compile error";
        }
        return `${diagnostics.length} compile errors`;
    }
    function hostErrorFromPayload(payload, fallbackMessage) {
        const record = asJsonRecord(payload);
        if (record) {
            const diagnostics = Array.isArray(record.diagnostics) ? record.diagnostics : null;
            const message = record.error || record.message || diagnosticSummary(diagnostics) || fallbackMessage;
            const error = new Error(String(message));
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
            }
            else {
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
            const error = hostErrorFromPayload(body, response.statusText);
            error.status = response.status;
            throw error;
        }
        return body;
    }
    window.PuzzleStudioHost = {
        mode() {
            return tauriInvoke() ? "tauri" : configuredWebHost();
        },
        async loadSource() {
            const invoke = tauriInvoke();
            if (invoke) {
                return invoke("load_source");
            }
            if (!serverBackendAvailable()) {
                throw backendUnavailableError();
            }
            return requireWorkspaceSource(await fetchJson("/api/source"), "editor server load source");
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
        async selectWorkspaceEntry(payload) {
            const invoke = tauriInvoke();
            if (!invoke) {
                return null;
            }
            return invoke("select_workspace_entry", { request: payload });
        },
        async removeWorkspace(payload) {
            const invoke = tauriInvoke();
            if (!invoke) {
                throw new Error("Remove workspace is only available in the desktop app.");
            }
            return invoke("remove_workspace", { request: payload });
        },
        setDesktopExitRequestHandler(handler) {
            if (handler !== null && typeof handler !== "function") {
                throw new Error("Desktop exit request handler must be a function or null.");
            }
            desktopExitRequestHandler = handler;
        },
        async completeDesktopExit(payload) {
            return completeDesktopExit(payload);
        },
        async listenWorkspaceChanged(handler) {
            if (typeof handler !== "function") {
                throw new Error("Workspace change handler must be a function.");
            }
            const listen = tauriListen("puzzlestudio-workspace-changed", (event) => {
                try {
                    const payload = requireJsonRecord(tauriEventPayload(event, "puzzlestudio-workspace-changed"), "puzzlestudio-workspace-changed");
                    handler(payload);
                }
                catch (error) {
                    console.error(error);
                }
            });
            if (!listen) {
                return () => { };
            }
            return listen;
        },
        async preview(payload, options = {}) {
            if (options.signal?.aborted) {
                throw new DOMException("Preview request was aborted.", "AbortError");
            }
            try {
                return requireJsonRecord(await callEditorRuntime("compilePreview", payload), "browser runtime compilePreview");
            }
            catch (error) {
                throw hostErrorFromPayload(error, "Preview compile failed");
            }
        },
        async exportStandaloneWebBundle(payload, options = {}) {
            if (options.signal?.aborted) {
                throw new DOMException("Export request was aborted.", "AbortError");
            }
            try {
                return requireStandaloneWebBundle(await callEditorRuntime("exportWebBundle", payload));
            }
            catch (error) {
                throw hostErrorFromPayload(error, "Web bundle export failed");
            }
        },
        async highlight(payload, options = {}) {
            if (options.signal?.aborted) {
                throw new DOMException("Highlight request was aborted.", "AbortError");
            }
            return requireString(await callEditorRuntime("highlightSource", payload), "browser runtime highlightSource");
        },
        async sourceOutline(payload, options = {}) {
            if (options.signal?.aborted) {
                throw new DOMException("Outline request was aborted.", "AbortError");
            }
            return requireString(await callEditorRuntime("sourceOutline", payload), "browser runtime sourceOutline");
        },
        async editorDocsHtml() {
            const invoke = tauriInvoke();
            if (invoke) {
                return invoke("editor_docs");
            }
            throw new Error("Editor documents must be embedded in the editor HTML outside desktop mode.");
        },
        async newPuzzleSource() {
            throw new Error("New puzzle source is browser-runtime owned, not host-owned.");
        },
        async save(payload) {
            const invoke = tauriInvoke();
            if (invoke) {
                return invoke("save_source", {
                    request: {
                        source: payload.source,
                        puzzlePath: payload.puzzlePath,
                        ...(payload.workspaceRoot === undefined
                            ? {}
                            : { workspaceRoot: payload.workspaceRoot }),
                    },
                });
            }
            if (!serverBackendAvailable()) {
                throw backendUnavailableError();
            }
            return fetchText("/api/save", {
                method: "POST",
                headers: { "Content-Type": "application/json; charset=utf-8" },
                body: JSON.stringify({
                    ...payload,
                    contentLoaded: payload?.contentLoaded === true,
                }),
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
            return requireEditorDocument(await fetchJson("/api/load-workspace-document", {
                method: "POST",
                headers: { "Content-Type": "application/json; charset=utf-8" },
                body: JSON.stringify(payload),
            }), "editor server load workspace document");
        },
        async exportWebBundle(payload) {
            const invoke = tauriInvoke();
            if (invoke) {
                return invoke("export_web_bundle", { request: payload });
            }
            return { handled: false };
        },
        async openExportedFile(payload) {
            const invoke = tauriInvoke();
            if (!invoke) {
                throw new Error("Opening exported files is only available in the desktop app.");
            }
            return invoke("open_exported_file", { request: payload });
        },
        async createSourceFile(payload) {
            const invoke = tauriInvoke();
            if (invoke) {
                return invoke("create_source_file", { request: payload });
            }
            if (!serverBackendAvailable()) {
                throw backendUnavailableError();
            }
            return requireCreatedSourceResult(await fetchJson("/api/create-source-file", {
                method: "POST",
                headers: { "Content-Type": "application/json; charset=utf-8" },
                body: JSON.stringify(payload),
            }), "editor server create source file");
        },
        async createBinaryFile(payload) {
            const invoke = tauriInvoke();
            if (invoke) {
                return invoke("create_binary_file", { request: payload });
            }
            if (!serverBackendAvailable()) {
                throw backendUnavailableError();
            }
            return requireCreatedSourceResult(await fetchJson("/api/create-binary-file", {
                method: "POST",
                headers: { "Content-Type": "application/json; charset=utf-8" },
                body: JSON.stringify(payload),
            }), "editor server create binary file");
        },
        async createSourceFolder(payload) {
            const invoke = tauriInvoke();
            if (invoke) {
                return invoke("create_source_folder", { request: payload });
            }
            if (!serverBackendAvailable()) {
                throw backendUnavailableError();
            }
            return requireCreatedFolderResult(await fetchJson("/api/create-source-folder", {
                method: "POST",
                headers: { "Content-Type": "application/json; charset=utf-8" },
                body: JSON.stringify(payload),
            }));
        },
        async renameWorkspaceEntry(payload) {
            const invoke = tauriInvoke();
            if (invoke) {
                return invoke("rename_workspace_entry", { request: payload });
            }
            if (!serverBackendAvailable()) {
                throw backendUnavailableError();
            }
            return requireRenamedEntryResult(await fetchJson("/api/rename-workspace-entry", {
                method: "POST",
                headers: { "Content-Type": "application/json; charset=utf-8" },
                body: JSON.stringify(payload),
            }));
        },
        async deleteWorkspaceEntry(payload) {
            const invoke = tauriInvoke();
            if (invoke) {
                return invoke("delete_workspace_entry", { request: payload });
            }
            if (!serverBackendAvailable()) {
                throw backendUnavailableError();
            }
            return requireOkResult(await fetchJson("/api/delete-workspace-entry", {
                method: "POST",
                headers: { "Content-Type": "application/json; charset=utf-8" },
                body: JSON.stringify(payload),
            }), "editor server delete workspace entry");
        },
    };
    const invoke = tauriInvoke();
    if (invoke) {
        const exitListener = tauriListen("puzzlestudio-exit-requested", (event) => {
            try {
                const request = requireDesktopExitRequest(event);
                const completion = desktopExitRequestHandler
                    ? desktopExitRequestHandler(request)
                    : completeDesktopExit({ ...request, accepted: true });
                Promise.resolve(completion).catch((error) => console.error(error));
            }
            catch (error) {
                console.error(error);
            }
        });
        if (!exitListener) {
            console.error("Desktop exit events are unavailable.");
        }
        else {
            Promise.resolve(exitListener).catch((error) => console.error(error));
        }
    }
})();
//# sourceMappingURL=editor_boot.js.map