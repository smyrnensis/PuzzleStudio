const assetVersion = new URL(globalThis.location.href).searchParams.get("v") || "";
const versioned = (path) => assetVersion
  ? `${path}?v=${encodeURIComponent(assetVersion)}`
  : path;

let wasmModulePromise = null;
let activeSource = null;
let activeRevision = 0;

function loadWasmModule() {
  if (!wasmModulePromise) {
    wasmModulePromise = import(versioned("./wasm/puzzle_wasm.js")).then(async (module) => {
      if (typeof module.default !== "function") {
        throw new Error("Editor analysis WASM loader is missing its default initializer.");
      }
      await module.default({ module_or_path: versioned("./wasm/puzzle_wasm_bg.wasm") });
      return module;
    });
  }
  return wasmModulePromise;
}

function requiredFunction(module, name) {
  const fn = module?.[name];
  if (typeof fn !== "function") {
    throw new Error(`Editor analysis WASM function is missing: ${name}`);
  }
  return fn;
}

function activateSource(module, source) {
  if (activeSource === source && Number.isInteger(activeRevision) && activeRevision > 0) {
    return activeRevision;
  }
  const revision = requiredFunction(module, "activate_source_analysis")(source);
  if (!Number.isInteger(revision) || revision <= 0) {
    throw new Error(`Editor analysis WASM returned an invalid revision: ${revision}`);
  }
  activeSource = source;
  activeRevision = revision;
  return revision;
}

async function queryAnalysis(request) {
  const module = await loadWasmModule();
  if (request.method === "reset") {
    const source = typeof request.source === "string" ? request.source : "";
    const revision = activateSource(module, source);
    return { revision, sourceLength: source.length };
  }
  if (request.method === "edit") {
    if (activeSource === null || !Number.isInteger(activeRevision) || activeRevision <= 0) {
      throw new Error("Editor analysis document is not initialized before an edit.");
    }
    const changes = Array.isArray(request.changes) ? request.changes.slice() : [];
    changes.sort((left, right) => Number(right.from) - Number(left.from));
    const updates = [];
    for (const change of changes) {
      const from = Number(change.from);
      const to = Number(change.to);
      const insert = typeof change.insert === "string" ? change.insert : "";
      if (!Number.isInteger(from) || !Number.isInteger(to) || from < 0 || from > to || to > activeSource.length) {
        throw new Error("Editor analysis edit has an invalid UTF-16 range.");
      }
      const raw = requiredFunction(module, "apply_source_analysis_edit")(
        activeRevision,
        from,
        to,
        insert,
      );
      const update = JSON.parse(raw || "{}");
      if (!Number.isInteger(update.revision) || update.revision <= activeRevision) {
        throw new Error("Editor analysis edit returned an invalid revision.");
      }
      activeSource = `${activeSource.slice(0, from)}${insert}${activeSource.slice(to)}`;
      activeRevision = update.revision;
      updates.push(update);
    }
    if (activeSource.length !== Number(request.sourceLength)) {
      throw new Error("Editor analysis edit did not produce the active CodeMirror document length.");
    }
    return { revision: activeRevision, sourceLength: activeSource.length, updates };
  }
  if (activeSource === null || !Number.isInteger(activeRevision) || activeRevision <= 0) {
    throw new Error("Editor analysis document is not initialized before a query.");
  }
  if (activeSource.length !== Number(request.sourceLength)) {
    throw new Error("Editor analysis query does not match the active document length.");
  }
  const revision = activeRevision;
  switch (request.method) {
    case "highlightRange":
      return requiredFunction(module, "active_source_analysis_highlight_range_json")(
        revision,
        Number(request.rangeStart),
        Number(request.rangeEnd),
        Boolean(request.includeOutline),
      );
    case "outline":
      return requiredFunction(module, "active_source_analysis_outline_json")(revision);
    case "entries":
      return requiredFunction(module, "active_source_analysis_entries_json")(revision);
    case "completion":
      return requiredFunction(module, "active_source_analysis_suggest_source_completions")(
        revision,
        Number(request.cursorOffset),
      );
    case "target":
      return requiredFunction(module, "active_source_analysis_resolve_source_target")(
        revision,
        Number(request.cursorOffset),
      );
    default:
      throw new Error(`Unknown editor analysis worker method: ${request.method}`);
  }
}

globalThis.onmessage = async (event) => {
  const request = event.data || {};
  const id = Number(request.id);
  if (!Number.isInteger(id) || id <= 0) {
    return;
  }
  try {
    globalThis.postMessage({ id, value: await queryAnalysis(request) });
  } catch (error) {
    globalThis.postMessage({
      id,
      error: error instanceof Error ? error.message : String(error),
    });
  }
};
