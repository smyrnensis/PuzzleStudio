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
const explorerSectionStoreKey = "PuzzleStudioExplorerSections:v1";
let sourceCursorPreviewKey = "";
let sourceTargetRequestId = 0;
let sourceCursorResolveSignature = null;
let sourceCursorResolveRegion = null;
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
let treeRowByNodeId = new Map();
let treeDragDecisionCache = null;
let currentDropTargetId = null;
let currentDropTargetElement = null;
let workspaceChangeUnlisten = null;
let workspaceHostMutationDepth = 0;
let deferredWorkspaceChangedPayload = null;
let recentWorkspaces = [];
let explorerFilesCollapsed = false;
let explorerOutlineCollapsed = false;
let explorerOutlineHeight = "35%";
let draggingOutlineSplitter = false;
let draggingOutlineSplitterPointerId = null;

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
  const desktop = isDesktopHost();
  if (openFileMenuButton) {
    openFileMenuButton.hidden = !desktop;
  }
  if (openProjectMenuButton) {
    openProjectMenuButton.hidden = !desktop;
    openProjectMenuButton.textContent = "Open folder";
  }
  if (importButton) {
    importButton.hidden = desktop;
  }
  if (importFolderButton) {
    importFolderButton.hidden = desktop;
  }
  renderRecentWorkspaceMenu();
  updateFileCreationAvailability();
  configureWorkspaceChangeListener();
  refreshRecentWorkspaces();
}

function isDesktopHost() {
  return window.PuzzleStudioHost.mode() === "tauri";
}

function openProjectActionButtons() {
  return Array.from(document.querySelectorAll("[data-open-project], [data-open-workspace], [data-open-recent-workspace]"));
}

function setOpenProjectButtonsDisabled(disabled) {
  for (const button of openProjectActionButtons()) {
    button.disabled = disabled;
  }
}

function updateFileCreationAvailability() {
  const disabled = isDesktopHost() && !hasWritableWorkspace();
  if (newDocumentButton) {
    newDocumentButton.disabled = disabled;
    newDocumentButton.title = disabled ? "Open a workspace before creating files" : "New puzzle";
  }
  if (newFolderButton) {
    newFolderButton.disabled = disabled;
  }
}

function hasWritableWorkspace() {
  if (!isDesktopHost()) {
    return true;
  }
  if (documents.length) {
    return true;
  }
  return Boolean(fileTree?.children?.some((child) => child?.kind === "folder" && child.isWorkspaceRoot));
}

function configureWorkspaceChangeListener() {
  if (!isDesktopHost() || workspaceChangeUnlisten || typeof window.PuzzleStudioHost.listenWorkspaceChanged !== "function") {
    return;
  }
  window.PuzzleStudioHost.listenWorkspaceChanged((payload) => {
    if (workspaceHostMutationDepth > 0) {
      deferredWorkspaceChangedPayload = payload;
      return;
    }
    applyWorkspaceChangedPayload(payload).catch((error) => {
      console.error(error);
      setEditorStatus(externalReloadErrorMessage(error), "is-error");
    });
  }).then((unlisten) => {
    workspaceChangeUnlisten = typeof unlisten === "function" ? unlisten : null;
  }).catch((error) => {
    console.error(error);
  });
}

function beginWorkspaceHostMutation() {
  workspaceHostMutationDepth += 1;
}

function endWorkspaceHostMutation() {
  workspaceHostMutationDepth = Math.max(0, workspaceHostMutationDepth - 1);
  if (workspaceHostMutationDepth > 0 || !deferredWorkspaceChangedPayload) {
    return;
  }
  queueMicrotask(() => {
    if (workspaceHostMutationDepth > 0 || !deferredWorkspaceChangedPayload) {
      return;
    }
    const payload = deferredWorkspaceChangedPayload;
    deferredWorkspaceChangedPayload = null;
    applyWorkspaceChangedPayload(payload).catch((error) => {
      console.error(error);
      setEditorStatus(externalReloadErrorMessage(error), "is-error");
    });
  });
}

async function loadSource() {
  setEditorStatus("Loading", "");
  // Start source loading and parser initialization independently, but do not
  // render a document until the synchronous workspace resolver is available.
  const wasmParserLoad = ensureEditorWasmParserLoaded();
  void wasmParserLoad.catch((error) => {
    console.error("Editor WASM parser failed to load", error);
  });
  if (editorSeed) {
    await wasmParserLoad;
    workspaceRoot = editorSeed.workspaceRoot || "";
    const embedded = embeddedDocuments();
    const key = embeddedSeedKey(embedded);
    const stored = loadDocumentStore();
    const useStored = stored?.seedKey === key;
    fileTree = useStored ? stored.tree : treeFromDocuments(embedded);
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
    runButton.title = "Play preview";
    setEditorStatus(useStored ? "Loaded files" : "Preview embedded", "is-ok");
    return;
  }

  const [payload] = await Promise.all([
    window.PuzzleStudioHost.loadSource(),
    wasmParserLoad,
  ]);
  await applyLoadedSourcePayload(payload);
}

async function applyLoadedSourcePayload(payload) {
  applyRecentWorkspacesPayload(payload);
  const workspacePayloads = Array.isArray(payload?.workspaces)
    ? payload.workspaces.filter((workspacePayload) => workspacePayload && !workspacePayload.empty)
    : [];
  if (workspacePayloads.length) {
    await applyLoadedWorkspacePayloads(workspacePayloads, payload);
    return;
  }
  workspaceRoot = payload.workspaceRoot || "";
  const sourceDocuments = Array.isArray(payload.documents)
    ? payload.documents
    : payload.empty
      ? []
      : [{
        puzzlePath: payload.puzzlePath || "Untitled puzzle",
        source: payload.source || "",
        previewHtml: "",
        gameCss: payload.gameCss || "",
      }];
  const sourceFolders = Array.isArray(payload.folders) ? payload.folders : [];
  fileTree = treeFromDocuments([]);
  let workspaceFolder = null;
  if (workspaceRoot) {
    workspaceFolder = workspaceFolderFromDocuments(workspaceRoot, sourceDocuments, sourceFolders);
    fileTree.children.push(workspaceFolder);
  } else {
    fileTree = treeFromDocuments(sourceDocuments, { workspaceRoot, workspaceFolders: sourceFolders });
  }
  syncDocumentsFromTree();
  activeFileId = workspaceRoot
    ? documents.find((document) => document.workspaceRoot === workspaceRoot && isPuzzleDocument(document))?.id
      || documents.find((document) => document.workspaceRoot === workspaceRoot)?.id
      || documents[0]?.id
      || ""
    : documents[0]?.id || "";
  openTabIds = [];
  selectedTreeId = activeFileId || workspaceFolder?.id || "";
  selectedFolderId = activeFileId
    ? findParentFolder(fileTree, activeFileId)?.id || ""
    : workspaceFolder?.id || "";
  currentDocumentIndex = activeDocumentIndex();
  openDocumentTab(activeFileId);
  if (activeFileId) {
    loadEmbeddedDocument(currentDocumentIndex);
    if (!reportWorkspaceRestoreErrors(payload)) {
      setEditorStatus(workspaceRoot ? "Opened workspace" : "Loaded files", "is-ok");
    }
  } else {
    resetEditorForNoOpenProject({
      source: payload.source || "",
      gameCss: payload.gameCss || "",
      status: "Open or create a project",
    });
    reportWorkspaceRestoreErrors(payload);
  }
}

function resetEditorForNoOpenProject(options = {}) {
  workspaceRoot = "";
  activeFileId = "";
  selectedTreeId = "";
  selectedFolderId = "";
  currentDocumentIndex = 0;
  openTabIds = [];
  sourceNavigationBackStack = [];
  sourceNavigationForwardStack = [];
  renderDocumentSelect();
  renderDocumentTabs();
  applyGameCss(options.gameCss || "");
  applyGameVisuals("");
  sourceEditor.readOnly = true;
  setSourceEditorValue(options.source || "");
  resetLevelBuilderFromSource();
  latestHtml = "";
  previewExport = null;
  latestPreviewState = null;
  setPreviewDocumentLoaded(false);
  setPreviewFrameHtml(emptyPreviewDocument());
  resetPreviewLog(options.previewMessage || "No project open");
  runButton.disabled = true;
  downloadButton.disabled = true;
  setEditorStatus(options.status || "Open or create a project", options.statusClass || "");
}

async function applyLoadedWorkspacePayloads(workspacePayloads, payload) {
  fileTree = treeFromDocuments([]);
  documents = [];
  workspaceRoot = "";
  activeFileId = "";
  selectedTreeId = "";
  selectedFolderId = "";
  openTabIds = [];
  sourceNavigationBackStack = [];
  sourceNavigationForwardStack = [];
  for (const workspacePayload of workspacePayloads) {
    await appendLoadedWorkspacePayload({
      ...workspacePayload,
      recentWorkspaces: payload?.recentWorkspaces || workspacePayload.recentWorkspaces,
    }, { activate: false, showStatus: false });
  }
  const activeRoot = workspacePayloads
    .find((workspacePayload) => workspacePayload?.workspaceRoot)
    ?.workspaceRoot || "";
  activateWorkspaceRoot(activeRoot);
  if (!reportWorkspaceRestoreErrors(payload)) {
    setEditorStatus(
      workspacePayloads.length === 1 ? "Opened workspace" : "Opened workspaces",
      "is-ok",
    );
  }
}

async function appendLoadedWorkspacePayload(payload, options = {}) {
  if (!payload || payload.empty) {
    return;
  }
  const activate = options.activate !== false;
  const showStatus = options.showStatus !== false;
  applyRecentWorkspacesPayload(payload);
  const root = payload.workspaceRoot || "";
  workspaceRoot = root || workspaceRoot;
  const sourceDocuments = Array.isArray(payload.documents)
    ? payload.documents
    : [{
      puzzlePath: payload.puzzlePath || "Untitled puzzle",
      workspaceRoot: root,
      source: payload.source || "",
      previewHtml: "",
      gameCss: payload.gameCss || "",
    }];
  const sourceFolders = Array.isArray(payload.folders) ? payload.folders : [];
  if (!fileTree) {
    fileTree = treeFromDocuments([]);
  }
  const workspaceFolder = workspaceFolderFromDocuments(root, sourceDocuments, sourceFolders);
  replaceWorkspaceTree(root, workspaceFolder);
  syncDocumentsFromTree();
  if (activate) {
    activateWorkspaceRoot(root, workspaceFolder);
  } else {
    renderDocumentSelect();
  }
  if (showStatus) {
    setEditorStatus("Opened workspace", "is-ok");
  }
}

function activateWorkspaceRoot(root, workspaceFolder = workspaceRootFolder(root)) {
  const normalizedRoot = normalizePath(root || "");
  workspaceRoot = root || workspaceRoot;
  activeFileId = documents.find((document) =>
    normalizePath(document.workspaceRoot || "") === normalizedRoot && isPuzzleDocument(document))?.id
    || documents.find((document) => normalizePath(document.workspaceRoot || "") === normalizedRoot)?.id
    || documents[0]?.id
    || "";
  selectedTreeId = activeFileId || workspaceFolder?.id || "";
  selectedFolderId = activeFileId
    ? findParentFolder(fileTree, activeFileId)?.id || workspaceFolder?.id || ""
    : workspaceFolder?.id || "";
  currentDocumentIndex = activeDocumentIndex();
  openDocumentTab(activeFileId);
  if (activeFileId) {
    loadEmbeddedDocument(currentDocumentIndex);
    return true;
  }
  renderDocumentSelect();
  latestHtml = "";
  previewExport = null;
  latestPreviewState = null;
  setPreviewDocumentLoaded(false);
  setPreviewFrameHtml(emptyPreviewDocument());
  resetPreviewLog("No game entry for preview");
  runButton.disabled = true;
  downloadButton.disabled = true;
  return false;
}

function reportWorkspaceRestoreErrors(payload) {
  const errors = Array.isArray(payload?.restoreErrors) ? payload.restoreErrors : [];
  if (!errors.length) {
    return false;
  }
  console.error("Workspace restore failed", errors);
  setEditorStatus(
    errors.length === 1 ? "Workspace restore failed" : "Some workspaces failed to restore",
    "is-error",
  );
  return true;
}

function workspaceFolderFromDocuments(root, sourceDocuments, sourceFolders = []) {
  const workspaceTree = treeFromDocuments(sourceDocuments, {
    workspaceRoot: root,
    workspaceFolders: sourceFolders,
  });
  const workspaceFolder = makeFolder(workspaceFolderNameForRoot(root), workspaceTree.children, {
    workspaceRoot: root,
    isWorkspaceRoot: true,
  });
  workspaceFolder.expanded = true;
  return workspaceFolder;
}

function applyRecentWorkspacesPayload(payload) {
  if (!Array.isArray(payload?.recentWorkspaces)) {
    return;
  }
  setRecentWorkspaces(payload.recentWorkspaces);
}

function setRecentWorkspaces(entries) {
  recentWorkspaces = (Array.isArray(entries) ? entries : [])
    .filter((entry) => entry && entry.workspaceRoot)
    .slice(0, 8);
  renderRecentWorkspaceMenu();
  if (fileTree && !documents.length) {
    renderDocumentSelect();
  }
}

async function refreshRecentWorkspaces() {
  if (!isDesktopHost() || typeof window.PuzzleStudioHost.recentWorkspaces !== "function") {
    return;
  }
  try {
    setRecentWorkspaces(await window.PuzzleStudioHost.recentWorkspaces());
  } catch (error) {
    console.error(error);
  }
}

async function openRecentWorkspaceFromDesktop(workspaceRoot) {
  if (!isDesktopHost() || !workspaceRoot) {
    return;
  }
  setOpenProjectButtonsDisabled(true);
  setEditorStatus("Opening recent folder", "");
  try {
    const payload = await window.PuzzleStudioHost.openRecentWorkspace({ workspaceRoot });
    await appendLoadedWorkspacePayload(payload);
  } catch (error) {
    console.error(error);
    setEditorStatus("Open recent failed", "is-error");
    refreshRecentWorkspaces();
  } finally {
    setOpenProjectButtonsDisabled(false);
  }
}

function renderRecentWorkspaceMenu() {
  const existing = fileActionsMenu?.querySelector("[data-recent-workspaces-section]");
  existing?.remove();
  if (!fileActionsMenu || !isDesktopHost() || !recentWorkspaces.length) {
    return;
  }
  const section = document.createElement("div");
  section.className = "file-actions-section";
  section.dataset.recentWorkspacesSection = "true";
  const heading = document.createElement("div");
  heading.className = "file-actions-menu-heading";
  heading.textContent = "Recent folders";
  section.append(heading);
  for (const entry of recentWorkspaces.slice(0, 6)) {
    const button = document.createElement("button");
    button.type = "button";
    button.setAttribute("role", "menuitem");
    button.dataset.openRecentWorkspace = entry.workspaceRoot;
    button.title = entry.workspaceRoot;
    button.textContent = entry.name || entry.workspaceRoot;
    section.append(button);
  }
  if (openProjectMenuButton) {
    openProjectMenuButton.after(section);
  } else {
    fileActionsMenu.prepend(section);
  }
}

function uniqueWorkspaceFolderName(root) {
  const base = fileName(root) || "Workspace";
  if (!fileTree?.children?.some((child) => child.name === base)) {
    return base;
  }
  return uniqueChildName(fileTree, base);
}

function workspaceFolderNameForRoot(root) {
  return workspaceRootFolder(root)?.name || uniqueWorkspaceFolderName(root);
}

function replaceWorkspaceTree(root, workspaceFolder) {
  if (!fileTree?.children) {
    throw new Error("workspace tree is unavailable");
  }
  const normalizedRoot = normalizePath(root || workspaceFolder?.workspaceRoot || "");
  if (!normalizedRoot) {
    throw new Error("workspace root is required");
  }
  const index = fileTree.children.findIndex((child) =>
    child.kind === "folder"
    && child.isWorkspaceRoot
    && normalizePath(child.workspaceRoot || "") === normalizedRoot
  );
  if (index >= 0) {
    applyPreservedFolderExpansion(fileTree.children[index], workspaceFolder);
    fileTree.children.splice(index, 1, workspaceFolder);
    return;
  }
  fileTree.children.push(workspaceFolder);
}

function applyPreservedFolderExpansion(previousFolder, nextFolder) {
  const expandedByPath = new Map();
  collectFolderExpansion(previousFolder, "", expandedByPath);
  restoreFolderExpansion(nextFolder, "", expandedByPath);
}

function collectFolderExpansion(folder, parentPath, expandedByPath) {
  if (!folder || folder.kind !== "folder") {
    return;
  }
  const path = folderExpansionPath(folder, parentPath);
  expandedByPath.set(folderExpansionKey(folder, path), folder.expanded !== false);
  for (const child of folder.children || []) {
    if (child.kind === "folder") {
      collectFolderExpansion(child, path, expandedByPath);
    }
  }
}

function restoreFolderExpansion(folder, parentPath, expandedByPath) {
  if (!folder || folder.kind !== "folder") {
    return;
  }
  const path = folderExpansionPath(folder, parentPath);
  const key = folderExpansionKey(folder, path);
  if (expandedByPath.has(key)) {
    folder.expanded = expandedByPath.get(key);
  }
  for (const child of folder.children || []) {
    if (child.kind === "folder") {
      restoreFolderExpansion(child, path, expandedByPath);
    }
  }
}

function folderExpansionPath(folder, parentPath) {
  if (folder.isWorkspaceRoot) {
    return "";
  }
  return normalizePath(parentPath ? joinPath(parentPath, folder.name || "") : folder.name || "");
}

function folderExpansionKey(folder, path) {
  return `${normalizePath(folder.workspaceRoot || "")}\n${normalizePath(path || "")}`;
}

async function applyWorkspaceChangedPayload(payload) {
  if (!payload || !payload.external) {
    return;
  }
  if (payload.error) {
    setEditorStatus(externalReloadErrorMessage(payload.error), "is-error");
    return;
  }
  const root = payload.workspaceRoot || "";
  if (!root) {
    return;
  }
  persistCurrentDocument();
  const previousActive = activeDocument();
  const activeKey = previousActive ? documentIdentityKey(previousActive) : "";
  const previousActiveSource = previousActive && isTextDocument(previousActive)
    ? currentSourceForDocument(previousActive)
    : "";
  const previousByKey = new Map(documents.map((document) => [documentIdentityKey(document), document]));
  const sourceDocuments = Array.isArray(payload.documents) && payload.documents.length
    ? payload.documents
    : [{
      puzzlePath: payload.puzzlePath || "Untitled puzzle",
      workspaceRoot: root,
      source: payload.source || "",
      previewHtml: "",
      gameCss: payload.gameCss || "",
    }];
  const sourceFolders = Array.isArray(payload.folders) ? payload.folders : [];
  let conflicts = 0;
  const mergedDocuments = sourceDocuments.map((document) => {
    const normalized = normalizeDocument(document, { workspaceRoot: root });
    const previous = previousByKey.get(documentIdentityKey(normalized));
    if (!previous) {
      normalized.syncedSource = normalized.source || "";
      return normalized;
    }
    normalized.id = previous.id;
    normalized.sourceFoldedBlockKeys = normalizeSourceFoldedBlockKeys(previous.sourceFoldedBlockKeys);
    if (isTextDocument(normalized) && isTextDocument(previous)) {
      const localSource = currentSourceForDocument(previous);
      const syncedSource = previous.syncedSource ?? previous.source ?? "";
      const externalSource = normalized.source || "";
      if (localSource !== syncedSource) {
        normalized.source = localSource;
        normalized.syncedSource = syncedSource;
        if (externalSource !== syncedSource) {
          normalized.externalDirty = true;
          normalized.externalSource = externalSource;
          conflicts += 1;
        }
      } else {
        normalized.syncedSource = externalSource;
        normalized.externalDirty = false;
        normalized.externalSource = "";
      }
    }
    return normalized;
  });

  if (!fileTree) {
    fileTree = treeFromDocuments([]);
  }
  const workspaceTree = treeFromDocuments(mergedDocuments, {
    workspaceRoot: root,
    workspaceFolders: sourceFolders,
  });
  const workspaceFolder = makeFolder(workspaceFolderNameForRoot(root), workspaceTree.children, {
    workspaceRoot: root,
    isWorkspaceRoot: true,
  });
  workspaceFolder.expanded = true;
  replaceWorkspaceTree(root, workspaceFolder);
  syncDocumentsFromTree();
  openTabIds = openTabIds.filter((id) => documents.some((document) => document.id === id));
  const activeAfter = documents.find((document) => documentIdentityKey(document) === activeKey);
  activeFileId = activeAfter?.id || activeFileId;
  if (!findNode(fileTree, activeFileId)) {
    activeFileId = documents.find((document) => document.workspaceRoot === root && isPuzzleDocument(document))?.id
      || documents.find((document) => document.workspaceRoot === root)?.id
      || documents[0]?.id
      || "";
  }
  selectedTreeId = activeFileId;
  selectedFolderId = activeFileId ? findParentFolder(fileTree, activeFileId)?.id || selectedFolderId : selectedFolderId;
  currentDocumentIndex = activeDocumentIndex();
  await ensureEditorWasmParserLoaded();
  if (activeFileId) {
    const activeAfterReload = activeDocument();
    const preserveActiveView = previousActive
      && activeAfterReload
      && documentIdentityKey(activeAfterReload) === activeKey
      && isTextDocument(previousActive)
      && isTextDocument(activeAfterReload)
      && (activeAfterReload.source || "") === previousActiveSource
      && (activeAfterReload.gameCss || "") === (previousActive.gameCss || "");
    if (preserveActiveView) {
      renderDocumentSelect();
      renderDocumentTabs();
      updateDocumentTabUnsavedStates();
      if (typeof syncPaneModesFromFocusedPuzzleSource === "function") {
        void syncPaneModesFromFocusedPuzzleSource({ switchOpenPane: true, loadFirst: false })
          .catch((error) => setEditorStatus(userFacingRuntimeError(error), "is-error"));
      }
    } else {
      loadEmbeddedDocument(currentDocumentIndex);
    }
  } else {
    renderDocumentSelect();
  }
  saveDocumentStore(false);
  if (conflicts > 0) {
    setEditorStatus("External change held: unsaved edits", "is-error");
  } else {
    setEditorStatus("Reloaded external changes", "is-ok");
  }
}

async function ensureEditorWasmParserLoaded() {
  if (typeof loadWasmCompiler !== "function") {
    throw new Error("Editor WASM parser loader is unavailable.");
  }
  await loadWasmCompiler();
}

function externalReloadErrorMessage(error) {
  const message = String(error?.message || error || "").trim();
  return message ? `External reload failed: ${message}` : "External reload failed";
}

function workspaceMutationErrorMessage(prefix, error) {
  const message = String(error?.message || error || "").trim();
  return message ? `${prefix}: ${message}` : prefix;
}

function documentIdentityKey(document) {
  return `${normalizePath(document?.workspaceRoot || workspaceRoot || "")}\n${normalizePath(document?.puzzlePath || "")}`;
}

function currentSourceForDocument(document) {
  return document?.id === activeDocument()?.id && isTextDocument(document)
    ? sourceEditorDocumentValue()
    : document?.source || "";
}

function documentNeedsContentLoad(document) {
  return Boolean(document && document.contentLoaded === false);
}

async function ensureDocumentContentLoaded(document) {
  if (!document || !documentNeedsContentLoad(document)) {
    return document;
  }
  if (document.contentLoadPromise) {
    await document.contentLoadPromise;
    return document;
  }
  if (editorSeed) {
    throw new Error(`Embedded document is missing content: ${document.puzzlePath || document.name || "document"}`);
  }
  if (typeof window.PuzzleStudioHost?.loadWorkspaceDocument !== "function") {
    throw new Error("Workspace document loading is unavailable.");
  }
  document.contentLoadPromise = window.PuzzleStudioHost.loadWorkspaceDocument({
    puzzlePath: hostPathForEditorPath(document.puzzlePath || document.name || "", document.workspaceRoot || workspaceRoot),
    workspaceRoot: document.workspaceRoot || workspaceRoot || "",
  });
  let payload = null;
  try {
    payload = await document.contentLoadPromise;
  } finally {
    document.contentLoadPromise = null;
  }
  const loaded = normalizeDocument(payload, {
    id: document.id,
    workspaceRoot: document.workspaceRoot || workspaceRoot,
    puzzlePath: document.puzzlePath,
    importedBy: document.importedBy,
    parentGamePath: document.parentGamePath,
  });
  Object.assign(document, {
    ...loaded,
    id: document.id,
    name: document.name || loaded.name,
    puzzlePath: document.puzzlePath || loaded.puzzlePath,
    workspaceRoot: document.workspaceRoot || loaded.workspaceRoot,
    sourceFoldedBlockKeys: normalizeSourceFoldedBlockKeys(document.sourceFoldedBlockKeys),
    externalDirty: false,
    externalSource: "",
  });
  document.contentLoaded = true;
  document.syncedSource = isTextDocument(document) ? document.source || "" : "";
  if (document.id === activeDocument()?.id) {
    sourceEditor.readOnly = !isTextDocument(document);
    const sourceText = isTextDocument(document)
      ? document.source || ""
      : `${document.name || fileName(document.puzzlePath)}\n${document.mimeType || "binary"}\n${document.dataUrl ? `${document.dataUrl.length} bytes encoded` : "No data"}`;
    setSourceEditorValue(sourceText, { preserveUndoOnSameValue: true });
    if (isTextDocument(document)) {
      restoreSourceFoldState(document.sourceFoldedBlockKeys);
    }
    updateDocumentTabUnsavedStates();
  }
  return document;
}

async function ensureWorkspaceDocumentsLoaded(root = workspaceRoot || "") {
  const normalizedRoot = normalizePath(root || workspaceRoot || "");
  for (const document of documents) {
    const documentRoot = normalizePath(document.workspaceRoot || workspaceRoot || "");
    if (normalizedRoot && documentRoot !== normalizedRoot) {
      continue;
    }
    await ensureDocumentContentLoaded(document);
  }
}

async function ensurePreviewDocumentsLoaded(document) {
  if (!document) {
    return;
  }
  const root = document.workspaceRoot || workspaceRoot || "";
  await ensureWorkspaceDocumentsLoaded(root);
  await ensureDeclaredPreviewAssetDocumentsLoaded(document, root);
}

async function ensureDeclaredPreviewAssetDocumentsLoaded(document, root) {
  const baseDir = directoryName(document?.puzzlePath || "");
  const assetDocuments = new Set();
  for (const kind of ["css", "script", "file"]) {
    for (const path of declaredAssetPaths(document, kind)) {
      const asset = documentByPathForWorkspace(normalizePath(joinPath(baseDir, path)), root);
      if (!asset) {
        throw new Error(`declared ${kind} asset not found: ${path}`);
      }
      assetDocuments.add(asset);
    }
  }
  for (const themeDocument of effectiveThemeCssDocuments(document, effectiveThemeName(document))) {
    assetDocuments.add(themeDocument);
  }
  for (const asset of assetDocuments) {
    await ensureDocumentContentLoaded(asset);
  }
  for (const asset of Array.from(assetDocuments)) {
    if (!isTextDocument(asset) || asset.mimeType !== "text/css") {
      continue;
    }
    for (const path of cssAssetPaths(asset.source || "")) {
      const cssAsset = documentByPathForWorkspace(
        normalizePath(joinPath(directoryName(asset.puzzlePath), path)),
        root,
      );
      if (cssAsset) {
        await ensureDocumentContentLoaded(cssAsset);
      }
    }
  }
}

function cssAssetPaths(css) {
  const out = [];
  String(css || "").replace(/url\(([^)]+)\)/g, (_match, raw) => {
    const value = raw.trim().replace(/^['"]|['"]$/g, "");
    if (value && !/^(data:|https?:|blob:|#)/i.test(value)) {
      out.push(value);
    }
    return "";
  });
  return out;
}

function isDocumentUnsaved(document) {
  if (!document || !isTextDocument(document)) {
    return false;
  }
  return currentSourceForDocument(document) !== (document.syncedSource ?? document.source ?? "");
}

function collectTextDocumentsInNode(node, out = []) {
  if (!node) {
    return out;
  }
  if (node.kind === "file") {
    if (isTextDocument(node)) {
      out.push(node);
    }
    return out;
  }
  for (const child of node.children || []) {
    collectTextDocumentsInNode(child, out);
  }
  return out;
}

function unsavedDocumentsInNode(node) {
  return collectTextDocumentsInNode(node).filter((document) => isDocumentUnsaved(document));
}

function confirmRemoveWorkspaceWithUnsavedChanges(node, unsavedDocuments) {
  if (!unsavedDocuments.length) {
    return true;
  }
  const workspaceName = node?.name || "this workspace";
  const fileList = unsavedDocuments
    .slice(0, 4)
    .map((document) => `- ${document.puzzlePath || document.name || "Untitled"}`)
    .join("\n");
  const more = unsavedDocuments.length > 4
    ? `\n- and ${unsavedDocuments.length - 4} more`
    : "";
  return window.confirm(
    `Close ${workspaceName} without saving?\n\nUnsaved changes will be lost:\n${fileList}${more}`,
  );
}

function unsavedWorkspaceDocuments() {
  persistCurrentDocument();
  return collectTextDocumentsInNode(fileTree).filter((document) => isDocumentUnsaved(document));
}

function confirmDesktopExitWithUnsavedChanges(actionLabel) {
  const unsavedDocuments = unsavedWorkspaceDocuments();
  if (!unsavedDocuments.length) {
    return true;
  }
  const fileList = unsavedDocuments
    .slice(0, 4)
    .map((document) => `- ${document.puzzlePath || document.name || "Untitled"}`)
    .join("\n");
  const more = unsavedDocuments.length > 4
    ? `\n- and ${unsavedDocuments.length - 4} more`
    : "";
  return window.confirm(
    `${actionLabel} without saving?\n\nUnsaved changes will be lost:\n${fileList}${more}`,
  );
}

function confirmDeleteWorkspaceEntry(node, options = {}) {
  const name = node?.name || fileName(node?.puzzlePath) || "this entry";
  const kind = node?.kind === "folder" ? "folder" : "file";
  const location = options.fromDisk ? "from disk" : "from this workspace";
  return window.confirm(`Delete ${kind} "${name}" ${location}?\n\nThis cannot be undone.`);
}

async function openProjectFromDesktop(kind = "folder") {
  if (!isDesktopHost()) {
    return;
  }
  setOpenProjectButtonsDisabled(true);
  setEditorStatus(kind === "file" ? "Opening file" : "Opening folder", "");
  try {
    const payload = await window.PuzzleStudioHost.openWorkspace({ kind });
    if (payload?.canceled) {
      setEditorStatus("Open canceled", "");
      return;
    }
    await appendLoadedWorkspacePayload(payload);
  } catch (error) {
    console.error(error);
    setEditorStatus(openWorkspaceErrorMessage(error), "is-error");
  } finally {
    setOpenProjectButtonsDisabled(false);
  }
}

function openWorkspaceErrorMessage(error) {
  const message = String(error?.message || error || "").trim();
  return message ? `Open failed: ${message}` : "Open failed";
}

function embeddedDocuments() {
  const seedDocuments = Array.isArray(editorSeed.documents) ? editorSeed.documents : [];
  if (seedDocuments.length) {
    return seedDocuments.map((document) => normalizeDocument(document));
  }
  return [normalizeDocument({
    puzzlePath: editorSeed.puzzlePath || "Embedded puzzle",
    source: editorSeed.source || "",
    gameCss: editorSeed.gameCss || "",
  })];
}

function embeddedSeedKey(seedDocuments) {
  return JSON.stringify((seedDocuments || []).map((document) => [
    document.puzzlePath || "",
    document.source || "",
    document.dataUrl || "",
    document.gameCss || "",
  ]));
}

function normalizeSourceFoldedBlockKeys(keys) {
  return Array.isArray(keys) ? keys.filter((key) => typeof key === "string" && key) : [];
}

function normalizeDocument(document, fallback = {}) {
  const path = document.puzzlePath || fallback.puzzlePath || document.name || "Embedded puzzle";
  const documentWorkspaceRoot = document.workspaceRoot || fallback.workspaceRoot || workspaceRoot;
  const editorPath = editorPathForHostPath(path, documentWorkspaceRoot);
  const encoding = document.encoding || (document.dataUrl ? "data_url" : "text");
  const hasSourceField = Object.prototype.hasOwnProperty.call(document, "source");
  const hasDataUrlField = Object.prototype.hasOwnProperty.call(document, "dataUrl");
  const contentLoaded = document.contentLoaded === false
    ? false
    : hasSourceField || hasDataUrlField || !documentWorkspaceRoot;
  const importedBy = Array.isArray(document.importedBy)
    ? document.importedBy.map((path) => editorPathForHostPath(path, documentWorkspaceRoot)).filter(Boolean)
    : Array.isArray(fallback.importedBy)
      ? fallback.importedBy.map((path) => editorPathForHostPath(path, documentWorkspaceRoot)).filter(Boolean)
      : [];
  const parentGamePath = document.parentGamePath || fallback.parentGamePath || "";
  return {
    id: document.id || createDocumentId(),
    name: document.name || fileName(editorPath),
    puzzlePath: editorPath,
    workspaceRoot: documentWorkspaceRoot,
    encoding,
    mimeType: document.mimeType || mimeTypeForPath(editorPath),
    source: document.source || "",
    syncedSource: document.syncedSource ?? document.source ?? "",
    dataUrl: document.dataUrl || "",
    contentLoaded,
    declaresGameEntry: document.declaresGameEntry === true,
    previewHtml: "",
    previewError: "",
    gameCss: document.gameCss ?? fallback.gameCss ?? "",
    sourceFoldedBlockKeys: normalizeSourceFoldedBlockKeys(
      document.sourceFoldedBlockKeys ?? fallback.sourceFoldedBlockKeys,
    ),
    importedBy,
    parentGamePath: parentGamePath ? editorPathForHostPath(parentGamePath, documentWorkspaceRoot) : "",
  };
}

function editorPathForHostPath(path, rootOverride = workspaceRoot) {
  const normalized = normalizePath(path);
  const root = normalizePath(rootOverride);
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

function makeFolder(name, children = [], fallback = {}) {
  return {
    id: createDocumentId(),
    kind: "folder",
    name,
    workspaceRoot: fallback.workspaceRoot || "",
    isWorkspaceRoot: fallback.isWorkspaceRoot === true,
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
      workspaceRoot: fallback.workspaceRoot || workspaceRoot,
      encoding: "text",
      source,
      dataUrl: "",
      previewHtml: "",
      gameCss: fallback.gameCss || "",
    }, fallback),
    kind: "file",
  };
}

function treeFromDocuments(sourceDocuments, fallback = {}) {
  const root = makeFolder("Files", []);
  const workspaceFolders = Array.isArray(fallback.workspaceFolders) ? fallback.workspaceFolders : [];
  for (const folderPathValue of workspaceFolders) {
    const editorPath = editorPathForHostPath(folderPathValue, fallback.workspaceRoot || workspaceRoot);
    const parts = String(editorPath).split(/[\\/]/).filter(Boolean);
    let folder = root;
    for (const part of parts) {
      folder = childFolder(folder, part, fallback.workspaceRoot || workspaceRoot);
    }
  }
  for (const document of sourceDocuments) {
    const normalized = normalizeDocument(document, fallback);
    const parts = String(normalized.puzzlePath || normalized.name)
      .split(/[\\/]/)
      .filter(Boolean);
    const fileNameValue = parts.pop() || normalized.name || "puzzle.puzzle";
    let folder = root;
    for (const part of parts) {
      folder = childFolder(folder, part, normalized.workspaceRoot);
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

function childFolder(parent, name, childWorkspaceRoot = "") {
  let folder = parent.children.find((child) => child.kind === "folder" && child.name === name);
  if (!folder) {
    folder = makeFolder(name, [], { workspaceRoot: childWorkspaceRoot || parent.workspaceRoot || "" });
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
  folder.workspaceRoot = node.workspaceRoot || "";
  folder.isWorkspaceRoot = node.isWorkspaceRoot === true;
  for (const child of Array.isArray(node.children) ? node.children : []) {
    if (child.kind === "folder") {
      folder.children.push(normalizeTree(child, joinPath(parentPath, child.name || "folder")));
    } else {
      const file = normalizeDocument(child, { workspaceRoot: folder.workspaceRoot || child.workspaceRoot || "" });
      file.kind = "file";
      file.name = child.name || fileName(file.puzzlePath);
      file.puzzlePath = joinPath(parentPath, file.name);
      file.workspaceRoot = child.workspaceRoot || folder.workspaceRoot || "";
      folder.children.push(file);
    }
  }
  return folder;
}

function syncDocumentsFromTree() {
  resetTreeDragDecisionCache();
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
  const nextParent = node.name === "Files" || node.isWorkspaceRoot
    ? parentPath
    : joinPath(parentPath, node.name);
  for (const child of node.children || []) {
    collectFiles(child, nextParent);
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
    workspaceRoot: document.workspaceRoot || "",
    encoding: document.encoding || "text",
    mimeType: document.mimeType || mimeTypeForPath(document.puzzlePath),
    source: document.source || "",
    dataUrl: document.dataUrl || "",
    contentLoaded: document.contentLoaded !== false,
    declaresGameEntry: document.declaresGameEntry === true,
    previewHtml: "",
    previewError: "",
    gameCss: document.gameCss || "",
    sourceFoldedBlockKeys: normalizeSourceFoldedBlockKeys(document.sourceFoldedBlockKeys),
    importedBy: Array.isArray(document.importedBy) ? document.importedBy : [],
    parentGamePath: document.parentGamePath || "",
  };
}

function storeTree(node) {
  if (node.kind === "folder") {
    return {
      id: node.id || createDocumentId(),
      kind: "folder",
      name: node.name || "folder",
      workspaceRoot: node.workspaceRoot || "",
      isWorkspaceRoot: node.isWorkspaceRoot === true,
      expanded: node.expanded !== false,
      children: (node.children || []).map((child) => storeTree(child)),
    };
  }
  return {
    ...storeDocument(node),
    kind: "file",
    name: node.name || fileName(node.puzzlePath),
    workspaceRoot: node.workspaceRoot || "",
  };
}

function saveDocumentStore(showStatus = true, options = {}) {
  if (options.persistCurrent !== false) {
    persistCurrentDocument();
  }
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
  let document = activeDocument();
  if (!document || !isTextDocument(document)) {
    if (showStatus) {
      setEditorStatus("Nothing to save", "is-error");
    }
    return false;
  }
  if (documentNeedsContentLoad(document)) {
    if (showStatus) {
      setEditorStatus("Loading before save", "");
    }
    saveButton.disabled = true;
    await ensureDocumentContentLoaded(document);
    if (activeDocument()?.id !== document.id) {
      throw new Error("Cannot save after the active document changed during file load.");
    }
    document = activeDocument();
    if (documentNeedsContentLoad(document)) {
      throw new Error(`Cannot save unloaded document: ${document.puzzlePath || document.name || "document"}`);
    }
  }
  saveDocumentStore(false);

  if (editorSeed) {
    document.syncedSource = document.source || "";
    updateDocumentTabUnsavedStates();
    if (showStatus) {
      setEditorStatus("Saved locally", "is-ok");
    }
    return true;
  }

  if (showStatus) {
    setEditorStatus("Saving", "");
  }
  saveButton.disabled = true;
  try {
    if (isDesktopHost()) {
      beginWorkspaceHostMutation();
    }
    try {
      await window.PuzzleStudioHost.save({
        source: document.source || "",
        puzzlePath: document.puzzlePath || "",
        workspaceRoot: document.workspaceRoot || workspaceRoot || "",
        contentLoaded: document.contentLoaded !== false,
      });
    } finally {
      if (isDesktopHost()) {
        endWorkspaceHostMutation();
      }
    }
    document.syncedSource = document.source || "";
    document.externalDirty = false;
    document.externalSource = "";
    updateDocumentTabUnsavedStates();
    if (showStatus) {
      setEditorStatus("Saved file", "is-ok");
    }
    return true;
  } catch (error) {
    console.error(error);
    if (showStatus) {
      setEditorStatus("Save failed", "is-error");
    }
    throw error;
  } finally {
    saveButton.disabled = false;
  }
}

function activeDocumentIndex() {
  const found = documents.findIndex((document) => document.id === activeFileId);
  return found >= 0 ? found : 0;
}

function activeDocument() {
  currentDocumentIndex = activeDocumentIndex();
  return documents[currentDocumentIndex] || null;
}

function editorNavigationLocation() {
  const document = activeDocument();
  const selectionStart = sourceViewOffsetToDocumentOffset(sourceEditor?.selectionStart || 0, "start");
  const selectionEnd = sourceViewOffsetToDocumentOffset(sourceEditor?.selectionEnd || sourceEditor?.selectionStart || 0, "end");
  return {
    documentId: document?.id || activeFileId || "",
    selectionStart,
    selectionEnd,
    scrollTop: sourceScrollTop(),
    scrollLeft: sourceScrollLeft(),
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
    const source = sourceEditorDocumentValue();
    const sourceStart = Math.max(0, Math.min(source.length, location.selectionStart || 0));
    const sourceEnd = Math.max(sourceStart, Math.min(source.length, location.selectionEnd || sourceStart));
    const start = sourceDocumentOffsetToViewOffset(sourceStart, "start");
    const end = sourceDocumentOffsetToViewOffset(sourceEnd, "end");
    sourceEditor.setSelectionRange(start, end);
    setSourceScrollTop(location.scrollTop || 0);
    setSourceScrollLeft(location.scrollLeft || 0);
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
  }
}

function goSourceNavigationBack() {
  const previous = sourceNavigationBackStack.pop();
  if (!previous) {
    return false;
  }
  sourceNavigationForwardStack.push(editorNavigationLocation());
  const restored = restoreEditorNavigationLocation(previous);
  if (!restored) {
    sourceNavigationForwardStack.pop();
  }
  return restored;
}

function goSourceNavigationForward() {
  const next = sourceNavigationForwardStack.pop();
  if (!next) {
    return false;
  }
  sourceNavigationBackStack.push(editorNavigationLocation());
  const restored = restoreEditorNavigationLocation(next);
  if (!restored) {
    sourceNavigationBackStack.pop();
  }
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
    const nextId = openTabIds[openTabIds.length - 1] || "";
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
    tab.classList.toggle("is-unsaved", isDocumentUnsaved(tabDocument));
    tab.setAttribute("aria-selected", documentId === activeFileId ? "true" : "false");
    tab.tabIndex = documentId === activeFileId ? 0 : -1;
    updateDocumentTabTitle(tab, tabDocument);

    const label = window.document.createElement("span");
    label.className = "document-tab-label";
    label.textContent = documentTabDisplayName(tabDocument);
    tab.append(label);

    const unsaved = window.document.createElement("span");
    unsaved.className = "document-tab-unsaved-dot";
    unsaved.setAttribute("aria-hidden", "true");
    tab.append(unsaved);

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
  return name;
}

function updateDocumentTabTitle(tab, document) {
  if (!tab) {
    return;
  }
  const title = document?.puzzlePath || document?.name || "";
  tab.title = isDocumentUnsaved(document) && title ? `${title} (unsaved changes)` : title;
  const label = documentTabDisplayName(document) || title;
  if (label) {
    tab.setAttribute("aria-label", isDocumentUnsaved(document) ? `${label}, unsaved changes` : label);
  }
}

function updateDocumentTabUnsavedStates() {
  if (!documentTabs) {
    return;
  }
  for (const tab of documentTabs.querySelectorAll("[data-document-tab]")) {
    const tabDocument = documents.find((item) => item.id === tab.dataset.documentTab);
    if (!tabDocument) {
      continue;
    }
    tab.classList.toggle("is-unsaved", isDocumentUnsaved(tabDocument));
    updateDocumentTabTitle(tab, tabDocument);
  }
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
    const folderPreview = previewDocumentForFolder(selected);
    if (folderPreview) {
      return folderPreview;
    }
  }
  return previewDocumentFor(activeDocument());
}

function previewDocumentForFolder(folder) {
  const dir = folderPath(folder);
  const active = activeDocument();
  if (active && documentPathIsInFolder(active, dir)) {
    const activePreview = previewDocumentFor(active);
    if (activePreview) {
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
    return previewDocumentFor(nestedGame);
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

function parentGameDocumentForImportFragment(document) {
  if (!isPuzzleDocument(document) || documentDeclaresGameEntry(document)) {
    return null;
  }
  return parentGameCandidatesForDocument(document)[0] || null;
}

function parentGameCandidatesForDocument(document) {
  if (!isPuzzleDocument(document)) {
    return [];
  }
  if (documentDeclaresGameEntry(document)) {
    return [document];
  }
  const targetRoot = normalizePath(document.workspaceRoot || workspaceRoot || "");
  return documents
    .filter((candidate) => {
      if (!isPuzzleDocument(candidate) || !isTextDocument(candidate) || !documentDeclaresGameEntry(candidate)) {
        return false;
      }
      const candidateRoot = normalizePath(candidate.workspaceRoot || workspaceRoot || "");
      if (targetRoot && candidateRoot && targetRoot !== candidateRoot) {
        return false;
      }
      return documentImportClosureContains(candidate, document, new Set());
    })
    .sort(comparePuzzleEntryDocuments);
}

function documentImportClosureContains(candidate, target, visited) {
  if (!candidate || !target || visited.has(candidate.id)) {
    return false;
  }
  visited.add(candidate.id);
  for (const importPath of puzzleImportPathsForDocument(candidate)) {
    const imported = documentByPathForWorkspace(importPath, candidate.workspaceRoot || workspaceRoot || "");
    if (!imported || !isPuzzleDocument(imported) || !isTextDocument(imported)) {
      continue;
    }
    if (imported.id === target.id || documentImportClosureContains(imported, target, visited)) {
      return true;
    }
  }
  return false;
}

function directImportersForDocument(document) {
  if (!isPuzzleDocument(document)) {
    return [];
  }
  const targetRoot = normalizePath(document.workspaceRoot || workspaceRoot || "");
  const importers = [];
  for (const candidate of documents) {
    if (!isPuzzleDocument(candidate) || !isTextDocument(candidate) || candidate.id === document.id) {
      continue;
    }
    const candidateRoot = normalizePath(candidate.workspaceRoot || workspaceRoot || "");
    if (targetRoot && candidateRoot && candidateRoot !== targetRoot) {
      continue;
    }
    const importsTarget = puzzleImportPathsForDocument(candidate).some((importPath) => {
      const imported = documentByPathForWorkspace(importPath, candidate.workspaceRoot || workspaceRoot || "");
      return imported?.id === document.id;
    });
    if (importsTarget) {
      importers.push(candidate);
    }
  }
  return importers.sort(comparePuzzleEntryDocuments);
}

function puzzleImportPathsForDocument(document) {
  if (!isPuzzleDocument(document) || !isTextDocument(document)) {
    return [];
  }
  const baseDir = directoryName(document.puzzlePath || "");
  const paths = [];
  for (const rawLine of String(currentSourceForDocument(document) || "").split("\n")) {
    const code = stripWorkspaceImportLineComment(rawLine).trim();
    const match = code.match(/^import\s+"((?:\\.|[^"\\])*)"\s*$/);
    if (match) {
      paths.push(resolveWorkspaceImportPath(baseDir, match[1]));
    }
  }
  return paths;
}

function stripWorkspaceImportLineComment(line) {
  let quoted = false;
  let escaped = false;
  const text = String(line || "");
  for (let index = 0; index < text.length; index += 1) {
    const char = text[index];
    const next = text[index + 1];
    if (escaped) {
      escaped = false;
      continue;
    }
    if (char === "\\") {
      escaped = true;
      continue;
    }
    if (char === "\"") {
      quoted = !quoted;
      continue;
    }
    if (!quoted && char === "/" && next === "/") {
      return text.slice(0, index);
    }
  }
  return text;
}

function resolveWorkspaceImportPath(baseDir, importPath, root = workspaceRoot || "") {
  const normalized = normalizePath(importPath);
  if (!normalized) {
    return "";
  }
  if (normalized.startsWith("/") || /^[A-Za-z]:\//.test(normalized)) {
    return editorPathForHostPath(normalized, root);
  }
  return normalizeWorkspacePathSegments(baseDir ? `${baseDir}/${normalized}` : normalized);
}

function normalizeWorkspacePathSegments(path) {
  const parts = [];
  for (const part of normalizePath(path).split("/")) {
    if (!part || part === ".") {
      continue;
    }
    if (part === "..") {
      parts.pop();
      continue;
    }
    parts.push(part);
  }
  return parts.join("/");
}

function documentByPathForWorkspace(path, root) {
  const target = normalizePath(path);
  const normalizedRoot = normalizePath(root || "");
  return documents.find((candidate) =>
    normalizePath(candidate.puzzlePath) === target
    && (!normalizedRoot || !candidate.workspaceRoot || normalizePath(candidate.workspaceRoot) === normalizedRoot)
  ) || null;
}

function previewDocumentFor(document) {
  if (isPuzzleDocument(document) && documentDeclaresGameEntry(document)) {
    return document;
  }
  if (isPuzzleDocument(document)) {
    return parentGameDocumentForImportFragment(document);
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
      && documentDeclaresGameEntry(item)
    )
    .sort(comparePuzzleEntryDocuments);
  return direct[0] || null;
}

function documentDeclaresGameEntry(document) {
  if (!isPuzzleDocument(document)) {
    return false;
  }
  if (document.contentLoaded === false) {
    return document.declaresGameEntry === true;
  }
  return sourceDeclaresGameEntry(currentSourceForDocument(document));
}

function sourceDeclaresGameEntry(source) {
  let depth = 0;
  for (const rawLine of String(source || "").split("\n")) {
    const code = rawLine.split("//", 1)[0] || "";
    const trimmed = code.trim();
    if (depth === 0 && /^(puzzle|puzzle3)(?:\s|$)/.test(trimmed)) {
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
  if (name === "game.puzzle3") {
    return 1;
  }
  if (folderName && name === `${folderName}.puzzle`) {
    return 2;
  }
  if (folderName && name === `${folderName}.puzzle3`) {
    return 3;
  }
  if (name === "main.puzzle") {
    return 4;
  }
  if (name === "main.puzzle3") {
    return 5;
  }
  return 6;
}

function activePreviewSource() {
  const document = activePreviewDocument();
  if (!document) {
    return "";
  }
  return document.id === activeDocument()?.id && isTextDocument(document)
    ? sourceEditorDocumentValue()
    : document.source || "";
}

function scheduleLocalSave() {
  updateDocumentTabUnsavedStates();
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
  treeRowByNodeId = new Map();
  clearDropTargets();
  documentList.replaceChildren();
  const importTitleIndex = buildTreeImportTitleIndex();
  renderTreeNode(fileTree, documentList, 0, importTitleIndex);
  renderExplorerEmptyState();
  updateFileCreationAvailability();
  focusDraftInput();
  focusRenameInput();
  renderDocumentTabs();
}

function renderExplorerEmptyState() {
  if (documents.length || draftEntry || !isDesktopHost() || editorSeed || hasWritableWorkspace()) {
    return;
  }
  const empty = document.createElement("div");
  empty.className = "explorer-empty-state";
  empty.setAttribute("role", "none");
  const button = document.createElement("button");
  button.className = "explorer-empty-open-button";
  button.type = "button";
  button.dataset.openWorkspace = "folder";
  button.textContent = "Open folder";
  empty.append(button);
  if (recentWorkspaces.length) {
    const recent = document.createElement("div");
    recent.className = "explorer-empty-recent";
    const heading = document.createElement("div");
    heading.className = "explorer-empty-recent-heading";
    heading.textContent = "Recent folders";
    recent.append(heading);
    for (const entry of recentWorkspaces.slice(0, 5)) {
      const recentButton = document.createElement("button");
      recentButton.className = "explorer-empty-recent-button";
      recentButton.type = "button";
      recentButton.dataset.openRecentWorkspace = entry.workspaceRoot;
      recentButton.title = entry.workspaceRoot;
      recentButton.textContent = entry.name || entry.workspaceRoot;
      recent.append(recentButton);
    }
    empty.append(recent);
  }
  documentList.append(empty);
}

document.addEventListener("click", (event) => {
  const button = event.target.closest("[data-open-recent-workspace]");
  if (!button) {
    return;
  }
  event.preventDefault();
  if (typeof setFileActionsMenuOpen === "function") {
    setFileActionsMenuOpen(false);
  }
  openRecentWorkspaceFromDesktop(button.dataset.openRecentWorkspace).catch((error) => {
    console.error(error);
    setEditorStatus("Open recent failed", "is-error");
    setOpenProjectButtonsDisabled(false);
  });
});

function loadExplorerSectionState() {
  try {
    const parsed = JSON.parse(window.localStorage.getItem(explorerSectionStoreKey) || "{}");
    explorerFilesCollapsed = parsed.filesCollapsed === true;
    explorerOutlineCollapsed = parsed.outlineCollapsed === true;
    explorerOutlineHeight = validExplorerOutlineHeight(parsed.outlineHeight) || explorerOutlineHeight;
  } catch {
    explorerFilesCollapsed = false;
    explorerOutlineCollapsed = false;
    explorerOutlineHeight = "35%";
  }
  if (explorerFilesCollapsed && explorerOutlineCollapsed) {
    explorerOutlineCollapsed = false;
  }
  applyExplorerSectionLayout();
}

function saveExplorerSectionState() {
  try {
    window.localStorage.setItem(explorerSectionStoreKey, JSON.stringify({
      filesCollapsed: explorerFilesCollapsed,
      outlineCollapsed: explorerOutlineCollapsed,
      outlineHeight: explorerOutlineHeight,
    }));
  } catch {
  }
}

function validExplorerOutlineHeight(value) {
  const text = String(value || "").trim();
  if (!/^\d+(?:\.\d+)?%$/.test(text)) {
    return "";
  }
  const numeric = Number.parseFloat(text);
  return Number.isFinite(numeric) && numeric >= 18 && numeric <= 75 ? text : "";
}

function explorerSectionsElement() {
  return document.querySelector(".explorer-sections");
}

function explorerSectionElement(id) {
  return document.querySelector(`[data-explorer-section="${id}"]`);
}

function applyExplorerSectionLayout() {
  const sections = explorerSectionsElement();
  if (!sections) {
    return;
  }
  sections.classList.toggle("is-files-collapsed", explorerFilesCollapsed);
  sections.classList.toggle("is-outline-collapsed", explorerOutlineCollapsed);
  sections.style.setProperty("--source-outline-height", explorerOutlineHeight);
  syncExplorerSectionToggle("files", explorerFilesCollapsed);
  syncExplorerSectionToggle("outline", explorerOutlineCollapsed);
}

function syncExplorerSectionToggle(id, collapsed) {
  explorerSectionElement(id)?.classList.toggle("is-collapsed", collapsed);
  const toggle = document.querySelector(`[data-explorer-section-toggle="${id}"]`);
  if (toggle) {
    toggle.setAttribute("aria-expanded", String(!collapsed));
  }
}

function toggleExplorerSection(id) {
  const outlineWasCollapsed = explorerOutlineCollapsed;
  if (id === "files") {
    explorerFilesCollapsed = !explorerFilesCollapsed;
    if (explorerFilesCollapsed && explorerOutlineCollapsed) {
      explorerOutlineCollapsed = false;
    }
  } else if (id === "outline") {
    explorerOutlineCollapsed = !explorerOutlineCollapsed;
    if (explorerOutlineCollapsed && explorerFilesCollapsed) {
      explorerFilesCollapsed = false;
    }
  } else {
    return;
  }
  applyExplorerSectionLayout();
  saveExplorerSectionState();
  const outlineShouldRefresh = !explorerOutlineCollapsed
    && (outlineWasCollapsed || (id === "files" && explorerFilesCollapsed));
  if (outlineShouldRefresh && typeof scheduleSourceOutlineRefresh === "function") {
    scheduleSourceOutlineRefresh(true, { force: true });
  }
}

function resizeExplorerOutlineFromPointer(clientY) {
  const sections = explorerSectionsElement();
  if (!sections || explorerFilesCollapsed || explorerOutlineCollapsed) {
    return;
  }
  const rect = sections.getBoundingClientRect();
  const headerHeight = 24 * 2;
  const splitterHeight = outlineSplitter?.offsetHeight || 5;
  const available = Math.max(1, rect.height - headerHeight - splitterHeight);
  const outlinePixels = rect.bottom - clientY - 12;
  const percent = Math.max(18, Math.min(75, (outlinePixels / available) * 100));
  explorerOutlineHeight = `${percent.toFixed(1)}%`;
  applyExplorerSectionLayout();
}

document.querySelectorAll("[data-explorer-section-toggle]").forEach((toggle) => {
  toggle.addEventListener("click", (event) => {
    event.preventDefault();
    event.stopPropagation();
    toggleExplorerSection(toggle.dataset.explorerSectionToggle);
  });
});

outlineSplitter?.addEventListener("pointerdown", (event) => {
  if (event.button !== 0 || explorerFilesCollapsed || explorerOutlineCollapsed) {
    return;
  }
  draggingOutlineSplitter = true;
  draggingOutlineSplitterPointerId = event.pointerId;
  outlineSplitter.setPointerCapture?.(event.pointerId);
  outlineSplitter.classList.add("is-active-splitter");
  event.preventDefault();
});

document.addEventListener("pointermove", (event) => {
  if (!draggingOutlineSplitter || event.pointerId !== draggingOutlineSplitterPointerId) {
    return;
  }
  resizeExplorerOutlineFromPointer(event.clientY);
});

function stopExplorerOutlineResize(event) {
  if (!draggingOutlineSplitter || event.pointerId !== draggingOutlineSplitterPointerId) {
    return;
  }
  draggingOutlineSplitter = false;
  draggingOutlineSplitterPointerId = null;
  outlineSplitter?.classList.remove("is-active-splitter");
  saveExplorerSectionState();
}

document.addEventListener("pointerup", stopExplorerOutlineResize);
document.addEventListener("pointercancel", stopExplorerOutlineResize);
loadExplorerSectionState();

function renderTreeNode(node, parent, depth, importTitleIndex) {
  if (!node) {
    return;
  }
  if (node.kind === "folder") {
    if (node !== fileTree) {
      const row = document.createElement("div");
      row.className = "tree-row folder-row";
      row.dataset.nodeId = node.id;
      row.dataset.dragId = node.id;
      treeRowByNodeId.set(node.id, row);
      row.tabIndex = 0;
      row.style.setProperty("--depth", depth);
      row.setAttribute("role", "treeitem");
      row.setAttribute("aria-expanded", node.expanded === false ? "false" : "true");
      row.setAttribute("aria-selected", node.id === selectedTreeId ? "true" : "false");
      row.classList.toggle("is-selected-folder", node.id === selectedFolderId);
      row.classList.toggle("is-active-tree", node.id === selectedTreeId);
      row.classList.toggle("is-renaming", renameEntry?.nodeId === node.id);
      row.innerHTML = `${folderChevronSvg(node.expanded !== false)}${folderIconSvg(node.isWorkspaceRoot)}${treeNameHtml(node)}${treeActionsHtml(node.isWorkspaceRoot ? "workspace" : "folder")}`;
      setTreeName(row, node);
      parent.append(row);
    }
    if (node === fileTree || node.expanded !== false) {
      for (const child of node.children || []) {
        renderTreeNode(child, parent, node === fileTree ? depth : depth + 1, importTitleIndex);
      }
      renderDraftEntry(node, parent, node === fileTree ? depth : depth + 1);
    }
    return;
  }

  const row = document.createElement("div");
  row.className = "tree-row file-row";
  row.dataset.fileId = node.id;
  row.dataset.dragId = node.id;
  treeRowByNodeId.set(node.id, row);
  row.tabIndex = 0;
  row.style.setProperty("--depth", depth);
  row.setAttribute("role", "treeitem");
  row.setAttribute("aria-selected", node.id === selectedTreeId ? "true" : "false");
  row.classList.toggle("is-active", node.id === activeFileId);
  row.classList.toggle("is-active-tree", node.id === selectedTreeId);
  row.classList.toggle("is-renaming", renameEntry?.nodeId === node.id);
  row.innerHTML = `${fileIconSvg(node)}${treeNameHtml(node)}${treeActionsHtml("file")}`;
  setTreeName(row, node);
  setTreeImportTitle(row, node, importTitleIndex);
  parent.append(row);
}

function folderChevronSvg(expanded) {
  return expanded
    ? `<svg class="tree-chevron" viewBox="0 0 16 16" aria-hidden="true"><path d="M4 6l4 4 4-4"></path></svg>`
    : `<svg class="tree-chevron" viewBox="0 0 16 16" aria-hidden="true"><path d="M6 4l4 4-4 4"></path></svg>`;
}

function folderIconSvg(workspace = false) {
  if (workspace) {
    return `<svg class="tree-icon lucide lucide-folder-open" viewBox="0 0 24 24" aria-hidden="true"><path d="m6 14 1.5-2.9A2 2 0 0 1 9.24 10H20a2 2 0 0 1 1.94 2.5l-1.54 6A2 2 0 0 1 18.46 20H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.69.9l.81 1.2A2 2 0 0 0 12.07 6H18a2 2 0 0 1 2 2v2"></path></svg>`;
  }
  return `<svg class="tree-icon lucide lucide-folder" viewBox="0 0 24 24" aria-hidden="true"><path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"></path></svg>`;
}

function fileIconSvg(node) {
  if (puzzleSourceProfile(node) === "puzzle3d") {
    return `<svg xmlns="http://www.w3.org/2000/svg" class="tree-icon lucide lucide-file-box-icon lucide-file-box" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M14.5 22H18a2 2 0 0 0 2-2V8a2.4 2.4 0 0 0-.706-1.706l-3.588-3.588A2.4 2.4 0 0 0 14 2H6a2 2 0 0 0-2 2v3.8"/><path d="M14 2v5a1 1 0 0 0 1 1h5"/><path d="M11.7 14.2 7 17l-4.7-2.8"/><path d="M3 13.1a2 2 0 0 0-.999 1.76v3.24a2 2 0 0 0 .969 1.78L6 21.7a2 2 0 0 0 2.03.01L11 19.9a2 2 0 0 0 1-1.76V14.9a2 2 0 0 0-.97-1.78L8 11.3a2 2 0 0 0-2.03-.01z"/><path d="M7 17v5"/></svg>`;
  }
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
        commitRenameEntry(input.value).catch((error) => {
          console.error(error);
          setEditorStatus("Rename failed", "is-error");
        });
      } else if (event.key === "Escape") {
        renameEntry = null;
        renderDocumentSelect();
      }
    });
    input.addEventListener("blur", () => {
      commitRenameEntry(input.value).catch((error) => {
        console.error(error);
        setEditorStatus("Rename failed", "is-error");
      });
    });
    return;
  }
  row.querySelector(".tree-label").textContent = node.name || fileName(node.puzzlePath);
}

function buildTreeImportTitleIndex() {
  const parentGamesById = new Map();
  const directImportersById = new Map();
  const puzzleDocuments = documents.filter((document) => isPuzzleDocument(document) && isTextDocument(document));
  const gameEntries = puzzleDocuments.filter((document) => documentDeclaresGameEntry(document));
  const pathsByDocumentId = new Map();

  for (const document of puzzleDocuments) {
    pathsByDocumentId.set(document.id, puzzleImportPathsForDocument(document));
  }

  const importedDocumentsFor = (document) => {
    const root = document.workspaceRoot || workspaceRoot || "";
    return (pathsByDocumentId.get(document.id) || [])
      .map((importPath) => documentByPathForWorkspace(importPath, root))
      .filter((imported) => imported && isPuzzleDocument(imported) && isTextDocument(imported));
  };

  for (const document of puzzleDocuments) {
    if (documentDeclaresGameEntry(document)) {
      parentGamesById.set(document.id, [document]);
    }
    const directImportedIds = new Set();
    for (const imported of importedDocumentsFor(document)) {
      if (imported.id === document.id || directImportedIds.has(imported.id)) {
        continue;
      }
      directImportedIds.add(imported.id);
      if (!directImportersById.has(imported.id)) {
        directImportersById.set(imported.id, []);
      }
      directImportersById.get(imported.id).push(document);
    }
  }

  for (const gameEntry of gameEntries) {
    const visited = new Set();
    const stack = importedDocumentsFor(gameEntry);
    while (stack.length) {
      const imported = stack.pop();
      if (!imported || visited.has(imported.id)) {
        continue;
      }
      visited.add(imported.id);
      if (documentDeclaresGameEntry(imported)) {
        stack.push(...importedDocumentsFor(imported));
        continue;
      }
      if (!parentGamesById.has(imported.id)) {
        parentGamesById.set(imported.id, []);
      }
      parentGamesById.get(imported.id).push(gameEntry);
      stack.push(...importedDocumentsFor(imported));
    }
  }

  for (const values of parentGamesById.values()) {
    values.sort(comparePuzzleEntryDocuments);
  }
  for (const values of directImportersById.values()) {
    values.sort(comparePuzzleEntryDocuments);
  }

  return { parentGamesById, directImportersById };
}

function setTreeImportTitle(row, node, importTitleIndex) {
  if (!isPuzzleDocument(node) || !isTextDocument(node)) {
    return;
  }
  const lines = [];
  const parentGames = importTitleIndex.parentGamesById.get(node.id) || [];
  if (parentGames.length > 1) {
    lines.push(`Parent games: ${parentGames.map((item) => item.puzzlePath || item.name || "game").join(", ")}`);
    lines.push(`Preview uses: ${parentGames[0].puzzlePath || parentGames[0].name || "game"}`);
  } else if (parentGames.length === 1 && parentGames[0].id !== node.id) {
    lines.push(`Parent game: ${parentGames[0].puzzlePath || parentGames[0].name || "game"}`);
  } else if (parentGames.length === 1) {
    lines.push("Game entry");
  }
  const importers = importTitleIndex.directImportersById.get(node.id) || [];
  if (importers.length) {
    lines.push(`Imported by: ${importers.map((item) => item.puzzlePath || item.name).join(", ")}`);
  } else if (!parentGames.length) {
    lines.push("Not imported by a game entry");
  }
  if (lines.length) {
    row.title = lines.join("\n");
  }
}

function treeActionsHtml(kind) {
  if (kind === "workspace") {
    return `<span class="tree-actions" aria-label="Workspace actions">
      <button class="tree-action-button" type="button" data-tree-action="remove-workspace" aria-label="Close workspace" title="Close workspace">${closeIconSvg()}</button>
    </span>`;
  }
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

function closeIconSvg() {
  return `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M18 6 6 18"></path><path d="m6 6 12 12"></path></svg>`;
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
    commitDraftEntry(input.value).catch((error) => {
      console.error(error);
      setEditorStatus(workspaceMutationErrorMessage("Create failed", error), "is-error");
      renderDocumentSelect();
    });
  });
  input.addEventListener("keydown", (event) => {
    if (event.key === "Escape") {
      draftEntry = null;
      renderDocumentSelect();
    }
  });
  input.addEventListener("blur", () => {
    commitDraftEntry(input.value).catch((error) => {
      console.error(error);
      setEditorStatus(workspaceMutationErrorMessage("Create failed", error), "is-error");
      renderDocumentSelect();
    });
  });
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
  return puzzleSourceProfile(document) !== "";
}

function puzzleSourceProfile(document) {
  const ext = extensionName(document?.puzzlePath || document?.name);
  if (ext === "puzzle") {
    return "puzzle2d";
  }
  if (ext === "puzzle3") {
    return "puzzle3d";
  }
  return "";
}

function isTextDocument(document) {
  return (document?.encoding || "text") !== "data_url";
}

function isTextFileName(name, mimeType = "") {
  const ext = extensionName(name);
  return [
    "puzzle", "puzzle3", "css", "js", "mjs", "json", "svg", "txt", "md", "html", "xml", "csv", "tsv",
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
    puzzle3: "text/plain",
    svg: "image/svg+xml",
    txt: "text/plain",
    wav: "audio/wav",
    webp: "image/webp",
  }[ext] || "application/octet-stream";
}

function workspaceAssetMap(root = "") {
  const assets = new Map();
  for (const document of documents) {
    if (!document?.puzzlePath) {
      continue;
    }
    if (root && document.workspaceRoot && normalizePath(document.workspaceRoot) !== normalizePath(root)) {
      continue;
    }
    assets.set(normalizePath(document.puzzlePath), document);
  }
  return assets;
}

function normalizePath(path) {
  return String(path || "").replaceAll("\\", "/").replace(/^\.\/+/, "");
}

function assetUrlForPath(path, baseDir = "", root = workspaceRoot) {
  const normalized = normalizePath(path);
  const fullPath = normalizePath(baseDir ? joinPath(baseDir, normalized) : normalized);
  const assets = workspaceAssetMap(root);
  const asset = assets.get(fullPath) || assets.get(normalized);
  if (!asset) {
    return "";
  }
  if (asset.encoding === "data_url") {
    return asset.dataUrl || "";
  }
  return `data:${asset.mimeType || mimeTypeForPath(asset.puzzlePath)};charset=utf-8,${encodeURIComponent(asset.source || "")}`;
}

function assetResolverScript(document) {
  const baseDir = directoryName(document?.puzzlePath);
  const root = document?.workspaceRoot || workspaceRoot;
  const entries = {};
  for (const path of declaredFileAssetPaths(document)) {
    const key = normalizePath(path);
    const url = assetUrlForPath(key, baseDir, root);
    if (!url) {
      throw new Error(`Declared puzzle asset not found: ${key}`);
    }
    entries[key] = url;
  }
  return `window.PuzzleAssets = { files: ${JSON.stringify(entries)}, url(path) { const key = String(path || "").replaceAll("\\\\", "/"); if (Object.prototype.hasOwnProperty.call(this.files, key)) return this.files[key]; if (/^(?:data:|https?:|#)/.test(key)) return key; throw new Error(\`Puzzle asset is not embedded: \${key}. Declare it with file "\${key}" in assets.\`); } };`;
}

function declaredFileAssetPaths(document) {
  const paths = declaredAssetPaths(document, "file");
  for (const path of declaredSpriteImagePaths(document)) {
    if (!paths.includes(path)) {
      paths.push(path);
    }
  }
  return paths;
}

function declaredSpriteImagePaths(document) {
  const expanded = expandedWorkspaceSourceForEditor(document);
  const out = [];
  for (const line of String(expanded || "").split("\n")) {
    const match = stripLineCommentForWasm(line).trim().match(/^image\s+"([^"]+)"$/);
    if (match && !out.includes(match[1])) {
      out.push(match[1]);
    }
  }
  return out;
}

function rewriteCssAssetUrls(css, baseDir = "", root = workspaceRoot) {
  return String(css || "").replace(/url\(([^)]+)\)/g, (match, raw) => {
    const value = raw.trim().replace(/^['"]|['"]$/g, "");
    if (!value || /^(data:|https?:|blob:|#)/i.test(value)) {
      return match;
    }
    const url = assetUrlForPath(value, baseDir, root);
    return url ? `url("${url}")` : match;
  });
}

function effectiveGameCss(document) {
  const baseDir = directoryName(document.puzzlePath);
  const declaredCssPaths = declaredAssetPaths(document, "css");
  const parts = [];
  for (const themeDocument of effectiveThemeCssDocuments(document, effectiveThemeName(document))) {
    parts.push(rewriteCssAssetUrls(
      themeDocument.source || "",
      directoryName(themeDocument.puzzlePath),
      document.workspaceRoot || workspaceRoot,
    ));
  }
  if (!declaredCssPaths.length && document.gameCss) {
    parts.push(document.gameCss);
    return parts.filter(Boolean).join("\n");
  }
  let missingDeclaredAsset = false;
  for (const path of declaredCssPaths) {
    const cssDocument = documentByPath(normalizePath(joinPath(baseDir, path)));
    const source = cssDocument?.source || "";
    if (source) {
      parts.push(rewriteCssAssetUrls(source, directoryName(cssDocument.puzzlePath), document.workspaceRoot || workspaceRoot));
    } else {
      missingDeclaredAsset = true;
    }
  }
  if (missingDeclaredAsset && document.gameCss) {
    parts.push(document.gameCss);
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
  const expanded = expandedWorkspaceSourceForEditor(document);
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
      if (header[1]) {
        latest = header[1];
      }
      activeTheme = trimmed.endsWith("{");
      depth = activeTheme ? 1 : 0;
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
  const baseDir = directoryName(document.puzzlePath);
  const declaredScriptPaths = declaredAssetPaths(document, "script");
  const scripts = [assetResolverScript(document)];
  for (const path of declaredScriptPaths) {
    const scriptDocument = documentByPath(normalizePath(joinPath(baseDir, path)));
    if (scriptDocument?.source) {
      scripts.push(scriptDocument.source);
    }
  }
  return scripts.filter(Boolean).join("\n");
}

function declaredAssetPaths(document, kind) {
  const expanded = expandedWorkspaceSourceForEditor(document);
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

function expandedWorkspaceSourceForEditor(document) {
  const compiler = window.PuzzleStudioRuntime?.cachedWasmCompiler?.();
  const expand = compiler?.expand_workspace_entry_source;
  if (typeof expand !== "function") {
    throw new Error("Editor WASM workspace source resolver is unavailable");
  }
  return expand(
    document?.puzzlePath || "game.puzzle",
    JSON.stringify(workspaceCompilerDocuments(document)),
  );
}

function folderDocument(document, name) {
  const target = normalizePath(joinPath(directoryName(document?.puzzlePath), name));
  return documents.find((candidate) => normalizePath(candidate.puzzlePath) === target) || null;
}

function persistCurrentDocument() {
  const document = activeDocument();
  if (!document) {
    return;
  }
  if (documentNeedsContentLoad(document)) {
    return;
  }
  if (!isTextDocument(document)) {
    return;
  }
  document.source = sourceEditorDocumentValue();
  document.sourceFoldedBlockKeys = sourceFoldStateForSource(document.source);
}

function loadEmbeddedDocument(index) {
  const document = documents[index];
  if (!document) {
    return;
  }
  const previousActiveFileId = activeFileId;
  const previousPreviewDocument = activePreviewDocument();
  const previousPreviewKey = previousPreviewDocument ? documentIdentityKey(previousPreviewDocument) : "";
  showWorkPane(SOURCE_WORK_PANE_ID);
  currentDocumentIndex = index;
  activeFileId = document.id;
  workspaceRoot = document.workspaceRoot || workspaceRoot || "";
  selectedTreeId = document.id;
  selectedFolderId = findParentFolder(fileTree, document.id)?.id || selectedFolderId;
  openDocumentTab(document.id);
  renderDocumentSelect();
  renderDocumentTabs();
  if (documentNeedsContentLoad(document)) {
    sourceEditor.readOnly = true;
    setSourceEditorValue(`Loading ${document.name || fileName(document.puzzlePath) || "file"}...`);
    resetPreviewLog("Loading file");
    runButton.disabled = true;
    saveButton.disabled = true;
    ensureDocumentContentLoaded(document).then(() => {
      if (activeFileId === document.id) {
        loadEmbeddedDocument(activeDocumentIndex());
      }
      renderDocumentSelect();
      renderDocumentTabs();
    }).catch((error) => {
      console.error(error);
      if (activeFileId === document.id) {
        setEditorStatus(`Load failed: ${userFacingRuntimeError(error)}`, "is-error");
        setSourceEditorValue(`Load failed: ${userFacingRuntimeError(error)}`);
      }
    });
    return;
  }
  const previewDocument = activePreviewDocument();
  applyGameCss(previewDocument ? effectiveGameCss(previewDocument) : "");
  applyGameVisuals(previewDocument ? effectiveGameVisualsJs(previewDocument) : "");
  runButton.disabled = !previewDocument;
  sourceEditor.readOnly = !isTextDocument(document);
  const sourceText = isTextDocument(document)
    ? document.source || ""
    : `${document.name || fileName(document.puzzlePath)}\n${document.mimeType || "binary"}\n${document.dataUrl ? `${document.dataUrl.length} bytes encoded` : "No data"}`;
  setSourceEditorValue(sourceText, {
    preserveUndoOnSameValue: document.id === previousActiveFileId,
  });
  if (isTextDocument(document)) {
    restoreSourceFoldState(document.sourceFoldedBlockKeys);
  }
  updateDocumentTabUnsavedStates();
  const activeSourceChanged = Boolean(previousActiveFileId && document.id !== previousActiveFileId);
  const previewTargetUnchanged = previewDocument
    && previousPreviewKey
    && documentIdentityKey(previewDocument) === previousPreviewKey;
  if (activeSourceChanged) {
    invalidateCompiledPreview(previewDocument);
  } else if (previewTargetUnchanged) {
    markPreviewDirty();
  } else {
    invalidateCompiledPreview(previewDocument);
  }
  if (typeof syncPaneModesFromFocusedPuzzleSource === "function") {
    void syncPaneModesFromFocusedPuzzleSource({ switchOpenPane: true })
      .catch((error) => setEditorStatus(userFacingRuntimeError(error), "is-error"));
  }
  syncPreviewViewportAspect();
  runButton.disabled = !previewDocument;
  setActiveLevelIndex(0);
  resetPreviewLog(previewDocument ? "Run preview to compile." : "No game entry for preview.");
  if (!previewDocument) {
    appendPreviewLog("error", "No game entry for preview.", { source: "workspace" });
  }
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
    return;
  }

  selectedFolderId = folder.id;
  selectedTreeId = folder.id;
  renderDocumentSelect();

  applyGameCss(previewDocument ? effectiveGameCss(previewDocument) : "");
  applyGameVisuals(previewDocument ? effectiveGameVisualsJs(previewDocument) : "");
  invalidateCompiledPreview(previewDocument);
  syncPreviewViewportAspect();
  runButton.disabled = !previewDocument;
  setActiveLevelIndex(0);
  resetPreviewLog(previewDocument
    ? `Run preview to compile ${previewDocument.puzzlePath || previewDocument.name || "preview"}.`
    : "No preview target");
  if (!previewDocument) {
    appendPreviewLog("error", "No game entry for preview.", { source: "workspace" });
  }
  updateSourceMeta();
  resetLevelBuilderFromPreviewSource();
  saveDocumentStore(false);
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
function workspaceRootForNode(node) {
  if (!node) {
    return workspaceRoot || "";
  }
  if (node.workspaceRoot) {
    return node.workspaceRoot;
  }
  const parent = findParentFolder(fileTree, node.id);
  return parent ? workspaceRootForNode(parent) : workspaceRoot || "";
}

function workspaceRootForFolder(folder) {
  return workspaceRootForNode(folder);
}

function workspaceRootFolder(root) {
  const normalizedRoot = normalizePath(root || workspaceRoot || "");
  if (!normalizedRoot) {
    return null;
  }
  return (fileTree?.children || []).find((child) =>
    child.kind === "folder"
    && child.isWorkspaceRoot
    && normalizePath(child.workspaceRoot || "") === normalizedRoot
  ) || null;
}

function hostPathForEditorPath(path, rootOverride = workspaceRoot) {
  const normalized = normalizePath(path);
  const rootValue = rootOverride || workspaceRoot;
  if (!rootValue || normalized.startsWith("/") || /^[A-Za-z]:\//.test(normalized)) {
    return normalized;
  }
  const root = normalizePath(rootValue);
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

async function newPuzzleSourceForFile(_name) {
  return STARTER_PUZZLE_SOURCE;
}

function startDraftEntry(kind) {
  const folder = activeFolder();
  if (!folder?.id) {
    setEditorStatus("Select a workspace before creating files", "is-error");
    return;
  }
  expandFolderPathToNode(folder.id);
  selectedTreeId = folder.id;
  selectedFolderId = folder.id;
  draftEntry = {
    kind,
    parentId: folder.id,
    name: kind === "folder" ? "folder" : "untitled",
  };
  renderDocumentSelect();
}

async function commitDraftEntry(rawName) {
  if (!draftEntry) {
    return;
  }
  const parent = findNode(fileTree, draftEntry.parentId) || fileTree;
  const name = sanitizeFileName(rawName);
  const kind = draftEntry.kind;
  draftEntry = null;
  if (!name) {
    renderDocumentSelect();
    return;
  }
  persistCurrentDocument();
  if (kind === "folder") {
    const folder = makeFolder(uniqueChildName(parent, name), []);
    folder.workspaceRoot = workspaceRootForFolder(parent);
    const folderPathValue = joinPath(folderPath(parent), folder.name);
    if (!editorSeed && isDesktopHost() && typeof window.PuzzleStudioHost.createSourceFolder === "function") {
      beginWorkspaceHostMutation();
      try {
        await window.PuzzleStudioHost.createSourceFolder({
          folderPath: hostPathForEditorPath(folderPathValue, folder.workspaceRoot),
          workspaceRoot: folder.workspaceRoot,
        });
      } finally {
        endWorkspaceHostMutation();
      }
    }
    parent.children.push(folder);
    parent.expanded = true;
    selectedFolderId = folder.id;
    renderDocumentSelect();
    saveDocumentStore(false);
    return;
  }
  const current = documents[currentDocumentIndex] || {};
  const fileNameValue = uniqueChildName(parent, name);
  const file = makeFile(fileNameValue, await newPuzzleSourceForFile(fileNameValue), {
    parentPath: folderPath(parent),
    workspaceRoot: workspaceRootForFolder(parent),
    gameCss: current.gameCss || editorSeed?.gameCss || "",
  });
  if (!editorSeed && isDesktopHost() && typeof window.PuzzleStudioHost.createSourceFile === "function") {
    beginWorkspaceHostMutation();
    try {
      await window.PuzzleStudioHost.createSourceFile({
        source: file.source || "",
        puzzlePath: hostPathForEditorPath(file.puzzlePath, file.workspaceRoot),
        workspaceRoot: file.workspaceRoot,
      });
    } finally {
      endWorkspaceHostMutation();
    }
  }
  parent.children.push(file);
  activeFileId = file.id;
  syncDocumentsFromTree();
  loadEmbeddedDocument(activeDocumentIndex());
  saveDocumentStore(false);
}

async function moveNodeToFolder(nodeId, targetFolderId) {
  if (!nodeId) {
    return false;
  }
  const source = findNodeWithParent(fileTree, nodeId);
  const targetFolder = source?.node ? moveTargetFolderForSource(source.node, targetFolderId) : null;
  if (!source || !source.parent || !source.node || targetFolder?.kind !== "folder") {
    return false;
  }
  if (source.node === fileTree || source.parent === targetFolder) {
    return false;
  }
  if (source.node.isWorkspaceRoot) {
    return false;
  }
  if (source.node.kind === "folder" && containsNode(source.node, targetFolder.id)) {
    return false;
  }

  persistCurrentDocument();
  const sourceWorkspaceRoot = workspaceRootForNode(source.node);
  const targetWorkspaceRoot = targetFolder === fileTree
    ? sourceWorkspaceRoot
    : workspaceRootForFolder(targetFolder);
  const sourcePath = source.node.kind === "folder"
    ? folderPath(source.node)
    : source.node.puzzlePath;
  const targetPath = joinPath(folderPath(targetFolder), source.node.name || "item");
  if (!sourcePath || !targetPath) {
    throw new Error("Cannot move an entry without a workspace path.");
  }
  if (!editorSeed && isDesktopHost()) {
    if (typeof window.PuzzleStudioHost.renameWorkspaceEntry !== "function") {
      throw new Error("Desktop workspace move requires the host rename command.");
    }
    beginWorkspaceHostMutation();
    try {
      await window.PuzzleStudioHost.renameWorkspaceEntry({
        fromPath: hostPathForEditorPath(sourcePath, sourceWorkspaceRoot),
        toPath: hostPathForEditorPath(targetPath, targetWorkspaceRoot),
        workspaceRoot: sourceWorkspaceRoot,
        targetWorkspaceRoot,
      });
    } finally {
      endWorkspaceHostMutation();
    }
  }

  source.parent.children = source.parent.children.filter((child) => child.id !== nodeId);
  if (!editorSeed && isDesktopHost()) {
    source.node.name = source.node.name || "item";
  } else {
    source.node.name = uniqueChildName(targetFolder, source.node.name || "item");
  }
  setWorkspaceRootForNode(source.node, targetWorkspaceRoot);
  targetFolder.children.push(source.node);
  targetFolder.expanded = true;
  selectedFolderId = targetFolder === fileTree ? "" : targetFolder.id;
  selectedTreeId = source.node.id;
  syncDocumentsFromTree();
  currentDocumentIndex = activeDocumentIndex();
  renderDocumentSelect();
  saveDocumentStore(false);
  return true;
}

function dropFolderIdForElement(element) {
  const row = element?.closest?.(".tree-row");
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

function dropFolderIdForEvent(event) {
  return dropFolderIdForElement(event.target);
}

function dropFolderIdForPoint(x, y) {
  return dropFolderIdForElement(document.elementFromPoint(x, y));
}

function moveTargetFolderForSource(sourceNode, targetFolderId) {
  const requestedFolder = targetFolderId ? findNode(fileTree, targetFolderId) : fileTree;
  if (requestedFolder !== fileTree) {
    return requestedFolder;
  }
  const sourceWorkspaceRoot = workspaceRootForNode(sourceNode);
  const sourceWorkspaceFolder = sourceWorkspaceRoot ? workspaceRootFolder(sourceWorkspaceRoot) : null;
  if (sourceWorkspaceFolder) {
    return sourceWorkspaceFolder;
  }
  return fileTree;
}

function resolvedDropFolderIdForNode(nodeId, targetFolderId) {
  const source = treeDragCacheForNode(nodeId)?.source;
  const targetFolder = source?.node ? moveTargetFolderForSource(source.node, targetFolderId) : null;
  if (!targetFolder || targetFolder === fileTree) {
    return "";
  }
  return targetFolder.id;
}

function canDropNodeOnFolder(nodeId, targetFolderId) {
  if (!nodeId) {
    return false;
  }
  const cache = treeDragCacheForNode(nodeId);
  if (!cache?.source?.node || !cache.source.parent) {
    return false;
  }
  const targetKey = targetFolderId || "";
  if (cache.targetDecisions.has(targetKey)) {
    return cache.targetDecisions.get(targetKey);
  }
  const source = cache.source;
  const targetFolder = moveTargetFolderForSource(source.node, targetFolderId);
  let allowed = true;
  if (targetFolder?.kind !== "folder") {
    allowed = false;
  }
  if (allowed && source.parent === targetFolder) {
    allowed = false;
  }
  if (allowed && source.node.isWorkspaceRoot) {
    allowed = false;
  }
  if (allowed && source.node.kind === "folder" && containsNode(source.node, targetFolder.id)) {
    allowed = false;
  }
  cache.targetDecisions.set(targetKey, allowed);
  return allowed;
}

function markDropTarget(folderId) {
  const targetId = folderId || "";
  const target = targetId ? treeRowByNodeId.get(targetId) : documentList;
  if (currentDropTargetId === targetId && currentDropTargetElement === target) {
    return;
  }
  clearDropTargets();
  currentDropTargetId = targetId;
  currentDropTargetElement = target || null;
  currentDropTargetElement?.classList.add("is-drop-target");
}

function clearDropTargets() {
  currentDropTargetElement?.classList.remove("is-drop-target");
  currentDropTargetId = null;
  currentDropTargetElement = null;
}

function resetTreeDragDecisionCache() {
  treeDragDecisionCache = null;
}

function treeDragCacheForNode(nodeId) {
  if (treeDragDecisionCache?.nodeId === nodeId) {
    return treeDragDecisionCache;
  }
  const source = findNodeWithParent(fileTree, nodeId);
  treeDragDecisionCache = {
    nodeId,
    source,
    targetDecisions: new Map(),
  };
  return treeDragDecisionCache;
}

function setWorkspaceRootForNode(node, nextRoot) {
  if (!node) {
    return;
  }
  node.workspaceRoot = nextRoot || "";
  if (node.kind === "folder") {
    for (const child of node.children || []) {
      setWorkspaceRootForNode(child, nextRoot);
    }
  }
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

async function commitRenameEntry(value) {
  if (!renameEntry) {
    return;
  }
  if (renameEntry.committing) {
    return;
  }
  renameEntry.committing = true;
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
  if (nextName === oldName) {
    renameEntry = null;
    renderDocumentSelect();
    return;
  }
  const parentPath = folderPath(target.parent);
  const fromPath = target.node.kind === "folder"
    ? folderPath(target.node)
    : target.node.puzzlePath;
  const toPath = joinPath(parentPath, nextName);
  const targetWorkspaceRoot = workspaceRootForNode(target.node);
  if (!editorSeed && isDesktopHost() && typeof window.PuzzleStudioHost.renameWorkspaceEntry === "function") {
    beginWorkspaceHostMutation();
    try {
      await window.PuzzleStudioHost.renameWorkspaceEntry({
        fromPath: hostPathForEditorPath(fromPath, targetWorkspaceRoot),
        toPath: hostPathForEditorPath(toPath, targetWorkspaceRoot),
        workspaceRoot: targetWorkspaceRoot,
      });
    } catch (error) {
      if (renameEntry) {
        renameEntry.committing = false;
      }
      throw error;
    } finally {
      endWorkspaceHostMutation();
    }
  }
  target.node.name = nextName;
  renameEntry = null;
  syncDocumentsFromTree();
  currentDocumentIndex = activeDocumentIndex();
  saveDocumentStore(false);
  renderDocumentSelect();
  if (target.node.id === activeFileId && typeof syncPaneModesFromFocusedPuzzleSource === "function") {
    void syncPaneModesFromFocusedPuzzleSource({ switchOpenPane: true, loadFirst: false })
      .catch((error) => setEditorStatus(userFacingRuntimeError(error), "is-error"));
  }
  setEditorStatus("Renamed", "is-ok");
}

async function deleteTreeNode(nodeId) {
  persistCurrentDocument();
  const target = findNodeWithParent(fileTree, nodeId);
  if (!target?.node || !target.parent || target.node === fileTree) {
    return;
  }

  const targetWorkspaceRoot = workspaceRootForNode(target.node);
  const entryPath = target.node.kind === "folder"
    ? folderPath(target.node)
    : target.node.puzzlePath;
  if (!confirmDeleteWorkspaceEntry(target.node, {
    fromDisk: !editorSeed && isDesktopHost() && typeof window.PuzzleStudioHost.deleteWorkspaceEntry === "function",
  })) {
    setEditorStatus("Delete canceled", "");
    return;
  }

  if (!editorSeed && typeof window.PuzzleStudioHost.deleteWorkspaceEntry === "function") {
    beginWorkspaceHostMutation();
    try {
      await window.PuzzleStudioHost.deleteWorkspaceEntry({
        entryPath: hostPathForEditorPath(entryPath, targetWorkspaceRoot),
        workspaceRoot: targetWorkspaceRoot,
      });
    } finally {
      endWorkspaceHostMutation();
    }
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
  if (removedActive || !findNode(fileTree, activeFileId)) {
    activeFileId = documents[0]?.id || "";
  }
  selectedTreeId = activeFileId;
  currentDocumentIndex = activeDocumentIndex();
  if (!activeFileId) {
    selectedTreeId = target.parent === fileTree ? "" : target.parent.id;
    selectedFolderId = selectedTreeId;
    renderDocumentSelect();
    renderDocumentTabs();
    runButton.disabled = true;
    sourceEditor.readOnly = false;
    setSourceEditorValue("");
    latestHtml = "";
    previewExport = null;
    setPreviewFrameHtml(emptyPreviewDocument());
    resetPreviewLog("No puzzle selected");
    saveDocumentStore(false);
    setEditorStatus("Deleted", "is-ok");
    return;
  }
  loadEmbeddedDocument(currentDocumentIndex);
  saveDocumentStore(false, { persistCurrent: false });
  setEditorStatus("Deleted", "is-ok");
}

async function removeWorkspaceNode(nodeId) {
  persistCurrentDocument();
  const target = findNodeWithParent(fileTree, nodeId);
  if (!target?.node || !target.parent || !target.node.isWorkspaceRoot) {
    return;
  }
  const unsaved = unsavedDocumentsInNode(target.node);
  if (!confirmRemoveWorkspaceWithUnsavedChanges(target.node, unsaved)) {
    setEditorStatus("Close canceled: unsaved changes", "is-error");
    return;
  }
  const removedRoot = target.node.workspaceRoot || "";
  if (isDesktopHost() && removedRoot && typeof window.PuzzleStudioHost.removeWorkspace === "function") {
    await window.PuzzleStudioHost.removeWorkspace({ workspaceRoot: removedRoot });
  }
  const removedActive = containsNode(target.node, activeFileId);
  target.parent.children = target.parent.children.filter((child) => child.id !== target.node.id);
  renameEntry = null;
  draftEntry = null;
  syncDocumentsFromTree();
  openTabIds = openTabIds.filter((id) => documents.some((document) => document.id === id));
  if (removedActive || !findNode(fileTree, activeFileId)) {
    activeFileId = documents[0]?.id || "";
  }
  selectedTreeId = activeFileId || "";
  selectedFolderId = activeFileId ? findParentFolder(fileTree, activeFileId)?.id || "" : "";
  currentDocumentIndex = activeDocumentIndex();
  if (activeFileId) {
    loadEmbeddedDocument(currentDocumentIndex);
    saveDocumentStore(false, { persistCurrent: false });
  } else {
    resetEditorForNoOpenProject({ status: "Closed workspace", statusClass: "is-ok" });
    saveDocumentStore(false, { persistCurrent: false });
  }
  if (activeFileId) {
    setEditorStatus("Closed workspace", "is-ok");
  }
}

const STARTER_PUZZLE_SOURCE = "";

function activeFolder() {
  const selected = selectedTreeNode();
  if (selected?.kind === "folder") {
    return selected;
  }
  if (selected?.kind === "file") {
    const selectedFileFolder = findParentFolder(fileTree, selected.id);
    if (selectedFileFolder?.kind === "folder") {
      return selectedFileFolder;
    }
  }
  const selectedFolder = selectedFolderId ? findNode(fileTree, selectedFolderId) : null;
  if (selectedFolder?.kind === "folder") {
    return selectedFolder;
  }
  const activeWorkspaceFolder = workspaceRootFolder(workspaceRoot);
  if (activeWorkspaceFolder?.kind === "folder") {
    return activeWorkspaceFolder;
  }
  const current = documents[currentDocumentIndex];
  const currentFolder = findParentFolder(fileTree, current?.id);
  if (currentFolder?.kind === "folder") {
    return currentFolder;
  }
  if (isDesktopHost()) {
    const workspaceFolders = (fileTree?.children || []).filter((child) => child?.kind === "folder" && child.isWorkspaceRoot);
    return workspaceFolders.length === 1 ? workspaceFolders[0] : null;
  }
  return fileTree;
}

function expandFolderPathToNode(nodeId) {
  const path = folderPathToNode(fileTree, nodeId);
  for (const folder of path) {
    folder.expanded = true;
  }
}

function folderPathToNode(folder, nodeId, path = []) {
  if (!folder || folder.kind !== "folder") {
    return [];
  }
  const nextPath = folder === fileTree ? path : [...path, folder];
  if (folder.id === nodeId) {
    return nextPath;
  }
  for (const child of folder.children || []) {
    if (child.kind !== "folder") {
      continue;
    }
    const found = folderPathToNode(child, nodeId, nextPath);
    if (found.length) {
      return found;
    }
  }
  return [];
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
  return path
    .filter((part) => part && part !== "Files" && !part.startsWith("\0"))
    .join("/");
}

function folderPathVisit(node, targetId, path) {
  if (!node) {
    return false;
  }
  if (node.kind === "folder") {
    path.push(node.isWorkspaceRoot ? "\0workspace" : node.name);
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
  if (typeof handleToolPaneSaveShortcut === "function" && handleToolPaneSaveShortcut(event)) {
    event.preventDefault();
    event.stopImmediatePropagation();
    return true;
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

let desktopExitConfirmationOpen = false;

function installDesktopExitGuards() {
  if (!isDesktopHost()) {
    return;
  }
  window.PuzzleStudioHost.listenDesktopCloseRequested(async (event) => {
    if (desktopExitConfirmationOpen) {
      event.preventDefault();
      return;
    }
    desktopExitConfirmationOpen = true;
    try {
      if (!confirmDesktopExitWithUnsavedChanges("Close this window")) {
        event.preventDefault();
        setEditorStatus("Close canceled: unsaved changes", "is-error");
      }
    } catch (error) {
      event.preventDefault();
      console.error(error);
      setEditorStatus("Close blocked: unsaved state unavailable", "is-error");
    } finally {
      desktopExitConfirmationOpen = false;
    }
  }).catch((error) => {
    console.error(error);
    setEditorStatus("Close guard unavailable", "is-error");
  });
}

installDesktopExitGuards();
