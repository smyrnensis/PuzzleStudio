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
let loadedSourceDocumentId = "";
let loadedPreviewTargetKey = "";
const previewEntryDocumentIdByWorkspace = new Map();
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
  importFolderButton.title = "Open folder zip";
  importFolderButton.setAttribute("aria-label", "Open folder zip");
  importFolderButton.textContent = "Open Folder (.zip)";
}

function configureDesktopHost() {
  const desktop = isDesktopHost();
  if (openFileMenuButton) {
    openFileMenuButton.hidden = !desktop;
  }
  if (openProjectMenuButton) {
    openProjectMenuButton.hidden = !desktop;
    openProjectMenuButton.textContent = "Open Folder";
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
  const disabled = !fileTree || (isDesktopHost() && !hasWritableWorkspace());
  if (newDocumentButton) {
    newDocumentButton.disabled = disabled;
    newDocumentButton.title = disabled ? "Open a workspace before creating files" : "New File";
  }
  if (newFolderButton) {
    newFolderButton.disabled = disabled;
  }
}

function setWorkspaceFileActionsReady() {
  if (!fileTree) {
    throw new Error("workspace tree did not initialize");
  }
  updateFileCreationAvailability();
  for (const button of [openFileMenuButton, openProjectMenuButton, importButton, importFolderButton]) {
    if (button) {
      button.disabled = false;
    }
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
    selectPreviewEntryDocument(
      documents.find((document) => document.puzzlePath === editorSeed.puzzlePath)
        || documents.find((document) => document.id === activeFileId),
    );
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
  const requestedEntry = documents.find((document) =>
    isPuzzleDocument(document)
    && normalizePath(document.workspaceRoot || "") === normalizePath(workspaceRoot || "")
    && normalizePath(document.puzzlePath) === normalizePath(editorPathForHostPath(payload.puzzlePath || "", workspaceRoot))
  );
  activeFileId = workspaceRoot
    ? requestedEntry?.id
      || documents.find((document) => document.workspaceRoot === workspaceRoot && isPuzzleDocument(document))?.id
      || documents.find((document) => document.workspaceRoot === workspaceRoot)?.id
      || documents[0]?.id
      || ""
    : documents[0]?.id || "";
  selectPreviewEntryDocument(requestedEntry || documents.find((document) => document.id === activeFileId));
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
  loadedSourceDocumentId = "";
  loadedPreviewTargetKey = "";
  previewEntryDocumentIdByWorkspace.clear();
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
  previewBuild = null;
  previewBuildIsStale = false;
  previewSession = null;
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
  const requestedEntry = documents.find((document) =>
    isPuzzleDocument(document)
    && normalizePath(document.workspaceRoot || "") === normalizePath(root)
    && normalizePath(document.puzzlePath) === normalizePath(editorPathForHostPath(payload.puzzlePath || "", root))
  );
  selectPreviewEntryDocument(requestedEntry);
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
  const selectedEntry = previewEntryDocumentForWorkspace(root);
  activeFileId = selectedEntry?.id || documents.find((document) =>
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
  previewBuild = null;
  previewBuildIsStale = false;
  previewSession = null;
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
  await refreshWorkspaceImportMetadata(normalizedRoot);
}

async function refreshWorkspaceImportMetadata(root) {
  const entry = documents.find((document) => {
    const documentRoot = normalizePath(document.workspaceRoot || workspaceRoot || "");
    return isPuzzleDocument(document) && isTextDocument(document) && (!root || documentRoot === root);
  });
  if (!entry) {
    return;
  }
  const resolveIndex = window.PuzzleStudioRuntime?.workspaceIndex;
  if (typeof resolveIndex !== "function") {
    throw new Error("Editor WASM workspace index is unavailable.");
  }
  const index = await resolveIndex({ workspaceDocuments: workspaceCompilerDocuments(entry) });
  if (!Array.isArray(index?.documents)) {
    throw new Error("Editor WASM workspace index is missing documents.");
  }
  for (const item of index.documents) {
    const document = documentByPathForWorkspace(item.path, root);
    if (!document) {
      continue;
    }
    document.importedBy = Array.isArray(item.directImporters) ? item.directImporters.slice() : [];
  }
}

async function ensurePreviewDocumentsLoaded(document) {
  if (!document) {
    throw new Error("Workspace presentation manifest requires a preview document.");
  }
  const root = document.workspaceRoot || workspaceRoot || "";
  await ensureWorkspaceDocumentsLoaded(root);
  const manifest = await workspacePresentationManifest(document);
  await ensureDeclaredPreviewAssetDocumentsLoaded(document, root, manifest);
  return manifest;
}

async function workspacePresentationManifest(document) {
  const resolveManifest = window.PuzzleStudioRuntime?.workspacePresentationManifest;
  if (typeof resolveManifest !== "function") {
    throw new Error("Editor WASM workspace presentation manifest is unavailable.");
  }
  const manifest = await resolveManifest({
    puzzlePath: workspaceCompilerPath(document),
    workspaceDocuments: workspaceCompilerDocuments(document),
  });
  for (const field of ["cssPaths", "scriptPaths", "filePaths", "visualImagePaths"]) {
    if (!Array.isArray(manifest?.[field])) {
      throw new Error(`Editor WASM workspace presentation manifest is missing ${field}.`);
    }
  }
  if (manifest.themeName !== null && typeof manifest.themeName !== "string") {
    throw new Error("Editor WASM workspace presentation manifest has an invalid themeName.");
  }
  return manifest;
}

async function ensureDeclaredPreviewAssetDocumentsLoaded(document, root, manifest) {
  const assetDocuments = new Set();
  for (const [kind, paths] of [
    ["css", manifest.cssPaths],
    ["script", manifest.scriptPaths],
    ["file", declaredFileAssetPaths(manifest)],
  ]) {
    for (const path of paths) {
      const asset = documentByPathForWorkspace(path, root);
      if (!asset) {
        throw new Error(`declared ${kind} asset not found: ${path}`);
      }
      assetDocuments.add(asset);
    }
  }
  for (const themeDocument of effectiveThemeCssDocuments(document, manifest.themeName || "")) {
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
      if (!cssAsset) {
        throw new Error(`CSS asset not found: ${path} from ${asset.puzzlePath}`);
      }
      await ensureDocumentContentLoaded(cssAsset);
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
    previewHtml: "",
    previewError: "",
    gameCss: document.gameCss ?? fallback.gameCss ?? "",
    sourceFoldedBlockKeys: normalizeSourceFoldedBlockKeys(
      document.sourceFoldedBlockKeys ?? fallback.sourceFoldedBlockKeys,
    ),
    importedBy,
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
    previewHtml: "",
    previewError: "",
    gameCss: document.gameCss || "",
    sourceFoldedBlockKeys: normalizeSourceFoldedBlockKeys(document.sourceFoldedBlockKeys),
    importedBy: Array.isArray(document.importedBy) ? document.importedBy : [],
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
    if ((location.previewMode === "edit" || location.previewMode === "solver") && previewBuild?.exportData?.levels?.length) {
      setActiveLevelIndex(Math.max(0, Math.min(previewBuild?.exportData.levels.length - 1, location.levelIndex || 0)));
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
    close.className = "icon-button document-tab-close";
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
  const root = activeDocument()?.workspaceRoot || workspaceRoot || "";
  return previewEntryDocumentForWorkspace(root);
}

function previewWorkspaceKey(root) {
  return normalizePath(root || "");
}

function selectPreviewEntryDocument(document) {
  if (!isPuzzleDocument(document)) {
    return false;
  }
  const key = previewWorkspaceKey(document.workspaceRoot || workspaceRoot || "");
  const changed = previewEntryDocumentIdByWorkspace.get(key) !== document.id;
  previewEntryDocumentIdByWorkspace.set(key, document.id);
  return changed;
}

function previewEntryDocumentForWorkspace(root) {
  const key = previewWorkspaceKey(root);
  const documentId = previewEntryDocumentIdByWorkspace.get(key);
  if (!documentId) {
    return null;
  }
  return documents.find((document) =>
    isPuzzleDocument(document)
    && previewWorkspaceKey(document.workspaceRoot || "") === key
    && document.id === documentId
  ) || null;
}

function recordLoadedPreviewTarget(document = activePreviewDocument()) {
  loadedPreviewTargetKey = document ? documentIdentityKey(document) : "";
  return loadedPreviewTargetKey;
}

function documentPathIsInFolder(document, folderDir) {
  const path = normalizePath(document?.puzzlePath || "");
  const dir = normalizePath(folderDir || "");
  if (!dir) {
    return !!path;
  }
  return path === dir || path.startsWith(`${dir}/`);
}

function directImportersForDocument(document) {
  if (!isPuzzleDocument(document)) {
    return [];
  }
  const root = document.workspaceRoot || workspaceRoot || "";
  return (Array.isArray(document.importedBy) ? document.importedBy : [])
    .map((path) => documentByPathForWorkspace(path, root))
    .filter(Boolean)
    .sort((left, right) => normalizePath(left.puzzlePath).localeCompare(normalizePath(right.puzzlePath)));
}

function documentByPathForWorkspace(path, root) {
  const target = normalizePath(path);
  const normalizedRoot = normalizePath(root || "");
  return documents.find((candidate) =>
    workspaceCompilerPath(candidate) === target
    && (!normalizedRoot || !candidate.workspaceRoot || normalizePath(candidate.workspaceRoot) === normalizedRoot)
  ) || null;
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
      recentButton.className = "option-button explorer-empty-recent-button";
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
      row.className = "navigation-row tree-row folder-row";
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
  row.className = "navigation-row tree-row file-row";
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
  return editorIconSvg(expanded ? "chevron-down" : "chevron-right", { className: "tree-chevron" });
}

function folderIconSvg(workspace = false) {
  return editorIconSvg(workspace ? "folder-open" : "folder", { className: "tree-icon" });
}

function fileIconSvg(node) {
  const extension = extensionName(node?.puzzlePath || node?.name || "");
  if (extension === "puzzle") {
    return editorIconSvg("puzzle", { className: "tree-icon" });
  }
  return editorIconSvg("file", { className: "tree-icon" });
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
  const directImportersById = new Map();
  const puzzleDocuments = documents.filter((document) => isPuzzleDocument(document) && isTextDocument(document));
  for (const document of puzzleDocuments) {
    directImportersById.set(document.id, directImportersForDocument(document));
  }

  return directImportersById;
}

function setTreeImportTitle(row, node, importTitleIndex) {
  if (!isPuzzleDocument(node) || !isTextDocument(node)) {
    return;
  }
  const lines = [];
  const importers = importTitleIndex.get(node.id) || [];
  if (importers.length) {
    lines.push(`Imported by: ${importers.map((item) => item.puzzlePath || item.name).join(", ")}`);
  }
  if (lines.length) {
    row.title = lines.join("\n");
  }
}

function treeActionsHtml(kind) {
  if (kind === "workspace") {
    return `<span class="tree-actions" aria-label="Workspace actions">
      <button class="icon-button tree-action-button" type="button" data-tree-action="remove-workspace" aria-label="Close workspace" title="Close workspace">${closeIconSvg()}</button>
    </span>`;
  }
  const label = kind === "folder" ? "Folder actions" : "File actions";
  return `<span class="tree-actions" aria-label="${label}">
    <button class="icon-button tree-action-button" type="button" data-tree-action="rename" aria-label="Rename" title="Rename">${renameIconSvg()}</button>
    <button class="icon-button tree-action-button" type="button" data-tree-action="delete" aria-label="Delete" title="Delete">${deleteIconSvg()}</button>
  </span>`;
}

function renameIconSvg() {
  return editorIconSvg("pencil");
}

function deleteIconSvg() {
  return editorIconSvg("trash-2");
}

function closeIconSvg() {
  return editorIconSvg("x");
}

function renderDraftEntry(parentFolder, parent, depth) {
  if (!draftEntry || draftEntry.parentId !== parentFolder.id) {
    return;
  }
  const row = document.createElement("form");
  row.className = "navigation-row tree-row draft-row";
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
    return editorIconSvg("folder", { className: "tree-icon" });
  }
  return editorIconSvg("file", { className: "tree-icon" });
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
  const ext = extensionName(document?.puzzlePath || document?.name);
  return ext === "puzzle";
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

function assetResolverScript(document, manifest) {
  const root = document?.workspaceRoot || workspaceRoot;
  const entries = {};
  for (const path of declaredFileAssetPaths(manifest)) {
    const key = normalizePath(path);
    const url = assetUrlForPath(key, "", root);
    if (!url) {
      throw new Error(`Declared puzzle asset not found: ${key}`);
    }
    entries[key] = url;
  }
  return `window.PuzzleAssets = { files: ${JSON.stringify(entries)}, url(path) { const key = String(path || "").replaceAll("\\\\", "/"); if (Object.prototype.hasOwnProperty.call(this.files, key)) return this.files[key]; if (/^(?:data:|https?:|#)/.test(key)) return key; throw new Error(\`Puzzle asset is not embedded: \${key}. Declare it with file "\${key}" in assets.\`); } };`;
}

function declaredFileAssetPaths(manifest) {
  const paths = manifest.filePaths.slice();
  for (const path of manifest.visualImagePaths) {
    if (!paths.includes(path)) {
      paths.push(path);
    }
  }
  return paths;
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

function effectiveGameCss(document, manifest) {
  const declaredCssPaths = manifest.cssPaths;
  const parts = [];
  for (const themeDocument of effectiveThemeCssDocuments(document, manifest.themeName || "")) {
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
  for (const path of declaredCssPaths) {
    const cssDocument = documentByPathForWorkspace(path, document.workspaceRoot || workspaceRoot);
    if (!cssDocument || !isTextDocument(cssDocument)) {
      throw new Error(`Declared CSS asset not found: ${path}`);
    }
    parts.push(rewriteCssAssetUrls(
      cssDocument.source || "",
      directoryName(cssDocument.puzzlePath),
      document.workspaceRoot || workspaceRoot,
    ));
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

function effectiveGameVisualsJs(document, manifest) {
  const scripts = [assetResolverScript(document, manifest)];
  for (const path of manifest.scriptPaths) {
    const scriptDocument = documentByPathForWorkspace(path, document.workspaceRoot || workspaceRoot);
    if (!scriptDocument || !isTextDocument(scriptDocument)) {
      throw new Error(`Declared script asset not found: ${path}`);
    }
    scripts.push(scriptDocument.source || "");
  }
  return scripts.filter(Boolean).join("\n");
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
  const previousActiveFileId = loadedSourceDocumentId;
  const previousPreviewKey = loadedPreviewTargetKey;
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
  const previewTargetKey = recordLoadedPreviewTarget(previewDocument);
  loadedSourceDocumentId = document.id;
  const displayedPreviewBuild = previewBuild
    && previewDocument
    && previewBuild.documentId === previewDocument.id
    ? previewBuild
    : null;
  applyGameCss(displayedPreviewBuild?.gameCss || "");
  applyGameVisuals(displayedPreviewBuild
    ? compiledPreviewGameVisualsJs(displayedPreviewBuild.html)
    : "");
  runButton.disabled = !previewDocument;
  const activeSourceChanged = Boolean(previousActiveFileId && document.id !== previousActiveFileId);
  const previewTargetUnchanged = previewDocument
    && previousPreviewKey
    && previewTargetKey === previousPreviewKey;
  const previewTargetChanged = activeSourceChanged
    && previewDocument
    && previousPreviewKey
    && previewTargetKey !== previousPreviewKey;
  if (previewTargetChanged) {
    invalidateCompiledPreview(previewDocument);
  } else if (previewTargetUnchanged) {
    syncPreviewLevelActionButtons();
  } else {
    invalidateCompiledPreview(previewDocument);
  }
  const previewTargetRequiresCompile = previewDocument
    && !displayedPreviewBuild
    && (!previousPreviewKey || previewTargetChanged);
  if (previewTargetRequiresCompile) {
    const expectedPreviewKey = documentIdentityKey(previewDocument);
    Promise.resolve().then(() => {
      const currentTarget = activePreviewDocument();
      if (
        !currentTarget
        || documentIdentityKey(currentTarget) !== expectedPreviewKey
        || typeof renderPreview !== "function"
      ) {
        return;
      }
      renderPreview().catch((error) => {
        setEditorStatus(`Preview compile failed: ${userFacingRuntimeError(error)}`, "is-error");
      });
    });
  }
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
  selectedFolderId = folder.id;
  selectedTreeId = folder.id;
  renderDocumentSelect();
  updateSourceMeta();
  saveDocumentStore(false);
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
    previewBuild = null;
    previewBuildIsStale = false;
    previewSession = null;
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
  window.PuzzleStudioHost.listenDesktopExitRequested(async (request) => {
    if (desktopExitConfirmationOpen) {
      return;
    }
    desktopExitConfirmationOpen = true;
    try {
      const kind = request?.kind;
      if (kind !== "window" && kind !== "app") {
        throw new Error(`Unsupported desktop exit request: ${String(kind || "missing kind")}`);
      }
      const actionLabel = kind === "app" ? "Quit PuzzleStudio" : "Close this window";
      if (!confirmDesktopExitWithUnsavedChanges(actionLabel)) {
        setEditorStatus(`${kind === "app" ? "Quit" : "Close"} canceled: unsaved changes`, "is-error");
        return;
      }
      await window.PuzzleStudioHost.completeDesktopExit({ kind });
    } catch (error) {
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
