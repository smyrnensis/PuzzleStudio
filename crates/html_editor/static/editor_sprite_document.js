function spritePaletteEntryBindInfo(entry) {
  const bind = entry?.bind ?? entry?.bound ?? entry?.sourceRef ?? null;
  if (!bind) return { available: true, linked: false, name: "", label: "Unlinked color" };
  if (typeof bind === "string") {
    return { available: true, linked: true, name: bind, label: `Bound to ${bind}` };
  }
  if (typeof bind === "object") {
    const name = bind.name || bind.ref || bind.source || bind.color || "";
    const linked = !(bind.linked === false || bind.unlinked === true || bind.detached === true);
    return { available: true, linked, name, label: name ? `Bound to ${name}` : "Bound color" };
  }
  return { available: true, linked: true, name: "", label: "Bound color" };
}

function spriteAssetBindInfo(bind, label) {
  if (!bind) return { linked: false, name: "", label: `Unlinked ${label}` };
  if (typeof bind === "string") return { linked: true, name: bind, label: `Bound to ${bind}` };
  const name = bind.name || bind.ref || bind.source || "";
  const linked = !(bind.linked === false || bind.unlinked === true || bind.detached === true);
  return { linked, name, label: name ? `Bound to ${name}` : `Bound ${label}` };
}

function spriteEditorOwnedDocument(state, { allowActive = false } = {}) {
  const owned = state?.editDocumentId
    ? documents.find((candidate) => candidate.id === state.editDocumentId)
    : null;
  if (owned && isTextDocument(owned) && isPuzzleDocument(owned)) return owned;
  if (!allowActive) return null;
  const active = activeDocument();
  return active && isTextDocument(active) && isPuzzleDocument(active) ? active : null;
}

function spriteEditorSourceSnapshot(state, options = {}) {
  const document = spriteEditorOwnedDocument(state, options);
  if (!document) return { document: null, source: "" };
  return {
    document,
    source: document.id === activeDocument()?.id ? sourceEditorDocumentValue() : document.source || "",
  };
}

function setSpriteEditorSourceTarget(state, target, document = activeDocument()) {
  state.editDocumentId = document && isTextDocument(document) && isPuzzleDocument(document)
    ? document.id
    : null;
  state.editSourceStart = Number.isInteger(target?.start) ? target.start : null;
  state.editSourceEnd = Number.isInteger(target?.end) ? target.end : null;
  state.editSourceBodyStart = Number.isInteger(target?.bodyStart) ? target.bodyStart : null;
  state.editSourceBodyEnd = Number.isInteger(target?.bodyEnd) ? target.bodyEnd : null;
  state.editSourceName = target?.name || "";
  state.sourceSpriteContract = target?.sourceSprite && typeof target.sourceSprite === "object"
    ? cloneVisualEditValue(target.sourceSprite)
    : null;
}

function clearSpriteEditorSourceTarget(state) {
  setSpriteEditorSourceTarget(state, null, null);
}

function invalidateSpriteEditorSourceTarget(state, document = activeDocument()) {
  if (!document || !state?.editDocumentId || document.id !== state.editDocumentId) return false;
  clearSpriteEditorSourceTarget(state);
  return true;
}

function spriteEditorSourceRange(state, source, indentForSource) {
  const start = state?.editSourceStart;
  const end = state?.editSourceEnd;
  if (!Number.isInteger(start) || !Number.isInteger(end) || start < 0 || end < start
    || end > String(source || "").length) return null;
  return {
    start,
    end,
    indent: indentForSource(source.slice(source.lastIndexOf("\n", start - 1) + 1, start)),
  };
}

async function commitSpriteEditorMutation({ state, request, allowActiveDocument = false }) {
  const { document, source } = spriteEditorSourceSnapshot(state, { allowActive: allowActiveDocument });
  if (!document) throw new Error("No puzzle source document is owned by this sprite editor.");
  const result = await mutateSpriteSourceFromRust(source, request(source, document));
  document.source = result.source;
  if (document.id === activeDocument()?.id) {
    setSourceEditorValue(result.source, { resetUndo: false });
    revealSpriteSourceResult(document, result);
  }
  scheduleLocalSave();
  schedulePreview();
  setSpriteEditorSourceTarget(state, { start: result.start, end: result.end, name: result.name }, document);
  sourceEditor.focus({ preventScroll: true });
  return { document, result };
}

function projectSpriteDocumentContract(contract) {
  if (!contract || typeof contract !== "object") return null;
  const dimension = contract.dimension === "2d" || contract.dimension === "3d"
    ? contract.dimension
    : null;
  const width = Number(contract?.extent?.width);
  const height = Number(contract?.extent?.height);
  const depth = Number(contract?.extent?.depth);
  const resolvedPalette = Array.isArray(contract.resolvedPalette) ? contract.resolvedPalette : [];
  const frames = Array.isArray(contract.frames) ? contract.frames : [];
  if (!dimension || contract.status !== "complete" || !Number.isInteger(width) || !Number.isInteger(height)
    || !Number.isInteger(depth) || width < 1 || height < 1 || depth < 1 || !resolvedPalette.length || !frames.length) {
    return null;
  }
  const layerCellCount = width * height;
  const cellsByFrame = frames.map((frame) => {
    if (!Array.isArray(frame?.layers) || frame.layers.length !== depth) return null;
    const layers = frame.layers.map((layer) => {
      if (!Array.isArray(layer?.cells) || layer.cells.length !== layerCellCount) return null;
      return layer.cells.map((cell) => cell === null
        || (Number.isInteger(cell) && cell >= 0 && cell < resolvedPalette.length)
        ? cell
        : NaN);
    });
    if (layers.some((layer) => !layer || layer.some(Number.isNaN))) return null;
    return layers;
  });
  if (cellsByFrame.some((frame) => !frame)) return null;
  return {
    dimension,
    extent: { width, height, depth },
    paletteTokens: Array.isArray(contract.paletteTokens) ? contract.paletteTokens : [],
    resolvedPalette,
    shapeRef: typeof contract.shapeRef === "string" ? contract.shapeRef : null,
    durationMs: Number.isFinite(contract.durationMs) ? contract.durationMs : null,
    frameDurationMs: Number.isFinite(contract.frameDurationMs) ? contract.frameDurationMs : null,
    spatialOps: Array.isArray(contract.spatialOps) ? contract.spatialOps : [],
    cellsByFrame,
  };
}
