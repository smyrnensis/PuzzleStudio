let solverModulePromise = null;
let solverService = null;
const activeSearches = new Map();

async function loadSolver(wasm = {}) {
  if (!solverModulePromise) {
    solverModulePromise = (async () => {
      const module = await import(wasm.moduleUrl);
      await module.default({ module_or_path: wasm.wasmUrl });
      if (typeof module.WasmSolverService !== "function") {
        throw new Error("WASM solver service is unavailable");
      }
      solverService = new module.WasmSolverService();
      return solverService;
    })();
  }
  return solverModulePromise;
}

function solverState(modelKind, source) {
  const state = source && typeof source === "object" ? source : {};
  if (modelKind === "2d") {
    return {
      kind: "2d",
      width: state.width,
      height: state.height,
      layerCount: state.layerCount,
      slots: state.slots,
      variables: state.variables,
      levelFiredRules: state.levelFiredRules,
    };
  }
  if (modelKind === "3d") {
    return {
      kind: "puzzle3d",
      width: state.width,
      depth: state.depth,
      height: state.height,
      layerCount: state.layerCount,
      slots: state.slots,
      variables: state.variables,
      levelFiredRules: state.levelFiredRules,
    };
  }
  throw new Error(`Unsupported solver model kind: ${String(modelKind)}`);
}

function postError(requestId, error) {
  self.postMessage({
    type: "error",
    requestId,
    error: String(error?.message || error),
  });
}

function scheduleAdvance(active) {
  self.setTimeout(() => advanceSearch(active), 0);
}

function advanceSearch(active) {
  if (active.cancelled || activeSearches.get(active.requestId) !== active) {
    return;
  }
  try {
    const response = active.service.advance(
      active.searchId,
      active.maxExpandedNodes,
      Date.now(),
    );
    if (response.observation) {
      self.postMessage({
        type: "progress",
        requestId: active.requestId,
        observation: response.observation,
      });
    }
    if (response.status === "paused") {
      scheduleAdvance(active);
      return;
    }
    activeSearches.delete(active.requestId);
    self.postMessage({
      type: "result",
      requestId: active.requestId,
      solution: response.result,
    });
  } catch (error) {
    activeSearches.delete(active.requestId);
    postError(active.requestId, error);
  }
}

self.onmessage = async (event) => {
  const data = event.data || {};
  const requestId = String(data.requestId || "");
  try {
    const service = await loadSolver(data.wasm || {});
    if (data.type === "prepare") {
      const puzzlePath = String(data.puzzlePath || "").trim();
      if (!puzzlePath) {
        throw new Error("Solver preparation requires an explicit puzzle path");
      }
      const prepared = service.prepare_workspace(
        puzzlePath,
        Array.isArray(data.documents) ? data.documents : [],
        Date.now(),
      );
      if (data.displayed === true) {
        service.pin_artifact(prepared.artifactId, Date.now());
      }
      self.postMessage({ type: "prepared", requestId, ...prepared });
      return;
    }
    if (data.type === "display") {
      service.pin_artifact(data.artifactId ? String(data.artifactId) : undefined, Date.now());
      return;
    }
    if (data.type === "materialize") {
      const state = service.materialize_state(
        String(data.artifactId || ""),
        Number(data.levelIndex),
        solverState(String(data.modelKind || ""), data.state),
        data.materializeLevelStart === true,
        Date.now(),
      );
      self.postMessage({ type: "materialized", requestId, state });
      return;
    }
    if (data.type === "cancel") {
      const active = activeSearches.get(requestId);
      if (!active) {
        return;
      }
      active.cancelled = true;
      activeSearches.delete(requestId);
      service.cancel(active.searchId, Date.now());
      self.postMessage({
        type: "result",
        requestId,
        solution: { result: "cancelled" },
      });
      return;
    }
    if (data.type !== "solve") {
      return;
    }
    if (activeSearches.has(requestId)) {
      throw new Error(`Solver request ${requestId} is already active`);
    }
    const request = data.request || {};
    const searchId = service.start(
      String(data.artifactId || ""),
      {
        levelIndex: Number(request.levelIndex),
        state: solverState(String(data.modelKind || ""), request.state),
        materializeLevelStart: request.materializeLevelStart === true,
        maxDepth: Number(request.maxDepth),
        maxNodes: Number(request.maxNodes),
      },
      Date.now(),
    );
    const active = {
      requestId,
      searchId,
      service,
      cancelled: false,
      maxExpandedNodes: 64,
    };
    activeSearches.set(requestId, active);
    scheduleAdvance(active);
  } catch (error) {
    postError(requestId, error);
  }
};
