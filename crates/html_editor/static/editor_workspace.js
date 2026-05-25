// Workspace store / file tree boundary.
//
// Owns the editor's in-browser workspace model: file tree, documents, active
// document, open tabs, persisted local workspace state, path normalization,
// workspace asset lookup, and explorer tree operations. It may call host IO only
// through PuzzleStudioHost and may ask source/preview/tool controllers to refresh
// after a document switch. It must not own pane layout, preview runtime protocol,
// compiler internals, or tool-specific editing state.
const documentStoreKey = "PuzzleStudioFileTree:v4";
const legacyDocumentStoreKey = "PuzzleStudioEditorStore:v1";
let sourceCursorPreviewKey = "";
let sourceTargetRequestId = 0;
let sourceNavigationBackStack = [];
let sourceNavigationForwardStack = [];
let sourceNavigationRestoring = false;
let localSaveTimer = 0;
let fileTree = null;
let documents = [];
let workspaceRoot = "";
let currentDocumentIndex = 0;
let activeFileId = "";
let selectedFolderId = "";
let selectedTreeId = "";
let openTabIds = [];
let draftEntry = null;
let renameEntry = null;
let draggedNodeId = "";

function configureFolderImport() {
  if (!importFolderInput || !importFolderButton || "webkitdirectory" in importFolderInput) {
    return;
  }
  importFolderInput.removeAttribute("webkitdirectory");
  importFolderInput.removeAttribute("directory");
  importFolderInput.accept = ".zip,application/zip,application/x-zip-compressed";
  importFolderButton.title = "Import folder zip";
  importFolderButton.setAttribute("aria-label", "Import folder zip");
  importFolderButton.textContent = "Import folder zip";
}

function configureDesktopHost() {
  if (openProjectMenuButton) {
    openProjectMenuButton.hidden = !isDesktopHost();
  }
}

function isDesktopHost() {
  return window.PuzzleStudioHost.mode() === "tauri";
}

function openProjectActionButtons() {
  return Array.from(document.querySelectorAll("[data-open-project]"));
}

function setOpenProjectButtonsDisabled(disabled) {
  for (const button of openProjectActionButtons()) {
    button.disabled = disabled;
  }
}

async function loadSource() {
  setEditorStatus("Loading", "");
  if (editorSeed) {
    workspaceRoot = editorSeed.workspaceRoot || "";
    const embedded = embeddedDocuments();
    const key = embeddedSeedKey(embedded);
    const stored = loadDocumentStore();
    const useStored = stored?.seedKey === key;
    fileTree = useStored ? treeWithEmbeddedFallbacks(stored.tree, embedded) : treeFromDocuments(embedded);
    syncDocumentsFromTree();
    activeFileId = useStored
      ? stored.activeFileId
      : documents[activeEmbeddedDocumentIndex()]?.id || documents[0]?.id || "";
    openTabIds = useStored ? (stored.openTabIds || []) : [];
    selectedTreeId = activeFileId;
    currentDocumentIndex = activeDocumentIndex();
    renderDocumentSelect();
    loadEmbeddedDocument(currentDocumentIndex);
    runButton.disabled = false;
    runButton.title = "Refresh preview";
    setEditorStatus(useStored ? "Loaded files" : "Preview embedded", "is-ok");
    return;
  }

  const payload = await window.PuzzleStudioHost.loadSource();
  await applyLoadedSourcePayload(payload);
}

async function applyLoadedSourcePayload(payload) {
  workspaceRoot = payload.workspaceRoot || "";
  const sourceDocuments = Array.isArray(payload.documents) && payload.documents.length
    ? payload.documents
    : payload.empty
      ? []
      : [{
        puzzlePath: payload.puzzlePath || "Untitled puzzle",
        source: payload.source || "",
        previewHtml: "",
        gameCss: payload.gameCss || "",
        gameVisualsJs: payload.gameVisualsJs || "",
      }];
  fileTree = treeFromDocuments(sourceDocuments);
  syncDocumentsFromTree();
  activeFileId = documents[0]?.id || "";
  openTabIds = [];
  selectedTreeId = activeFileId;
  currentDocumentIndex = activeDocumentIndex();
  openDocumentTab(activeFileId);
  renderDocumentSelect();
  renderDocumentTabs();
  applyGameCss(payload.gameCss || "");
  applyGameVisuals(payload.gameVisualsJs || "");
  setSourceEditorValue(payload.source || "");
  resetLevelBuilderFromSource();
  if (documents.length) {
    await renderPreview();
  } else {
    latestHtml = "";
    previewExport = null;
    latestPreviewState = null;
    setPreviewDocumentLoaded(false);
    setPreviewFrameHtml(emptyPreviewDocument());
    resetPreviewLog("No project open");
    runButton.disabled = true;
    downloadButton.disabled = true;
    setEditorStatus("Open or create a project", "");
  }
}

async function openProjectFromDesktop() {
  if (!isDesktopHost()) {
    return;
  }
  setOpenProjectButtonsDisabled(true);
  setEditorStatus("Opening project", "");
  try {
    const payload = await window.PuzzleStudioHost.openProject();
    if (payload?.canceled) {
      setEditorStatus("Open canceled", "");
      return;
    }
    await applyLoadedSourcePayload(payload);
    setEditorStatus("Opened project", "is-ok");
  } catch (error) {
    console.error(error);
    setEditorStatus("Open failed", "is-error");
  } finally {
    setOpenProjectButtonsDisabled(false);
  }
}

function embeddedDocuments() {
  const seedDocuments = Array.isArray(editorSeed.documents) ? editorSeed.documents : [];
  if (seedDocuments.length) {
    return seedDocuments.map((document) => normalizeDocument(document));
  }
  return [normalizeDocument({
    puzzlePath: editorSeed.puzzlePath || "Embedded puzzle",
    source: editorSeed.source || "",
    previewHtml: editorSeed.previewHtml || "",
    gameCss: editorSeed.gameCss || "",
    gameVisualsJs: editorSeed.gameVisualsJs || "",
  })];
}

function embeddedSeedKey(seedDocuments) {
  return JSON.stringify((seedDocuments || []).map((document) => [
    document.puzzlePath || "",
    document.source || "",
    document.dataUrl || "",
    document.gameCss || "",
    document.gameVisualsJs || "",
  ]));
}

function normalizeDocument(document, fallback = {}) {
  const path = document.puzzlePath || fallback.puzzlePath || document.name || "Embedded puzzle";
  const editorPath = editorPathForHostPath(path);
  const encoding = document.encoding || (document.dataUrl ? "data_url" : "text");
  return {
    id: document.id || createDocumentId(),
    name: document.name || fileName(editorPath),
    puzzlePath: editorPath,
    encoding,
    mimeType: document.mimeType || mimeTypeForPath(editorPath),
    source: document.source || "",
    dataUrl: document.dataUrl || "",
    previewHtml: document.previewHtml || "",
    gameCss: document.gameCss ?? fallback.gameCss ?? "",
    gameVisualsJs: document.gameVisualsJs ?? fallback.gameVisualsJs ?? "",
  };
}

function editorPathForHostPath(path) {
  const normalized = normalizePath(path);
  const root = normalizePath(workspaceRoot);
  if (!root || !normalized) {
    return normalized;
  }
  if (normalized === root) {
    return fileName(normalized);
  }
  if (normalized.startsWith(`${root}/`)) {
    return normalized.slice(root.length + 1) || fileName(normalized);
  }
  const rootWithoutSlash = root.replace(/^\/+/, "");
  if (root.startsWith("/") && rootWithoutSlash && normalized.startsWith(`${rootWithoutSlash}/`)) {
    return normalized.slice(rootWithoutSlash.length + 1) || fileName(normalized);
  }
  return normalized;
}

function createDocumentId() {
  if (window.crypto?.randomUUID) {
    return window.crypto.randomUUID();
  }
  return `doc-${Date.now()}-${Math.random().toString(36).slice(2)}`;
}

function makeFolder(name, children = []) {
  return {
    id: createDocumentId(),
    kind: "folder",
    name,
    expanded: false,
    children,
  };
}

function makeFile(name, source = "", fallback = {}) {
  return {
    ...normalizeDocument({
      id: createDocumentId(),
      kind: "file",
      name,
      puzzlePath: joinPath(fallback.parentPath || "", name),
      encoding: "text",
      source,
      dataUrl: "",
      previewHtml: "",
      gameCss: fallback.gameCss || "",
      gameVisualsJs: fallback.gameVisualsJs || "",
    }, fallback),
    kind: "file",
  };
}

function treeFromDocuments(sourceDocuments) {
  const root = makeFolder("Files", []);
  for (const document of sourceDocuments) {
    const normalized = normalizeDocument(document);
    const parts = String(normalized.puzzlePath || normalized.name)
      .split(/[\\/]/)
      .filter(Boolean);
    const fileNameValue = parts.pop() || normalized.name || "puzzle.puzzle";
    let folder = root;
    for (const part of parts) {
      folder = childFolder(folder, part);
    }
    folder.children.push({
      ...normalized,
      kind: "file",
      name: fileNameValue,
      puzzlePath: [...parts, fileNameValue].join("/"),
    });
  }
  return root;
}

function treeWithEmbeddedFallbacks(tree, seedDocuments) {
  const fallbackByPath = new Map((seedDocuments || []).map((document) => [
    normalizePath(document.puzzlePath),
    normalizeDocument(document),
  ]));
  return mergeEmbeddedFallbacks(tree, fallbackByPath);
}

function mergeEmbeddedFallbacks(node, fallbackByPath) {
  if (!node || node.kind === "folder") {
    return {
      ...(node || makeFolder("Files", [])),
      children: (node?.children || []).map((child) => mergeEmbeddedFallbacks(child, fallbackByPath)),
    };
  }
  const fallback = fallbackByPath.get(normalizePath(node.puzzlePath));
  if (!fallback) {
    return node;
  }
  return {
    ...node,
    encoding: node.encoding || fallback.encoding,
    mimeType: node.mimeType || fallback.mimeType,
    dataUrl: node.dataUrl || fallback.dataUrl,
    previewHtml: node.previewHtml || fallback.previewHtml,
    gameCss: node.gameCss || fallback.gameCss,
    gameVisualsJs: node.gameVisualsJs || fallback.gameVisualsJs,
  };
}

function childFolder(parent, name) {
  let folder = parent.children.find((child) => child.kind === "folder" && child.name === name);
  if (!folder) {
    folder = makeFolder(name, []);
    parent.children.push(folder);
  }
  return folder;
}

function normalizeTree(node, parentPath = "") {
  if (!node || node.kind !== "folder") {
    return treeFromDocuments([]);
  }
  const folder = makeFolder(node.name || "Files", []);
  folder.id = node.id || createDocumentId();
  folder.expanded = node.expanded === true;
  for (const child of Array.isArray(node.children) ? node.children : []) {
    if (child.kind === "folder") {
      folder.children.push(normalizeTree(child, joinPath(parentPath, child.name || "folder")));
    } else {
      const file = normalizeDocument(child);
      file.kind = "file";
      file.name = child.name || fileName(file.puzzlePath);
      file.puzzlePath = joinPath(parentPath, file.name);
      folder.children.push(file);
    }
  }
  return folder;
}

function syncDocumentsFromTree() {
  documents = [];
  collectFiles(fileTree, "");
}

function collectFiles(node, parentPath) {
  if (!node) {
    return;
  }
  if (node.kind === "file") {
    node.puzzlePath = joinPath(parentPath, node.name || fileName(node.puzzlePath));
    documents.push(node);
    return;
  }
  for (const child of node.children || []) {
    collectFiles(child, node.name === "Files" ? parentPath : joinPath(parentPath, node.name));
  }
}

function joinPath(parent, name) {
  return parent ? `${parent}/${name}` : name;
}

function loadDocumentStore() {
  try {
    const raw = window.localStorage.getItem(documentStoreKey);
    if (!raw) {
      return loadLegacyDocumentStore();
    }
    const parsed = JSON.parse(raw);
    return {
      activeFileId: parsed.activeFileId || "",
      openTabIds: Array.isArray(parsed.openTabIds) ? parsed.openTabIds : [],
      seedKey: parsed.seedKey || "",
      tree: normalizeTree(parsed.tree),
    };
  } catch (error) {
    console.error(error);
    return null;
  }
}

function loadLegacyDocumentStore() {
  try {
    const raw = window.localStorage.getItem(legacyDocumentStoreKey);
    if (!raw) {
      return null;
    }
    const parsed = JSON.parse(raw);
    const storedDocuments = Array.isArray(parsed.documents)
      ? parsed.documents.map((document) => normalizeDocument(document)).filter((document) => document.source)
      : [];
    return {
      activeFileId: parsed.activeDocumentId || "",
      openTabIds: [],
      tree: treeFromDocuments(storedDocuments),
    };
  } catch (error) {
    console.error(error);
    return null;
  }
}

function storeDocument(document) {
  return {
    id: document.id || createDocumentId(),
    puzzlePath: document.puzzlePath || "puzzle.puzzle",
    encoding: document.encoding || "text",
    mimeType: document.mimeType || mimeTypeForPath(document.puzzlePath),
    source: document.source || "",
    dataUrl: document.dataUrl || "",
    previewHtml: "",
    gameCss: document.gameCss || "",
    gameVisualsJs: document.gameVisualsJs || "",
  };
}

function storeTree(node) {
  if (node.kind === "folder") {
    return {
      id: node.id || createDocumentId(),
      kind: "folder",
      name: node.name || "folder",
      expanded: node.expanded !== false,
      children: (node.children || []).map((child) => storeTree(child)),
    };
  }
  return {
    ...storeDocument(node),
    kind: "file",
    name: node.name || fileName(node.puzzlePath),
  };
}

function saveDocumentStore(showStatus = true) {
  persistCurrentDocument();
  try {
    window.localStorage.setItem(documentStoreKey, JSON.stringify({
      version: 1,
      seedKey: editorSeed ? embeddedSeedKey(embeddedDocuments()) : "",
      activeFileId,
      openTabIds,
      tree: storeTree(fileTree),
    }));
    if (showStatus) {
      setEditorStatus("Saved", "is-ok");
    }
  } catch (error) {
    console.error(error);
    if (showStatus) {
      setEditorStatus("Save failed", "is-error");
    }
  }
}

async function saveCurrentDocument(showStatus = true) {
  saveDocumentStore(false);
  const document = activeDocument();
  if (!document || !isTextDocument(document)) {
    if (showStatus) {
      setEditorStatus("Nothing to save", "is-error");
    }
    return;
  }

  if (editorSeed) {
    if (showStatus) {
      setEditorStatus("Saved locally", "is-ok");
    }
    return;
  }

  if (showStatus) {
    setEditorStatus("Saving", "");
  }
  saveButton.disabled = true;
  try {
    await window.PuzzleStudioHost.save({
      source: document.source || "",
      puzzlePath: document.puzzlePath || "",
    });
    if (showStatus) {
      setEditorStatus("Saved file", "is-ok");
    }
  } catch (error) {
    console.error(error);
    if (showStatus) {
      setEditorStatus("Save failed", "is-error");
    }
  } finally {
    saveButton.disabled = false;
  }
}

function activeDocumentIndex() {
  const found = documents.findIndex((document) => document.id === activeFileId);
  return found >= 0 ? found : 0;
}

function activeDocument() {
  return documents[currentDocumentIndex] || null;
}

function editorNavigationLocation() {
  const document = activeDocument();
  return {
    documentId: document?.id || activeFileId || "",
    selectionStart: sourceEditor?.selectionStart || 0,
    selectionEnd: sourceEditor?.selectionEnd || sourceEditor?.selectionStart || 0,
    scrollTop: sourceEditor?.scrollTop || 0,
    scrollLeft: sourceEditor?.scrollLeft || 0,
    previewMode: currentPreviewMode || "play",
    levelIndex: Number.isInteger(activeLevelIndex) ? activeLevelIndex : 0,
  };
}

function sameEditorNavigationLocation(a, b) {
  return Boolean(a && b)
    && a.documentId === b.documentId
    && a.selectionStart === b.selectionStart
    && a.selectionEnd === b.selectionEnd
    && a.scrollTop === b.scrollTop
    && a.scrollLeft === b.scrollLeft
    && a.previewMode === b.previewMode
    && a.levelIndex === b.levelIndex;
}

function pushSourceNavigationHistory() {
  if (sourceNavigationRestoring || !activeDocument()) {
    return;
  }
  const location = editorNavigationLocation();
  if (!sourceNavigationBackStack.length || !sameEditorNavigationLocation(sourceNavigationBackStack.at(-1), location)) {
    sourceNavigationBackStack.push(location);
    if (sourceNavigationBackStack.length > 100) {
      sourceNavigationBackStack.shift();
    }
  }
  sourceNavigationForwardStack = [];
  updateSourceNavigationButtons();
}

function updateSourceNavigationButtons() {
  if (sourceBackButton) {
    sourceBackButton.disabled = sourceNavigationBackStack.length === 0;
  }
  if (sourceForwardButton) {
    sourceForwardButton.disabled = sourceNavigationForwardStack.length === 0;
  }
}

function restoreEditorNavigationLocation(location) {
  if (!location?.documentId) {
    return false;
  }
  sourceNavigationRestoring = true;
  try {
    const index = documents.findIndex((document) => document.id === location.documentId);
    if (index < 0) {
      return false;
    }
    if (index !== currentDocumentIndex) {
      persistCurrentDocument();
      loadEmbeddedDocument(index);
    }
    if (location.previewMode && location.previewMode !== currentPreviewMode) {
      setPreviewMode(location.previewMode);
    }
    if ((location.previewMode === "edit" || location.previewMode === "solver") && previewExport?.levels?.length) {
      setActiveLevelIndex(Math.max(0, Math.min(previewExport.levels.length - 1, location.levelIndex || 0)));
      loadLevelFromPreviewState({ requestRender: false });
    }
    const start = Math.max(0, Math.min(sourceEditor.value.length, location.selectionStart || 0));
    const end = Math.max(start, Math.min(sourceEditor.value.length, location.selectionEnd || start));
    sourceEditor.setSelectionRange(start, end);
    sourceEditor.scrollTop = Math.max(0, location.scrollTop || 0);
    sourceEditor.scrollLeft = Math.max(0, location.scrollLeft || 0);
    if (typeof syncSourceHighlightScroll === "function") {
      syncSourceHighlightScroll();
    }
    if (typeof updateSourceMeta === "function") {
      updateSourceMeta();
    }
    sourceEditor.focus({ preventScroll: true });
    return true;
  } finally {
    sourceNavigationRestoring = false;
    updateSourceNavigationButtons();
  }
}

function goSourceNavigationBack() {
  const previous = sourceNavigationBackStack.pop();
  if (!previous) {
    updateSourceNavigationButtons();
    return false;
  }
  sourceNavigationForwardStack.push(editorNavigationLocation());
  const restored = restoreEditorNavigationLocation(previous);
  if (!restored) {
    sourceNavigationForwardStack.pop();
  }
  updateSourceNavigationButtons();
  return restored;
}

function goSourceNavigationForward() {
  const next = sourceNavigationForwardStack.pop();
  if (!next) {
    updateSourceNavigationButtons();
    return false;
  }
  sourceNavigationBackStack.push(editorNavigationLocation());
  const restored = restoreEditorNavigationLocation(next);
  if (!restored) {
    sourceNavigationBackStack.pop();
  }
  updateSourceNavigationButtons();
  return restored;
}

function openDocumentTab(documentId = activeFileId) {
  if (!documentId || !findNode(fileTree, documentId)) {
    return;
  }
  openTabIds = openTabIds.filter((id) => findNode(fileTree, id));
  if (!openTabIds.includes(documentId)) {
    openTabIds.push(documentId);
  }
}

function closeDocumentTab(documentId) {
  openTabIds = openTabIds.filter((id) => id !== documentId);
  if (documentId === activeFileId) {
    const nextId = openTabIds[openTabIds.length - 1] || documents.find((document) => document.id !== documentId)?.id || "";
    if (nextId) {
      persistCurrentDocument();
      activeFileId = nextId;
      selectedTreeId = activeFileId;
      selectedFolderId = findParentFolder(fileTree, activeFileId)?.id || "";
      loadEmbeddedDocument(activeDocumentIndex());
      return;
    }
  }
  renderDocumentTabs();
  saveDocumentStore(false);
}

function renderDocumentTabs() {
  if (!documentTabs) {
    return;
  }
  openTabIds = openTabIds.filter((id) => documents.some((document) => document.id === id));
  documentTabs.replaceChildren();
  for (const documentId of openTabIds) {
    const tabDocument = documents.find((item) => item.id === documentId);
    if (!tabDocument) {
      continue;
    }
    const tab = window.document.createElement("button");
    tab.className = "document-tab";
    tab.type = "button";
    tab.setAttribute("role", "tab");
    tab.dataset.documentTab = documentId;
    tab.classList.toggle("is-active", documentId === activeFileId);
    tab.setAttribute("aria-selected", documentId === activeFileId ? "true" : "false");
    tab.tabIndex = documentId === activeFileId ? 0 : -1;
    tab.title = tabDocument.puzzlePath || tabDocument.name || "";

    const label = window.document.createElement("span");
    label.className = "document-tab-label";
    label.textContent = documentTabDisplayName(tabDocument);
    tab.append(label);

    const close = window.document.createElement("span");
    close.className = "document-tab-close";
    close.textContent = "×";
    close.setAttribute("role", "button");
    close.setAttribute("aria-label", `Close ${label.textContent}`);
    close.dataset.closeTab = documentId;
    tab.append(close);
    documentTabs.append(tab);
  }
  updateDocumentTabScrollState();
  window.requestAnimationFrame(() => {
    scrollActiveDocumentTabIntoView();
    updateDocumentTabScrollState();
  });
}

function documentTabDisplayName(document) {
  const name = document?.name || fileName(document?.puzzlePath) || "";
  return name.endsWith(".puzzle") ? name.slice(0, -".puzzle".length) : name;
}

function updateDocumentTabScrollState() {
  if (!documentTabs) {
    return;
  }
  const maxScroll = Math.max(0, documentTabs.scrollWidth - documentTabs.clientWidth);
  documentTabs.closest(".document-tabbar")?.classList.toggle("has-overflow", maxScroll > 1);
}

function scrollActiveDocumentTabIntoView() {
  if (!documentTabs || !activeFileId) {
    return;
  }
  const activeTab = Array.from(documentTabs.querySelectorAll("[data-document-tab]"))
    .find((tab) => tab.dataset.documentTab === activeFileId);
  if (!activeTab) {
    return;
  }
  const padding = 12;
  const visibleLeft = documentTabs.scrollLeft;
  const visibleRight = visibleLeft + documentTabs.clientWidth;
  const tabLeft = activeTab.offsetLeft;
  const tabRight = tabLeft + activeTab.offsetWidth;
  if (tabLeft < visibleLeft + padding) {
    documentTabs.scrollTo({ left: Math.max(0, tabLeft - padding), behavior: "auto" });
  } else if (tabRight > visibleRight - padding) {
    documentTabs.scrollTo({ left: tabRight - documentTabs.clientWidth + padding, behavior: "auto" });
  }
}

function activateDocumentTab(documentId) {
  if (!documentId || documentId === activeFileId) {
    return;
  }
  persistCurrentDocument();
  saveDocumentStore(false);
  activeFileId = documentId;
  selectedTreeId = activeFileId;
  selectedFolderId = findParentFolder(fileTree, activeFileId)?.id || "";
  loadEmbeddedDocument(activeDocumentIndex());
}

function moveDocumentTabFocus(offset) {
  if (!documentTabs || !openTabIds.length) {
    return;
  }
  const currentIndex = Math.max(0, openTabIds.indexOf(activeFileId));
  const nextIndex = Math.max(0, Math.min(openTabIds.length - 1, currentIndex + offset));
  activateDocumentTab(openTabIds[nextIndex]);
}

function normalizedDocumentTabWheelDelta(event) {
  const raw = Math.abs(event.deltaX) > Math.abs(event.deltaY) ? event.deltaX : event.deltaY;
  if (!raw) {
    return 0;
  }
  const unit = event.deltaMode === 1
    ? 32
    : event.deltaMode === 2
      ? Math.max(160, documentTabs.clientWidth * 0.65)
      : 1;
  const scaled = raw * unit;
  const magnitude = Math.abs(scaled);
  if (magnitude < 1) {
    return 0;
  }
  return Math.sign(scaled) * Math.min(96, Math.max(28, magnitude));
}

function activePreviewDocument() {
  const selected = selectedTreeNode();
  if (selected?.kind === "folder") {
    return previewDocumentForFolder(selected);
  }
  return previewDocumentFor(activeDocument());
}

function previewDocumentForFolder(folder) {
  const dir = folderPath(folder);
  const active = activeDocument();
  if (active && documentPathIsInFolder(active, dir)) {
    const activePreview = previewDocumentFor(active);
    if (activePreview && documentPathIsInFolder(activePreview, dir)) {
      return activePreview;
    }
  }

  const directEntry = preferredPuzzleDocumentForDirectory(dir);
  if (directEntry) {
    return directEntry;
  }
  const inFolder = (document) => documentPathIsInFolder(document, dir);
  const nestedGame = documents
    .filter((item) => inFolder(item) && isPuzzleDocument(item))
    .sort(comparePuzzleEntryDocuments)[0];
  if (nestedGame) {
    return nestedGame;
  }
  return null;
}

function documentPathIsInFolder(document, folderDir) {
  const path = normalizePath(document?.puzzlePath || "");
  const dir = normalizePath(folderDir || "");
  if (!dir) {
    return !!path;
  }
  return path === dir || path.startsWith(`${dir}/`);
}

function previewDocumentFor(document) {
  if (isPuzzleDocument(document) && documentHasGamePrelude(document)) {
    return document;
  }

  let dir = directoryName(document?.puzzlePath || "");
  while (dir) {
    const candidate = preferredPuzzleDocumentForDirectory(dir);
    if (candidate) {
      return candidate;
    }
    const parent = directoryName(dir);
    if (!parent || parent === dir) {
      break;
    }
    dir = parent;
  }

  const seeded = documents.find((item) => item.puzzlePath === editorSeed?.puzzlePath);
  if (seeded) {
    return seeded;
  }
  return documents.find((item) => isPuzzleDocument(item)) || null;
}

function preferredPuzzleDocumentForDirectory(dir) {
  const normalizedDir = normalizePath(dir || "");
  const direct = documents
    .filter((item) =>
      isPuzzleDocument(item)
      && normalizePath(directoryName(item.puzzlePath)) === normalizedDir
      && documentHasGamePrelude(item)
    )
    .sort(comparePuzzleEntryDocuments);
  return direct[0] || null;
}

function documentHasGamePrelude(document) {
  return isPuzzleDocument(document) && sourceHasGamePrelude(sourceForDocument(document));
}

function sourceHasGamePrelude(source) {
  let depth = 0;
  for (const rawLine of String(source || "").split("\n")) {
    const code = rawLine.split("//", 1)[0] || "";
    const trimmed = code.trim();
    if (depth === 0 && /^(title|subtitle|author|homepage)(?:\s|$)/.test(trimmed)) {
      return true;
    }
    for (const ch of code) {
      if (ch === "{") {
        depth += 1;
      } else if (ch === "}") {
        depth = Math.max(0, depth - 1);
      }
    }
  }
  return false;
}

function comparePuzzleEntryDocuments(left, right) {
  const leftDir = directoryName(left?.puzzlePath || "");
  const rightDir = directoryName(right?.puzzlePath || "");
  const leftRank = puzzleEntryRank(left?.puzzlePath || "", leftDir);
  const rightRank = puzzleEntryRank(right?.puzzlePath || "", rightDir);
  return leftRank - rightRank || normalizePath(left.puzzlePath).localeCompare(normalizePath(right.puzzlePath));
}

function puzzleEntryRank(path, dir) {
  const name = fileName(path);
  const folderName = fileName(dir);
  if (name === "game.puzzle") {
    return 0;
  }
  if (folderName && name === `${folderName}.puzzle`) {
    return 1;
  }
  if (name === "main.puzzle") {
    return 2;
  }
  return 3;
}

function activePreviewSource() {
  const document = activePreviewDocument();
  if (!document) {
    return "";
  }
  return document.id === activeDocument()?.id && isTextDocument(document)
    ? sourceEditor.value
    : document.source || "";
}

function scheduleLocalSave() {
  window.clearTimeout(localSaveTimer);
  localSaveTimer = window.setTimeout(() => saveDocumentStore(false), 250);
}

function activeEmbeddedDocumentIndex() {
  const index = Number(editorSeed.activeDocumentIndex);
  if (Number.isFinite(index) && index >= 0 && index < documents.length) {
    return Math.trunc(index);
  }
  const found = documents.findIndex((document) => document.puzzlePath === editorSeed.puzzlePath);
  return found >= 0 ? found : 0;
}

function renderDocumentSelect() {
  documentList.replaceChildren();
  renderTreeNode(fileTree, documentList, 0);
  renderExplorerEmptyState();
  focusDraftInput();
  focusRenameInput();
  renderDocumentTabs();
}

function renderExplorerEmptyState() {
  if (documents.length || draftEntry || !isDesktopHost() || editorSeed) {
    return;
  }
  const empty = document.createElement("div");
  empty.className = "explorer-empty-state";
  empty.setAttribute("role", "none");
  const button = document.createElement("button");
  button.className = "explorer-empty-open-button";
  button.type = "button";
  button.dataset.openProject = "true";
  button.textContent = "Open folder";
  empty.append(button);
  documentList.append(empty);
}

function renderTreeNode(node, parent, depth) {
  if (!node) {
    return;
  }
  if (node.kind === "folder") {
    if (node !== fileTree) {
      const row = document.createElement("div");
      row.className = "tree-row folder-row";
      row.dataset.nodeId = node.id;
      row.dataset.dragId = node.id;
      row.draggable = true;
      row.tabIndex = 0;
      row.style.setProperty("--depth", depth);
      row.setAttribute("role", "treeitem");
      row.setAttribute("aria-expanded", node.expanded === false ? "false" : "true");
      row.setAttribute("aria-selected", node.id === selectedTreeId ? "true" : "false");
      row.classList.toggle("is-selected-folder", node.id === selectedFolderId);
      row.classList.toggle("is-active-tree", node.id === selectedTreeId);
      row.classList.toggle("is-renaming", renameEntry?.nodeId === node.id);
      row.innerHTML = `${folderChevronSvg(node.expanded !== false)}${folderIconSvg()}${treeNameHtml(node)}${treeActionsHtml("folder")}`;
      setTreeName(row, node);
      parent.append(row);
    }
    if (node === fileTree || node.expanded !== false) {
      for (const child of node.children || []) {
        renderTreeNode(child, parent, node === fileTree ? depth : depth + 1);
      }
      renderDraftEntry(node, parent, node === fileTree ? depth : depth + 1);
    }
    return;
  }

  const row = document.createElement("div");
  row.className = "tree-row file-row";
  row.dataset.fileId = node.id;
  row.dataset.dragId = node.id;
  row.draggable = true;
  row.tabIndex = 0;
  row.style.setProperty("--depth", depth);
  row.setAttribute("role", "treeitem");
  row.setAttribute("aria-selected", node.id === selectedTreeId ? "true" : "false");
  row.classList.toggle("is-active", node.id === activeFileId);
  row.classList.toggle("is-active-tree", node.id === selectedTreeId);
  row.classList.toggle("is-renaming", renameEntry?.nodeId === node.id);
  row.innerHTML = `${fileIconSvg()}${treeNameHtml(node)}${treeActionsHtml("file")}`;
  setTreeName(row, node);
  parent.append(row);
}

function folderChevronSvg(expanded) {
  return expanded
    ? `<svg class="tree-chevron" viewBox="0 0 16 16" aria-hidden="true"><path d="M4 6l4 4 4-4"></path></svg>`
    : `<svg class="tree-chevron" viewBox="0 0 16 16" aria-hidden="true"><path d="M6 4l4 4-4 4"></path></svg>`;
}

function folderIconSvg() {
  return `<svg class="tree-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7l-2-2H4a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2z"></path></svg>`;
}

function fileIconSvg() {
  return `<svg class="tree-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path><path d="M14 2v6h6"></path></svg>`;
}

function treeNameHtml(node) {
  return renameEntry?.nodeId === node.id
    ? `<input class="rename-input" data-rename-input spellcheck="false" autocomplete="off">`
    : `<span class="tree-label"></span>`;
}

function setTreeName(row, node) {
  const input = row.querySelector(".rename-input");
  if (input) {
    input.value = node.name || fileName(node.puzzlePath);
    input.addEventListener("click", (event) => event.stopPropagation());
    input.addEventListener("pointerdown", (event) => event.stopPropagation());
    input.addEventListener("keydown", (event) => {
      if (event.key === "Enter") {
        event.preventDefault();
        commitRenameEntry(input.value);
      } else if (event.key === "Escape") {
        renameEntry = null;
        renderDocumentSelect();
      }
    });
    input.addEventListener("blur", () => commitRenameEntry(input.value));
    return;
  }
  row.querySelector(".tree-label").textContent = node.name || fileName(node.puzzlePath);
}

function treeActionsHtml(kind) {
  const label = kind === "folder" ? "Folder actions" : "File actions";
  return `<span class="tree-actions" aria-label="${label}">
    <button class="tree-action-button" type="button" data-tree-action="rename" aria-label="Rename" title="Rename">${renameIconSvg()}</button>
    <button class="tree-action-button" type="button" data-tree-action="delete" aria-label="Delete" title="Delete">${deleteIconSvg()}</button>
  </span>`;
}

function renameIconSvg() {
  return `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 20h9"></path><path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4z"></path></svg>`;
}

function deleteIconSvg() {
  return `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M3 6h18"></path><path d="M8 6V4h8v2"></path><path d="M19 6l-1 14H6L5 6"></path><path d="M10 11v5"></path><path d="M14 11v5"></path></svg>`;
}

function renderDraftEntry(parentFolder, parent, depth) {
  if (!draftEntry || draftEntry.parentId !== parentFolder.id) {
    return;
  }
  const row = document.createElement("form");
  row.className = "tree-row draft-row";
  row.style.setProperty("--depth", depth);
  row.innerHTML = `${draftIconSvg(draftEntry.kind)}<input class="draft-input" spellcheck="false" autocomplete="off">`;
  const input = row.querySelector(".draft-input");
  input.value = draftEntry.name;
  input.dataset.draftInput = "true";
  row.addEventListener("submit", (event) => {
    event.preventDefault();
    commitDraftEntry(input.value);
  });
  input.addEventListener("keydown", (event) => {
    if (event.key === "Escape") {
      draftEntry = null;
      renderDocumentSelect();
    }
  });
  input.addEventListener("blur", () => commitDraftEntry(input.value));
  parent.append(row);
}

function draftIconSvg(kind) {
  if (kind === "folder") {
    return `<svg class="tree-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7l-2-2H4a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2z"></path></svg>`;
  }
  return `<svg class="tree-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path><path d="M14 2v6h6"></path></svg>`;
}

function focusDraftInput() {
  const input = documentList.querySelector("[data-draft-input]");
  if (!input) {
    return;
  }
  input.focus();
  input.select();
}

function focusRenameInput() {
  const input = documentList.querySelector("[data-rename-input]");
  if (!input) {
    return;
  }
  input.focus();
  input.select();
}

function fileName(path) {
  return String(path || "Untitled puzzle").split(/[\\/]/).filter(Boolean).pop() || "Untitled puzzle";
}

function extensionName(path) {
  const name = fileName(path).toLowerCase();
  const index = name.lastIndexOf(".");
  return index >= 0 ? name.slice(index + 1) : "";
}

function directoryName(path) {
  const parts = String(path || "").split(/[\\/]/).filter(Boolean);
  parts.pop();
  return parts.join("/");
}

function isPuzzleDocument(document) {
  return extensionName(document?.puzzlePath || document?.name) === "puzzle";
}

function isTextDocument(document) {
  return (document?.encoding || "text") !== "data_url";
}

function isTextFileName(name, mimeType = "") {
  const ext = extensionName(name);
  return [
    "puzzle", "css", "js", "mjs", "json", "svg", "txt", "md", "html", "xml", "csv", "tsv",
  ].includes(ext) || String(mimeType || "").startsWith("text/");
}

function isZipFileName(name, mimeType = "") {
  return extensionName(name) === "zip" || /(?:^|\/)(?:x-)?zip(?:$|-)/i.test(String(mimeType || ""));
}

function mimeTypeForPath(path) {
  const ext = extensionName(path);
  return {
    css: "text/css",
    gif: "image/gif",
    html: "text/html",
    jpeg: "image/jpeg",
    jpg: "image/jpeg",
    js: "text/javascript",
    json: "application/json",
    mjs: "text/javascript",
    mp3: "audio/mpeg",
    ogg: "audio/ogg",
    png: "image/png",
    puzzle: "text/plain",
    svg: "image/svg+xml",
    txt: "text/plain",
    wav: "audio/wav",
    webp: "image/webp",
  }[ext] || "application/octet-stream";
}

function workspaceAssetMap() {
  const assets = new Map();
  for (const document of documents) {
    if (!document?.puzzlePath) {
      continue;
    }
    assets.set(normalizePath(document.puzzlePath), document);
  }
  return assets;
}

function normalizePath(path) {
  return String(path || "").replaceAll("\\", "/").replace(/^\.\/+/, "");
}

function assetUrlForPath(path, baseDir = "") {
  const normalized = normalizePath(path);
  const fullPath = normalizePath(baseDir ? joinPath(baseDir, normalized) : normalized);
  const asset = workspaceAssetMap().get(fullPath) || workspaceAssetMap().get(normalized);
  if (!asset) {
    return "";
  }
  if (asset.encoding === "data_url") {
    return asset.dataUrl || "";
  }
  return `data:${asset.mimeType || mimeTypeForPath(asset.puzzlePath)};charset=utf-8,${encodeURIComponent(asset.source || "")}`;
}

function assetResolverScript(baseDir = "") {
  const entries = {};
  for (const document of documents) {
    if (isPuzzleDocument(document)) {
      continue;
    }
    const url = assetUrlForPath(document.puzzlePath);
    if (url) {
      const normalizedPath = normalizePath(document.puzzlePath);
      const normalizedBase = normalizePath(baseDir);
      entries[normalizedPath] = url;
      if (normalizedBase && normalizedPath.startsWith(`${normalizedBase}/`)) {
        entries[normalizedPath.slice(normalizedBase.length + 1)] = url;
      } else if (directoryName(document.puzzlePath) === baseDir) {
        entries[fileName(document.puzzlePath)] = url;
      }
    }
  }
  return `window.PuzzleAssets = { files: ${JSON.stringify(entries)}, url(path) { return this.files[String(path || "").replaceAll("\\\\", "/")] || String(path || ""); } };`;
}

function rewriteCssAssetUrls(css, baseDir = "") {
  return String(css || "").replace(/url\(([^)]+)\)/g, (match, raw) => {
    const value = raw.trim().replace(/^['"]|['"]$/g, "");
    if (!value || /^(data:|https?:|blob:|#)/i.test(value)) {
      return match;
    }
    const url = assetUrlForPath(value, baseDir);
    return url ? `url("${url}")` : match;
  });
}

function effectiveGameCss(document) {
  const baseDir = directoryName(document.puzzlePath);
  const declaredCssPaths = declaredAssetPaths(document, "css");
  if (!declaredCssPaths.length && document.gameCss) {
    return document.gameCss;
  }
  const parts = [];
  let missingDeclaredAsset = false;
  for (const path of declaredCssPaths) {
    const cssDocument = documentByPath(normalizePath(joinPath(baseDir, path)));
    const source = cssDocument?.source || "";
    if (source) {
      parts.push(rewriteCssAssetUrls(source, directoryName(cssDocument.puzzlePath)));
    } else {
      missingDeclaredAsset = true;
    }
  }
  if (missingDeclaredAsset && document.gameCss) {
    return document.gameCss;
  }
  return parts.filter(Boolean).join("\n");
}

function effectiveThemeCssDocuments(document, themeName) {
  const baseDir = directoryName(document.puzzlePath);
  const safeName = normalizeThemeAssetName(themeName);
  if (!safeName) {
    return [];
  }
  const paths = [
    joinPath(baseDir, `${safeName}.css`),
    joinPath(baseDir, `themes/${safeName}.css`),
  ].map(normalizePath);
  const seen = new Set();
  const out = [];
  for (const path of paths) {
    if (seen.has(path)) {
      continue;
    }
    seen.add(path);
    const css = documentByPath(path);
    if (css && isTextDocument(css)) {
      out.push(css);
    }
  }
  return out;
}

function normalizeThemeAssetName(name) {
  const normalized = String(name || "").trim().toLowerCase().replaceAll("_", "-");
  return /^[a-z][a-z0-9-]*$/.test(normalized) ? normalized : "";
}

function effectiveThemeName(document) {
  const source = document?.source || "";
  let expanded = source;
  try {
    expanded = expandPuzzleImportsForWasm(source, document?.puzzlePath || "game.puzzle");
  } catch {
    expanded = source;
  }
  return themeNameFromPuzzleSource(expanded) || "clean";
}

function themeNameFromPuzzleSource(source) {
  let activeTheme = false;
  let depth = 0;
  let latest = "";
  for (const line of String(source || "").split("\n")) {
    const trimmed = stripLineCommentForWasm(line).trim();
    if (!trimmed) {
      continue;
    }
    const header = trimmed.match(/^theme(?:\s+([A-Za-z][A-Za-z0-9_-]*))?(?:\s*\{)?$/);
    if (header) {
      activeTheme = true;
      depth = trimmed.endsWith("{") ? 1 : 0;
      if (header[1]) {
        latest = header[1];
      }
      continue;
    }
    if (!activeTheme) {
      continue;
    }
    const nameEntry = trimmed.match(/^name\s+([A-Za-z][A-Za-z0-9_-]*)$/);
    if (nameEntry) {
      latest = nameEntry[1];
      continue;
    }
    if (trimmed.endsWith("{")) {
      depth += 1;
    }
    if (trimmed === "end" || trimmed === "}") {
      if (depth <= 1) {
        activeTheme = false;
        depth = 0;
      } else {
        depth -= 1;
      }
    }
  }
  return normalizeThemeAssetName(latest);
}

function effectiveGameVisualsJs(document) {
  if (document.gameVisualsJs) {
    return document.gameVisualsJs;
  }
  const baseDir = directoryName(document.puzzlePath);
  const declaredScriptPaths = declaredAssetPaths(document, "script");
  const scripts = [assetResolverScript(baseDir)];
  for (const path of declaredScriptPaths) {
    const scriptDocument = documentByPath(normalizePath(joinPath(baseDir, path)));
    if (scriptDocument?.source) {
      scripts.push(scriptDocument.source);
    }
  }
  return scripts.filter(Boolean).join("\n");
}

function declaredAssetPaths(document, kind) {
  const source = document?.source || "";
  let expanded = source;
  try {
    expanded = expandPuzzleImportsForWasm(source, document?.puzzlePath || "game.puzzle");
  } catch {
    expanded = source;
  }
  const out = [];
  let inAssets = false;
  let depth = 0;
  for (const line of String(expanded || "").split("\n")) {
    const trimmed = stripLineCommentForWasm(line).trim();
    if (!trimmed) {
      continue;
    }
    if (!inAssets) {
      if (/^assets(?:\s*\{)?$/.test(trimmed)) {
        inAssets = true;
        depth = trimmed.endsWith("{") ? 1 : 0;
      }
      continue;
    }
    if (trimmed === "end" || trimmed === "}") {
      if (depth <= 1) {
        inAssets = false;
        depth = 0;
      } else {
        depth -= 1;
      }
      continue;
    }
    if (trimmed.endsWith("{")) {
      depth += 1;
      continue;
    }
    const entry = trimmed.match(/^([A-Za-z_][A-Za-z0-9_]*)\s+"([^"]+)"$/);
    if (entry && entry[1] === kind) {
      out.push(entry[2]);
    }
  }
  return out;
}

function folderDocument(document, name) {
  const target = normalizePath(joinPath(directoryName(document?.puzzlePath), name));
  return documents.find((candidate) => normalizePath(candidate.puzzlePath) === target) || null;
}

function persistCurrentDocument() {
  const document = documents[currentDocumentIndex];
  if (!document) {
    return;
  }
  if (!isTextDocument(document)) {
    return;
  }
  document.source = sourceEditor.value;
  if (isPuzzleDocument(document)) {
    document.previewHtml = latestHtml;
  }
}

function loadEmbeddedDocument(index) {
  const document = documents[index];
  if (!document) {
    return;
  }
  showWorkPane(SOURCE_WORK_PANE_ID);
  currentDocumentIndex = index;
  activeFileId = document.id;
  selectedTreeId = document.id;
  selectedFolderId = findParentFolder(fileTree, document.id)?.id || selectedFolderId;
  openDocumentTab(document.id);
  renderDocumentSelect();
  renderDocumentTabs();
  const previewDocument = activePreviewDocument();
  applyGameCss(previewDocument ? effectiveGameCss(previewDocument) : "");
  applyGameVisuals(previewDocument ? effectiveGameVisualsJs(previewDocument) : "");
  sourceEditor.readOnly = !isTextDocument(document);
  setSourceEditorValue(isTextDocument(document)
    ? document.source || ""
    : `${document.name || fileName(document.puzzlePath)}\n${document.mimeType || "binary"}\n${document.dataUrl ? `${document.dataUrl.length} bytes encoded` : "No data"}`);
  latestHtml = previewDocument?.previewHtml || "";
  previewExport = extractPreviewExport(latestHtml);
  if (typeof syncPaneModesFromFocusedPuzzleSource === "function") {
    syncPaneModesFromFocusedPuzzleSource({ switchOpenPane: true });
  }
  syncPreviewViewportAspect();
  setPreviewDocumentLoaded(Boolean(latestHtml));
  if (latestHtml) {
    applyPreviewTheme(previewExport?.theme || null);
  }
  setActiveLevelIndex(previewExport?.initialLevelIndex ?? 0);
  latestPreviewState = null;
  resetPreviewLog("Embedded preview loaded");
  if (latestHtml) {
    setPreviewFrameHtml(editorPreviewDocument(latestHtml));
  } else {
    setPreviewFrameHtml(emptyPreviewDocument());
    appendPreviewLog("error", "No embedded preview.");
  }
  downloadButton.disabled = !latestHtml;
  resetLevelBuilderFromPreviewSource();
  if (currentPreviewMode === "level3d" && typeof renderLevel3dBuilder === "function") {
    renderLevel3dBuilder();
  }
}

function loadFolderPreview(folder) {
  persistCurrentDocument();
  const previewDocument = previewDocumentForFolder(folder);
  if (previewDocument) {
    activeFileId = previewDocument.id;
    selectedTreeId = activeFileId;
    selectedFolderId = findParentFolder(fileTree, activeFileId)?.id || folder.id;
    loadEmbeddedDocument(activeDocumentIndex());
    if (!editorSeed) {
      renderPreview();
    }
    return;
  }

  selectedFolderId = folder.id;
  selectedTreeId = folder.id;
  renderDocumentSelect();

  applyGameCss(previewDocument ? effectiveGameCss(previewDocument) : "");
  applyGameVisuals(previewDocument ? effectiveGameVisualsJs(previewDocument) : "");
  latestHtml = previewDocument?.previewHtml || "";
  previewExport = extractPreviewExport(latestHtml);
  syncPreviewViewportAspect();
  setPreviewDocumentLoaded(Boolean(latestHtml));
  if (latestHtml) {
    applyPreviewTheme(previewExport?.theme || null);
  }
  setActiveLevelIndex(previewExport?.initialLevelIndex ?? 0);
  latestPreviewState = null;
  resetPreviewLog(previewDocument
    ? `Folder preview loaded: ${previewDocument.puzzlePath || previewDocument.name || "preview"}`
    : "No preview target");
  if (latestHtml) {
    setPreviewFrameHtml(editorPreviewDocument(latestHtml));
  } else {
    setPreviewFrameHtml(emptyPreviewDocument());
    appendPreviewLog("error", previewDocument ? "No embedded preview." : "No game entry for preview.");
  }
  downloadButton.disabled = !latestHtml;
  updateSourceMeta();
  resetLevelBuilderFromPreviewSource();
  saveDocumentStore(false);
  if (previewDocument && !editorSeed) {
    renderPreview();
  }
}

function previewSelectionIsDetachedFromActiveDocument() {
  const selected = selectedTreeNode();
  const active = activeDocument();
  return selected?.kind === "folder"
    && active
    && !documentPathIsInFolder(active, folderPath(selected));
}

function ensurePreviewTargetsActiveDocument() {
  if (!previewSelectionIsDetachedFromActiveDocument()) {
    return false;
  }
  loadEmbeddedDocument(activeDocumentIndex());
  return true;
}
function hostPathForEditorPath(path) {
  const normalized = normalizePath(path);
  if (!workspaceRoot || normalized.startsWith("/") || /^[A-Za-z]:\//.test(normalized)) {
    return normalized;
  }
  const root = normalizePath(workspaceRoot);
  const rootWithoutSlash = root.replace(/^\/+/, "");
  if (root.startsWith("/") && rootWithoutSlash && (normalized === rootWithoutSlash || normalized.startsWith(`${rootWithoutSlash}/`))) {
    return `/${normalized}`;
  }
  return normalized;
}

function createNewFile() {
  startDraftEntry("file");
}

function createNewFolder() {
  startDraftEntry("folder");
}

function startDraftEntry(kind) {
  const folder = activeFolder();
  folder.expanded = true;
  draftEntry = {
    kind,
    parentId: folder.id,
    name: kind === "folder" ? "folder" : "new.puzzle",
  };
  renderDocumentSelect();
}

function commitDraftEntry(rawName) {
  if (!draftEntry) {
    return;
  }
  const parent = findNode(fileTree, draftEntry.parentId) || fileTree;
  const name = draftEntry.kind === "folder"
    ? sanitizeFileName(rawName)
    : ensurePuzzleExtension(rawName);
  const kind = draftEntry.kind;
  draftEntry = null;
  if (!name) {
    renderDocumentSelect();
    return;
  }
  persistCurrentDocument();
  if (kind === "folder") {
    const folder = makeFolder(uniqueChildName(parent, name), []);
    parent.children.push(folder);
    parent.expanded = true;
    selectedFolderId = folder.id;
    renderDocumentSelect();
    saveDocumentStore(false);
    return;
  }
  const current = documents[currentDocumentIndex] || {};
  const fileNameValue = uniqueChildName(parent, name);
  const file = makeFile(fileNameValue, starterPuzzleSource(fileNameValue), {
    parentPath: folderPath(parent),
    gameCss: current.gameCss || editorSeed?.gameCss || "",
    gameVisualsJs: current.gameVisualsJs || editorSeed?.gameVisualsJs || "",
  });
  parent.children.push(file);
  activeFileId = file.id;
  syncDocumentsFromTree();
  loadEmbeddedDocument(activeDocumentIndex());
  saveDocumentStore(false);
}

function moveNodeToFolder(nodeId, targetFolderId) {
  if (!nodeId) {
    return false;
  }
  const targetFolder = targetFolderId ? findNode(fileTree, targetFolderId) : fileTree;
  const source = findNodeWithParent(fileTree, nodeId);
  if (!source || !source.parent || !source.node || targetFolder?.kind !== "folder") {
    return false;
  }
  if (source.node === fileTree || source.parent === targetFolder) {
    return false;
  }
  if (source.node.kind === "folder" && containsNode(source.node, targetFolder.id)) {
    return false;
  }

  source.parent.children = source.parent.children.filter((child) => child.id !== nodeId);
  source.node.name = uniqueChildName(targetFolder, source.node.name || "item");
  targetFolder.children.push(source.node);
  targetFolder.expanded = true;
  selectedFolderId = targetFolder.id;
  selectedTreeId = source.node.id;
  syncDocumentsFromTree();
  currentDocumentIndex = activeDocumentIndex();
  renderDocumentSelect();
  saveDocumentStore(false);
  return true;
}

function dropFolderIdForEvent(event) {
  const row = event.target.closest(".tree-row");
  if (!row || !documentList.contains(row)) {
    return "";
  }
  if (row.dataset.nodeId) {
    return row.dataset.nodeId;
  }
  if (row.dataset.fileId) {
    return findParentFolder(fileTree, row.dataset.fileId)?.id || "";
  }
  return "";
}

function canDropNodeOnFolder(nodeId, targetFolderId) {
  if (!nodeId) {
    return false;
  }
  const source = findNodeWithParent(fileTree, nodeId);
  const targetFolder = targetFolderId ? findNode(fileTree, targetFolderId) : fileTree;
  if (!source?.node || !source.parent || targetFolder?.kind !== "folder") {
    return false;
  }
  if (source.parent === targetFolder) {
    return false;
  }
  if (source.node.kind === "folder" && containsNode(source.node, targetFolder.id)) {
    return false;
  }
  return true;
}

function markDropTarget(folderId) {
  clearDropTargets();
  const target = folderId
    ? Array.from(documentList.querySelectorAll("[data-node-id]")).find((row) => row.dataset.nodeId === folderId)
    : documentList;
  target?.classList.add("is-drop-target");
}

function clearDropTargets() {
  documentList.classList.remove("is-drop-target");
  documentList.querySelectorAll(".is-drop-target").forEach((row) => row.classList.remove("is-drop-target"));
}

function findNodeWithParent(folder, nodeId, parent = null) {
  if (!folder) {
    return null;
  }
  if (folder.id === nodeId) {
    return { node: folder, parent };
  }
  for (const child of folder.children || []) {
    const found = child.id === nodeId
      ? { node: child, parent: folder }
      : findNodeWithParent(child, nodeId, folder);
    if (found) {
      return found;
    }
  }
  return null;
}

function containsNode(folder, nodeId) {
  if (!folder || folder.kind !== "folder") {
    return false;
  }
  if (folder.id === nodeId) {
    return true;
  }
  return (folder.children || []).some((child) =>
    child.id === nodeId || (child.kind === "folder" && containsNode(child, nodeId)),
  );
}

function selectedTreeNode() {
  return selectedTreeId ? findNode(fileTree, selectedTreeId) : activeDocument();
}

function treeNodeFromRow(row) {
  const nodeId = row?.dataset.nodeId || row?.dataset.fileId || "";
  return nodeId ? findNode(fileTree, nodeId) : null;
}

function startRenameEntry(nodeId) {
  const target = findNodeWithParent(fileTree, nodeId);
  if (!target?.node || target.node === fileTree) {
    return;
  }
  renameEntry = { nodeId };
  draftEntry = null;
  selectedTreeId = nodeId;
  renderDocumentSelect();
}

function commitRenameEntry(value) {
  if (!renameEntry) {
    return;
  }
  persistCurrentDocument();
  const target = findNodeWithParent(fileTree, renameEntry.nodeId);
  if (!target?.node || !target.parent) {
    renameEntry = null;
    renderDocumentSelect();
    return;
  }

  const oldName = target.node.name || fileName(target.node.puzzlePath);
  const cleaned = sanitizeFileName(value) || oldName;
  const nextName = uniqueChildNameExcept(target.parent, cleaned, target.node.id);
  target.node.name = nextName;
  renameEntry = null;
  syncDocumentsFromTree();
  currentDocumentIndex = activeDocumentIndex();
  saveDocumentStore(false);
  renderDocumentSelect();
  setEditorStatus("Renamed", "is-ok");
}

function deleteTreeNode(nodeId) {
  persistCurrentDocument();
  const target = findNodeWithParent(fileTree, nodeId);
  if (!target?.node || !target.parent || target.node === fileTree) {
    return;
  }

  const removedActive = target.node.id === activeFileId
    || (target.node.kind === "folder" && containsNode(target.node, activeFileId));
  target.parent.children = target.parent.children.filter((child) => child.id !== target.node.id);
  renameEntry = null;
  draftEntry = null;
  if (selectedFolderId === target.node.id || (target.node.kind === "folder" && containsNode(target.node, selectedFolderId))) {
    selectedFolderId = target.parent === fileTree ? "" : target.parent.id;
  }
  syncDocumentsFromTree();
  openTabIds = openTabIds.filter((id) => documents.some((document) => document.id === id));
  if (!documents.length) {
    const file = makeFile("new.puzzle", starterPuzzleSource("new.puzzle"));
    fileTree.children.push(file);
    syncDocumentsFromTree();
  }
  if (removedActive || !findNode(fileTree, activeFileId)) {
    activeFileId = documents[0]?.id || "";
  }
  selectedTreeId = activeFileId;
  currentDocumentIndex = activeDocumentIndex();
  saveDocumentStore(false);
  loadEmbeddedDocument(currentDocumentIndex);
  setEditorStatus("Deleted", "is-ok");
}

function starterPuzzleSource(name) {
  const title = name.replace(/\.puzzle$/i, "").replace(/[^\w]+/g, " ").trim() || "New Puzzle";
  return `title ${JSON.stringify(title)}\n\npuzzle main {\n\tlayers {\n\t\tfloor = Goal\n\t\tsolid = Player Wall\n\t}\n\n\tinputs {\n\t\tup <- w ArrowUp\n\t\tdown <- s ArrowDown\n\t\tleft <- a ArrowLeft\n\t\tright <- d ArrowRight\n\t\trestart <- r\n\t}\n\n\twin_conditions {\n\t\texists(Goal)\n\t\tnone([ Goal no Player ])\n\t}\n\n\trules {\n\t\tfor d in directions {\n\t\t\tif input == d {\n\t\t\t\tonce d [ Player | no solid ] -> [ | Player ]\n\t\t\t}\n\t\t}\n\t}\n\n\tlevels {\n\t\tlegend {\n\t\t\t. = empty\n\t\t\t# = Wall\n\t\t\tP = Player\n\t\t\tG = Goal\n\t\t\t+ = Player Goal\n\t\t}\n\n\t\tlevel level_1\n\t\t\t#######\n\t\t\t#P...G#\n\t\t\t#######\n\n\t\tlevel level_2\n\t\t\t#######\n\t\t\t#P....#\n\t\t\t#..G..#\n\t\t\t#######\n\t}\n}\n\nscene playing {\n\tstate {\n\t\tpuzzle main\n\t}\n\tview size 4 3 {\n\t\tcolumn gap 1 align center top {\n\t\t\ttitle\n\t\t\tmain\n\t\t}\n\t}\n\trules {\n\t\tstep main\n\t}\n}\n`;
}

function activeFolder() {
  const selected = selectedTreeNode();
  if (selected?.kind === "folder") {
    return selected;
  }
  const selectedFolder = selectedFolderId ? findNode(fileTree, selectedFolderId) : null;
  if (selectedFolder?.kind === "folder") {
    return selectedFolder;
  }
  const current = documents[currentDocumentIndex];
  return findParentFolder(fileTree, current?.id) || fileTree;
}

function findParentFolder(folder, childId) {
  if (!folder || folder.kind !== "folder") {
    return null;
  }
  for (const child of folder.children || []) {
    if (child.id === childId) {
      return folder;
    }
    if (child.kind === "folder") {
      const found = findParentFolder(child, childId);
      if (found) {
        return found;
      }
    }
  }
  return null;
}

function findNode(folder, nodeId) {
  if (!folder) {
    return null;
  }
  if (folder.id === nodeId) {
    return folder;
  }
  for (const child of folder.children || []) {
    const found = child.id === nodeId ? child : findNode(child, nodeId);
    if (found) {
      return found;
    }
  }
  return null;
}

function folderPath(target) {
  const path = [];
  if (!folderPathVisit(fileTree, target?.id, path)) {
    return "";
  }
  return path.filter((part) => part !== "Files").join("/");
}

function folderPathVisit(node, targetId, path) {
  if (!node) {
    return false;
  }
  if (node.kind === "folder") {
    path.push(node.name);
    if (node.id === targetId) {
      return true;
    }
    for (const child of node.children || []) {
      if (folderPathVisit(child, targetId, path)) {
        return true;
      }
    }
    path.pop();
  }
  return false;
}

function ensurePuzzleExtension(name) {
  const cleaned = sanitizeFileName(name);
  if (!cleaned) {
    return "";
  }
  return cleaned.endsWith(".puzzle") ? cleaned : `${cleaned}.puzzle`;
}

function sanitizeFileName(name) {
  return String(name || "").trim().replace(/[\\/]+/g, "-");
}

function uniqueChildName(folder, name) {
  const existing = new Set((folder.children || []).map((child) => child.name));
  if (!existing.has(name)) {
    return name;
  }
  const dot = name.lastIndexOf(".");
  const base = dot > 0 ? name.slice(0, dot) : name;
  const ext = dot > 0 ? name.slice(dot) : "";
  for (let index = 2; index < 1000; index += 1) {
    const candidate = `${base}-${index}${ext}`;
    if (!existing.has(candidate)) {
      return candidate;
    }
  }
  return `${base}-${Date.now()}${ext}`;
}

function uniqueChildNameExcept(folder, name, ignoredId) {
  const existing = new Set((folder.children || [])
    .filter((child) => child.id !== ignoredId)
    .map((child) => child.name));
  if (!existing.has(name)) {
    return name;
  }
  const dot = name.lastIndexOf(".");
  const base = dot > 0 ? name.slice(0, dot) : name;
  const ext = dot > 0 ? name.slice(dot) : "";
  for (let index = 2; index < 1000; index += 1) {
    const candidate = `${base}-${index}${ext}`;
    if (!existing.has(candidate)) {
      return candidate;
    }
  }
  return `${base}-${Date.now()}${ext}`;
}
function handleSaveShortcut(event) {
  if (!((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "s")) {
    return false;
  }
  event.preventDefault();
  event.stopImmediatePropagation();
  saveCurrentDocument(true).catch((error) => {
    console.error(error);
    setEditorStatus("Save failed", "is-error");
    saveButton.disabled = false;
  });
  return true;
}

function setFileActionsMenuOpen(open) {
  if (!fileActionsButton || !fileActionsMenu) {
    return;
  }
  fileActionsMenu.hidden = !open;
  fileActionsButton.setAttribute("aria-expanded", open ? "true" : "false");
}
