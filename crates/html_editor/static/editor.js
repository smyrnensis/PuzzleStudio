const documentStoreKey = "PuzzleStudioFileTree:v4";
const legacyDocumentStoreKey = "PuzzleStudioEditorStore:v1";
const themeStoreKey = "PuzzleStudioEditorTheme:v1";
const previewVirtualWidth = 1132;
const previewMinimumHeight = 720;
const wasmCompilerAssetVersion = Date.now().toString(36);
const solverProgressIntervalMs = 300;
const solverYieldEveryExpanded = 1;
const solutionPlaybackBaseIntervalMs = 350;
let previewVirtualHeight = previewMinimumHeight;
const boardVirtualCellSize = 56;
const levelEditorEdgeSize = 24;
const levelEditorGap = 6;
const SPRITE_COLOR_PRESETS = [
  "#000000", "#1d2b53", "#7e2553", "#008751",
  "#ab5236", "#5f574f", "#c2c3c7", "#fff1e8",
  "#ff004d", "#ffa300", "#ffec27", "#00e436",
  "#29adff", "#83769c", "#ff77a8", "#ffccaa",
];
const SPRITE_COLOR_TOKENS = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
const BUILTIN_THEME_IMPORTS = window.PuzzleEditorThemeImports || {};
const PREVIEW_THEME_PRESETS = {
  clean: {
    colorScheme: "light",
    bg: "#f5f3ef",
    ink: "#1f2428",
    muted: "#66727c",
    line: "#d7dde2",
    panelBg: "rgba(255, 255, 255, 0.94)",
    background: "var(--preview-game-bg)",
  },
  terminal: {
    colorScheme: "dark",
    bg: "#000000",
    ink: "#ffffff",
    muted: "#ffffff",
    line: "#ffffff",
    panelBg: "#000000",
    background: "var(--preview-game-bg)",
  },
  paper: {
    colorScheme: "light",
    bg: "#f4ecd9",
    ink: "#2b2419",
    muted: "#756852",
    line: "#cdbd9a",
    panelBg: "rgba(255, 250, 240, 0.96)",
    background: "linear-gradient(rgba(255, 255, 255, 0.26), rgba(255, 255, 255, 0.26)), repeating-linear-gradient(0deg, transparent 0 23px, rgba(141, 93, 42, 0.08) 23px 24px), var(--preview-game-bg)",
  },
  pixel: {
    colorScheme: "dark",
    bg: "#08080c",
    ink: "#f8f8f8",
    muted: "#d8d8d8",
    line: "#f8f8f8",
    panelBg: "#08080c",
    background: "var(--preview-game-bg)",
  },
  puzzlescript: {
    colorScheme: "dark",
    bg: "#000000",
    ink: "#ffffff",
    muted: "#ffffff",
    line: "#ffffff",
    panelBg: "#000000",
    background: "var(--preview-game-bg)",
  },
  candy: {
    colorScheme: "light",
    bg: "#fff7fb",
    ink: "#33404a",
    muted: "#7a8790",
    line: "#efbfd3",
    panelBg: "rgba(255, 255, 255, 0.96)",
    background: "repeating-linear-gradient(135deg, rgba(215, 111, 151, 0.045) 0 14px, transparent 14px 28px), var(--preview-game-bg)",
  },
  blueprint: {
    colorScheme: "dark",
    bg: "#0d334e",
    ink: "#e9f8ff",
    muted: "#aad0e0",
    line: "#78c7e8",
    panelBg: "rgba(11, 42, 64, 0.94)",
    background: "repeating-linear-gradient(0deg, rgba(120, 199, 232, 0.11) 0 1px, transparent 1px 24px), repeating-linear-gradient(90deg, rgba(120, 199, 232, 0.11) 0 1px, transparent 1px 24px), var(--preview-game-bg)",
  },
  noir: {
    colorScheme: "dark",
    bg: "#101010",
    ink: "#f4f1e8",
    muted: "#a9a097",
    line: "#59544e",
    panelBg: "rgba(24, 24, 24, 0.96)",
    background: "linear-gradient(90deg, rgba(242, 193, 78, 0.055), transparent 38%, transparent 62%, rgba(242, 193, 78, 0.035)), var(--preview-game-bg)",
  },
};
const EXPLORER_PANE_ID = "explorer";
const SOURCE_WORK_PANE_ID = "source";
const PREVIEW_WORK_PANE_ID = "preview";
const WORK_PANE_IDS = [
  SOURCE_WORK_PANE_ID,
  PREVIEW_WORK_PANE_ID,
  "level",
  "solver",
  "sprite",
  "sounds",
  "psimport",
  "docs",
];

function spriteEditorScaleFactor(scaleInput, maxSize) {
  const factor = Math.trunc(Number(scaleInput?.value) || 2);
  return Math.max(2, Math.min(maxSize, factor));
}

function renderSpriteScaleControl({
  size,
  maxSize,
  scaleInput,
  scaleUpButton,
  scaleDownButton,
  canScaleDown,
  noun,
}) {
  if (!scaleInput || !scaleUpButton || !scaleDownButton) {
    return;
  }
  const maxScale = Math.floor(maxSize / size);
  const factor = spriteEditorScaleFactor(scaleInput, maxSize);
  const capitalizedNoun = `${noun.slice(0, 1).toUpperCase()}${noun.slice(1)}`;
  scaleInput.max = String(Math.max(2, size, maxScale));
  scaleInput.disabled = false;
  scaleUpButton.disabled = maxScale < 2 || factor > maxScale;
  scaleUpButton.title = maxScale < 2
    ? `${capitalizedNoun} is already at maximum size`
    : `Scale up ${noun} by ${factor}x`;
  scaleDownButton.disabled = !canScaleDown(factor);
  scaleDownButton.title = scaleDownButton.disabled
    ? `Scale down requires size divisible by ${factor}`
    : `Scale down ${noun} by ${factor}x`;
}

const PANE_ID_ALIASES = {
  code: SOURCE_WORK_PANE_ID,
  level3d: "level",
  sprite3d: "sprite",
};
const PREVIEW_MODE_TO_WORK_PANE_ID = {
  play: PREVIEW_WORK_PANE_ID,
  edit: "level",
  level3d: "level",
  solver: "solver",
  sprite: "sprite",
  sprite3d: "sprite",
  sounds: "sounds",
  psimport: "psimport",
  docs: "docs",
};
const PREVIEW_MODE_IDS = Object.keys(PREVIEW_MODE_TO_WORK_PANE_ID);
const PREVIEW_HOST_WORK_PANE_IDS = WORK_PANE_IDS.filter((paneId) => paneId !== SOURCE_WORK_PANE_ID);
const MAX_VISIBLE_WORK_PANES = 2;
const WORK_PANE_DEFAULT_WIDTHS = {
  [SOURCE_WORK_PANE_ID]: "42%",
  [PREVIEW_WORK_PANE_ID]: "420px",
  level: "420px",
  solver: "420px",
  sprite: "520px",
  sounds: "420px",
  psimport: "520px",
  docs: "480px",
};

let latestHtml = "";
let previewExport = null;
let previewTimer = 0;
let sourceCursorPreviewKey = "";
let sourceTargetRequestId = 0;
let sourceNavigationBackStack = [];
let sourceNavigationForwardStack = [];
let sourceNavigationRestoring = false;
let previewFrameObjectUrl = "";
let previewFrameLoadId = 0;
let previewViewportSyncFrame = 0;
let previewViewportSyncPasses = 0;
let currentPreviewTheme = null;
let previewDocumentLoaded = false;
let boardScaleSyncFrame = 0;
let boardScaleSyncPasses = 0;
let localSaveTimer = 0;
let statusClearTimer = 0;
let editorStatusClearTimer = 0;
let activePreviewRequest = null;
let wasmCompiler = null;
let wasmCompilerPromise = null;
let previewLogEntries = [];
let explorerPaneVisible = true;
let visibleWorkPanes = [SOURCE_WORK_PANE_ID, PREVIEW_WORK_PANE_ID];
let focusedWorkPaneId = PREVIEW_WORK_PANE_ID;
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
let draggingSplitter = false;
let draggingExplorerSplitter = false;
let draggingPreviewLogSplitter = false;
let draggingPaneSplitterElement = null;
let draggingWorkPaneId = "";
let paneDropTargetId = "";
let paneDropSide = "";
let draggingSplitterPointerId = null;
let draggingExplorerSplitterPointerId = null;
let draggingPreviewLogSplitterPointerId = null;
let previewLogHeightPinned = false;
let pendingExplorerCollapse = false;
let pendingPaneCollapse = "";
let explorerWidthBeforeResize = "";
let paneWidthBeforeResize = "";
let lastExplorerPaneWidth = "";
let lastSplitCodePaneWidth = "";
let resizingPaneEdge = null;
let workPaneWidths = { ...WORK_PANE_DEFAULT_WIDTHS };
let latestPreviewState = null;
let pendingPreviewKeyStateSync = 0;
let previewPaneSourceKey = "";
let activeLevelIndex = 0;
let activeLevelSolveRequest = null;
let levelSolutionPreview = null;
let levelSolutionTimer = 0;
let levelSolveFlashTimer = 0;
let levelSolveFlashRestore = null;
let currentPreviewMode = "play";
let currentLevelPaneMode = "edit";
let currentSpritePaneMode = "sprite";
let psImportConvertTimer = 0;
let levelPaintDrag = null;
let levelPlaytestActive = false;
let spritePaintDrag = null;
let sprite3dPaintDrag = null;
let level = {
  width: 9,
  height: 5,
  selectedObjectId: 0,
  paletteCollapsed: false,
  palette: [],
  regions: [],
  cells: [],
};
let levelDisplayCells = null;
let sprite = {
  size: 5,
  selectedColorIndex: 0,
  addPaletteOpen: false,
  editPaletteOpen: false,
  customColorOpen: false,
  addDraftColorIndex: null,
  paletteBind: null,
  shapeBind: null,
  solidSource: false,
  cells: [],
  palette: [
    { color: "#ff004d" },
  ],
};
let sprite3d = {
  size: 5,
  axis: "z",
  slice: 0,
  editScope: "slice",
  selectedColorIndex: 0,
  addPaletteOpen: false,
  editPaletteOpen: false,
  customColorOpen: false,
  addDraftColorIndex: null,
  palette: [
    { color: "#ff004d" },
  ],
  sliceClipboard: null,
  hoverSlice: null,
  camera: {
    yawDegrees: 340,
    pitchDegrees: 28,
    zoom: 1,
  },
  cells: [],
};
let sounds = {
  mode: "sfx",
  context: null,
  sfxPlayer: null,
  musicPlayer: null,
  musicPlaying: false,
  musicProgress: 0,
  musicRestartTimer: 0,
  progressFrame: 0,
  initialized: false,
};

initializeEditorTheme();
configureFolderImport();
configureDesktopHost();
preloadSourceHighlighter();

function initializeEditorTheme() {
  const theme = normalizeTheme(document.documentElement.dataset.theme);
  applyEditorTheme(theme);
}

function normalizeTheme(theme) {
  return theme === "light" ? "light" : "dark";
}

function applyEditorTheme(theme) {
  const normalized = normalizeTheme(theme);
  document.documentElement.dataset.theme = normalized;
  if (!themeToggleButton) {
    return;
  }
  const dark = normalized === "dark";
  themeToggleButton.setAttribute("aria-pressed", dark ? "true" : "false");
  themeToggleButton.setAttribute("aria-label", dark ? "Switch to light mode" : "Switch to dark mode");
  themeToggleButton.title = dark ? "Switch to light mode" : "Switch to dark mode";
  if (!previewDocumentLoaded) {
    applyUnloadedPreviewTheme();
  }
  if (typeof renderSprite3dPreview === "function") {
    window.requestAnimationFrame(renderSprite3dPreview);
  }
}

function setEditorTheme(theme) {
  const normalized = normalizeTheme(theme);
  try {
    window.localStorage.setItem(themeStoreKey, normalized);
  } catch {
    // Theme persistence is optional; private browsing can reject localStorage.
  }
  applyEditorTheme(normalized);
}

function toggleEditorTheme() {
  setEditorTheme(normalizeTheme(document.documentElement.dataset.theme) === "dark" ? "light" : "dark");
}

async function requestText(url, options = {}) {
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

async function requestJson(url) {
  const response = await fetch(url);
  const body = await response.json();
  if (!response.ok) {
    throw new Error(body.error || response.statusText);
  }
  return body;
}

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

function applyGameCss(css) {
  let style = document.querySelector("#gameStyle");
  if (!style) {
    style = document.createElement("style");
    style.id = "gameStyle";
    const link = document.querySelector("#gameStyleLink");
    if (link) {
      link.replaceWith(style);
    } else {
      document.head.append(style);
    }
  }
  style.textContent = scopeGameCss(css || "");
}

function scopeGameCss(css, scope = ".game-preview-scope") {
  return scopeCssBlock(String(css || ""), scope);
}

function scopeCssBlock(css, scope) {
  let output = "";
  let index = 0;
  while (index < css.length) {
    const open = css.indexOf("{", index);
    if (open < 0) {
      output += css.slice(index);
      break;
    }
    const selector = css.slice(index, open).trim();
    const close = matchingCssBrace(css, open);
    if (close < 0) {
      output += css.slice(index);
      break;
    }
    const body = css.slice(open + 1, close);
    if (selector.startsWith("@media") || selector.startsWith("@supports") || selector.startsWith("@container")) {
      output += `${selector}{${scopeCssBlock(body, scope)}}`;
    } else if (selector.startsWith("@")) {
      output += `${selector}{${body}}`;
    } else {
      output += `${scopeSelectorList(selector, scope)}{${body}}`;
    }
    index = close + 1;
  }
  return output;
}

function matchingCssBrace(css, openIndex) {
  let depth = 0;
  let quote = "";
  for (let index = openIndex; index < css.length; index += 1) {
    const char = css[index];
    const previous = css[index - 1];
    if (quote) {
      if (char === quote && previous !== "\\") {
        quote = "";
      }
      continue;
    }
    if (char === '"' || char === "'") {
      quote = char;
      continue;
    }
    if (char === "{") {
      depth += 1;
    } else if (char === "}") {
      depth -= 1;
      if (depth === 0) {
        return index;
      }
    }
  }
  return -1;
}

function scopeSelectorList(selector, scope) {
  return splitCssSelectors(selector)
    .map((part) => scopeSelector(part, scope))
    .join(", ");
}

function splitCssSelectors(selector) {
  const parts = [];
  let start = 0;
  let depth = 0;
  let quote = "";
  for (let index = 0; index < selector.length; index += 1) {
    const char = selector[index];
    const previous = selector[index - 1];
    if (quote) {
      if (char === quote && previous !== "\\") {
        quote = "";
      }
      continue;
    }
    if (char === '"' || char === "'") {
      quote = char;
      continue;
    }
    if (char === "(" || char === "[") {
      depth += 1;
    } else if (char === ")" || char === "]") {
      depth = Math.max(0, depth - 1);
    } else if (char === "," && depth === 0) {
      parts.push(selector.slice(start, index).trim());
      start = index + 1;
    }
  }
  parts.push(selector.slice(start).trim());
  return parts.filter(Boolean);
}

function scopeSelector(selector, scope) {
  if (selector === ":root" || selector === "html" || selector === "body") {
    return scope;
  }
  if (selector.startsWith(":root ")) {
    return `${scope}${selector.slice(5)}`;
  }
  if (selector.startsWith("html ") || selector.startsWith("body ")) {
    return `${scope} ${selector.slice(5)}`;
  }
  const descendant = `${scope} ${selector}`;
  if (/^[.#[:]/.test(selector)) {
    return `${scope}${selector}, ${descendant}`;
  }
  return descendant;
}

function applyPreviewTheme(theme) {
  const root = playPreview;
  if (!root) {
    return;
  }
  const resolved = resolvePreviewTheme(theme);
  currentPreviewTheme = resolved;
  root.style.setProperty("--preview-game-bg", resolved.bg);
  root.style.setProperty("--preview-game-ink", resolved.ink);
  root.style.setProperty("--preview-game-muted", resolved.muted);
  root.style.setProperty("--preview-game-line", resolved.line);
  root.style.setProperty("--preview-game-panel-bg", resolved.panelBg);
  root.style.setProperty("--preview-game-background", resolved.background);
  root.style.colorScheme = resolved.colorScheme;
}

function setPreviewDocumentLoaded(loaded) {
  previewDocumentLoaded = Boolean(loaded);
  playPreview?.classList.toggle("is-preview-unloaded", !previewDocumentLoaded);
  if (!previewDocumentLoaded) {
    applyUnloadedPreviewTheme();
  }
}

function applyUnloadedPreviewTheme() {
  const root = playPreview;
  if (!root) {
    return;
  }
  currentPreviewTheme = editorPreviewTheme();
  root.style.setProperty("--preview-game-bg", currentPreviewTheme.bg);
  root.style.setProperty("--preview-game-ink", currentPreviewTheme.ink);
  root.style.setProperty("--preview-game-muted", currentPreviewTheme.muted);
  root.style.setProperty("--preview-game-line", currentPreviewTheme.line);
  root.style.setProperty("--preview-game-panel-bg", currentPreviewTheme.panelBg);
  root.style.setProperty("--preview-game-background", currentPreviewTheme.background);
  root.style.colorScheme = currentPreviewTheme.colorScheme;
}

function editorPreviewTheme() {
  const light = normalizeTheme(document.documentElement.dataset.theme) === "light";
  return {
    colorScheme: light ? "light" : "dark",
    bg: editorCssVariable("--workspace-bg", light ? "#edf2f6" : "#1e1e1e"),
    ink: editorCssVariable("--ink", light ? "#20272e" : "#d4d4d4"),
    muted: editorCssVariable("--muted", light ? "#65727d" : "#9da3aa"),
    line: editorCssVariable("--line", light ? "#d6dde3" : "#3c3c3c"),
    danger: editorCssVariable("--danger", light ? "#b32634" : "#b43b43"),
    panelBg: editorCssVariable("--side-bg", light ? "#f8fafc" : "#181818"),
    background: editorCssVariable("--workspace-bg", light ? "#edf2f6" : "#1e1e1e"),
  };
}

function editorCssVariable(name, fallback) {
  return window.getComputedStyle(document.documentElement).getPropertyValue(name).trim() || fallback;
}

function resolvePreviewTheme(theme) {
  const name = previewThemePresetName(theme?.name);
  const preset = PREVIEW_THEME_PRESETS[name] || PREVIEW_THEME_PRESETS.clean;
  const resolved = { ...preset };
  for (const [rawName, rawValue] of Object.entries(theme?.variables || {})) {
    const name = previewThemeVariableName(rawName);
    const value = safePreviewCssValue(rawValue);
    if (!value) {
      continue;
    }
    if (name === "bg") {
      resolved.bg = value;
      resolved.background = "var(--preview-game-bg)";
    } else if (name === "ink") {
      resolved.ink = value;
    } else if (name === "muted") {
      resolved.muted = value;
    } else if (name === "line") {
      resolved.line = value;
    } else if (name === "panel-bg") {
      resolved.panelBg = value;
    }
  }
  return resolved;
}

function previewThemePresetName(name) {
  const normalized = String(name || "clean")
    .replace(/[^a-zA-Z0-9_-]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .toLowerCase();
  return normalized || "clean";
}

function previewThemeVariableName(name) {
  const normalized = String(name || "")
    .replace(/^--/, "")
    .replace(/_/g, "-")
    .toLowerCase();
  return /^[a-z0-9-]*[a-z][a-z0-9-]*$/.test(normalized) ? normalized : "";
}

function safePreviewCssValue(value) {
  const text = String(value || "").trim();
  return /^[a-zA-Z0-9#.,%()+_/: -]+$/.test(text) ? text : "";
}

function ensureGameVisualsRuntime() {
  if (!window.PuzzleSpriteRegistry) {
    window.PuzzleSpriteRegistry = {
      create(config = {}) {
        return {
          aliases: { ...(config.aliases || {}) },
          sprites: { ...(config.sprites || {}) },
          boardClass: config.boardClass || "",
          themeClass: config.themeClass || "",
          editorPuzzle: { ...(config.editorPuzzle || {}) },
          autoAdvanceDelayMs: config.autoAdvanceDelayMs,
        };
      },
    };
  }

  if (window.PuzzleStudio?.registerAssetScript && window.PuzzleStudio?.disposeAssetScripts) {
    return;
  }

  const assetScripts = [];
  const renderCallbacks = [];
  const disposers = [];

  function ensureVisuals() {
    if (!window.GameVisuals) {
      window.GameVisuals = window.PuzzleSpriteRegistry.create();
    }
    return window.GameVisuals;
  }

  function apiFor(definition = {}) {
    return {
      name: definition.name || "",
      onRender(callback) {
        if (typeof callback === "function") {
          renderCallbacks.push(callback);
        }
      },
      setBoardClass(name) {
        ensureVisuals().boardClass = String(name || "");
      },
      setThemeClass(name) {
        ensureVisuals().themeClass = String(name || "");
      },
      addDisposer(callback) {
        if (typeof callback === "function") {
          disposers.push(callback);
        }
      },
      assetUrl(path) {
        return window.PuzzleAssets?.url ? window.PuzzleAssets.url(path) : String(path || "");
      },
    };
  }

  window.PuzzleStudio = {
    registerAssetScript(definition = {}) {
      assetScripts.push(definition);
      if (typeof definition.setup === "function") {
        definition.setup(apiFor(definition));
      }
    },
    dispatchRender(payload = {}) {
      if (!renderCallbacks.length) {
        return;
      }
      window.requestAnimationFrame(() => {
        const event = {
          ...payload,
          board: payload.board || document.querySelector("#board"),
          screenView: payload.screenView || document.querySelector("#screenView"),
          scene: payload.scene || window.__PuzzleCurrentScene,
          state: window.__PuzzleCurrentState,
          assetUrl: (path) => (window.PuzzleAssets?.url ? window.PuzzleAssets.url(path) : String(path || "")),
        };
        for (const callback of renderCallbacks) {
          callback(event);
        }
      });
    },
    disposeAssetScripts() {
      while (disposers.length) {
        const dispose = disposers.pop();
        dispose();
      }
      renderCallbacks.length = 0;
      assetScripts.length = 0;
    },
  };
}

function applyGameVisuals(script) {
  ensureGameVisualsRuntime();
  window.PuzzleStudio.disposeAssetScripts();
  window.GameVisuals = window.PuzzleSpriteRegistry.create();
  if (!script) {
    return;
  }
  try {
    Function(script)();
  } catch (error) {
    window.PuzzleStudio.disposeAssetScripts();
    window.GameVisuals = window.PuzzleSpriteRegistry.create();
    console.error(error);
  }
}

function schedulePreview() {
  window.clearTimeout(previewTimer);
  markPreviewDirty();
}

async function renderPreview() {
  persistCurrentDocument();
  const document = activePreviewDocument();
  if (!isPuzzleDocument(document)) {
    setStatus("No game entry for preview", "is-error");
    return;
  }

  const source = document.source || "";
  updateSourceMeta();
  resetPreviewLog(`Compiling ${document.puzzlePath || "preview"}`);
  setStatus("Compiling", "");
  runButton.disabled = true;

  if (activePreviewRequest) {
    activePreviewRequest.abort();
  }

  const controller = new AbortController();
  activePreviewRequest = controller;

  try {
    const html = await window.PuzzleStudioHost.preview({
      source,
      puzzlePath: document.puzzlePath,
      gameCss: effectiveGameCss(document),
      gameVisualsJs: effectiveGameVisualsJs(document),
    }, { signal: controller.signal });
    applyCompiledPreviewHtml(html, document, source);
  } catch (error) {
    if (error.name === "AbortError") {
      return;
    }
    if (previewBackendUnavailable(error)) {
      try {
        appendPreviewLog("system", "Compiling in browser");
        const html = await compilePreviewWithWasm(document, source);
        applyCompiledPreviewHtml(html, document, source);
        appendPreviewLog("system", "Preview ready");
        return;
      } catch (wasmError) {
        if (editorSeed) {
          appendPreviewLog("warn", "Run Preview needs the editor server or generated browser assets.");
          appendPreviewLog("error", userFacingRuntimeError(wasmError));
          setStatus("Run Preview unavailable", "is-error");
          downloadButton.disabled = !latestHtml;
          return;
        }
        error = wasmError;
      }
    }
    downloadButton.disabled = true;
    appendPreviewLog("error", error.message || String(error));
    setStatus("Compile error", "is-error");
  } finally {
    if (activePreviewRequest === controller) {
      activePreviewRequest = null;
    }
    runButton.disabled = false;
  }
}

function applyCompiledPreviewHtml(html, document, source) {
  latestHtml = html;
  const previousLevelIndex = currentEditableLevelIndex(previewExport);
  previewExport = extractPreviewExport(html);
  setPreviewDocumentLoaded(true);
  applyPreviewTheme(previewExport?.theme || null);
  setActiveLevelIndex(previousLevelIndex, previewExport);
  latestPreviewState = null;
  setPreviewFrameHtml(editorPreviewDocument(html));
  document.source = source;
  document.previewHtml = html;
  applyGameCss(effectiveGameCss(document));
  applyGameVisuals(effectiveGameVisualsJs(document));
  resetLevelBuilderFromPreviewSource();
  if (!level3dBuilder.hidden) {
    renderLevel3dBuilder();
  }
  scheduleLocalSave();
  downloadButton.disabled = false;
  appendPreviewLog("system", "Preview ready");
  setStatus("Preview ready", "is-ok");
}

async function compilePreviewWithWasm(document, source) {
  const compiler = await loadWasmCompiler();
  const expandedSource = expandPuzzleImportsForWasm(source, document.puzzlePath || "game.puzzle");
  const html = compiler.compile_preview(
    expandedSource,
    document.puzzlePath || "game.puzzle",
    effectiveGameCss(document),
    effectiveGameVisualsJs(document),
  );
  return embedStandaloneRuntimeWasm(html);
}

function embedStandaloneRuntimeWasm(html) {
  if (String(html || "").includes("window.PuzzleStandaloneEmbeddedWasm =")) {
    return html;
  }
  const embedded = window.PuzzleStudioEmbeddedWasm;
  if (!embedded?.moduleSource || !embedded?.wasmBase64) {
    return html;
  }
  const exportMarker = "window.PuzzleExport = JSON.parse(";
  const markerIndex = html.indexOf(exportMarker);
  if (markerIndex < 0) {
    return html;
  }
  const scriptEnd = html.indexOf("</script>", markerIndex);
  if (scriptEnd < 0) {
    return html;
  }
  const assignment = `\nwindow.PuzzleStandaloneEmbeddedWasm = { moduleSource: ${JSON.stringify(embedded.moduleSource)}, wasmBase64: ${JSON.stringify(embedded.wasmBase64)} };\n`;
  return `${html.slice(0, scriptEnd)}${assignment}${html.slice(scriptEnd)}`;
}

function expandPuzzleImportsForWasm(source, puzzlePath, importStack = []) {
  const normalizedPath = normalizePath(puzzlePath || "game.puzzle");
  if (importStack.includes(normalizedPath)) {
    throw new Error(`cyclic import: ${[...importStack, normalizedPath].join(" -> ")}`);
  }
  const nextStack = [...importStack, normalizedPath];
  const baseDir = directoryName(normalizedPath);
  const out = [];
  for (const line of expandPuzzleSectionHeadersForWasm(source).split("\n")) {
    const trimmed = line.split("//", 1)[0].trim();
    const match = trimmed.match(/^import\s+"([^"]+)"\s*$/);
    if (!match) {
      out.push(line);
      continue;
    }
    const importPath = resolveWasmImportPath(baseDir, match[1]);
    const imported = documentByPath(importPath);
    if (!imported || !isTextDocument(imported)) {
      const builtinSource = builtinThemeImportSource(importPath);
      if (builtinSource) {
        out.push(expandPuzzleImportsForWasm(builtinSource, importPath, nextStack));
        continue;
      }
      throw new Error(`import not found: ${match[1]} from ${normalizedPath}`);
    }
    out.push(expandPuzzleImportsForWasm(imported.source || "", importPath, nextStack));
  }
  return out.join("\n");
}

function builtinThemeImportSource(path) {
  const normalized = normalizePathSegments(path);
  const parts = normalized.split("/");
  const fileName = parts.at(-1) || "";
  const themePath = `themes/${fileName}`;
  return parts.includes("themes") ? BUILTIN_THEME_IMPORTS[themePath] || "" : "";
}

function expandPuzzleSectionHeadersForWasm(source) {
  const lines = String(source || "").split("\n");
  const out = [];
  let openSection = null;
  let i = 0;
  while (i < lines.length) {
    const section = sectionHeaderAtForWasm(lines, i);
    if (section) {
      if (openSection) {
        out.push("end");
      }
      out.push(section.block);
      openSection = section;
      i += 3;
      continue;
    }

    const line = lines[i];
    const trimmed = stripLineCommentForWasm(line).trim();
    if (openSection && trimmed) {
      const normalizedLine = braceNormalizedLineForSectionForWasm(trimmed);
      if (normalizedLine === "end") {
        if (openSection.nestedDepth === 0) {
          out.push("end");
          openSection = null;
        } else {
          openSection.nestedDepth -= 1;
        }
      } else {
        const tokens = normalizedLine.split(/\s+/).filter(Boolean);
        if (openSection.nestedDepth === 0 && sectionBoundaryForWasm(openSection.block, tokens)) {
          out.push("end");
          openSection = null;
          continue;
        }
        if (startsNestedBlockForWasm(openSection.block, tokens, normalizedLine)) {
          openSection.nestedDepth += 1;
        }
      }
    }

    out.push(line);
    i += 1;
  }
  if (openSection) {
    out.push("end");
  }
  return out.join("\n");
}

function sectionHeaderAtForWasm(lines, start) {
  if (start + 2 >= lines.length) {
    return null;
  }
  const first = stripLineCommentForWasm(lines[start]).trim();
  const title = stripLineCommentForWasm(lines[start + 1]).trim();
  const last = stripLineCommentForWasm(lines[start + 2]).trim();
  if (!isSectionSeparatorForWasm(first) || !isSectionSeparatorForWasm(last)) {
    return null;
  }
  const block = sectionBlockNameForWasm(title);
  return block ? { block, nestedDepth: 0 } : null;
}

function isSectionSeparatorForWasm(line) {
  return line.length >= 3 && /^=+$/.test(line);
}

function sectionBlockNameForWasm(title) {
  const normalized = normalizeSectionTitleForWasm(title);
  if (!normalized) {
    return "";
  }
  return {
    objects: "objects",
    display_object: "display_objects",
    display_objects: "display_objects",
    scratch: "scratch",
    group: "group",
    groups: "group",
    layer: "layers",
    layers: "layers",
    legend: "legend",
    legends: "legend",
    win_condition: "win_conditions",
    win_conditions: "win_conditions",
    lose_condition: "lose_conditions",
    lose_conditions: "lose_conditions",
    sprite: "sprites",
    sprites: "sprites",
    asset: "assets",
    assets: "assets",
    screen: "screen",
    view: "view",
    main: "main",
    rule: "rules",
    rules: "rules",
    transition: "transitions",
    transitions: "transitions",
    level: "levels",
    levels: "levels",
    on_display: "on_display",
    level_start: "on_level_start",
    on_level_start: "on_level_start",
    level_clear: "on_level_clear",
    on_level_clear: "on_level_clear",
    scene_start: "on_scene_start",
    on_scene_start: "on_scene_start",
    state: "state",
    keys: "keys",
    resources: "resources",
    row: "row",
    column: "column",
    box: "box",
    level_menu: "level_menu",
  }[normalized] || "";
}

function normalizeSectionTitleForWasm(title) {
  let normalized = "";
  let previousSeparator = false;
  for (const ch of String(title || "").trim()) {
    if (/^[A-Za-z0-9]$/.test(ch)) {
      normalized += ch.toLowerCase();
      previousSeparator = false;
    } else if (/^\s$/.test(ch) || ch === "_" || ch === "-") {
      if (normalized && !previousSeparator) {
        normalized += "_";
        previousSeparator = true;
      }
    } else {
      return "";
    }
  }
  return previousSeparator ? normalized.slice(0, -1) : normalized;
}

function sectionBoundaryForWasm(block, tokens) {
  if (!tokens.length) {
    return false;
  }
  if (block === "legend") {
    return !isLegendRowForWasm(tokens);
  }
  if (["objects", "display_objects", "scratch", "group", "layers", "collision_layers", "win_conditions", "lose_conditions", "transitions", "levels", "sprites", "assets", "on_display"].includes(block)) {
    return startsPuzzleSectionForWasm(tokens);
  }
  return false;
}

function isLegendRowForWasm(tokens) {
  return tokens.length >= 3 && tokens[1] === "=";
}

function startsPuzzleSectionForWasm(tokens) {
  const first = tokens[0] || "";
  if (["map", "level_start", "on_level_start", "level_clear", "on_level_clear", "on_display", "objects", "display_objects", "scratch", "group", "layers", "collision_layers", "legend", "sprites", "assets", "screen", "view", "effect", "rule", "rules", "main", "transitions", "levels", "level", "resources"].includes(first)) {
    return true;
  }
  return first === "win_conditions" || first === "lose_conditions";
}

function startsNestedBlockForWasm(block, tokens, line) {
  if (block === "legend") {
    return false;
  }
  if (block === "levels") {
    return tokens[0] === "level" || (tokens.length === 1 && isIdentifierForWasm(tokens[0])) || startsInlineBlockForWasm(tokens, line);
  }
  return startsInlineBlockForWasm(tokens, line);
}

function startsInlineBlockForWasm(tokens, line) {
  const first = tokens[0] || "";
  return ["map", "level_start", "on_level_start", "level_clear", "on_level_clear", "on_display", "objects", "display_objects", "scratch", "group", "layers", "collision_layers", "legend", "sprites", "assets", "screen", "view", "effect", "rule", "rules", "main", "transitions", "levels", "level", "state", "keys", "resources", "scene_start", "on_scene_start", "transition", "input", "component_effect", "action", "if", "row", "column", "box", "for", "level_menu", "fix", "repeat", "once", "once_all", "once_per_level", "display"].includes(first)
    || first === "win_conditions"
    || first === "lose_conditions"
    || (tokens[0] === "menu" && (tokens.length === 2 || (tokens.length === 5 && tokens[2] === "=" && tokens[4] === "with")))
    || (tokens[0] === "button" && line.trimEnd().endsWith(" with"));
}

function braceNormalizedLineForSectionForWasm(line) {
  if (line === "}") {
    return "end";
  }
  if (line === "else {" || line === "else{") {
    return "else";
  }
  if (line.endsWith("{")) {
    return line.slice(0, -1).trimEnd();
  }
  return line;
}

function stripLineCommentForWasm(line) {
  return String(line || "").split("//", 1)[0];
}

function isIdentifierForWasm(value) {
  return /^[_A-Za-z][_A-Za-z0-9]*$/.test(value || "");
}

function resolveWasmImportPath(baseDir, importPath) {
  const normalized = normalizePath(importPath);
  if (!normalized || normalized.startsWith("/")) {
    return normalizePath(normalized.replace(/^\/+/, ""));
  }
  return normalizePathSegments(baseDir ? `${baseDir}/${normalized}` : normalized);
}

function normalizePathSegments(path) {
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

function documentByPath(path) {
  const target = normalizePath(path);
  return documents.find((candidate) => normalizePath(candidate.puzzlePath) === target) || null;
}

async function loadWasmCompiler() {
  if (!wasmCompilerPromise) {
    const version = encodeURIComponent(wasmCompilerAssetVersion);
    const embedded = window.PuzzleStudioEmbeddedWasm;
    wasmCompilerPromise = (
      embedded?.moduleSource && embedded?.wasmBase64
        ? loadEmbeddedWasmCompiler(embedded, version)
        : loadExternalWasmCompiler(version)
    )
      .catch((error) => {
        wasmCompiler = null;
        wasmCompilerPromise = null;
        throw error;
      });
  }
  return wasmCompilerPromise;
}

async function loadExternalWasmCompiler(version) {
  const module = await import(`./wasm/puzzle_wasm.js?v=${version}`);
  await module.default(`./wasm/puzzle_wasm_bg.wasm?v=${version}`);
  wasmCompiler = module;
  return module;
}

async function loadEmbeddedWasmCompiler(embedded, version) {
  const url = URL.createObjectURL(new Blob([embedded.moduleSource], {
    type: "text/javascript",
  }));
  try {
    const module = await import(`${url}#${version}`);
    await module.default({ module_or_path: base64ToUint8Array(embedded.wasmBase64) });
    wasmCompiler = module;
    return module;
  } finally {
    URL.revokeObjectURL(url);
  }
}

function base64ToUint8Array(value) {
  const binary = atob(value || "");
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
}

function preloadSourceHighlighter() {
  loadWasmCompiler()
    .then(() => {
      renderSourceHighlightWithLoadedWasm();
    })
    .catch(() => {
      // Server highlighting is still available in the local editor.
    });
}

function previewBackendUnavailable(error) {
  if (error instanceof TypeError) {
    return true;
  }
  return [404, 405, 501].includes(Number(error?.status));
}

function markEmbeddedPreviewDirty() {
  markPreviewDirty();
}

function markPreviewDirty() {
  const current = activeDocument();
  if (current && isTextDocument(current)) {
    current.source = sourceEditor.value;
  }
  // Keep the last compiled export available for play/edit/solver rendering
  // while marking it stale. Run Preview performs the explicit recompile when
  // a server backend is available.
  latestPreviewState = null;
  scheduleLocalSave();
  downloadButton.disabled = true;
  setStatus("Preview is stale", "");
}

function updateSourceMeta() {
  const source = sourceEditor.value;
  const lineCount = source.length ? source.split("\n").length : 0;
  sourceMeta.textContent = `${lineCount} lines`;
}

function setStatus(text, className) {
  window.clearTimeout(statusClearTimer);
  statusLabel.className = `pane-status ${className || ""}`.trim();
  statusLabel.textContent = text;
  if (text && className === "is-ok") {
    statusClearTimer = window.setTimeout(() => {
      if (statusLabel.textContent === text && statusLabel.classList.contains("is-ok")) {
        statusLabel.textContent = "";
        statusLabel.className = "pane-status";
      }
    }, 1800);
  }
}

function setEditorStatus(text, className) {
  window.clearTimeout(editorStatusClearTimer);
  editorStatusLabel.className = `document-status ${className || ""}`.trim();
  editorStatusLabel.textContent = text;
  if (text && className === "is-ok") {
    editorStatusClearTimer = window.setTimeout(() => {
      if (editorStatusLabel.textContent === text && editorStatusLabel.classList.contains("is-ok")) {
        editorStatusLabel.textContent = "";
        editorStatusLabel.className = "document-status";
      }
    }, 1800);
  }
}

function resetPreviewLog(message = "waiting for preview output") {
  previewLogEntries = [];
  appendPreviewLog("system", message);
}

function appendPreviewLog(level, message) {
  const normalizedLevel = ["system", "info", "log", "warn", "error", "debug"].includes(level)
    ? level
    : "log";
  const text = String(message || "").trimEnd();
  previewLogEntries.push({
    level: normalizedLevel,
    message: text || "(empty)",
    time: new Date(),
  });
  if (previewLogEntries.length > 200) {
    previewLogEntries = previewLogEntries.slice(-200);
  }
  renderPreviewLog();
}

function clearPreviewLog() {
  previewLogEntries = [];
  renderPreviewLog();
}

function renderPreviewLog() {
  if (!previewLogOutput) {
    return;
  }
  previewLogOutput.replaceChildren();
  if (!previewLogEntries.length) {
    const empty = document.createElement("div");
    empty.className = "preview-log-line is-muted";
    empty.textContent = "$ waiting for preview output";
    previewLogOutput.append(empty);
    return;
  }
  for (const entry of previewLogEntries) {
    const line = document.createElement("div");
    const classLevel = entry.level === "log" || entry.level === "info" || entry.level === "debug"
      ? ""
      : ` is-${entry.level}`;
    line.className = `preview-log-line${classLevel}`;
    const label = entry.level === "system" ? "system" : entry.level;
    line.textContent = `[${entry.time.toLocaleTimeString()}] ${label}: ${entry.message}`;
    previewLogOutput.append(line);
  }
  previewLogOutput.scrollTop = previewLogOutput.scrollHeight;
}

function normalizePaneId(paneId) {
  return PANE_ID_ALIASES[paneId] || paneId || "";
}

function isWorkPaneId(paneId) {
  return WORK_PANE_IDS.includes(normalizePaneId(paneId));
}

function isPreviewHostPaneId(paneId) {
  return PREVIEW_HOST_WORK_PANE_IDS.includes(normalizePaneId(paneId));
}

function isPreviewHostVisible() {
  return visibleWorkPanes.some((paneId) => isPreviewHostPaneId(paneId));
}

function isPaneVisible(paneId) {
  const normalized = normalizePaneId(paneId);
  if (normalized === EXPLORER_PANE_ID) {
    return explorerPaneVisible;
  }
  return visibleWorkPanes.includes(normalized);
}

function normalizePreviewMode(mode) {
  return PREVIEW_MODE_IDS.includes(mode) ? mode : "play";
}

function workPaneIdForPreviewMode(mode) {
  return PREVIEW_MODE_TO_WORK_PANE_ID[normalizePreviewMode(mode)] || PREVIEW_WORK_PANE_ID;
}

function previewModeForWorkPaneId(paneId) {
  const normalized = normalizePaneId(paneId);
  return Object.entries(PREVIEW_MODE_TO_WORK_PANE_ID)
    .find(([, workPaneId]) => workPaneId === normalized)?.[0] || "play";
}

function workPaneElementForPaneId(paneId) {
  const normalized = normalizePaneId(paneId);
  if (normalized === SOURCE_WORK_PANE_ID) {
    return workbench.querySelector(".code-pane");
  }
  if (isPreviewHostPaneId(normalized)) {
    return workbench.querySelector(`.preview-pane[data-work-pane="${normalized}"]`);
  }
  return null;
}

function toolPaneTitle(paneId) {
  return {
    [PREVIEW_WORK_PANE_ID]: "Preview",
    level: "Level",
    level3d: "3D Level",
    solver: "Solve",
    sprite: "Sprite",
    sounds: "Sound",
    psimport: "PuzzleScript import",
    docs: "Docs",
  }[paneId] || paneId;
}

function toolPanePanelForPaneId(paneId) {
  return {
    [PREVIEW_WORK_PANE_ID]: playPreview,
    level: levelBuilder,
    level3d: level3dBuilder,
    solver: solverPanel,
    sprite: spriteBuilder,
    sounds: soundsBuilder,
    psimport: psImportPanel,
    docs: docsPanel,
  }[paneId] || null;
}

function createPaneCloseButton(paneId) {
  const button = document.createElement("button");
  button.className = "pane-close-button";
  button.type = "button";
  button.dataset.paneClose = paneId;
  button.setAttribute("aria-label", `Hide ${toolPaneTitle(paneId)} pane`);
  button.title = `Hide ${toolPaneTitle(paneId)} pane`;
  button.innerHTML = `
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M18 6 6 18"></path>
      <path d="m6 6 12 12"></path>
    </svg>
  `;
  return button;
}

function createToolPane(paneId, panel) {
  const pane = document.createElement("section");
  pane.className = "preview-pane";
  pane.dataset.workPane = paneId;
  pane.setAttribute("aria-label", toolPaneTitle(paneId));

  const header = document.createElement("div");
  header.className = "pane-header";
  header.dataset.paneDragHandle = paneId;
  header.draggable = true;
  header.title = `Drag header to move ${toolPaneTitle(paneId)} pane`;
  const title = document.createElement("div");
  title.className = "pane-title";
  const label = document.createElement("span");
  label.className = "tool-pane-title";
  label.textContent = toolPaneTitle(paneId);
  const actions = document.createElement("div");
  actions.className = "pane-actions";
  title.append(label);
  if (paneId === "level" && levelPaneModeSwitch) {
    title.append(levelPaneModeSwitch);
  }
  if (paneId === "sprite" && spritePaneModeSwitch) {
    title.append(spritePaneModeSwitch);
  }
  actions.append(createPaneCloseButton(paneId));
  header.append(title, actions);
  pane.append(header, panel);
  if (paneId === "level" && level3dBuilder) {
    pane.append(level3dBuilder);
  }
  if (paneId === "sprite" && sprite3dBuilder) {
    pane.append(sprite3dBuilder);
  }
  return pane;
}

function initializePhysicalWorkPanes() {
  const previewPane = workbench.querySelector(".preview-pane");
  if (!previewPane || previewPane.dataset.physicalPanesInitialized === "true") {
    return;
  }
  previewPane.dataset.physicalPanesInitialized = "true";
  previewPane.dataset.workPane = PREVIEW_WORK_PANE_ID;
  previewPane.setAttribute("aria-label", toolPaneTitle(PREVIEW_WORK_PANE_ID));

  const previewDragHandle = previewPane.querySelector("[data-pane-drag-handle]");
  if (previewDragHandle) {
    previewDragHandle.dataset.paneDragHandle = PREVIEW_WORK_PANE_ID;
    previewDragHandle.draggable = true;
    previewDragHandle.title = "Drag header to move Preview pane";
  }
  const previewClose = previewPane.querySelector("[data-pane-close]");
  if (previewClose) {
    previewClose.className = "pane-close-button";
    previewClose.dataset.paneClose = PREVIEW_WORK_PANE_ID;
    previewClose.setAttribute("aria-label", "Hide Preview pane");
    previewClose.title = "Hide Preview pane";
    previewClose.innerHTML = `
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M18 6 6 18"></path>
        <path d="m6 6 12 12"></path>
      </svg>
    `;
  }
  if (gamePaneTitle) {
    gamePaneTitle.textContent = toolPaneTitle(PREVIEW_WORK_PANE_ID);
  }
  if (runButton) {
    runButton.hidden = false;
  }
  if (levelPaneModeSwitch) {
    levelPaneModeSwitch.hidden = true;
  }
  if (spritePaneModeSwitch) {
    spritePaneModeSwitch.hidden = true;
  }

  for (const paneId of PREVIEW_HOST_WORK_PANE_IDS) {
    if (paneId === PREVIEW_WORK_PANE_ID) {
      continue;
    }
    const panel = toolPanePanelForPaneId(paneId);
    if (!panel) {
      continue;
    }
    workbench.append(createToolPane(paneId, panel));
  }
}

function layoutPaneIdsFor(paneIds, previewMode = currentPreviewMode || "play", options = {}) {
  const result = [];
  for (const paneId of paneIds) {
    const normalized = normalizePaneId(paneId);
    if (!isWorkPaneId(normalized) || result.includes(normalized)) {
      continue;
    }
    const element = workPaneElementForPaneId(normalized);
    if (!element) {
      continue;
    }
    result.push(normalized);
  }
  if (result.length || options.allowEmpty) {
    return result;
  }
  return [SOURCE_WORK_PANE_ID];
}

function layoutPaneIds() {
  return layoutPaneIdsFor(visibleWorkPanes);
}

function normalizeVisibleWorkPaneList(paneIds, options = {}) {
  const limit = Number.isFinite(options.limit) ? options.limit : MAX_VISIBLE_WORK_PANES;
  const next = [];
  for (const paneId of paneIds || []) {
    const normalized = normalizePaneId(paneId);
    if (isWorkPaneId(normalized) && !next.includes(normalized) && workPaneElementForPaneId(normalized)) {
      next.push(normalized);
    }
    if (next.length >= limit) {
      break;
    }
  }
  if (!next.length && !options.allowEmpty) {
    next.push(SOURCE_WORK_PANE_ID);
  }
  return next;
}

function workPaneWidth(paneId) {
  return workPaneWidths[paneId] || WORK_PANE_DEFAULT_WIDTHS[paneId] || "420px";
}

function setWorkPaneWidth(paneId, width) {
  if (!isWorkPaneId(paneId) || !width) {
    return;
  }
  workPaneWidths[paneId] = width;
  if (paneId === SOURCE_WORK_PANE_ID) {
    lastSplitCodePaneWidth = width;
    workbench.style.setProperty("--code-pane-width", width);
  }
}

function renderedWorkPaneWidth(paneId) {
  const element = workPaneElementForPaneId(paneId);
  const width = element?.getBoundingClientRect?.().width || 0;
  return width > 0 ? `${Math.round(width)}px` : "";
}

function inheritReplacedPaneSlotWidth(replacedPaneId, nextPaneId, replaceIndex, paneCount) {
  if (!isWorkPaneId(replacedPaneId) || !isWorkPaneId(nextPaneId) || replaceIndex >= paneCount - 1) {
    return;
  }
  const width = renderedWorkPaneWidth(replacedPaneId) || workPaneWidth(replacedPaneId);
  setWorkPaneWidth(nextPaneId, width);
}

function ensurePaneSplitterCount(count) {
  const splitters = Array.from(workbench.querySelectorAll(".pane-splitter"));
  while (splitters.length < count) {
    const splitter = document.createElement("div");
    splitter.className = "pane-splitter";
    splitter.setAttribute("role", "separator");
    splitter.setAttribute("aria-label", "Resize panes");
    splitter.setAttribute("aria-orientation", "vertical");
    splitter.addEventListener("pointerdown", startPaneResize);
    splitter.addEventListener("lostpointercapture", stopPaneResize);
    workbench.append(splitter);
    splitters.push(splitter);
  }
  splitters.forEach((splitter, index) => {
    splitter.hidden = index >= count;
  });
  return splitters.slice(0, count);
}

function syncWorkbenchGridLayout() {
  const panes = layoutPaneIds();
  const columns = [];
  let columnIndex = 1;

  if (explorerPaneVisible) {
    columns.push("var(--explorer-pane-width)");
    columns.push("6px");
    const explorer = workbench.querySelector(".explorer-pane");
    if (explorer) {
      explorer.style.gridColumn = String(columnIndex);
    }
    if (explorerSplitter) {
      explorerSplitter.style.gridColumn = String(columnIndex + 1);
      explorerSplitter.hidden = false;
    }
    columnIndex += 2;
  } else if (explorerSplitter) {
    explorerSplitter.hidden = true;
  }

  const splitters = ensurePaneSplitterCount(Math.max(0, panes.length - 1));
  panes.forEach((paneId, index) => {
    const isLast = index === panes.length - 1;
    columns.push(isLast ? "minmax(0, 1fr)" : `minmax(0, ${workPaneWidth(paneId)})`);
    const element = workPaneElementForPaneId(paneId);
    if (element) {
      element.style.gridColumn = String(columnIndex);
    }
    columnIndex += 1;
    const splitter = splitters[index];
    if (splitter) {
      columns.push("6px");
      splitter.style.gridColumn = String(columnIndex);
      splitter.dataset.leftPane = paneId;
      splitter.dataset.rightPane = panes[index + 1];
      columnIndex += 1;
    }
  });

  workbench.style.setProperty("--workbench-grid-columns", columns.join(" "));
}

function focusWorkPane(paneId) {
  const normalized = normalizePaneId(paneId);
  if (!isWorkPaneId(normalized)) {
    return false;
  }
  focusedWorkPaneId = normalized;
  workbench.dataset.focusedWorkPane = focusedWorkPaneId;
  return true;
}

function setVisibleWorkPanes(nextPaneIds) {
  const next = normalizeVisibleWorkPaneList(nextPaneIds);
  if (!next.length) {
    return false;
  }
  visibleWorkPanes = next;
  return true;
}

function workPaneIdToReplaceForOpen(options = {}) {
  const requested = normalizePaneId(options.replacePaneId || "");
  if (visibleWorkPanes.includes(requested)) {
    return requested;
  }
  if (visibleWorkPanes.includes(focusedWorkPaneId)) {
    const unfocused = visibleWorkPanes.find((paneId) => paneId !== focusedWorkPaneId);
    if (unfocused) {
      return unfocused;
    }
  }
  return visibleWorkPanes[visibleWorkPanes.length - 1] || "";
}

function activePreviewWorkPaneId() {
  const active = workPaneIdForPreviewMode(currentPreviewMode || "play");
  if (visibleWorkPanes.includes(active)) {
    return active;
  }
  return visibleWorkPanes.find((paneId) => isPreviewHostPaneId(paneId)) || active;
}

function normalizePaneDragId(paneId) {
  return paneId === "active-preview" ? activePreviewWorkPaneId() : normalizePaneId(paneId);
}

function workPaneElementForDragEvent(event) {
  const element = event.target?.closest?.(".code-pane, .preview-pane[data-work-pane]");
  return element && workbench.contains(element) ? element : null;
}

function workPaneIdForElement(element) {
  return normalizePaneId(element?.dataset?.workPane || (element?.classList.contains("code-pane") ? SOURCE_WORK_PANE_ID : ""));
}

function paneDropSideForEvent(event, element) {
  const rect = element.getBoundingClientRect();
  return event.clientX < rect.left + rect.width / 2 ? "before" : "after";
}

function shouldIgnorePaneDragStart(event) {
  return Boolean(event.target?.closest?.(
    "button, input, textarea, select, a, [role='button'], [role='group'], .document-tab"
  ));
}

function clearWorkPaneDropState(options = {}) {
  paneDropTargetId = "";
  paneDropSide = "";
  for (const element of workbench.querySelectorAll(".code-pane, .preview-pane[data-work-pane]")) {
    element.classList.remove("is-pane-drop-before", "is-pane-drop-after");
    if (!options.keepDragSource) {
      element.classList.remove("is-pane-drag-source");
    }
  }
  if (!options.keepDragSource) {
    workbench.classList.remove("is-dragging-work-pane");
  }
}

function markWorkPaneDropTarget(targetPaneId, side) {
  clearWorkPaneDropState({ keepDragSource: true });
  paneDropTargetId = targetPaneId;
  paneDropSide = side;
  const element = workPaneElementForPaneId(targetPaneId);
  element?.classList.add(side === "before" ? "is-pane-drop-before" : "is-pane-drop-after");
}

function moveWorkPane(draggedPaneId, targetPaneId, side) {
  const dragged = normalizePaneDragId(draggedPaneId);
  const target = normalizePaneDragId(targetPaneId);
  if (!isWorkPaneId(dragged) || !isWorkPaneId(target) || dragged === target) {
    return false;
  }
  const next = visibleWorkPanes.filter((paneId) => paneId !== dragged);
  const targetIndex = next.indexOf(target);
  if (targetIndex < 0) {
    return false;
  }
  next.splice(side === "after" ? targetIndex + 1 : targetIndex, 0, dragged);
  visibleWorkPanes = next;
  focusWorkPane(dragged);
  applyPaneVisibility();
  return true;
}

function startWorkPaneDrag(event) {
  if (shouldIgnorePaneDragStart(event)) {
    event.preventDefault();
    return;
  }
  const handle = event.target?.closest?.("[data-pane-drag-handle]");
  const paneId = normalizePaneDragId(handle?.dataset.paneDragHandle);
  if (!isWorkPaneId(paneId) || !isPaneVisible(paneId)) {
    event.preventDefault();
    return;
  }
  draggingWorkPaneId = paneId;
  paneDropTargetId = "";
  paneDropSide = "";
  workbench.classList.add("is-dragging-work-pane");
  workPaneElementForPaneId(paneId)?.classList.add("is-pane-drag-source");
  event.dataTransfer.effectAllowed = "move";
  event.dataTransfer.setData("text/x-puzzle-work-pane", paneId);
  event.dataTransfer.setData("text/plain", paneId);
}

function handleWorkPaneDragOver(event) {
  if (!draggingWorkPaneId) {
    return;
  }
  const targetElement = workPaneElementForDragEvent(event);
  const targetPaneId = workPaneIdForElement(targetElement);
  if (!targetElement || !isWorkPaneId(targetPaneId)) {
    return;
  }
  event.preventDefault();
  event.dataTransfer.dropEffect = "move";
  const side = paneDropSideForEvent(event, targetElement);
  if (targetPaneId === draggingWorkPaneId) {
    clearWorkPaneDropState({ keepDragSource: true });
    return;
  }
  if (paneDropTargetId !== targetPaneId || paneDropSide !== side) {
    markWorkPaneDropTarget(targetPaneId, side);
  }
}

function handleWorkPaneDrop(event) {
  if (!draggingWorkPaneId) {
    return;
  }
  event.preventDefault();
  const targetElement = workPaneElementForDragEvent(event);
  const targetPaneId = workPaneIdForElement(targetElement);
  const side = targetElement ? paneDropSideForEvent(event, targetElement) : paneDropSide;
  moveWorkPane(draggingWorkPaneId, targetPaneId || paneDropTargetId, side || paneDropSide || "after");
  draggingWorkPaneId = "";
  clearWorkPaneDropState();
}

function handleWorkPaneFocus(event) {
  const element = workPaneElementForDragEvent(event);
  const paneId = workPaneIdForElement(element);
  if (element && isPaneVisible(paneId)) {
    focusWorkPane(paneId);
  }
}

function stopWorkPaneDrag() {
  draggingWorkPaneId = "";
  clearWorkPaneDropState();
}

function selectFallbackPreviewPane(closedPaneId) {
  if (!isPreviewHostPaneId(closedPaneId) || workPaneIdForPreviewMode(currentPreviewMode) !== closedPaneId) {
    return;
  }
  const nextPreviewPane = visibleWorkPanes.find((paneId) => isPreviewHostPaneId(paneId));
  if (nextPreviewPane) {
    setPreviewMode(previewModeForWorkPaneId(nextPreviewPane), { skipPaneSync: true });
  }
}

function closeWorkPane(paneId) {
  const normalized = paneId === "active-preview" ? activePreviewWorkPaneId() : normalizePaneId(paneId);
  const next = visibleWorkPanes.filter((candidate) => candidate !== normalized);
  if (!isWorkPaneId(normalized) || !visibleWorkPanes.includes(normalized) || !layoutPaneIdsFor(next, currentPreviewMode, { allowEmpty: true }).length) {
    return false;
  }
  visibleWorkPanes = next;
  selectFallbackPreviewPane(normalized);
  applyPaneVisibility();
  return true;
}

function showWorkPane(paneId, options = {}) {
  const normalized = normalizePaneId(paneId);
  if (!isWorkPaneId(normalized)) {
    return false;
  }
  visibleWorkPanes = normalizeVisibleWorkPaneList(visibleWorkPanes);
  if (visibleWorkPanes.includes(normalized)) {
    if (options.focus !== false) {
      focusWorkPane(normalized);
    }
    applyPaneVisibility();
    return true;
  }
  if (visibleWorkPanes.length < MAX_VISIBLE_WORK_PANES) {
    visibleWorkPanes.push(normalized);
  } else {
    const replacePaneId = workPaneIdToReplaceForOpen(options);
    const replaceIndex = visibleWorkPanes.indexOf(replacePaneId);
    if (replaceIndex < 0) {
      inheritReplacedPaneSlotWidth(visibleWorkPanes[visibleWorkPanes.length - 1], normalized, visibleWorkPanes.length - 1, visibleWorkPanes.length);
      visibleWorkPanes[visibleWorkPanes.length - 1] = normalized;
    } else {
      inheritReplacedPaneSlotWidth(replacePaneId, normalized, replaceIndex, visibleWorkPanes.length);
      visibleWorkPanes[replaceIndex] = normalized;
    }
  }
  visibleWorkPanes = normalizeVisibleWorkPaneList(visibleWorkPanes);
  if (options.focus !== false) {
    focusWorkPane(normalized);
  }
  applyPaneVisibility();
  return true;
}

function showPreviewModePane(mode, options = {}) {
  const paneId = workPaneIdForPreviewMode(mode);
  if (options.replaceWorkPanes) {
    if (!setVisibleWorkPanes([paneId])) {
      return false;
    }
    if (options.focus !== false) {
      focusWorkPane(paneId);
    }
    applyPaneVisibility();
    return true;
  }
  return showWorkPane(paneId, options);
}

function openPreviewModePane(mode, options = {}) {
  const previewMode = normalizePreviewMode(mode);
  showPreviewModePane(previewMode, {
    focus: true,
    replaceWorkPanes: options.replaceWorkPanes === true,
  });
  setPreviewMode(previewMode, { skipPaneSync: true });
}

function setVisiblePanes(nextPanes) {
  const requested = Array.from(nextPanes || []).map((paneId) => normalizePaneId(paneId));
  const nextExplorerVisible = requested.includes(EXPLORER_PANE_ID);
  const nextWorkPanes = requested.filter((paneId) => isWorkPaneId(paneId));
  if (!setVisibleWorkPanes(nextWorkPanes)) {
    return;
  }
  explorerPaneVisible = nextExplorerVisible;
  applyPaneVisibility();
}

function applyPaneVisibility() {
  visibleWorkPanes = normalizeVisibleWorkPaneList(visibleWorkPanes);
  if (explorerPaneVisible && lastExplorerPaneWidth) {
    workbench.style.setProperty("--explorer-pane-width", lastExplorerPaneWidth);
  }
  if (lastSplitCodePaneWidth) {
    setWorkPaneWidth(SOURCE_WORK_PANE_ID, lastSplitCodePaneWidth);
  }
  pendingExplorerCollapse = false;
  pendingPaneCollapse = "";
  if (!visibleWorkPanes.includes(focusedWorkPaneId)) {
    focusWorkPane(visibleWorkPanes[visibleWorkPanes.length - 1] || SOURCE_WORK_PANE_ID);
  } else {
    workbench.dataset.focusedWorkPane = focusedWorkPaneId;
  }
  workbench.dataset.activePreviewMode = currentPreviewMode || "play";
  workbench.dataset.activePreviewPane = workPaneIdForPreviewMode(currentPreviewMode || "play");
  workbench.classList.toggle("is-explorer-hidden", !explorerPaneVisible);
  workbench.classList.toggle("is-code-hidden", !isPaneVisible(SOURCE_WORK_PANE_ID));
  workbench.classList.toggle("is-preview-hidden", !isPreviewHostVisible());
  workbench.dataset.collapsingPane = "";
  workbench.dataset.collapsingPreview = "false";
  for (const paneId of WORK_PANE_IDS) {
    const element = workPaneElementForPaneId(paneId);
    const visible = isPaneVisible(paneId);
    if (element) {
      element.hidden = !visible;
    }
    if (paneId === "level") {
      if (levelBuilder) {
        levelBuilder.hidden = !visible || currentLevelPaneMode !== "edit";
      }
      if (level3dBuilder) {
        level3dBuilder.hidden = !visible || currentLevelPaneMode !== "level3d";
      }
      continue;
    }
    if (paneId === "sprite") {
      if (spriteBuilder) {
        spriteBuilder.hidden = !visible || currentSpritePaneMode !== "sprite";
      }
      if (sprite3dBuilder) {
        sprite3dBuilder.hidden = !visible || currentSpritePaneMode !== "sprite3d";
      }
      continue;
    }
    const panel = toolPanePanelForPaneId(paneId);
    if (panel) {
      panel.hidden = !visible;
    }
  }
  if (levelPaneModeSwitch) {
    levelPaneModeSwitch.hidden = !isPaneVisible("level");
  }
  if (spritePaneModeSwitch) {
    spritePaneModeSwitch.hidden = !isPaneVisible("sprite");
  }
  syncWorkbenchGridLayout();
  paneToggleButtons.forEach((button) => {
    const normalized = normalizePaneId(button.dataset.paneToggle);
    const active = normalized === PREVIEW_WORK_PANE_ID
      ? isPreviewHostVisible()
      : isPaneVisible(normalized);
    button.classList.toggle("is-active", active);
    button.setAttribute("aria-pressed", active ? "true" : "false");
  });
  document.querySelectorAll("[data-pane-close]").forEach((button) => {
    const paneId = button.dataset.paneClose === "active-preview"
      ? activePreviewWorkPaneId()
      : normalizePaneId(button.dataset.paneClose);
    const next = visibleWorkPanes.filter((candidate) => candidate !== paneId);
    const disabled = !isWorkPaneId(paneId) || !isPaneVisible(paneId) || !layoutPaneIdsFor(next, currentPreviewMode, { allowEmpty: true }).length;
    button.disabled = disabled;
    button.setAttribute("aria-disabled", String(disabled));
  });
  syncPreviewModeButtonState();
  scheduleBoardScaleSync();
  requestAnimationFrame(syncPreviewViewportScale);
}

function togglePaneVisibility(pane) {
  const normalized = normalizePaneId(pane);
  if (normalized === EXPLORER_PANE_ID) {
    explorerPaneVisible = !explorerPaneVisible;
    applyPaneVisibility();
    return;
  }
  if (!isWorkPaneId(normalized)) {
    return;
  }
  if (normalized === PREVIEW_WORK_PANE_ID) {
    showPreviewModePane(currentPreviewMode);
    return;
  }
  if (isPaneVisible(normalized)) {
    focusWorkPane(normalized);
  } else {
    showWorkPane(normalized);
  }
  applyPaneVisibility();
}

function revealPreviewPane() {
  showWorkPane(PREVIEW_WORK_PANE_ID);
}

function revealCodePane() {
  showWorkPane(SOURCE_WORK_PANE_ID);
}

function activeResizePointerMatches(event, pointerId) {
  return !event || pointerId === null || event.pointerId === pointerId;
}

function releasePointerCaptureIfHeld(element, pointerId) {
  if (!element || pointerId === null) {
    return;
  }
  try {
    if (element.hasPointerCapture(pointerId)) {
      element.releasePointerCapture(pointerId);
    }
  } catch {
    // The pointer can already be gone after blur/cancel or browser edge release.
  }
}

function stopActiveResize(event) {
  stopPaneResize(event);
  stopExplorerResize(event);
  stopPreviewLogResize(event);
}

function handleExplorerToggleShortcut(event) {
  if (!(event.metaKey && !event.ctrlKey && !event.altKey && !event.shiftKey && event.key.toLowerCase() === "b")) {
    return false;
  }
  event.preventDefault();
  event.stopImmediatePropagation();
  togglePaneVisibility("explorer");
  return true;
}

function startPaneResize(event) {
  const splitter = event.currentTarget;
  const leftPaneId = normalizePaneId(splitter?.dataset.leftPane);
  const rightPaneId = normalizePaneId(splitter?.dataset.rightPane);
  if (!isWorkPaneId(leftPaneId) || !isWorkPaneId(rightPaneId)) {
    return;
  }
  stopActiveResize();
  draggingSplitter = true;
  draggingPaneSplitterElement = splitter;
  draggingSplitterPointerId = event.pointerId;
  const leftPane = workPaneElementForPaneId(leftPaneId);
  const rightPane = workPaneElementForPaneId(rightPaneId);
  if (!leftPane || !rightPane) {
    draggingSplitter = false;
    draggingPaneSplitterElement = null;
    draggingSplitterPointerId = null;
    return;
  }
  resizingPaneEdge = { leftPaneId, rightPaneId };
  paneWidthBeforeResize = `${leftPane.getBoundingClientRect().width}px`;
  workbench.classList.add("is-resizing-panes");
  splitter.classList.add("is-active-splitter");
  splitter.setPointerCapture(event.pointerId);
  event.preventDefault();
}

function resizePanes(event) {
  if (!draggingSplitter || !activeResizePointerMatches(event, draggingSplitterPointerId)) {
    return;
  }
  const leftPane = workPaneElementForPaneId(resizingPaneEdge?.leftPaneId);
  const rightPane = workPaneElementForPaneId(resizingPaneEdge?.rightPaneId);
  if (!leftPane || !rightPane || !draggingPaneSplitterElement) {
    return;
  }
  const leftRect = leftPane.getBoundingClientRect();
  const rightRect = rightPane.getBoundingClientRect();
  const availableWidth = rightRect.right - leftRect.left;
  const minLeft = 240;
  const minRight = 300;
  const snapWidth = 72;
  const splitHandleWidth = draggingPaneSplitterElement.getBoundingClientRect().width || 6;
  const maxLeft = Math.max(minLeft, availableWidth - splitHandleWidth - minRight);
  const pointerX = event.clientX - leftRect.left;
  pendingPaneCollapse = "";
  let next = Math.max(minLeft, Math.min(maxLeft, pointerX));
  if (pointerX >= availableWidth - snapWidth) {
    pendingPaneCollapse = resizingPaneEdge.rightPaneId;
    next = Math.max(0, availableWidth - splitHandleWidth);
  }
  setWorkPaneWidth(resizingPaneEdge.leftPaneId, `${next}px`);
  syncWorkbenchGridLayout();
  syncPreviewViewportScale();
  workbench.dataset.collapsingPane = pendingPaneCollapse || "";
  workbench.dataset.collapsingPreview = pendingPaneCollapse && isPreviewHostPaneId(pendingPaneCollapse) ? "true" : "false";
}

function stopPaneResize(event) {
  if (!draggingSplitter || !activeResizePointerMatches(event, draggingSplitterPointerId)) {
    return;
  }
  const pointerId = draggingSplitterPointerId;
  const splitter = draggingPaneSplitterElement;
  draggingSplitter = false;
  draggingPaneSplitterElement = null;
  draggingSplitterPointerId = null;
  workbench.classList.remove("is-resizing-panes");
  splitter?.classList.remove("is-active-splitter");
  workbench.dataset.collapsingPane = "";
  workbench.dataset.collapsingPreview = "false";
  releasePointerCaptureIfHeld(splitter, pointerId);
  if (pendingPaneCollapse) {
    setWorkPaneWidth(resizingPaneEdge?.leftPaneId, paneWidthBeforeResize || workPaneWidth(resizingPaneEdge?.leftPaneId));
    if (visibleWorkPanes.length > 1) {
      visibleWorkPanes = visibleWorkPanes.filter((paneId) => paneId !== pendingPaneCollapse);
    }
    if (isPreviewHostPaneId(pendingPaneCollapse) && workPaneIdForPreviewMode(currentPreviewMode) === pendingPaneCollapse) {
      const nextPreviewPane = visibleWorkPanes.find((paneId) => isPreviewHostPaneId(paneId));
      if (nextPreviewPane) {
        setPreviewMode(previewModeForWorkPaneId(nextPreviewPane), { skipPaneSync: true });
      }
    }
    applyPaneVisibility();
  } else {
    const leftPaneId = resizingPaneEdge?.leftPaneId;
    if (leftPaneId) {
      setWorkPaneWidth(leftPaneId, workPaneWidth(leftPaneId));
    }
  }
  pendingPaneCollapse = "";
  paneWidthBeforeResize = "";
  resizingPaneEdge = null;
}

function startExplorerResize(event) {
  if (!explorerPaneVisible) {
    return;
  }
  stopActiveResize();
  draggingExplorerSplitter = true;
  draggingExplorerSplitterPointerId = event.pointerId;
  const explorer = workbench.querySelector(".explorer-pane");
  explorerWidthBeforeResize = `${explorer.getBoundingClientRect().width}px`;
  workbench.classList.add("is-resizing-explorer");
  explorerSplitter.setPointerCapture(event.pointerId);
  event.preventDefault();
}

function resizeExplorer(event) {
  if (!draggingExplorerSplitter || !activeResizePointerMatches(event, draggingExplorerSplitterPointerId)) {
    return;
  }
  const rect = workbench.getBoundingClientRect();
  const minWidth = 150;
  const snapWidth = 54;
  const maxWidth = Math.max(minWidth, Math.min(420, rect.width - 360));
  const pointerX = event.clientX - rect.left;
  pendingExplorerCollapse = pointerX <= snapWidth;
  const next = pendingExplorerCollapse ? 0 : Math.max(minWidth, Math.min(maxWidth, pointerX));
  workbench.style.setProperty("--explorer-pane-width", `${next}px`);
  syncPreviewViewportScale();
  workbench.classList.toggle("is-explorer-collapse-pending", pendingExplorerCollapse);
}

function stopExplorerResize(event) {
  if (!draggingExplorerSplitter || !activeResizePointerMatches(event, draggingExplorerSplitterPointerId)) {
    return;
  }
  const pointerId = draggingExplorerSplitterPointerId;
  draggingExplorerSplitter = false;
  draggingExplorerSplitterPointerId = null;
  workbench.classList.remove("is-resizing-explorer", "is-explorer-collapse-pending");
  releasePointerCaptureIfHeld(explorerSplitter, pointerId);
  if (pendingExplorerCollapse) {
    lastExplorerPaneWidth = explorerWidthBeforeResize || lastExplorerPaneWidth;
    explorerPaneVisible = false;
    applyPaneVisibility();
  } else {
    lastExplorerPaneWidth = workbench.style.getPropertyValue("--explorer-pane-width");
  }
  pendingExplorerCollapse = false;
  explorerWidthBeforeResize = "";
}

function startPreviewLogResize(event) {
  if (!playPreview || playPreview.hidden) {
    return;
  }
  stopActiveResize();
  draggingPreviewLogSplitter = true;
  draggingPreviewLogSplitterPointerId = event.pointerId;
  previewLogHeightPinned = true;
  playPreview.classList.add("is-resizing-log");
  previewLogSplitter.setPointerCapture(event.pointerId);
  event.preventDefault();
}

function resizePreviewLog(event) {
  if (!draggingPreviewLogSplitter || !playPreview || !activeResizePointerMatches(event, draggingPreviewLogSplitterPointerId)) {
    return;
  }
  const rect = playPreview.getBoundingClientRect();
  const splitterHeight = previewLogSplitter?.getBoundingClientRect().height || 6;
  const minPreviewHeight = 180;
  const minLogHeight = 72;
  const maxLogHeight = Math.max(minLogHeight, rect.height - splitterHeight - minPreviewHeight);
  const next = Math.max(minLogHeight, Math.min(maxLogHeight, rect.bottom - event.clientY - 12));
  playPreview.style.setProperty("--preview-log-height", `${Math.round(next)}px`);
  syncPreviewViewportScale();
  schedulePreviewViewportSync(3);
  event.preventDefault();
}

function stopPreviewLogResize(event) {
  if (!draggingPreviewLogSplitter || !activeResizePointerMatches(event, draggingPreviewLogSplitterPointerId)) {
    return;
  }
  const pointerId = draggingPreviewLogSplitterPointerId;
  draggingPreviewLogSplitter = false;
  draggingPreviewLogSplitterPointerId = null;
  playPreview?.classList.remove("is-resizing-log");
  releasePointerCaptureIfHeld(previewLogSplitter, pointerId);
  syncPreviewViewportScale();
  schedulePreviewViewportSync(3);
}

function errorDocument(error) {
  const theme = editorPreviewTheme();
  return `<!doctype html>
<html>
  <head>
    <meta charset="utf-8">
    <style>
      :root {
        color-scheme: ${theme.colorScheme};
      }
      body {
        margin: 0;
        min-height: 100vh;
        display: grid;
        place-items: center;
        background: ${theme.background.replaceAll("var(--preview-game-bg)", theme.bg)};
        color: ${theme.ink};
        font-family: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      }
      main {
        width: min(720px, calc(100vw - 32px));
        padding: 24px;
        border: 1px solid ${theme.line};
        border-radius: 8px;
        background: ${theme.panelBg};
      }
      h1 {
        margin: 0 0 12px;
        color: ${theme.danger};
        font-size: 18px;
      }
      pre {
        margin: 0;
        overflow: auto;
        white-space: pre-wrap;
        word-break: break-word;
        font: 13px/1.5 ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
      }
    </style>
  </head>
  <body>
    <main>
      <h1>Compile error</h1>
      <pre>${escapeHtml(error.message || String(error))}</pre>
    </main>
  </body>
</html>`;
}

function emptyPreviewDocument() {
  return `<!doctype html>
<html>
  <head>
    <meta charset="utf-8">
    <style>
      body {
        margin: 0;
        min-height: 100vh;
        background: transparent;
      }
    </style>
  </head>
  <body></body>
</html>`;
}

function setPreviewFrameHtml(html) {
  if (!previewViewport || !previewFrame) {
    return;
  }

  schedulePreviewViewportSync(6);
  const loadId = previewFrameLoadId + 1;
  previewFrameLoadId = loadId;
  const previousFrame = previewFrame;
  const previousObjectUrl = previewFrameObjectUrl;
  let nextObjectUrl = "";
  const nextFrame = document.createElement("iframe");
  nextFrame.className = "preview-frame";
  nextFrame.title = "Compiled puzzle preview";
  nextFrame.setAttribute("sandbox", "allow-scripts");
  nextFrame.setAttribute("scrolling", "no");
  nextFrame.setAttribute("aria-hidden", "true");
  nextFrame.style.visibility = "hidden";
  previewViewport.append(nextFrame);

  nextFrame.addEventListener("load", () => {
    if (loadId !== previewFrameLoadId) {
      nextFrame.remove();
      if (nextObjectUrl) {
        URL.revokeObjectURL(nextObjectUrl);
      }
      return;
    }
    previousFrame.removeAttribute("id");
    nextFrame.id = "previewFrame";
    nextFrame.removeAttribute("aria-hidden");
    nextFrame.style.visibility = "";
    previousFrame.remove();
    previewFrame = nextFrame;
    previewFrameObjectUrl = nextObjectUrl;
    schedulePreviewViewportSync(6);
    if (!levelBuilder.hidden || !solverPanel.hidden) {
      sendLevelStateToPreview();
    }
    if (previousObjectUrl) {
      URL.revokeObjectURL(previousObjectUrl);
    }
  }, { once: true });

  nextFrame.srcdoc = html;
}

function editorPreviewDocument(html) {
  const consoleScript = `<script id="puzzle-studio-editor-preview-log-script">
(() => {
  const formatArg = (value, depth = 0) => {
    if (typeof value === "string") {
      return value;
    }
    if (value instanceof Error) {
      return value.stack || value.message || String(value);
    }
    if (value === undefined) {
      return "undefined";
    }
    if (value === null || typeof value === "number" || typeof value === "boolean" || typeof value === "bigint") {
      return String(value);
    }
    if (depth > 1) {
      return Object.prototype.toString.call(value);
    }
    try {
      return JSON.stringify(value, (_key, nested) => {
        if (typeof nested === "function") {
          return "[Function]";
        }
        return nested;
      });
    } catch (_error) {
      return String(value);
    }
  };
  const postLog = (level, args) => {
    try {
      window.parent.postMessage({
        type: "PuzzleStudioPreviewLog",
        level,
        message: Array.from(args || []).map((arg) => formatArg(arg)).join(" "),
      }, "*");
    } catch (_error) {
      // Logging must not affect the preview runtime.
    }
  };
  for (const level of ["debug", "log", "info", "warn", "error"]) {
    const original = console[level]?.bind(console);
    console[level] = (...args) => {
      postLog(level, args);
      if (original) {
        original(...args);
      }
    };
  }
  window.addEventListener("error", (event) => {
    postLog("error", [event.error || event.message || "Runtime error"]);
  });
  window.addEventListener("unhandledrejection", (event) => {
    postLog("error", [event.reason || "Unhandled promise rejection"]);
  });
})();
<\/script>`;
  let next = html;
  if (!next.includes("puzzle-studio-editor-preview-log-script")) {
    if (next.includes("</head>")) {
      next = next.replace("</head>", `${consoleScript}\n  </head>`);
    } else if (next.includes("<body")) {
      next = next.replace("<body", `${consoleScript}\n<body`);
    } else {
      next = `${consoleScript}\n${next}`;
    }
  }
  return next;
}

function updatePreviewFrameLayout(layout) {
  void layout;
  previewVirtualHeight = previewMinimumHeight;
  previewFrameWrap.style.setProperty("--preview-virtual-width", `${previewVirtualWidth}px`);
  previewFrameWrap.style.setProperty("--preview-virtual-height", `${previewVirtualHeight}px`);
  syncPreviewViewportScale();
}

function syncPreviewViewportScale() {
  if (!previewFrameWrap || !previewViewport) {
    return;
  }
  if (previewFrameWrap.getClientRects().length === 0 || previewViewport.getClientRects().length === 0) {
    return;
  }
  // Fit the preview to the pane by resizing the iframe viewport itself.
  // CSS transforms and preview-driven resize feedback cause refresh/scroll flicker.
  const availableWidth = Math.max(1, Math.floor(previewFrameWrapContentWidth()));
  const availableHeight = Math.max(1, Math.floor(previewFrameWrapContentHeight()));
  const viewportWidth = availableWidth || previewVirtualWidth;
  const viewportHeight = availableHeight || previewVirtualHeight;
  const framePaddingAndBorder = 0;
  previewFrameWrap.style.setProperty("--preview-scale", "1");
  previewFrameWrap.style.setProperty("--preview-virtual-width", `${viewportWidth}px`);
  previewFrameWrap.style.setProperty("--preview-virtual-height", `${viewportHeight}px`);
  previewFrameWrap.style.setProperty("--preview-viewport-width", `${viewportWidth}px`);
  previewFrameWrap.style.setProperty("--preview-viewport-height", `${viewportHeight}px`);
  previewFrameWrap.style.setProperty("--preview-frame-height", `${viewportHeight + framePaddingAndBorder}px`);
}

function schedulePreviewViewportSync(passes = 2) {
  previewViewportSyncPasses = Math.max(
    previewViewportSyncPasses,
    Math.max(1, Math.trunc(Number(passes) || 1)),
  );
  if (previewViewportSyncFrame) {
    return;
  }
  const tick = () => {
    previewViewportSyncFrame = 0;
    syncPreviewViewportScale();
    previewViewportSyncPasses -= 1;
    if (previewViewportSyncPasses > 0) {
      previewViewportSyncFrame = requestAnimationFrame(tick);
    }
  };
  previewViewportSyncFrame = requestAnimationFrame(tick);
}

function syncPreviewAutoLogHeight(frameHeight) {
  if (!playPreview || !previewFrameWrap || !previewLogSplitter || previewLogHeightPinned) {
    return;
  }
  const playRect = playPreview.getBoundingClientRect();
  if (playRect.height <= 0) {
    return;
  }
  const splitterHeight = previewLogSplitter.getBoundingClientRect().height || 6;
  const wrapStyle = window.getComputedStyle(previewFrameWrap);
  const wrapMargins = parseFloat(wrapStyle.marginTop || "0") + parseFloat(wrapStyle.marginBottom || "0");
  const minLogHeight = 72;
  const previewRowHeight = Math.ceil(frameHeight + wrapMargins);
  const next = Math.max(minLogHeight, playRect.height - splitterHeight - previewRowHeight);
  playPreview.style.setProperty("--preview-log-height", `${Math.round(next)}px`);
}

function previewFrameWrapContentWidth() {
  if (!previewFrameWrap) {
    return 0;
  }
  const parent = previewFrameWrap.parentElement;
  const parentWidth = elementContentWidth(parent);
  if (parentWidth > 0) {
    const style = window.getComputedStyle(previewFrameWrap);
    const horizontalSpace =
      parseFloat(style.marginLeft || "0") +
      parseFloat(style.marginRight || "0") +
      parseFloat(style.borderLeftWidth || "0") +
      parseFloat(style.borderRightWidth || "0") +
      parseFloat(style.paddingLeft || "0") +
      parseFloat(style.paddingRight || "0");
    return Math.max(0, parentWidth - horizontalSpace);
  }
  return elementContentWidth(previewFrameWrap);
}

function previewFrameWrapContentHeight() {
  return elementContentHeight(previewFrameWrap);
}

function elementContentWidth(element) {
  if (!element) {
    return 0;
  }
  const style = window.getComputedStyle(element);
  const padding = parseFloat(style.paddingLeft || "0") + parseFloat(style.paddingRight || "0");
  return Math.max(0, element.clientWidth - padding);
}

function elementContentHeight(element) {
  if (!element) {
    return 0;
  }
  const style = window.getComputedStyle(element);
  const padding = parseFloat(style.paddingTop || "0") + parseFloat(style.paddingBottom || "0");
  return Math.max(0, element.clientHeight - padding);
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function downloadHtml() {
  if (!latestHtml) {
    return;
  }
  const blob = new Blob([latestHtml], { type: "text/html;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = htmlDownloadFileName();
  link.click();
  URL.revokeObjectURL(url);
}

function htmlDownloadFileName() {
  const previewDocument = activePreviewDocument();
  const path = previewDocument?.puzzlePath || previewDocument?.name || "";
  const sourceName = path ? fileName(path) : "";
  const baseName = sourceName
    .replace(/\.puzzle$/i, "")
    .replace(/\.html?$/i, "") || "game";
  return `${sanitizeFileName(baseName) || "game"}.html`;
}

function downloadPuzzle() {
  persistCurrentDocument();
  const selected = selectedTreeNode();
  if (selected?.kind === "folder") {
    downloadFolder(selected);
    return;
  }
  downloadFile(selected?.kind === "file" ? selected : documents[currentDocumentIndex]);
}

function downloadFile(document) {
  if (!document) {
    return;
  }
  const blob = document.encoding === "data_url"
    ? new Blob([bytesForDocument(document)], { type: document.mimeType || "application/octet-stream" })
    : new Blob([document.source || sourceEditor.value], { type: `${document.mimeType || "text/plain"};charset=utf-8` });
  const name = document.name || fileName(document.puzzlePath);
  downloadBlob(blob, name || "file");
}

function downloadFolder(folder) {
  const entries = folderZipEntries(folder);
  if (!entries.length) {
    setEditorStatus("Folder is empty", "is-error");
    return;
  }
  const zip = zipBlob(entries);
  downloadBlob(zip, `${sanitizeFileName(folder.name || "folder") || "folder"}.zip`);
}

function folderZipEntries(folder) {
  const entries = [];
  const rootName = sanitizeFileName(folder.name || "folder") || "folder";
  collectFolderZipEntries(folder, rootName, entries);
  return entries;
}

function collectFolderZipEntries(node, parentPath, entries) {
  for (const child of node.children || []) {
    const childName = sanitizeZipPathSegment(child.name || fileName(child.puzzlePath));
    const childPath = joinPath(parentPath, childName);
    if (child.kind === "folder") {
      collectFolderZipEntries(child, childPath, entries);
      continue;
    }
    entries.push({
      path: childPath,
      bytes: bytesForDocument(child),
    });
  }
}

function bytesForDocument(document) {
  if (document.encoding === "data_url") {
    return dataUrlBytes(document.dataUrl || "");
  }
  return new TextEncoder().encode(document.source || "");
}

function dataUrlBytes(dataUrl) {
  const match = String(dataUrl).match(/^data:([^,]*),(.*)$/);
  if (!match) {
    return new Uint8Array();
  }
  const meta = match[1] || "";
  const data = match[2] || "";
  if (meta.includes(";base64")) {
    const raw = atob(data);
    const bytes = new Uint8Array(raw.length);
    for (let index = 0; index < raw.length; index += 1) {
      bytes[index] = raw.charCodeAt(index);
    }
    return bytes;
  }
  return new TextEncoder().encode(decodeURIComponent(data));
}

function sanitizeZipPathSegment(name) {
  return sanitizeFileName(name).replace(/^\.|\.$/g, "") || "item";
}

function downloadBlob(blob, filename) {
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  link.click();
  URL.revokeObjectURL(url);
}

function zipBlob(entries) {
  const encoder = new TextEncoder();
  const parts = [];
  const centralParts = [];
  let offset = 0;
  const now = new Date();
  const dosTime = ((now.getHours() & 31) << 11) | ((now.getMinutes() & 63) << 5) | ((Math.floor(now.getSeconds() / 2)) & 31);
  const dosDate = (((now.getFullYear() - 1980) & 127) << 9) | (((now.getMonth() + 1) & 15) << 5) | (now.getDate() & 31);

  for (const entry of entries) {
    const nameBytes = encoder.encode(normalizePath(entry.path));
    const dataBytes = entry.bytes || new Uint8Array();
    const crc = crc32(dataBytes);
    const localHeader = new Uint8Array(30 + nameBytes.length);
    const localView = new DataView(localHeader.buffer);
    localView.setUint32(0, 0x04034b50, true);
    localView.setUint16(4, 20, true);
    localView.setUint16(6, 0x0800, true);
    localView.setUint16(8, 0, true);
    localView.setUint16(10, dosTime, true);
    localView.setUint16(12, dosDate, true);
    localView.setUint32(14, crc, true);
    localView.setUint32(18, dataBytes.length, true);
    localView.setUint32(22, dataBytes.length, true);
    localView.setUint16(26, nameBytes.length, true);
    localHeader.set(nameBytes, 30);
    parts.push(localHeader, dataBytes);

    const centralHeader = new Uint8Array(46 + nameBytes.length);
    const centralView = new DataView(centralHeader.buffer);
    centralView.setUint32(0, 0x02014b50, true);
    centralView.setUint16(4, 20, true);
    centralView.setUint16(6, 20, true);
    centralView.setUint16(8, 0x0800, true);
    centralView.setUint16(10, 0, true);
    centralView.setUint16(12, dosTime, true);
    centralView.setUint16(14, dosDate, true);
    centralView.setUint32(16, crc, true);
    centralView.setUint32(20, dataBytes.length, true);
    centralView.setUint32(24, dataBytes.length, true);
    centralView.setUint16(28, nameBytes.length, true);
    centralView.setUint32(42, offset, true);
    centralHeader.set(nameBytes, 46);
    centralParts.push(centralHeader);
    offset += localHeader.length + dataBytes.length;
  }

  const centralSize = centralParts.reduce((sum, part) => sum + part.length, 0);
  const end = new Uint8Array(22);
  const endView = new DataView(end.buffer);
  endView.setUint32(0, 0x06054b50, true);
  endView.setUint16(8, entries.length, true);
  endView.setUint16(10, entries.length, true);
  endView.setUint32(12, centralSize, true);
  endView.setUint32(16, offset, true);

  return new Blob([...parts, ...centralParts, end], { type: "application/zip" });
}

function crc32(bytes) {
  let crc = 0xffffffff;
  for (const byte of bytes) {
    crc ^= byte;
    for (let bit = 0; bit < 8; bit += 1) {
      crc = (crc >>> 1) ^ (0xedb88320 & -(crc & 1));
    }
  }
  return (crc ^ 0xffffffff) >>> 0;
}

async function importFiles(fileList) {
  return importFilesIntoFolder(fileList, activeFolder());
}

async function importFilesIntoFolder(fileList, targetFolder) {
  const files = Array.from(fileList || []);
  if (!files.length) {
    return;
  }
  persistCurrentDocument();
  selectedFolderId = targetFolder?.kind === "folder" && targetFolder !== fileTree ? targetFolder.id : "";
  selectedTreeId = selectedFolderId || selectedTreeId;
  let firstImportedPuzzleId = "";
  let importedCount = 0;
  for (const file of files) {
    if (isZipFileName(file.name, file.type)) {
      const result = await importZipFile(file, targetFolder);
      importedCount += result.count;
      if (!firstImportedPuzzleId && result.firstImportedPuzzleId) {
        firstImportedPuzzleId = result.firstImportedPuzzleId;
      }
      continue;
    }

    let imported = null;
    if (isTextFileName(file.name, file.type)) {
      imported = importWorkspaceFile(file.webkitRelativePath || file.name, {
        encoding: "text",
        source: await file.text(),
        mimeType: file.type || mimeTypeForPath(file.name),
      }, targetFolder);
    } else {
      imported = importWorkspaceFile(file.webkitRelativePath || file.name, {
        encoding: "data_url",
        dataUrl: await readFileAsDataUrl(file),
        mimeType: file.type || mimeTypeForPath(file.name),
      }, targetFolder);
    }
    if (!firstImportedPuzzleId && isPuzzleDocument(imported)) {
      firstImportedPuzzleId = imported.id;
    }
    if (imported) {
      importedCount += 1;
    }
  }
  if (!importedCount) {
    setEditorStatus("No importable files", "is-error");
    return;
  }
  if (firstImportedPuzzleId) {
    activeFileId = firstImportedPuzzleId;
  }
  syncDocumentsFromTree();
  currentDocumentIndex = activeDocumentIndex();
  renderDocumentSelect();
  loadEmbeddedDocument(currentDocumentIndex);
  if (!editorSeed) {
    await renderPreview();
  }
  saveDocumentStore(false);
  const folderName = targetFolder && targetFolder !== fileTree ? folderPath(targetFolder) || targetFolder.name : "Files";
  setEditorStatus(`Imported to ${folderName}`, "is-ok");
}

async function importZipFile(file, targetFolder) {
  const entries = await unzipFileEntries(file);
  let firstImportedPuzzleId = "";
  let count = 0;
  for (const entry of entries) {
    const entryPath = safeZipEntryPath(entry.path);
    if (!entryPath) {
      continue;
    }

    let imported = null;
    if (isTextFileName(entryPath, entry.mimeType)) {
      imported = importWorkspaceFile(entryPath, {
        encoding: "text",
        source: new TextDecoder().decode(entry.bytes),
        mimeType: entry.mimeType || mimeTypeForPath(entryPath),
      }, targetFolder);
    } else {
      imported = importWorkspaceFile(entryPath, {
        encoding: "data_url",
        dataUrl: bytesToDataUrl(entry.bytes, entry.mimeType || mimeTypeForPath(entryPath)),
        mimeType: entry.mimeType || mimeTypeForPath(entryPath),
      }, targetFolder);
    }

    if (!firstImportedPuzzleId && isPuzzleDocument(imported)) {
      firstImportedPuzzleId = imported.id;
    }
    if (imported) {
      count += 1;
    }
  }
  return { count, firstImportedPuzzleId };
}

async function unzipFileEntries(file) {
  const bytes = new Uint8Array(await file.arrayBuffer());
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const endOffset = findZipEndOffset(view);
  if (endOffset < 0) {
    throw new Error("Invalid zip file");
  }

  const entryCount = view.getUint16(endOffset + 10, true);
  let centralOffset = view.getUint32(endOffset + 16, true);
  const entries = [];

  for (let index = 0; index < entryCount; index += 1) {
    if (centralOffset + 46 > bytes.length || view.getUint32(centralOffset, true) !== 0x02014b50) {
      throw new Error("Invalid zip directory");
    }
    const flags = view.getUint16(centralOffset + 8, true);
    const method = view.getUint16(centralOffset + 10, true);
    const compressedSize = view.getUint32(centralOffset + 20, true);
    const nameLength = view.getUint16(centralOffset + 28, true);
    const extraLength = view.getUint16(centralOffset + 30, true);
    const commentLength = view.getUint16(centralOffset + 32, true);
    const localOffset = view.getUint32(centralOffset + 42, true);
    const nameStart = centralOffset + 46;
    const nameBytes = bytes.slice(nameStart, nameStart + nameLength);
    const path = decodeZipName(nameBytes, flags);
    centralOffset = nameStart + nameLength + extraLength + commentLength;

    if (!path || path.endsWith("/")) {
      continue;
    }
    if (localOffset + 30 > bytes.length || view.getUint32(localOffset, true) !== 0x04034b50) {
      throw new Error("Invalid zip entry");
    }

    const localNameLength = view.getUint16(localOffset + 26, true);
    const localExtraLength = view.getUint16(localOffset + 28, true);
    const dataStart = localOffset + 30 + localNameLength + localExtraLength;
    const compressed = bytes.slice(dataStart, dataStart + compressedSize);
    const entryBytes = method === 0
      ? compressed
      : method === 8
        ? await inflateZipDeflate(compressed)
        : null;
    if (!entryBytes) {
      throw new Error(`Unsupported zip compression for ${path}`);
    }
    entries.push({
      path,
      bytes: entryBytes,
      mimeType: mimeTypeForPath(path),
    });
  }

  return entries;
}

function findZipEndOffset(view) {
  const minOffset = Math.max(0, view.byteLength - 0xffff - 22);
  for (let offset = view.byteLength - 22; offset >= minOffset; offset -= 1) {
    if (view.getUint32(offset, true) === 0x06054b50) {
      return offset;
    }
  }
  return -1;
}

function decodeZipName(bytes, flags) {
  const decoder = flags & 0x0800 ? new TextDecoder("utf-8") : new TextDecoder();
  return decoder.decode(bytes);
}

async function inflateZipDeflate(bytes) {
  if (typeof DecompressionStream !== "function") {
    throw new Error("Zip deflate is not supported in this browser");
  }
  try {
    const stream = new Blob([bytes]).stream().pipeThrough(new DecompressionStream("deflate-raw"));
    return new Uint8Array(await new Response(stream).arrayBuffer());
  } catch (error) {
    const stream = new Blob([bytes]).stream().pipeThrough(new DecompressionStream("deflate"));
    return new Uint8Array(await new Response(stream).arrayBuffer());
  }
}

function safeZipEntryPath(path) {
  const normalized = normalizePath(path);
  if (!normalized || normalized.startsWith("/") || /^[A-Za-z]:\//.test(normalized)) {
    return "";
  }
  const parts = normalized.split("/").filter(Boolean);
  if (!parts.length || parts.includes("..") || parts[0] === "__MACOSX" || parts.at(-1) === ".DS_Store") {
    return "";
  }
  return parts.map(sanitizeZipPathSegment).join("/");
}

function bytesToDataUrl(bytes, mimeType = "application/octet-stream") {
  let binary = "";
  const chunkSize = 0x8000;
  for (let index = 0; index < bytes.length; index += chunkSize) {
    const chunk = bytes.slice(index, index + chunkSize);
    binary += String.fromCharCode(...chunk);
  }
  return `data:${mimeType};base64,${btoa(binary)}`;
}

function readFileAsDataUrl(file) {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.addEventListener("load", () => resolve(String(reader.result || "")));
    reader.addEventListener("error", () => reject(reader.error || new Error("File read failed")));
    reader.readAsDataURL(file);
  });
}

function importWorkspaceFile(fileNameValue, fileData, targetFolder = activeFolder()) {
  const current = documents[currentDocumentIndex] || {};
  const parts = String(fileNameValue || "imported.file").split(/[\\/]/).filter(Boolean);
  const name = sanitizeFileName(parts.pop() || "imported.file");
  let folder = targetFolder || fileTree;
  for (const part of parts) {
    folder = childFolder(folder, part);
  }
  const file = makeFile(uniqueChildName(folder, name), fileData.source || "", {
    parentPath: folderPath(folder),
    gameCss: current.gameCss || editorSeed?.gameCss || "",
    gameVisualsJs: current.gameVisualsJs || editorSeed?.gameVisualsJs || "",
  });
  file.encoding = fileData.encoding || "text";
  file.mimeType = fileData.mimeType || mimeTypeForPath(name);
  file.source = fileData.source || "";
  file.dataUrl = fileData.dataUrl || "";
  if (!isPuzzleDocument(file)) {
    file.previewHtml = "";
    file.gameCss = "";
    file.gameVisualsJs = "";
  }
  folder.children.push(file);
  selectedFolderId = folder.id;
  activeFileId = file.id;
  return file;
}

function setPuzzleScriptImportStatus(message, tone = "") {
  if (!psImportStatus) {
    return;
  }
  psImportStatus.textContent = message;
  psImportStatus.classList.toggle("is-ok", tone === "is-ok");
  psImportStatus.classList.toggle("is-error", tone === "is-error");
}

function schedulePuzzleScriptImportConversion(delay = 220) {
  window.clearTimeout(psImportConvertTimer);
  psImportConvertTimer = window.setTimeout(() => {
    convertPuzzleScriptImport().catch((error) => {
      console.error(error);
      setPuzzleScriptImportStatus(error.message || String(error), "is-error");
    });
  }, delay);
}

async function convertPuzzleScriptImport() {
  const source = psImportSourceInput?.value || "";
  if (!source.trim()) {
    if (psImportOutput) {
      psImportOutput.value = "";
    }
    if (psImportCopyButton) {
      psImportCopyButton.disabled = true;
    }
    if (psImportAddFileButton) {
      psImportAddFileButton.disabled = true;
    }
    setPuzzleScriptImportStatus("", "");
    return "";
  }
  setPuzzleScriptImportStatus("Converting", "");
  const compiler = await loadWasmCompiler();
  if (typeof compiler.translate_puzzlescript !== "function") {
    throw new Error("PuzzleScript import is unavailable in this editor build.");
  }
  const canonical = compiler.translate_puzzlescript(source);
  if (psImportOutput) {
    psImportOutput.value = canonical;
  }
  if (psImportCopyButton) {
    psImportCopyButton.disabled = false;
  }
  if (psImportAddFileButton) {
    psImportAddFileButton.disabled = false;
  }
  setPuzzleScriptImportStatus("Converted", "is-ok");
  return canonical;
}

function puzzleScriptImportTitle(source, canonical) {
  const explicitTitle = String(source || "")
    .split("\n")
    .map((line) => line.split("//", 1)[0].trim())
    .find((line) => /^title(?:\s+|$)/i.test(line))
    ?.replace(/^title\s*/i, "")
    .trim();
  if (explicitTitle) {
    return explicitTitle;
  }
  const canonicalTitle = String(canonical || "")
    .split("\n")
    .find((line) => /^title(?:\s+|$)/.test(line.trim()))
    ?.trim()
    .replace(/^title\s*/, "")
    .trim();
  if (canonicalTitle) {
    try {
      return JSON.parse(canonicalTitle);
    } catch {
      return canonicalTitle.replace(/^"|"$/g, "");
    }
  }
  return "PuzzleScript import";
}

async function copyPuzzleScriptImportOutput() {
  const output = psImportOutput?.value || await convertPuzzleScriptImport();
  if (!output.trim()) {
    setPuzzleScriptImportStatus("Nothing to copy", "is-error");
    return;
  }
  try {
    psImportCopyButton?.focus({ preventScroll: true });
    await copyTextToClipboard(output);
    setPuzzleScriptImportStatus("Copied", "is-ok");
  } catch (error) {
    setPuzzleScriptImportStatus("Copy failed", "is-error");
    setStatus(`Could not copy PuzzleScript import: ${error?.message || error}`, "is-error");
  }
}

async function addPuzzleScriptImportFile() {
  let output = psImportOutput?.value || "";
  if (!output.trim()) {
    output = await convertPuzzleScriptImport();
  }
  if (!output.trim()) {
    setPuzzleScriptImportStatus("Nothing to add", "is-error");
    return;
  }

  persistCurrentDocument();
  const targetFolder = activeFolder();
  targetFolder.expanded = true;
  const title = puzzleScriptImportTitle(psImportSourceInput?.value || "", output);
  const fileNameValue = uniqueChildName(targetFolder, ensurePuzzleExtension(title || "PuzzleScript import"));
  const parentPath = folderPath(targetFolder);
  const editorPath = joinPath(parentPath, fileNameValue);

  if (!editorSeed && typeof window.PuzzleStudioHost.createSourceFile === "function") {
    await window.PuzzleStudioHost.createSourceFile({
      source: output,
      puzzlePath: hostPathForEditorPath(editorPath),
    });
  }

  const current = documents[currentDocumentIndex] || {};
  const file = makeFile(fileNameValue, output, {
    parentPath,
    gameCss: current.gameCss || editorSeed?.gameCss || "",
    gameVisualsJs: current.gameVisualsJs || editorSeed?.gameVisualsJs || "",
  });
  targetFolder.children.push(file);
  activeFileId = file.id;
  selectedTreeId = file.id;
  selectedFolderId = targetFolder === fileTree ? "" : targetFolder.id;
  syncDocumentsFromTree();
  loadEmbeddedDocument(activeDocumentIndex());
  saveDocumentStore(false);
  setPuzzleScriptImportStatus(`Added ${fileNameValue}`, "is-ok");
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
  return `title ${JSON.stringify(title)}\n\nmodel puzzle main {\n\tlayers {\n\t\tfloor = Goal\n\t\tsolid = Player Wall\n\t}\n\n\tinputs {\n\t\tup <- w ArrowUp\n\t\tdown <- s ArrowDown\n\t\tleft <- a ArrowLeft\n\t\tright <- d ArrowRight\n\t\trestart <- r\n\t}\n\n\twin_conditions {\n\t\tall Goal on Player\n\t}\n\n\trules {\n\t\tonce input directions [ Player | no solid ] -> [ | Player ]\n\t}\n\n\tlevels {\n\t\tlegend {\n\t\t\t. = empty\n\t\t\t# = Wall\n\t\t\tP = Player\n\t\t\tG = Goal\n\t\t\t+ = Player Goal\n\t\t}\n\n\t\tlevel level_1\n\t\t\t#######\n\t\t\t#P...G#\n\t\t\t#######\n\n\t\tlevel level_2\n\t\t\t#######\n\t\t\t#P....#\n\t\t\t#..G..#\n\t\t\t#######\n\t}\n}\n\nscene playing {\n\tstate {\n\t\tpuzzle main\n\t}\n\tview {\n\t\trow {\n\t\t\ttitle\n\t\t}\n\t\tmain\n\t}\n\trules {\n\t\tstep main\n\t\tif main.win_conditions -> {\n\t\t\twait 0.25s\n\t\t\tmain.next_level\n\t\t}\n\t}\n}\n`;
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

function syncPreviewModeButtonState() {
  const previewMode = normalizePreviewMode(currentPreviewMode);
  const paneVisible = isPaneVisible(workPaneIdForPreviewMode(previewMode));
  const spritePaneVisible = isPaneVisible("sprite");
  playModeButton.classList.toggle("is-active", paneVisible && previewMode === "play");
  editModeButton.classList.toggle("is-active", isPaneVisible("level"));
  solverModeButton.classList.toggle("is-active", paneVisible && previewMode === "solver");
  spriteModeButton.classList.toggle("is-active", spritePaneVisible);
  sprite3dModeButton?.classList.toggle("is-active", spritePaneVisible && currentSpritePaneMode === "sprite3d");
  for (const button of levelPaneModeButtons) {
    const active = isPaneVisible("level") && button.dataset.levelPaneMode === currentLevelPaneMode;
    button.classList.toggle("is-active", active);
    button.setAttribute("aria-pressed", String(active));
  }
  for (const button of spritePaneModeButtons) {
    const active = spritePaneVisible && button.dataset.spritePaneMode === currentSpritePaneMode;
    button.classList.toggle("is-active", active);
    button.setAttribute("aria-pressed", String(active));
  }
  soundsTopbarButton.classList.toggle("is-active", paneVisible && previewMode === "sounds");
  psImportTopbarButton?.classList.toggle("is-active", paneVisible && previewMode === "psimport");
  docsTopbarButton?.classList.toggle("is-active", paneVisible && previewMode === "docs");
}

function setPreviewMode(mode, options = {}) {
  const wasLevelMode = isPaneVisible("level") || isPaneVisible("solver");
  const previewMode = normalizePreviewMode(mode);
  if (previewMode !== "edit" && levelPlaytestActive) {
    stopLevelPlaytest({ syncPreview: false });
  }
  if (!options.skipPaneSync) {
    showPreviewModePane(previewMode);
  }
  currentPreviewMode = previewMode;
  workbench.dataset.activePreviewMode = previewMode;
  workbench.dataset.activePreviewPane = workPaneIdForPreviewMode(previewMode);
  syncWorkbenchGridLayout();
  const editMode = previewMode === "edit";
  const level3dMode = previewMode === "level3d";
  const solverMode = previewMode === "solver";
  const enteringLevelMode = (editMode || solverMode) && !wasLevelMode;
  const spriteMode = previewMode === "sprite";
  const sprite3dMode = previewMode === "sprite3d";
  const soundsMode = previewMode === "sounds";
  const psImportMode = previewMode === "psimport";
  if (editMode || level3dMode) {
    currentLevelPaneMode = previewMode;
  }
  if (spriteMode || sprite3dMode) {
    currentSpritePaneMode = previewMode;
  }
  if (Number.isInteger(latestPreviewState?.levelIndex)) {
    setActiveLevelIndex(latestPreviewState.levelIndex);
  }
  if (levelPaneModeSwitch) {
    levelPaneModeSwitch.hidden = !isPaneVisible("level");
  }
  if (spritePaneModeSwitch) {
    spritePaneModeSwitch.hidden = !isPaneVisible("sprite");
  }
  syncPreviewModeButtonState();
  if (gamePaneTitle) {
    gamePaneTitle.textContent = "Preview";
  }
  if (runButton) {
    runButton.hidden = false;
  }
  applyPaneVisibility();
  syncPreviewViewportScale();
  scheduleBoardScaleSync(3);
  if (!isPaneVisible("sounds")) {
    stopSoundPlayback();
  }
  if (editMode || solverMode) {
    resetLevelBuilderFromSource(false);
    if (enteringLevelMode || !level.cells.length) {
      loadLevelFromPreviewState();
    } else if (editMode && levelSolutionPreview) {
      clearSolutionPreview();
      renderLevelBoard();
    }
  }
  if (solverMode) {
    renderSolverBoard();
    updateSolutionControls();
  }
  if (spriteMode) {
    renderSpriteBuilder();
  }
  if (sprite3dMode) {
    renderSprite3dBuilder();
  }
  if (level3dMode) {
    renderLevel3dBuilder();
  }
  if (soundsMode) {
    renderSoundsBuilder();
  }
  if (psImportMode) {
    schedulePuzzleScriptImportConversion(0);
  }
  if (previewMode === "play" && wasLevelMode) {
    sendLevelStateToPreview();
  }
}

function resetLevelBuilderFromSource(resetCells = true) {
  levelDisplayCells = null;
  level.palette = levelPaletteFromExport(activePreviewSource());
  const size = initialLevelSize();
  if (resetCells) {
    level.width = size.width || level.width;
    level.height = size.height || level.height;
    level.regions = defaultLevelRegions(level.width, level.height);
    level.cells = makeEmptyCells(level.width, level.height);
  }
  if (!level.palette.some((entry) => entry.id === level.selectedObjectId)) {
    level.selectedObjectId = level.palette[0]?.id ?? 0;
  }
  updateLevelSizeLabel();
  renderLevelPalette();
  renderLevelBoard();
}

function resetLevelBuilderFromPreviewSource() {
  resetLevelBuilderFromSource(false);
  if (!loadLevelFromPreviewState()) {
    resetLevelBuilderFromSource(true);
  }
}

function blockLines(source, name) {
  const block = findNamedBlock(source, name);
  if (!block) {
    return [];
  }
  return source.slice(block.bodyStart, block.bodyEnd).split("\n");
}

function titleLabel(value) {
  return String(value || "Tile")
    .replace(/[:_-]+/g, " ")
    .replace(/\s+/g, " ")
    .trim()
    .replace(/\b\w/g, (letter) => letter.toUpperCase());
}

function levelPaletteFromExport(source) {
  const placeableObjects = sourcePlaceableObjectNames(source, previewExport);
  const objects = engineObjects().filter((object) => placeableObjects.has(object.name));
  return [
    { id: 0, name: "Empty", layer: null, sprite: "empty" },
    ...objects,
  ];
}

function sourcePlaceableObjectNames(source, exportData = previewExport) {
  return new Set(sourceCharEntries(source, exportData)
    .filter((entry) => entry.objects.length === 1)
    .map((entry) => entry.objects[0]));
}

function engineObjects(exportData = previewExport) {
  return [...(exportData?.engine?.objects || [])]
    .sort((left, right) => left.layer - right.layer || left.name.localeCompare(right.name));
}

function engineObjectById(objectId, exportData = previewExport) {
  return (exportData?.engine?.objects || []).find((object) => object.id === objectId) || null;
}

function isVisualObject(object, exportData = previewExport) {
  return (exportData?.engine?.visualObjects || []).includes(object.id);
}

function visualObjectNameSet(exportData = previewExport) {
  const visualIds = new Set(exportData?.engine?.visualObjects || []);
  return new Set((exportData?.engine?.objects || [])
    .filter((object) => visualIds.has(object.id))
    .map((object) => object.name));
}

function layerCount(exportData = previewExport) {
  return exportData?.engine?.layerCount
    || exportData?.levels?.[0]?.initialState?.layerCount
    || 1;
}

function initialLevelSize() {
  const state = previewExport?.levels?.[currentEditableLevelIndex()]?.initialState;
  if (state?.width && state?.height) {
    return { width: state.width, height: state.height };
  }
  return { width: 9, height: 5 };
}

function currentEditableLevelIndex(exportData = previewExport) {
  return setActiveLevelIndex(activeLevelIndex, exportData);
}

function setActiveLevelIndex(index, exportData = previewExport) {
  const levels = exportData?.levels || [];
  if (!levels.length) {
    activeLevelIndex = 0;
    return 0;
  }
  const fallback = exportData.initialLevelIndex ?? 0;
  const rawIndex = index ?? fallback;
  activeLevelIndex = Math.max(0, Math.min(levels.length - 1, Math.trunc(Number(rawIndex) || 0)));
  return activeLevelIndex;
}

function levelRows(source) {
  const block = findNamedBlock(source, "levels");
  if (!block) {
    return [];
  }
  return source
    .slice(block.bodyStart, block.bodyEnd)
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line && !line.includes("{") && !line.includes("}") && !line.includes("="));
}

function loadLevelFromSourceClick(event = null) {
  if (!isPuzzleDocument(activeDocument()) || !isTextDocument(activeDocument())) {
    return;
  }
  const source = sourceEditor.value || "";
  const clickOffset = sourceOffsetFromEditorClick(event, source);
  loadLevelFromSourcePosition(clickOffset ?? sourceEditor.selectionStart);
}

function loadLevelFromSourcePosition(position, options = {}) {
  if (!isPuzzleDocument(activeDocument()) || !isTextDocument(activeDocument())) {
    return null;
  }
  const source = sourceEditor.value || "";
  const entry = findLevelDefinitionAtPosition(source, position);
  if (!entry) {
    return null;
  }
  ensurePreviewTargetsActiveDocument();
  if (!previewExport?.levels?.length) {
    if (!options.silent) {
      setStatus("No preview level to edit", "is-error");
    }
    return null;
  }
  if (options.recordHistory) {
    pushSourceNavigationHistory();
  }
  if (currentPreviewMode !== "edit") {
    setPreviewMode("edit");
  }
  const levelIndex = setActiveLevelIndex(entry.levelIndex);
  loadLevelFromPreviewState();
  const levelName = previewExport?.levels?.[levelIndex]?.name || entry.name || `level_${levelIndex + 1}`;
  setLevelNameInputs(levelName);
  if (!options.silent) {
    setStatus(`Loaded level ${levelName}`, "is-ok");
  }
  return `level:${levelIndex}:${levelName}`;
}

async function resolveSourceTargetFromWasm(source, position) {
  const compiler = await loadWasmCompiler();
  if (typeof compiler?.resolve_source_target !== "function") {
    return null;
  }
  const cursorByteOffset = sourceByteOffset(source, position);
  const raw = compiler.resolve_source_target(source, cursorByteOffset);
  const payload = JSON.parse(raw || "{}");
  return normalizeResolvedSourceTarget(source, payload?.target || null, position);
}

function normalizeResolvedSourceTarget(source, target, position = null) {
  if (!target || typeof target !== "object") {
    return null;
  }
  const normalized = { ...target };
  for (const key of ["start", "end", "bodyStart", "bodyEnd"]) {
    if (Number.isInteger(normalized[key])) {
      normalized[key] = sourceUtf16OffsetFromByteOffset(source, normalized[key]);
    }
  }
  if (normalized.kind === "sprite") {
    const sprite3dTarget = sourceSprite3dTargetAtPosition(
      source,
      Number.isInteger(position) ? position : normalized.start,
    );
    if (sprite3dTarget) {
      return sprite3dTarget;
    }
  }
  return normalized;
}

function sourceSprite3dTargetAtPosition(source, position) {
  if (typeof findSprite3dDefinitionAtPosition !== "function") {
    return null;
  }
  const entry = findSprite3dDefinitionAtPosition(source, position);
  if (!entry) {
    return null;
  }
  return {
    kind: "sprite3d",
    name: entry.name,
    start: entry.start,
    end: entry.end,
    bodyStart: entry.bodyStart,
    bodyEnd: entry.bodyEnd,
  };
}

function loadLevelSourceTarget(target, options = {}) {
  if (!isPuzzleDocument(activeDocument()) || !isTextDocument(activeDocument())) {
    return null;
  }
  ensurePreviewTargetsActiveDocument();
  if (!previewExport?.levels?.length) {
    if (!options.silent) {
      setStatus("No preview level to edit", "is-error");
    }
    return null;
  }
  let levelIndex = Number.isInteger(target.levelIndex) ? target.levelIndex : -1;
  const levels = previewExport.levels || [];
  if (!levels[levelIndex] || (target.name && levels[levelIndex]?.name !== target.name)) {
    const byName = levels.findIndex((level) => level.name === target.name);
    if (byName >= 0) {
      levelIndex = byName;
    }
  }
  if (!levels[levelIndex]) {
    return null;
  }
  if (options.recordHistory) {
    pushSourceNavigationHistory();
  }
  if (currentPreviewMode !== "edit") {
    setPreviewMode("edit");
  }
  levelIndex = setActiveLevelIndex(levelIndex);
  loadLevelFromPreviewState();
  const levelName = levels[levelIndex]?.name || target.name || `level_${levelIndex + 1}`;
  setLevelNameInputs(levelName);
  if (!options.silent) {
    setStatus(`Loaded level ${levelName}`, "is-ok");
  }
  return `level:${levelIndex}:${levelName}`;
}

function loadResolvedSourceTarget(target, options = {}) {
  if (!target?.kind) {
    return null;
  }
  if (target.kind === "level") {
    return loadLevelSourceTarget(target, options);
  }
  if (target.kind === "sprite" && typeof loadSpriteSourceTarget === "function") {
    return loadSpriteSourceTarget(target, options);
  }
  if (target.kind === "sprite3d" && typeof loadSprite3dSourceTarget === "function") {
    return loadSprite3dSourceTarget(target, options);
  }
  if (target.kind === "sounds" && typeof loadSoundSourceTarget === "function") {
    return loadSoundSourceTarget(target, options);
  }
  return null;
}

function loadSourceTargetWithJsFallback(source, position, options = {}) {
  if (findLevelDefinitionAtPosition(source, position)) {
    return loadLevelFromSourcePosition(position, { silent: true, recordHistory: options.recordHistory }) || "";
  }
  if (
    typeof findSprite3dDefinitionAtPosition === "function"
    && typeof loadSprite3dFromSourcePosition === "function"
    && findSprite3dDefinitionAtPosition(source, position)
  ) {
    return loadSprite3dFromSourcePosition(position, { silent: true, switchMode: true, recordHistory: options.recordHistory }) || "";
  }
  if (
    typeof findSpriteDefinitionAtPosition === "function"
    && typeof loadSpriteFromSourcePosition === "function"
    && findSpriteDefinitionAtPosition(source, position)
  ) {
    return loadSpriteFromSourcePosition(position, { silent: true, switchMode: true, recordHistory: options.recordHistory }) || "";
  }
  if (
    typeof findSoundsDefinitionAtPosition === "function"
    && typeof loadSoundFromSourcePosition === "function"
    && findSoundsDefinitionAtPosition(source, position)
  ) {
    return loadSoundFromSourcePosition(position, { silent: true, switchMode: true, recordHistory: options.recordHistory }) || "";
  }
  return "";
}

function finishSourceTargetSync(key, options = {}) {
  if (!key) {
    sourceCursorPreviewKey = "";
    return false;
  }
  if (!options.force && key === sourceCursorPreviewKey) {
    return true;
  }
  sourceCursorPreviewKey = key;
  return true;
}

function syncPreviewModeFromSourceCursor(options = {}) {
  sourceTargetRequestId += 1;
  sourceCursorPreviewKey = "";
  return false;
}

function syncPreviewModeFromSourcePointer(event) {
  return syncPreviewModeFromSourceCursor();
}

function syncSourceFromPreviewPane(mode = currentPreviewMode, options = {}) {
  if (!isTextDocument(activePreviewDocument())) {
    return false;
  }
  const target = sourceLocationForPreviewPane(mode);
  if (!target) {
    return false;
  }
  const key = `${mode}:${target.key}`;
  if (!options.force && key === previewPaneSourceKey) {
    return true;
  }
  if (!revealSourceLocation(target, { revealPane: options.revealPane === true })) {
    return false;
  }
  previewPaneSourceKey = key;
  return true;
}

function sourceLocationForPreviewPane(mode) {
  if (mode === "edit" || mode === "solver") {
    return currentLevelSourceLocation();
  }
  if (mode === "level3d") {
    return currentLevel3dSourceLocation();
  }
  if (mode === "sprite") {
    return currentSpriteSourceLocation();
  }
  if (mode === "sprite3d") {
    return currentSprite3dSourceLocation();
  }
  if (mode === "sounds") {
    return currentSoundSourceLocation();
  }
  return null;
}

function revealSourceLocation(target, options = {}) {
  if (!target?.document) {
    return false;
  }
  if (options.revealPane === false && !isPaneVisible(SOURCE_WORK_PANE_ID)) {
    return false;
  }
  if (options.recordHistory !== false) {
    pushSourceNavigationHistory();
  }
  if (options.revealPane !== false) {
    revealCodePane();
  }
  const preservedMode = currentPreviewMode;
  const preservedLevelIndex = activeLevelIndex;
  const index = documents.findIndex((document) => document.id === target.document.id);
  if (index >= 0 && index !== currentDocumentIndex) {
    persistCurrentDocument();
    loadEmbeddedDocument(index);
    if (preservedMode === "edit" || preservedMode === "solver") {
      setActiveLevelIndex(Number.isInteger(target.levelIndex) ? target.levelIndex : preservedLevelIndex);
      loadLevelFromPreviewState({ requestRender: false });
    }
  }
  const start = Math.max(0, Math.min(sourceEditor.value.length, target.start || 0));
  sourceEditor.setSelectionRange(start, start);
  scrollSourceEditorToPosition(start);
  if (typeof updateSourceMeta === "function") {
    updateSourceMeta();
  }
  return true;
}

function scrollSourceEditorToPosition(position) {
  const source = sourceEditor.value || "";
  const lines = editorSourceLinesWithOffsets(source);
  const lineIndex = Math.max(0, lines.findIndex((line) => position >= line.start && position <= line.absoluteEnd));
  const style = window.getComputedStyle(sourceEditor);
  const lineHeight = Number.parseFloat(style.lineHeight) || 20;
  const paddingTop = Number.parseFloat(style.paddingTop) || 0;
  const targetTop = paddingTop + lineIndex * lineHeight;
  sourceEditor.scrollTop = Math.max(0, targetTop - sourceEditor.clientHeight * 0.28);
  sourceEditor.scrollLeft = 0;
  if (typeof syncSourceHighlightScroll === "function") {
    syncSourceHighlightScroll();
  }
}

function currentLevelSourceLocation() {
  const levelIndex = currentEditableLevelIndex();
  const levelName = previewExport?.levels?.[levelIndex]?.name || "";
  const allEntries = [];
  for (const document of puzzleTextDocuments()) {
    const source = sourceForDocument(document);
    const entries = findLevelSourceEntries(source, document);
    allEntries.push(...entries);
    const entry = levelName
      ? entries.find((candidate) => sourceTitleMatches(candidate.name, levelName))
      : null;
    if (entry) {
      return {
        document: entry.document,
        start: entry.start,
        end: entry.end,
        levelIndex,
        key: `${entry.document.id}:level:${levelIndex}:${levelName}:${entry.start}`,
      };
    }
  }
  const fallback = allEntries[levelIndex] || null;
  if (fallback) {
    return {
      document: fallback.document,
      start: fallback.start,
      end: fallback.end,
      levelIndex,
      key: `${fallback.document.id}:level:${levelIndex}:${levelName}:${fallback.start}`,
    };
  }
  return null;
}

function findLevelSourceEntries(source, document) {
  const lines = editorSourceLinesWithOffsets(source);
  const entries = [];
  for (const line of lines) {
    const code = stripLineCommentForWasm(line.raw).trim();
    const match = code.match(/^level(?:\s+(.+?))?\s*(?:\{|$)/);
    if (!match) {
      continue;
    }
    const rawName = String(match[1] || "").trim();
    entries.push({
      document,
      name: rawName.replace(/\s*\{\s*$/, ""),
      start: firstEditorSourceCodeIndex(line),
      end: line.absoluteEnd,
    });
  }
  return entries;
}

function currentSpriteSourceLocation() {
  if (typeof findSpritesBlock !== "function" || typeof findSpriteDefinitionBlock !== "function") {
    return null;
  }
  const name = typeof spriteObjectName === "function" ? spriteObjectName() : "";
  if (!name) {
    return null;
  }
  for (const document of puzzleTextDocuments()) {
    const source = sourceForDocument(document);
    const block = findSpritesBlock(source);
    const entry = block ? findSpriteDefinitionBlock(source, block, name) : null;
    if (entry) {
      return {
        document,
        start: entry.start,
        end: entry.end,
        key: `${document.id}:sprite:${name}:${entry.start}`,
      };
    }
  }
  return null;
}

function currentSprite3dSourceLocation() {
  if (typeof findSprites3dBlock !== "function" || typeof findSprite3dDefinitionBlock !== "function") {
    return null;
  }
  const name = typeof sprite3dObjectName === "function" ? sprite3dObjectName() : "";
  if (!name) {
    return null;
  }
  for (const document of puzzleTextDocuments()) {
    const source = sourceForDocument(document);
    const block = findSprites3dBlock(source);
    const entry = block ? findSprite3dDefinitionBlock(source, block, name) : null;
    if (entry) {
      return {
        document,
        start: entry.start,
        end: entry.end,
        key: `${document.id}:sprite3d:${name}:${entry.start}`,
      };
    }
  }
  return null;
}

function currentSoundSourceLocation() {
  const kind = sounds?.mode === "music" ? "music" : "sfx";
  const titleInput = kind === "music" ? soundsMusicTitleInput : soundsSfxTitleInput;
  const fallback = kind === "music" ? "music" : "sfx";
  const name = typeof soundIdentifierAtom === "function"
    ? soundIdentifierAtom(titleInput?.value, fallback)
    : String(titleInput?.value || fallback).trim();
  if (!kind || !name) {
    return null;
  }
  for (const document of puzzleTextDocuments()) {
    const source = sourceForDocument(document);
    const entry = findSoundsDefinitionByName(source, kind, name);
    if (entry) {
      return {
        document,
        start: entry.start,
        end: entry.end,
        key: `${document.id}:sounds:${kind}:${name}:${entry.start}`,
      };
    }
  }
  return null;
}

function findSoundsDefinitionByName(source, kind, name) {
  const lines = editorSourceLinesWithOffsets(source);
  for (const line of lines) {
    const parsed = typeof parseSoundsDefinitionLine === "function"
      ? parseSoundsDefinitionLine(line.raw)
      : null;
    if (parsed?.kind === kind && parsed?.name === name) {
      return { start: firstEditorSourceCodeIndex(line), end: line.absoluteEnd };
    }
  }
  return null;
}

function puzzleTextDocuments() {
  return documents.filter((document) => isPuzzleDocument(document) && isTextDocument(document));
}

function sourceForDocument(document) {
  return document?.id === activeDocument()?.id && isTextDocument(document)
    ? sourceEditor.value || ""
    : document?.source || "";
}

function editorSourceLinesWithOffsets(source) {
  const lines = [];
  let start = 0;
  const text = String(source || "");
  for (const raw of text.split("\n")) {
    const end = start + raw.length;
    const hasNewline = end < text.length;
    lines.push({
      raw,
      text: hasNewline ? `${raw}\n` : raw,
      start,
      end,
      absoluteEnd: end + (hasNewline ? 1 : 0),
      hasNewline,
    });
    start = end + 1;
  }
  return lines;
}

function firstEditorSourceCodeIndex(line) {
  const offset = String(line?.raw || "").search(/\S/);
  return (line?.start || 0) + Math.max(0, offset);
}

function sourceOffsetFromEditorClick(event, source) {
  if (!event || !sourceEditorWrap?.contains(event.target)) {
    return null;
  }
  const rect = sourceEditor.getBoundingClientRect();
  const style = window.getComputedStyle(sourceEditor);
  const paddingTop = Number.parseFloat(style.paddingTop) || 0;
  const lineHeight = Number.parseFloat(style.lineHeight) || 20;
  const y = event.clientY - rect.top + sourceEditor.scrollTop - paddingTop;
  const lineIndex = Math.floor(y / lineHeight);
  if (!Number.isFinite(lineIndex) || lineIndex < 0) {
    return null;
  }
  const lines = sourceLinesWithOffsets(source);
  const line = lines[lineIndex];
  return line ? line.start : null;
}

function findLevelDefinitionAtPosition(source, position) {
  const levelsRange = findLevelsRangeAtPosition(source, position);
  if (levelsRange) {
    const entry = findLevelDefinitions(source, levelsRange)
      .find((entry) => position >= entry.start && position <= entry.end) || null;
    return entry || findLevelHeaderAtPosition(source, position);
  }
  return findStandaloneLevelDefinitionAtPosition(source, position)
    || findLevelHeaderAtPosition(source, position);
}

function findLevelHeaderAtPosition(source, position) {
  const lines = sourceLinesWithOffsets(source);
  const lineIndex = sourceLineIndexAtOffset(lines, position);
  const line = lines[lineIndex];
  if (!line || position < line.start || position > line.end) {
    return null;
  }
  const code = levelScannerCode(line.raw);
  const tokens = splitLevelTokens(code);
  if (tokens[0] !== "level") {
    return null;
  }
  const nameTokens = tokens.at(-1) === "{" ? tokens.slice(1, -1) : tokens.slice(1);
  let levelIndex = 0;
  for (const previous of lines.slice(0, lineIndex)) {
    const previousTokens = splitLevelTokens(levelScannerCode(previous.raw));
    if (previousTokens[0] === "level") {
      levelIndex += 1;
    }
  }
  return {
    name: levelNameFromTokens(nameTokens),
    start: firstCodeIndex(line),
    end: line.absoluteEnd,
    nextIndex: lineIndex + 1,
    levelIndex,
  };
}

function findLevelsRangeAtPosition(source, position) {
  const ranges = findLevelsRanges(source);
  return ranges.find((range) => position >= range.bodyStart && position <= range.bodyEnd) || null;
}

function findLevelsRanges(source) {
  const lines = sourceLinesWithOffsets(source);
  const rawLines = lines.map((line) => line.raw);
  const ranges = [];

  for (let index = 0; index < lines.length; index += 1) {
    const section = sectionHeaderAtForWasm(rawLines, index);
    if (section?.block === "levels") {
      ranges.push({
        headerStart: lines[index].start,
        bodyStart: lines[index + 2].end + (lines[index + 2].hasNewline ? 1 : 0),
        bodyEnd: findSectionLevelsEnd(lines, rawLines, index + 3),
        indent: "",
        namespace: "",
      });
      index += 2;
      continue;
    }

    const code = levelScannerCode(lines[index].raw);
    const tokens = splitLevelTokens(code);
    if (tokens[0] === "levels" && tokens.at(-1) === "{") {
      const openIndex = source.indexOf("{", lines[index].start);
      const closeIndex = findMatchingBrace(source, openIndex);
      if (openIndex >= 0 && closeIndex >= 0) {
        ranges.push({
          headerStart: lines[index].start,
          bodyStart: openIndex + 1,
          bodyEnd: closeIndex,
          indent: `${lineIndent(lines[index].raw)}\t`,
          namespace: levelsNamespaceFromTokens(tokens),
        });
      }
      continue;
    }

    if (
      tokens.length >= 1
      && tokens.length <= 2
      && tokens[0] === "levels"
      && !isSectionTitleLine(rawLines, index)
    ) {
      ranges.push({
        headerStart: lines[index].start,
        bodyStart: lines[index].end + (lines[index].hasNewline ? 1 : 0),
        bodyEnd: findEndDelimitedLevelsEnd(lines, index + 1),
        indent: `${lineIndent(lines[index].raw)}\t`,
        namespace: levelsNamespaceFromTokens(tokens),
      });
      continue;
    }
  }
  return ranges;
}

function findSectionLevelsEnd(lines, rawLines, startIndex) {
  let nestedDepth = 0;
  for (let index = startIndex; index < lines.length; index += 1) {
    if (nestedDepth === 0 && sectionHeaderAtForWasm(rawLines, index)) {
      return lines[index].start;
    }
    const code = levelScannerCode(lines[index].raw);
    if (!code) {
      continue;
    }
    const normalized = braceNormalizedLineForSectionForWasm(code);
    const tokens = splitLevelTokens(normalized);
    if (nestedDepth === 0) {
      if (normalized === "}") {
        return lines[index].start;
      }
    }
    if (normalized === "end" || normalized === "}") {
      nestedDepth = Math.max(0, nestedDepth - 1);
    } else if (startsLevelNestedBlock(tokens, normalized)) {
      nestedDepth += 1;
    }
  }
  return lines.at(-1)?.absoluteEnd ?? 0;
}

function findEndDelimitedLevelsEnd(lines, startIndex) {
  let nestedDepth = 0;
  for (let index = startIndex; index < lines.length; index += 1) {
    const code = levelScannerCode(lines[index].raw);
    if (!code) {
      continue;
    }
    const normalized = braceNormalizedLineForSectionForWasm(code);
    const tokens = splitLevelTokens(normalized);
    if (normalized === "end") {
      if (nestedDepth === 0) {
        return lines[index].start;
      }
      nestedDepth -= 1;
    } else if (startsLevelNestedBlock(tokens, normalized)) {
      nestedDepth += 1;
    }
  }
  return lines.at(-1)?.absoluteEnd ?? 0;
}

function findLevelDefinitions(source, levelsRange) {
  const lines = sourceLinesWithOffsets(source);
  const entries = [];
  let index = lines.findIndex((line) => line.absoluteEnd >= levelsRange.bodyStart);
  if (index < 0) {
    return entries;
  }

  while (index < lines.length && lines[index].start <= levelsRange.bodyEnd) {
    const line = lines[index];
    if (line.start < levelsRange.bodyStart) {
      index += 1;
      continue;
    }
    const code = levelScannerCode(line.raw);
    if (!code) {
      index += 1;
      continue;
    }
    const tokens = splitLevelTokens(code);
    if (isLevelsSectionBoundary(tokens) || code === "}" || code === "end") {
      break;
    }

    let entry = null;
    const ordinal = entries.length + 1;
    if (tokens[0] === "level") {
      const nameTokens = tokens.at(-1) === "{" ? tokens.slice(1, -1) : tokens.slice(1);
      const name = levelDefinitionName(levelsRange, levelNameFromTokens(nameTokens), ordinal);
      entry = tokens.at(-1) === "{"
        ? bracedLevelEntry(source, lines, index, name, levelsRange.bodyEnd)
        : unbracedLevelEntry(lines, index, index + 1, name, levelsRange.bodyEnd);
    } else if (tokens.length === 1 && tokens[0] === "{") {
      entry = bracedLevelEntry(source, lines, index, levelDefinitionName(levelsRange, "", ordinal), levelsRange.bodyEnd);
    } else if (tokens.at(-1) === "{") {
      entry = bracedLevelEntry(
        source,
        lines,
        index,
        levelDefinitionName(levelsRange, levelNameFromTokens(tokens.slice(0, -1)), ordinal),
        levelsRange.bodyEnd,
      );
    } else {
      entry = unbracedLevelEntry(lines, index, index, levelDefinitionName(levelsRange, "", ordinal), levelsRange.bodyEnd);
    }

    if (!entry) {
      index += 1;
      continue;
    }
    entries.push(entry);
    index = Math.max(index + 1, entry.nextIndex);
  }
  return assignLevelLevelIndexes(entries);
}

function findStandaloneLevelDefinitionAtPosition(source, position) {
  const lines = sourceLinesWithOffsets(source);
  for (let index = 0; index < lines.length; index += 1) {
    const code = levelScannerCode(lines[index].raw);
    const tokens = splitLevelTokens(code);
    if (tokens[0] !== "level") {
      continue;
    }
    const entry = tokens.at(-1) === "{"
      ? bracedLevelEntry(source, lines, index, levelNameFromTokens(tokens.slice(1, -1)), source.length)
      : endDelimitedStandaloneLevelEntry(lines, index, levelNameFromTokens(tokens.slice(1)));
    if (entry && position >= entry.start && position <= entry.end) {
      return assignLevelLevelIndexes([entry])[0] || null;
    }
  }
  return null;
}

function endDelimitedStandaloneLevelEntry(lines, headerIndex, name) {
  let index = headerIndex + 1;
  let nestedDepth = 0;
  let lastContentEnd = lines[headerIndex].end;
  while (index < lines.length) {
    const line = lines[index];
    const code = levelScannerCode(line.raw);
    if (code) {
      const normalized = braceNormalizedLineForSectionForWasm(code);
      const tokens = splitLevelTokens(normalized);
      if (normalized === "end") {
        if (nestedDepth === 0) {
          return {
            name,
            start: firstCodeIndex(lines[headerIndex]),
            end: line.start,
            nextIndex: index + 1,
          };
        }
        nestedDepth -= 1;
      } else if (startsLevelBodyBlock(tokens, normalized)) {
        nestedDepth += 1;
      }
    }
    lastContentEnd = line.end;
    index += 1;
  }
  return {
    name,
    start: firstCodeIndex(lines[headerIndex]),
    end: lastContentEnd,
    nextIndex: lines.length,
  };
}

function bracedLevelEntry(source, lines, lineIndex, name, rangeEnd) {
  const line = lines[lineIndex];
  const openIndex = source.indexOf("{", line.start);
  const closeIndex = findMatchingBrace(source, openIndex);
  if (openIndex < 0 || closeIndex < 0 || closeIndex > rangeEnd) {
    return null;
  }
  return {
    name,
    start: firstCodeIndex(line),
    end: closeIndex,
    nextIndex: nextLineIndexAfterPosition(lines, closeIndex),
  };
}

function unbracedLevelEntry(lines, headerIndex, contentIndex, name, rangeEnd) {
  let index = contentIndex;
  let nestedDepth = 0;
  let lastContentEnd = lines[headerIndex].end;
  while (index < lines.length && lines[index].start <= rangeEnd) {
    const line = lines[index];
    const code = levelScannerCode(line.raw);
    if (nestedDepth === 0 && (!code || code === "end" || code === "}" || isLevelHeaderCode(code) || isLevelsSectionBoundary(splitLevelTokens(code)))) {
      break;
    }
    if (code) {
      const normalized = braceNormalizedLineForSectionForWasm(code);
      const tokens = splitLevelTokens(normalized);
      if (normalized === "end" || normalized === "}") {
        nestedDepth = Math.max(0, nestedDepth - 1);
      } else if (startsLevelBodyBlock(tokens, normalized)) {
        nestedDepth += 1;
      }
    }
    lastContentEnd = Math.min(line.end, rangeEnd);
    index += 1;
  }
  return {
    name,
    start: firstCodeIndex(lines[headerIndex]),
    end: lastContentEnd,
    nextIndex: index,
  };
}

function isLevelHeaderCode(code) {
  const tokens = splitLevelTokens(code);
  return tokens[0] === "level"
    || (tokens.length === 1 && tokens[0] === "{")
    || (tokens.at(-1) === "{" && tokens[0] !== "legend");
}

function startsLevelBodyBlock(tokens, line) {
  return (tokens.length === 1 && tokens[0] === "legend") || isLevelLifecycleHeader(tokens);
}

function startsLevelNestedBlock(tokens, line) {
  return (tokens[0] === "level" && tokens.at(-1) === "{")
    || (tokens.length === 1 && tokens[0] === "{")
    || (tokens.at(-1) === "{" && tokens[0] !== "level")
    || (tokens[0] !== "level" && startsInlineBlockForWasm(tokens, line));
}

function isLevelsSectionBoundary(tokens) {
  return startsPuzzleSectionForWasm(tokens) && !["level"].includes(tokens[0] || "");
}

function levelNameFromTokens(tokens) {
  return tokens.filter(Boolean).join(" ");
}

function levelsNamespaceFromTokens(tokens) {
  const parts = tokens.at(-1) === "{" ? tokens.slice(1, -1) : tokens.slice(1);
  if (!parts.length) {
    return "";
  }
  const ofIndex = parts.indexOf("of");
  const namespaceParts = ofIndex >= 0 ? parts.slice(0, ofIndex) : parts;
  return namespaceParts.length === 1 ? namespaceParts[0] : "";
}

function levelDefinitionName(levelsRange, name, ordinal) {
  const namespace = String(levelsRange?.namespace || "").trim();
  const rawName = String(name || "").trim();
  if (!rawName) {
    return namespace ? `${namespace}.${ordinal}` : "";
  }
  if (namespace && !rawName.startsWith(`${namespace}.`)) {
    return `${namespace}.${rawName}`;
  }
  return rawName;
}

function levelScannerCode(line) {
  return stripLineCommentForWasm(line).trim();
}

function splitLevelTokens(line) {
  return String(line || "").split(/\s+/).filter(Boolean);
}

function sourceLinesWithOffsets(source) {
  const lines = [];
  let start = 0;
  const text = String(source || "");
  for (const raw of text.split("\n")) {
    const end = start + raw.length;
    const hasNewline = end < text.length;
    lines.push({
      raw,
      start,
      end,
      absoluteEnd: end + (hasNewline ? 1 : 0),
      hasNewline,
    });
    start = end + 1;
  }
  return lines;
}

function firstCodeIndex(line) {
  const offset = line.raw.search(/\S/);
  return line.start + Math.max(0, offset);
}

function nextLineIndexAfterPosition(lines, position) {
  const index = lines.findIndex((line) => line.start > position);
  return index < 0 ? lines.length : index;
}

function isSectionTitleLine(rawLines, index) {
  return index > 0
    && index + 1 < rawLines.length
    && isSectionSeparatorForWasm(stripLineCommentForWasm(rawLines[index - 1]).trim())
    && isSectionSeparatorForWasm(stripLineCommentForWasm(rawLines[index + 1]).trim());
}

function assignLevelLevelIndexes(entries) {
  const levels = previewExport?.levels || [];
  const usedIndexes = new Set();
  return entries.map((entry, ordinal) => {
    let levelIndex = -1;
    if (entry.name) {
      levelIndex = levels.findIndex((levelData, index) => (
        !usedIndexes.has(index) && levelData?.name === entry.name
      ));
    }
    if (levelIndex < 0 && ordinal < levels.length && !usedIndexes.has(ordinal)) {
      levelIndex = ordinal;
    }
    if (levelIndex < 0) {
      levelIndex = Math.max(0, Math.min(levels.length - 1, ordinal));
    }
    usedIndexes.add(levelIndex);
    return {
      ...entry,
      levelIndex,
    };
  });
}

function makeEmptyCells(width, height) {
  return Array.from({ length: width * height }, () => makeEmptyCell());
}

function makeEmptyCell(exportData = previewExport) {
  return Array.from({ length: layerCount(exportData) }, () => 0);
}

function cloneCellSlots(slots, exportData = previewExport) {
  const next = makeEmptyCell(exportData);
  if (Array.isArray(slots)) {
    for (let index = 0; index < Math.min(slots.length, next.length); index += 1) {
      next[index] = Number(slots[index]) || 0;
    }
  }
  return next;
}

function renderLevelPalette() {
  const toggleButton = levelPaletteCollapseButton;
  levelPalette.replaceChildren(toggleButton);
  levelPalette.classList.add("is-sprite-only");
  levelPalette.classList.toggle("is-collapsed", level.paletteCollapsed);
  levelPaletteCollapseButton.classList.toggle("is-active", level.paletteCollapsed);
  levelPaletteCollapseButton.classList.toggle("is-collapsed", level.paletteCollapsed);
  levelPaletteCollapseButton.setAttribute("aria-expanded", String(!level.paletteCollapsed));
  levelPaletteCollapseButton.setAttribute("aria-label", level.paletteCollapsed ? "Show palette" : "Hide palette");
  levelPaletteCollapseButton.title = level.paletteCollapsed ? "Show palette" : "Hide palette";
  const mainObjects = level.palette.filter((object) => object.id === 0 || !isVisualObject(object));
  const visualObjects = level.palette.filter((object) => object.id !== 0 && isVisualObject(object));
  renderLevelPaletteGroup("", mainObjects);
  renderLevelPaletteGroup("Visual", visualObjects);
  updateLevelPlaytestControls();
}

function renderLevelPaletteGroup(label, objects) {
  if (!objects.length) {
    return;
  }
  const group = document.createElement("div");
  group.className = "level-palette-group";
  if (label) {
    const heading = document.createElement("div");
    heading.className = "level-palette-heading";
    heading.textContent = label;
    group.append(heading);
  }
  for (const object of objects) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "level-token";
    button.classList.toggle("is-selected", object.id === level.selectedObjectId);
    button.title = object.name;
    button.dataset.label = object.name;
    button.setAttribute("aria-label", `Paint ${object.name}`);
    button.append(renderObjectPreview(object));

    const label = document.createElement("span");
    label.className = "tile-label";
    label.textContent = object.name;
    button.append(label);

    button.addEventListener("click", () => {
      level.selectedObjectId = object.id;
      renderLevelPalette();
    });
    group.append(button);
  }
  levelPalette.append(group);
}

function renderLevelBoard() {
  updateLevelSizeLabel();
  renderLevelSourcePreview();
  const cells = displayedLevelCells();
  if (levelRenderer) {
    levelRenderer.render(levelScene(cells));
    levelBoard.querySelectorAll(".cell").forEach((cell, index) => {
      cell.dataset.index = String(index);
      cell.setAttribute("aria-label", cellLabel(cells[index]));
      cell.setAttribute("role", "button");
      cell.tabIndex = 0;
    });
    syncLevelBoardScale();
    scheduleBoardScaleSync();
    renderSolverBoard();
    return;
  }
  levelBoard.replaceChildren();
  syncLevelBoardScale();
  scheduleBoardScaleSync();
  renderSolverBoard();
}

function renderSolverBoard() {
  if (!solverBoard) {
    return;
  }
  const cells = displayedSolverCells();
  if (solverRenderer) {
    solverRenderer.render(levelScene(cells));
    syncSolverBoardScale();
    scheduleBoardScaleSync();
    return;
  }
  solverBoard.replaceChildren();
  syncSolverBoardScale();
  scheduleBoardScaleSync();
}

function scheduleBoardScaleSync(passes = 2) {
  boardScaleSyncPasses = Math.max(boardScaleSyncPasses, Math.max(1, Math.trunc(Number(passes) || 1)));
  if (boardScaleSyncFrame) {
    return;
  }
  const tick = () => {
    boardScaleSyncFrame = 0;
    if (!levelBuilder.hidden) {
      syncLevelBoardScale();
    }
    if (!solverPanel.hidden) {
      syncSolverBoardScale();
    }
    boardScaleSyncPasses -= 1;
    if (boardScaleSyncPasses > 0) {
      boardScaleSyncFrame = requestAnimationFrame(tick);
    }
  };
  boardScaleSyncFrame = requestAnimationFrame(tick);
}

function syncLevelBoardScale() {
  const wrap = levelBoardViewport?.closest(".level-board-wrap");
  syncBoardViewportScale(wrap, levelBoardViewport, levelBoard, boardFrameSize(levelBoard, level.width, level.height), {
    width: levelEditorEdgeSize * 2 + levelEditorGap * 2,
    height: levelEditorEdgeSize * 2 + levelEditorGap * 2,
  });
}

function syncSolverBoardScale() {
  const wrap = solverBoardViewport?.closest(".solver-board-wrap");
  syncBoardViewportScale(wrap, solverBoardViewport, solverBoard, boardFrameSize(solverBoard, level.width, level.height));
}

function syncBoardViewportScale(wrap, viewport, board, frame, chrome = {}) {
  if (!wrap || !viewport || !board || !frame) {
    return;
  }
  if (wrap.getClientRects().length === 0 || viewport.getClientRects().length === 0) {
    return;
  }
  const frameWidth = Math.max(1, Number(frame.width || 1));
  const frameHeight = Math.max(1, Number(frame.height || 1));
  const chromeWidth = Math.max(0, Number(chrome.width || 0));
  const chromeHeight = Math.max(0, Number(chrome.height || 0));
  const availableWidth = boardViewportContentWidth(wrap) - chromeWidth;
  if (availableWidth <= 0) {
    return;
  }
  const maxCellSize = Math.max(1, Math.floor(editorPuzzleCellSize()));
  const fitCellSize = Math.max(1, Math.min(maxCellSize, Math.floor(availableWidth / frameWidth)));
  const cellSize = quantizedEditorCellSize(fitCellSize, editorPuzzleQuantum(board));
  const boardWidth = frameWidth * cellSize;
  const boardHeight = frameHeight * cellSize;
  const naturalWidth = boardWidth + chromeWidth;
  const naturalHeight = boardHeight + chromeHeight;
  wrap.style.setProperty("--editor-board-cell-size", `${cellSize}px`);
  wrap.style.setProperty("--board-natural-width", `${Math.ceil(naturalWidth)}px`);
  wrap.style.setProperty("--board-natural-height", `${Math.ceil(naturalHeight)}px`);
  wrap.style.setProperty("--board-scale", "1");
  wrap.style.setProperty("--board-viewport-width", `${Math.ceil(naturalWidth)}px`);
  wrap.style.setProperty("--board-viewport-height", `${Math.ceil(naturalHeight)}px`);
}

function boardViewportContentWidth(wrap) {
  if (!wrap) {
    return 0;
  }
  const parent = wrap.parentElement;
  const parentWidth = elementContentWidth(parent);
  if (parentWidth > 0) {
    const style = window.getComputedStyle(wrap);
    const horizontalSpace =
      parseFloat(style.marginLeft || "0") +
      parseFloat(style.marginRight || "0") +
      parseFloat(style.borderLeftWidth || "0") +
      parseFloat(style.borderRightWidth || "0") +
      parseFloat(style.paddingLeft || "0") +
      parseFloat(style.paddingRight || "0");
    return Math.max(0, parentWidth - horizontalSpace);
  }
  return elementContentWidth(wrap);
}

function editorPuzzleCellSize() {
  const configured = Number(window.GameVisuals?.editorPuzzle?.cellSize);
  return Number.isFinite(configured) && configured > 0 ? configured : boardVirtualCellSize;
}

function editorPuzzleQuantum(board) {
  let quantum = 1;
  for (const sprite of board.querySelectorAll(".visual-sprite")) {
    const style = window.getComputedStyle(sprite);
    const cols = Math.max(1, Math.trunc(Number(style.getPropertyValue("--sprite-cols")) || 1));
    const rows = Math.max(1, Math.trunc(Number(style.getPropertyValue("--sprite-rows")) || 1));
    quantum = boundedLeastCommonMultiple(quantum, cols, 512);
    quantum = boundedLeastCommonMultiple(quantum, rows, 512);
  }
  return quantum > 1 && quantum <= 128 ? quantum : 1;
}

function quantizedEditorCellSize(size, quantum) {
  const cellSize = Math.max(1, Math.floor(size));
  const step = Math.max(1, Math.floor(quantum || 1));
  if (step <= 1 || cellSize < step) {
    return cellSize;
  }
  return Math.max(step, Math.floor(cellSize / step) * step);
}

function boundedLeastCommonMultiple(a, b, limit) {
  const left = Math.max(1, Math.trunc(Number(a) || 1));
  const right = Math.max(1, Math.trunc(Number(b) || 1));
  const value = (left / greatestCommonDivisor(left, right)) * right;
  return value > limit ? limit + 1 : value;
}

function greatestCommonDivisor(a, b) {
  let left = Math.abs(Math.trunc(a));
  let right = Math.abs(Math.trunc(b));
  while (right) {
    const next = left % right;
    left = right;
    right = next;
  }
  return left || 1;
}

function boardFrameSize(board, fallbackWidth, fallbackHeight) {
  const width = Math.max(1, Number(board?.dataset.frameWidth || fallbackWidth || 1));
  const height = Math.max(1, Number(board?.dataset.frameHeight || fallbackHeight || 1));
  return { width, height };
}

function loadLevelFromPreviewState(options = {}) {
  const requestRender = options.requestRender !== false;
  const levelIndex = currentEditableLevelIndex();
  const scene = previewSceneForLevel(levelIndex);
  if (!scene?.width || !scene?.height || !Array.isArray(scene.cells)) {
    return false;
  }
  clearSolutionPreview();
  stopLevelPlaytest({ syncPreview: false });
  levelDisplayCells = null;
  level.width = scene.width;
  level.height = scene.height;
  level.regions = normalizedLevelRegions(scene.regions, level.width, level.height);
  level.cells = scene.cells.map((cell) => cellSlotsFromLayers(cell.layers || []));
  const levelName = previewExport?.levels?.[levelIndex]?.name;
  if (levelName) {
    setLevelNameInputs(levelName);
  }
  renderLevelBoard();
  if (requestRender) {
    sendLevelStateToPreview(levelIndex, levelStateData(previewExport));
  }
  return true;
}

function applyPreviewSceneToLevel(scene) {
  if (!scene?.width || !scene?.height || !Array.isArray(scene.cells)) {
    return false;
  }
  clearSolutionPreview();
  stopLevelPlaytest({ syncPreview: false });
  levelDisplayCells = null;
  level.width = scene.width;
  level.height = scene.height;
  level.regions = normalizedLevelRegions(scene.regions, level.width, level.height);
  level.cells = scene.cells.map((cell) => cellSlotsFromLayers(cell.layers || []));
  renderLevelBoard();
  scheduleBoardScaleSync(2);
  return true;
}

function initialPreviewScene() {
  return previewSceneForLevel(previewExport?.initialLevelIndex || 0);
}

function previewSceneForLevel(levelIndex, exportData = previewExport) {
  const index = Math.max(0, Math.trunc(Number(levelIndex) || 0));
  const state = exportData?.levels?.[index]?.initialState;
  if (!state) {
    return null;
  }
  const regions = exportData.levels?.[index]?.regions || [];
  return sceneFromStateData(state, { regions, exportData });
}

function sceneFromStateData(state, options = {}) {
  if (!state?.width || !state?.height || !state?.layerCount || !Array.isArray(state.slots)) {
    return null;
  }
  const exportData = options.exportData || previewExport;
  const objectsById = new Map((exportData.engine?.objects || []).map((object) => [object.id, object]));
  const cells = [];
  for (let y = 0; y < state.height; y += 1) {
    for (let x = 0; x < state.width; x += 1) {
      const layers = [];
      for (let layer = 0; layer < state.layerCount; layer += 1) {
        const objectId = state.slots[((y * state.width + x) * state.layerCount) + layer];
        const object = objectsById.get(objectId);
        if (object) {
          layers.push({
            layer,
            objectId,
            object: object.name,
            sprite: object.sprite,
          });
        }
      }
      cells.push({ x, y, layers });
    }
  }
  return {
    width: state.width,
    height: state.height,
    layerCount: state.layerCount,
    regions: options.regions || [],
    cells,
  };
}

function cellSlotsFromLayers(layers, exportData = previewExport) {
  const slots = makeEmptyCell(exportData);
  for (const layer of layers) {
    if (Number.isInteger(layer.layer) && layer.layer >= 0 && layer.layer < slots.length) {
      slots[layer.layer] = objectIdForLayer(layer, exportData);
    }
  }
  return slots;
}

function objectIdForLayer(layer, exportData = previewExport) {
  const explicit = Number(layer?.objectId) || 0;
  if (explicit) {
    return explicit;
  }
  const name = layer?.object || "";
  const sprite = layer?.sprite || "";
  const object = (exportData?.engine?.objects || []).find((entry) =>
    (name && entry.name === name) || (sprite && entry.sprite === sprite)
  );
  return object?.id || 0;
}

function renderObjectPreview(object) {
  const root = document.createElement("span");
  root.className = "game-preview-scope level-token-visual board";
  root.setAttribute("aria-hidden", "true");
  if (window.PuzzleRenderer) {
    new window.PuzzleRenderer(root, { renderMode: "dom", themeRoot: root }).render(objectScene(object));
  }
  return root;
}

function objectScene(object) {
  const slots = makeEmptyCell();
  if (object?.id && Number.isInteger(object.layer) && object.layer >= 0 && object.layer < slots.length) {
    slots[object.layer] = object.id;
  }
  return sceneFromCellSlots([slots], {
    width: 1,
    height: 1,
    regions: [],
  });
}

function levelScene(sourceCells = level.cells) {
  return sceneFromCellSlots(sourceCells, {
    width: level.width,
    height: level.height,
    regions: levelRegions(),
  });
}

function sceneFromCellSlots(sourceCells, options = {}) {
  const width = Math.max(1, Number(options.width || level.width || 1));
  const height = Math.max(1, Number(options.height || level.height || 1));
  const cells = sourceCells.map((slots, index) => ({
    x: index % width,
    y: Math.floor(index / width),
    layers: layersForSlots(normalizedCellSlots(slots)),
  }));
  return {
    width,
    height,
    layerCount: layerCount(),
    regions: options.regions || [],
    cells,
  };
}

function normalizedCellSlots(slots, exportData = previewExport) {
  if (Array.isArray(slots) && slots.length === layerCount(exportData)) {
    return slots;
  }
  const next = makeEmptyCell(exportData);
  for (const objectId of slots || []) {
    const object = engineObjectById(objectId, exportData);
    if (object) {
      next[object.layer] = object.id;
    }
  }
  return next;
}

function displayedLevelCells() {
  return levelPlaytestActive && levelDisplayCells?.length === level.cells.length ? levelDisplayCells : level.cells;
}

function displayedSolverCells() {
  if (levelSolutionPreview) {
    return levelSolutionPreview.cells;
  }
  return displayedLevelCells();
}

function layersForSlots(slots, exportData = previewExport) {
  return cloneCellSlots(slots, exportData)
    .map((objectId) => engineObjectById(objectId, exportData))
    .filter(Boolean)
    .map(layerForObject)
    .sort((left, right) => left.layer - right.layer);
}

function layerForObject(object) {
  return {
    layer: object.layer,
    objectId: object.id,
    object: object.name,
    sprite: object.sprite,
  };
}

function cellLabel(slots) {
  const names = layersForSlots(slots).map((layer) => layer.object);
  return names.length ? names.join(", ") : "Empty";
}

function addLevelEdge(edge) {
  clearSolutionPreview();
  levelDisplayCells = null;
  const nextWidth = level.width + ((edge === "left" || edge === "right") ? 1 : 0);
  const nextHeight = level.height + ((edge === "top" || edge === "bottom") ? 1 : 0);
  if (nextWidth > 40 || nextHeight > 30) {
    setStatus("Level size limit", "is-error");
    return;
  }

  const nextCells = makeEmptyCells(nextWidth, nextHeight);
  const offsetX = edge === "left" ? 1 : 0;
  const offsetY = edge === "top" ? 1 : 0;
  for (let y = 0; y < level.height; y += 1) {
    for (let x = 0; x < level.width; x += 1) {
      nextCells[(y + offsetY) * nextWidth + x + offsetX] = cloneCellSlots(level.cells[y * level.width + x]);
    }
  }

  level.width = nextWidth;
  level.height = nextHeight;
  level.regions = resizeLevelRegions(levelRegions(), edge, nextWidth, nextHeight);
  level.cells = nextCells;
  setLevelSolveStatus("");
  renderLevelBoard();
  syncPreviewStateFromLevel();
  setStatus("Level resized", "is-ok");
}

function updateLevelSizeLabel() {
  levelSizeLabel.textContent = `${level.width} × ${level.height}`;
}

function paintLevelCellFromElement(element) {
  const index = levelCellIndexFromElement(element);
  return paintLevelCellAtIndex(index, level.selectedObjectId);
}

function levelCellIndexFromElement(element) {
  const cell = element?.closest?.(".cell");
  if (!cell || !levelBoard.contains(cell)) {
    return -1;
  }
  const index = Number(cell.dataset.index);
  if (!Number.isInteger(index) || index < 0 || index >= level.cells.length) {
    return -1;
  }
  return index;
}

function paintLevelCellAtIndex(index, objectId) {
  if (levelPlaytestActive) {
    return false;
  }
  clearSolutionPreview();
  levelDisplayCells = null;
  if (!Number.isInteger(index) || index < 0 || index >= level.cells.length) {
    return false;
  }
  const next = paintCellSlots(level.cells[index], objectId);
  if (sameCellSlots(level.cells[index], next)) {
    return false;
  }
  level.cells[index] = next;
  setLevelSolveStatus("");
  renderLevelBoard();
  syncPreviewStateFromLevel();
  return true;
}

function paintLevelCellFromPoint(clientX, clientY, objectId) {
  return paintLevelCellAtIndex(
    levelCellIndexFromElement(document.elementFromPoint(clientX, clientY)),
    objectId,
  );
}

function startLevelPaint(event) {
  if (levelPlaytestActive) {
    return;
  }
  if (event.button !== 0) {
    return;
  }
  const objectId = level.selectedObjectId;
  const index = levelCellIndexFromElement(document.elementFromPoint(event.clientX, event.clientY));
  if (!Number.isInteger(index) || index < 0) {
    return;
  }
  event.preventDefault();
  levelPaintDrag = {
    pointerId: event.pointerId,
    objectId,
    lastIndex: -1,
  };
  if (levelBoard.setPointerCapture) {
    levelBoard.setPointerCapture(event.pointerId);
  }
  paintLevelDragIndex(index);
}

function continueLevelPaint(event) {
  if (!levelPaintDrag || levelPaintDrag.pointerId !== event.pointerId) {
    return;
  }
  event.preventDefault();
  const element = document.elementFromPoint(event.clientX, event.clientY);
  paintLevelDragIndex(levelCellIndexFromElement(element));
}

function stopLevelPaint(event) {
  if (!levelPaintDrag || levelPaintDrag.pointerId !== event.pointerId) {
    return;
  }
  if (levelBoard.hasPointerCapture?.(event.pointerId)) {
    levelBoard.releasePointerCapture(event.pointerId);
  }
  levelPaintDrag = null;
}

function paintLevelDragIndex(index) {
  if (!levelPaintDrag || !Number.isInteger(index) || index < 0) {
    return;
  }
  if (index === levelPaintDrag.lastIndex) {
    return;
  }
  levelPaintDrag.lastIndex = index;
  paintLevelCellAtIndex(index, levelPaintDrag.objectId);
}

function startLevelPlaytest() {
  if (levelPlaytestActive) {
    return;
  }
  const exportData = previewExport || extractPreviewExport(latestHtml);
  if (!exportData) {
    setStatus("No level to play", "is-error");
    return;
  }
  const stateData = levelStateData(exportData);
  if (!stateData) {
    setStatus("No level to play", "is-error");
    return;
  }
  clearSolutionPreview();
  levelPlaytestActive = true;
  levelDisplayCells = null;
  pendingPreviewKeyStateSync = 0;
  updateLevelPlaytestControls();
  renderLevelBoard();
  sendLevelStateToPreview(currentEditableLevelIndex(exportData), stateData, {
    materializeLevelStart: true,
    materializeDisplay: true,
    silent: false,
  });
  levelBoard?.focus?.();
}

function stopLevelPlaytest(options = {}) {
  if (!levelPlaytestActive && !levelDisplayCells) {
    updateLevelPlaytestControls();
    return;
  }
  levelPlaytestActive = false;
  levelDisplayCells = null;
  pendingPreviewKeyStateSync = 0;
  if (levelPaintDrag && levelBoard.hasPointerCapture?.(levelPaintDrag.pointerId)) {
    levelBoard.releasePointerCapture(levelPaintDrag.pointerId);
  }
  levelPaintDrag = null;
  updateLevelPlaytestControls();
  renderLevelBoard();
  if (options.syncPreview !== false) {
    const exportData = previewExport || extractPreviewExport(latestHtml);
    const stateData = exportData ? levelStateData(exportData) : null;
    if (stateData) {
      sendLevelStateToPreview(currentEditableLevelIndex(exportData), stateData, {
        materializeLevelStart: false,
        materializeDisplay: false,
        silent: true,
      });
    }
  }
}

function toggleLevelPlaytest() {
  if (levelPlaytestActive) {
    stopLevelPlaytest();
  } else {
    startLevelPlaytest();
  }
}

function updateLevelPlaytestControls() {
  if (!levelBuilder) {
    return;
  }
  levelBuilder.classList.toggle("is-playtesting", levelPlaytestActive);
  if (levelPlaytestButton) {
    const label = levelPlaytestActive ? "Stop level playtest" : "Play level";
    levelPlaytestButton.classList.toggle("is-playing", levelPlaytestActive);
    levelPlaytestButton.setAttribute("aria-label", label);
    levelPlaytestButton.title = label;
  }
  for (const element of [
    levelNamespaceInput,
    levelNameInput,
    copyLevelButton,
    addLevelButton,
    updateLevelButton,
    levelPaletteCollapseButton,
  ]) {
    if (element) {
      element.disabled = levelPlaytestActive;
    }
  }
  levelPalette?.querySelectorAll("button").forEach((button) => {
    button.disabled = levelPlaytestActive;
  });
  levelEdgeButtons.forEach((button) => {
    button.disabled = levelPlaytestActive;
  });
}

function sameCellSlots(left, right) {
  if (!Array.isArray(left) || !Array.isArray(right) || left.length !== right.length) {
    return false;
  }
  return left.every((value, index) => value === right[index]);
}

function paintCellSlots(slots, objectId) {
  if (!objectId) {
    return makeEmptyCell();
  }
  const object = engineObjectById(objectId);
  if (!object) {
    return cloneCellSlots(slots);
  }
  const next = cloneCellSlots(slots);
  next[object.layer] = object.id;
  return next;
}

function syncPreviewStateFromLevel() {
  const exportData = previewExport || extractPreviewExport(latestHtml);
  if (!exportData) {
    return;
  }
  const stateData = levelStateData(exportData);
  if (!stateData) {
    return;
  }

  const levelIndex = currentEditableLevelIndex(exportData);
  if (previewExport?.levels?.[levelIndex]) {
    const nextExport = JSON.parse(JSON.stringify(previewExport));
    nextExport.levels[levelIndex].initialState = stateData;
    nextExport.levels[levelIndex].regions = levelRegions();
    previewExport = nextExport;
    const nextHtml = replacePreviewExport(latestHtml, nextExport);
    if (nextHtml) {
      latestHtml = nextHtml;
      const previewDocument = activePreviewDocument();
      if (previewDocument) {
        previewDocument.previewHtml = nextHtml;
      }
      scheduleLocalSave();
    }
  }

  latestPreviewState = {
    ...(latestPreviewState || {}),
    levelIndex,
    scene: null,
  };

  sendLevelStateToPreview(levelIndex, stateData);
}

function sendLevelStateToPreview(levelIndex = currentEditableLevelIndex(), stateData = null, options = {}) {
  const exportData = previewExport || extractPreviewExport(latestHtml);
  const state = stateData || levelStateData(exportData);
  if (!state) {
    return;
  }
  const materializeLevelStart = options.materializeLevelStart ?? (currentPreviewMode === "play" || levelPlaytestActive);
  const materializeDisplay = options.materializeDisplay ?? (currentPreviewMode === "play" || levelPlaytestActive);
  previewFrame.contentWindow?.postMessage({
    type: "PuzzleStudioSetState",
    levelIndex,
    state,
    regions: levelRegions(),
    materializeLevelStart,
    materializeDisplay,
    silent: options.silent ?? (currentPreviewMode !== "play" && !levelPlaytestActive),
  }, "*");
}

async function solveLevel() {
  if (activeLevelSolveRequest) {
    cancelLevelSolve();
    return;
  }
  const exportData = previewExport || extractPreviewExport(latestHtml);
  if (!exportData) {
    setLevelSolveStatus("No preview to solve", "is-error");
    return;
  }
  const stateData = levelStateData(exportData);
  if (!stateData) {
    setLevelSolveStatus("No level state", "is-error");
    return;
  }
  if (!previewFrame.contentWindow) {
    setLevelSolveStatus("Preview is not ready", "is-error");
    return;
  }

  clearSolutionPreview();
  renderLevelBoard();
  const requestId = createDocumentId();
  activeLevelSolveRequest = { id: requestId, backend: "wasm" };
  setSolveLevelButtonState(true);
  setLevelSolveStatus("Solving", "");
  syncPreviewStateFromLevel();
  try {
    await new Promise((resolve) => setTimeout(resolve, 0));
    const compiler = await loadWasmCompiler();
    if (typeof compiler.solve_state !== "function") {
      throw new Error("Solver is not available");
    }
    const solution = JSON.parse(compiler.solve_state(
      exportData.source || activePreviewSource(),
      exportData.puzzlePath || activePreviewDocument()?.puzzlePath || "game.puzzle",
      JSON.stringify(stateData),
      512,
      5_000_000,
      0,
    ));
    handleLevelSolveResult({ requestId, solution });
  } catch (error) {
    handleLevelSolveResult({
      requestId,
      error: `Solver failed: ${userFacingRuntimeError(error)}`,
    });
  }
}

function cancelLevelSolve() {
  if (!activeLevelSolveRequest || activeLevelSolveRequest.backend === "wasm" || !previewFrame.contentWindow) {
    return;
  }
  previewFrame.contentWindow.postMessage({
    type: "PuzzleStudioCancelSolve",
    requestId: activeLevelSolveRequest.id,
  }, "*");
  setLevelSolveStatus("Cancelling", "");
}

function setSolveLevelButtonState(isSolving) {
  const label = isSolving ? "Cancel solve" : "Solve level";
  const visibleLabel = isSolving ? "Cancel" : "Solve";
  solveLevelButton.classList.toggle("is-solving", Boolean(isSolving));
  solveLevelButton.setAttribute("aria-label", label);
  solveLevelButton.title = label;
  solveLevelButton.querySelector(".solve-button-label").textContent = visibleLabel;
}

function handleLevelSolveProgress(message) {
  if (!activeLevelSolveRequest || message.requestId !== activeLevelSolveRequest.id) {
    return;
  }
  const progress = message.progress || {};
  setLevelSolveStatus(
    `Solving: ${formatNumber(progress.visited || 0)} states, depth ${progress.maxDepthReached || 0}, frontier ${formatNumber(progress.frontier || 0)}, ${formatSeconds(progress.elapsedMs || 0)}`,
    "",
  );
}

function handleLevelSolveResult(message) {
  if (!activeLevelSolveRequest || message.requestId !== activeLevelSolveRequest.id) {
    return;
  }
  activeLevelSolveRequest = null;
  setSolveLevelButtonState(false);

  if (message.error) {
    setLevelSolveStatus(message.error, "is-error");
    return;
  }

  const solution = message.solution;
  if (!solution) {
    setLevelSolveStatus("No solver result", "is-error");
    return;
  }

  if (solution.result === "solved") {
    showSolutionPreview(solution);
    return;
  }

  if (solution.result === "cancelled") {
    setLevelSolveStatus("Cancelled", "");
    return;
  }

  const stats = solution.stats;
  const reason = solution.reason ? `: ${solution.reason}` : "";
  const suffix = stats
    ? ` (${stats.visited} states, depth ${stats.maxDepthReached}, ${stats.elapsedMs}ms)`
    : "";
  setLevelSolveStatus(`${titleLabel(solution.result)}${reason}${suffix}`, "is-error");
}

function setLevelSolveStatus(text, className = "") {
  if (levelSolveFlashTimer) {
    window.clearTimeout(levelSolveFlashTimer);
    levelSolveFlashTimer = 0;
    levelSolveFlashRestore = null;
  }
  levelSolveStatus.className = `level-solve-status ${className}`.trim();
  levelSolveStatus.textContent = text;
}

function userFacingRuntimeError(error) {
  const message = String(error?.message || error || "unknown error");
  return /\b(wasm|webassembly|rust)\b/i.test(message)
    ? "browser runtime could not start"
    : message;
}

function flashLevelSolveStatus(text, className = "", duration = 900) {
  const restore = levelSolveFlashRestore || {
    text: levelSolveStatus.textContent,
    className: [...levelSolveStatus.classList]
      .filter((name) => name !== "level-solve-status")
      .join(" "),
  };
  setLevelSolveStatus(text, className);
  levelSolveFlashRestore = restore;
  levelSolveFlashTimer = window.setTimeout(() => {
    const next = levelSolveFlashRestore;
    levelSolveFlashTimer = 0;
    levelSolveFlashRestore = null;
    setLevelSolveStatus(next?.text || "", next?.className || "");
  }, duration);
}

function showSolutionPreview(solution) {
  const steps = Array.isArray(solution.steps) ? solution.steps : [];
  if (!steps.length) {
    setLevelSolveStatus("Solved, but no steps were returned", "is-error");
    return;
  }
  levelSolutionPreview = {
    steps,
    moves: solutionMoves(solution),
    index: 0,
    cells: sceneCellsToSlots(steps[0].scene, level.cells),
  };
  updateSolutionControls();
  renderLevelBoard();
  setLevelSolveStatus(solution.depth ? `Solved in ${solution.depth} moves` : "Already solved", "is-ok");
}

function solutionMoves(solution) {
  if (Array.isArray(solution.moves) && solution.moves.length) {
    return solution.moves;
  }
  return (solution.steps || [])
    .map((step) => step.move)
    .filter(Boolean);
}

function sceneCellsToSlots(scene, fallback = []) {
  const cells = (scene?.cells || []).map((cell) => cellSlotsFromLayers(cell.layers || []));
  return cells.length ? cells : fallback.map(cloneCellSlots);
}

function setSolutionStep(index) {
  if (!levelSolutionPreview) {
    return;
  }
  const nextIndex = Math.max(0, Math.min(levelSolutionPreview.steps.length - 1, index));
  levelSolutionPreview.index = nextIndex;
  levelSolutionPreview.cells = sceneCellsToSlots(
    levelSolutionPreview.steps[nextIndex].scene,
    levelSolutionPreview.cells.length ? levelSolutionPreview.cells : level.cells,
  );
  updateSolutionControls();
  renderLevelBoard();
}

function updateSolutionControls() {
  const active = Boolean(levelSolutionPreview);
  levelSolutionControls.hidden = false;
  levelSolutionControls.classList.toggle("is-empty", !active);
  if (!active) {
    solutionPrevButton.disabled = true;
    solutionNextButton.disabled = true;
    solutionPlayButton.disabled = true;
    solutionSpeedSelect.disabled = true;
    solutionResetButton.disabled = true;
    solutionExportButton.disabled = true;
    solutionSeekInput.disabled = true;
    solutionSeekInput.max = "0";
    solutionSeekInput.value = "0";
    solutionStepText.textContent = "0/0";
    solutionPlayButton.classList.remove("is-playing");
    solutionPlayButton.setAttribute("aria-label", "Play solution");
    solutionPlayButton.title = "Play solution";
    solutionText.textContent = "No solution yet";
    solutionText.title = "";
    return;
  }
  const index = levelSolutionPreview.index;
  const maxIndex = levelSolutionPreview.steps.length - 1;
  solutionPrevButton.disabled = index <= 0;
  solutionNextButton.disabled = index >= maxIndex;
  solutionPlayButton.disabled = maxIndex <= 0;
  solutionSpeedSelect.disabled = maxIndex <= 0;
  solutionResetButton.disabled = index <= 0;
  solutionExportButton.disabled = maxIndex <= 0;
  solutionSeekInput.disabled = maxIndex <= 0;
  solutionSeekInput.max = String(maxIndex);
  solutionSeekInput.value = String(index);
  solutionStepText.textContent = `${index}/${maxIndex}`;
  solutionStepText.title = `Step ${index} of ${maxIndex}`;
  const playLabel = levelSolutionTimer ? "Pause solution" : "Play solution";
  solutionPlayButton.classList.toggle("is-playing", Boolean(levelSolutionTimer));
  solutionPlayButton.setAttribute("aria-label", playLabel);
  solutionPlayButton.title = playLabel;
  const move = levelSolutionPreview.steps[index]?.move?.name;
  const label = move ? `Step ${index}/${maxIndex}: ${move}` : `Step ${index}/${maxIndex}`;
  levelSolveStatus.title = label;
  updateSolutionText();
}

function seekSolutionStep(event) {
  if (!levelSolutionPreview) {
    return;
  }
  const nextIndex = Math.trunc(Number(event.currentTarget.value) || 0);
  stopSolutionPlayback();
  setSolutionStep(nextIndex);
}

function toggleSolutionPlayback() {
  if (!levelSolutionPreview) {
    return;
  }
  if (levelSolutionTimer) {
    stopSolutionPlayback();
    return;
  }
  startSolutionPlayback();
}

function startSolutionPlayback() {
  if (!levelSolutionPreview) {
    return;
  }
  levelSolutionTimer = window.setInterval(() => {
    if (!levelSolutionPreview) {
      stopSolutionPlayback();
      return;
    }
    if (levelSolutionPreview.index >= levelSolutionPreview.steps.length - 1) {
      stopSolutionPlayback();
      return;
    }
    setSolutionStep(levelSolutionPreview.index + 1);
  }, solutionPlaybackIntervalMs());
  updateSolutionControls();
}

function solutionPlaybackIntervalMs() {
  const speed = Math.max(0.25, Number(solutionSpeedSelect.value) || 1);
  return Math.max(40, Math.round(solutionPlaybackBaseIntervalMs / speed));
}

function changeSolutionPlaybackSpeed() {
  if (!levelSolutionTimer) {
    return;
  }
  stopSolutionPlayback();
  startSolutionPlayback();
}

function stopSolutionPlayback() {
  if (levelSolutionTimer) {
    window.clearInterval(levelSolutionTimer);
    levelSolutionTimer = 0;
  }
  updateSolutionControls();
}

function clearSolutionPreview() {
  if (levelSolutionTimer) {
    window.clearInterval(levelSolutionTimer);
    levelSolutionTimer = 0;
  }
  levelSolutionPreview = null;
  levelSolveStatus.title = "";
  updateSolutionControls();
}

function resetSolutionPreview() {
  if (!levelSolutionPreview) {
    return;
  }
  stopSolutionPlayback();
  setSolutionStep(0);
}

function updateSolutionText() {
  const text = solutionTextForUdlr();
  const displayText = text ? abbreviatedSolutionText(text) : solutionSummaryText();
  solutionText.textContent = displayText;
  solutionText.title = text ? `Solution: ${text}` : displayText;
  const label = "Copy solution as UDLR";
  solutionExportButton.setAttribute("aria-label", label);
  solutionExportButton.title = label;
}

function abbreviatedSolutionText(text) {
  const maxLength = 36;
  return text.length <= maxLength ? text : `${text.slice(0, maxLength)}...`;
}

function solutionSummaryText() {
  if (!levelSolutionPreview) {
    return "";
  }
  const moveCount = Math.max(0, (levelSolutionPreview.steps || []).length - 1);
  return moveCount === 1 ? "1 move" : `${moveCount} moves`;
}

function solutionTextForUdlr() {
  if (!levelSolutionPreview) {
    return "";
  }
  const tokens = (levelSolutionPreview.moves || [])
    .map(solutionMoveToken)
    .filter(Boolean);
  return tokens.every((token) => token.length === 1)
    ? tokens.join("")
    : tokens.join(" ");
}

function solutionMoveToken(move) {
  const direction = solutionMoveDirection(move);
  if (direction) {
    return { up: "u", down: "d", left: "l", right: "r" }[direction];
  }
  if (/^[udlr]$/i.test(move?.key || "")) {
    return move.key.toLowerCase();
  }
  return move?.name ? `[${move.name}]` : "?";
}

function solutionMoveDirection(move) {
  const name = String(move?.name || "").toLowerCase();
  if (["up", "down", "left", "right"].includes(name)) {
    return name;
  }
  const arrow = String(move?.arrow || "");
  if (arrow === "ArrowUp") {
    return "up";
  }
  if (arrow === "ArrowDown") {
    return "down";
  }
  if (arrow === "ArrowLeft") {
    return "left";
  }
  if (arrow === "ArrowRight") {
    return "right";
  }
  const key = String(move?.key || "").toLowerCase();
  return { w: "up", s: "down", a: "left", d: "right" }[key] || "";
}

async function exportSolution() {
  const text = solutionTextForUdlr();
  if (!text) {
    setLevelSolveStatus("No solution to copy", "is-error");
    return;
  }
  try {
    window.focus();
    solutionExportButton.focus({ preventScroll: true });
    await copyTextToClipboard(text);
    flashLevelSolveStatus("Copied solution", "is-ok");
  } catch (error) {
    setLevelSolveStatus(`Could not copy solution: ${error?.message || error}`, "is-error");
  }
}

async function copyTextToClipboard(text) {
  if (copyTextWithCopyEvent(text)) {
    return;
  }

  if (navigator.clipboard?.writeText) {
    try {
      await navigator.clipboard.writeText(text);
      return;
    } catch (_error) {
      // Fall through for embedded or unfocused contexts.
    }
  }

  if (copyTextWithSelection(text)) {
    return;
  }

  throw new Error("clipboard copy was rejected");
}

function copyTextWithCopyEvent(text) {
  let handled = false;
  const onCopy = (event) => {
    event.clipboardData?.setData("text/plain", text);
    event.preventDefault();
    handled = true;
  };
  document.addEventListener("copy", onCopy);
  try {
    return document.execCommand("copy") && handled;
  } finally {
    document.removeEventListener("copy", onCopy);
  }
}

function copyTextWithSelection(text) {
  const textarea = document.createElement("textarea");
  textarea.value = text;
  textarea.setAttribute("readonly", "");
  textarea.style.position = "fixed";
  textarea.style.left = "-9999px";
  textarea.style.top = "0";
  document.body.append(textarea);
  textarea.select();
  try {
    return document.execCommand("copy");
  } finally {
    textarea.remove();
  }
}

function handleSolutionKey(event) {
  if (!levelSolutionPreview) {
    return false;
  }
  const key = event.key.length === 1 ? event.key.toLowerCase() : event.key;
  if (key !== "r") {
    return false;
  }
  resetSolutionPreview();
  event.preventDefault();
  event.stopPropagation();
  return true;
}

function formatNumber(value) {
  return new Intl.NumberFormat("en-US").format(Number(value) || 0);
}

function formatSeconds(milliseconds) {
  return `${((Number(milliseconds) || 0) / 1000).toFixed(1)}s`;
}

function sendPreviewKey(event) {
  if (!levelBuilder.hidden && levelPlaytestActive) {
    pendingPreviewKeyStateSync += 1;
  }
  previewFrame.contentWindow?.postMessage({
    type: "PuzzleStudioKey",
    key: event.key,
  }, "*");
}

async function copyLevelToClipboard() {
  const levelName = sanitizeLevelName(levelNameInput.value);
  const source = levelSourceText();
  try {
    await copyTextToClipboard(source);
    setStatus(`Copied level ${levelName}`, "is-ok");
  } catch (error) {
    setStatus(`Could not copy level: ${error?.message || error}`, "is-error");
  }
}

function renderLevelSourcePreview() {
  if (!levelSourcePreview) {
    return;
  }
  levelSourcePreview.textContent = levelSourceText();
}

function levelSourceText() {
  const levelName = sanitizeLevelName(levelNameInput.value);
  return levelDefinitionSource(levelName, levelSourceData(), "", { leadingBlank: false });
}

function addLevelToSource() {
  ensurePreviewTargetsActiveDocument();
  const previewDocument = activePreviewDocument();
  if (!previewDocument) {
    setStatus("No game entry for level", "is-error");
    return;
  }
  const levelName = sanitizeLevelName(levelNameInput.value);
  const levelNamespace = sanitizeLevelNamespace(levelNamespaceInput.value);
  const sourceData = levelSourceData();
  const nextSource = insertLevel(activePreviewSource(), levelName, sourceData, levelNamespace);
  if (!nextSource) {
    setStatus(levelNamespace ? `No levels named ${levelNamespace}` : "No levels block", "is-error");
    return;
  }
  previewDocument.source = nextSource;
  if (previewDocument.id === activeDocument()?.id) {
    setSourceEditorValue(nextSource, { resetUndo: false });
  }
  levelNameInput.value = nextLevelName(levelName);
  scheduleLocalSave();
  if (editorSeed && appendLevelToPreview(levelName, sourceData.rows)) {
    return;
  }
  schedulePreview();
}

function updateLevelInSource() {
  ensurePreviewTargetsActiveDocument();
  const previewDocument = activePreviewDocument();
  if (!previewDocument) {
    setStatus("No game entry for level", "is-error");
    return;
  }
  const levelName = sanitizeLevelName(levelNameInput.value);
  const levelNamespace = sanitizeLevelNamespace(levelNamespaceInput.value);
  const result = replaceLevelByName(activePreviewSource(), levelName, levelSourceData(), levelNamespace);
  if (!result) {
    setStatus(`No level named ${qualifiedLevelName(levelNamespace, levelName)}`, "is-error");
    return;
  }
  previewDocument.source = result.source;
  if (previewDocument.id === activeDocument()?.id) {
    setSourceEditorValue(result.source, { resetUndo: false });
  }
  scheduleLocalSave();
  schedulePreview();
  setStatus(`Updated level ${levelName}`, "is-ok");
}

function appendLevelToPreview(levelName, rows) {
  const exportData = previewExport || extractPreviewExport(latestHtml);
  if (!exportData) {
    markEmbeddedPreviewDirty();
    return false;
  }

  const levelData = exportLevelData(exportData, levelName);
  if (!levelData) {
    markEmbeddedPreviewDirty();
    return false;
  }

  const nextExport = JSON.parse(JSON.stringify(exportData));
  levelData.index = nextExport.levels.length;
  nextExport.levels.push(levelData);
  nextExport.initialLevelIndex = levelData.index;

  const nextHtml = replacePreviewExport(latestHtml, nextExport);
  if (!nextHtml) {
    markEmbeddedPreviewDirty();
    return false;
  }

  previewExport = nextExport;
  latestHtml = nextHtml;
  setActiveLevelIndex(levelData.index, nextExport);
  latestPreviewState = {
    ...(latestPreviewState || {}),
    levelIndex: levelData.index,
    scene: previewSceneForLevel(levelData.index),
  };
  const previewDocument = activePreviewDocument();
  if (previewDocument) {
    previewDocument.previewHtml = nextHtml;
  }
  scheduleLocalSave();
  setPreviewFrameHtml(editorPreviewDocument(nextHtml));
  downloadButton.disabled = false;
  setPreviewMode("play");
  setStatus("Preview updated", "is-ok");
  return true;
}

function exportLevelData(exportData, levelName) {
  const initialState = levelStateData(exportData);
  if (!initialState) {
    return null;
  }

  return {
    index: exportData.levels.length,
    name: levelName,
    regions: levelRegions(),
    initialState,
  };
}

function levelStateData(exportData) {
  const width = level.width;
  const height = level.height;
  const layerCount = exportData?.engine?.layerCount;
  if (!width || !height || !layerCount) {
    return null;
  }

  const slots = Array.from({ length: width * height * layerCount }, () => 0);
  level.cells.forEach((cellSlots, cellIndex) => {
    const sourceSlots = cloneCellSlots(cellSlots, exportData);
    for (let layer = 0; layer < layerCount; layer += 1) {
      slots[(cellIndex * layerCount) + layer] = sourceSlots[layer] || 0;
    }
  });

  const levelIndex = currentEditableLevelIndex(exportData);
  const globalsLength = exportData.levels?.[levelIndex]?.initialState?.globals?.length
    || exportData.levels?.[0]?.initialState?.globals?.length
    || 0;

  return {
    width,
    height,
    layerCount,
    levelIndex,
    slots,
    globals: Array.from({ length: globalsLength }, () => 0),
  };
}

function extractPreviewExport(html) {
  if (!html) {
    return null;
  }
  for (const candidate of [
    { kind: "puzzle2d", pattern: /window\.PuzzleExport\s*=\s*JSON\.parse\(("(?:(?:\\.)|[^"\\])*")\);/ },
    { kind: "puzzle3d", pattern: /window\.Puzzle3DFixture\s*=\s*JSON\.parse\(("(?:(?:\\.)|[^"\\])*")\);/ },
  ]) {
    const match = html.match(candidate.pattern);
    if (!match) {
      continue;
    }
    try {
      const parsed = JSON.parse(JSON.parse(match[1]));
      if (parsed && typeof parsed === "object" && !parsed.__kind) {
        parsed.__kind = candidate.kind;
      }
      return parsed;
    } catch (error) {
      console.error(error);
      return null;
    }
  }
  return null;
}

function replacePreviewExport(html, exportData) {
  const encoded = JSON.stringify(JSON.stringify(exportData));
  const pattern = exportData?.__kind === "puzzle3d"
    ? /window\.Puzzle3DFixture\s*=\s*JSON\.parse\("(?:(?:\\.)|[^"\\])*"\);/
    : /window\.PuzzleExport\s*=\s*JSON\.parse\("(?:(?:\\.)|[^"\\])*"\);/;
  const globalName = exportData?.__kind === "puzzle3d" ? "Puzzle3DFixture" : "PuzzleExport";
  const nextHtml = html.replace(pattern, `window.${globalName} = JSON.parse(${encoded});`);
  return nextHtml === html ? "" : nextHtml;
}

function sanitizeLevelName(value) {
  const cleaned = editableLevelName(value)
    .trim()
    .replace(/[^\w]+/g, "_")
    .replace(/^_+|_+$/g, "");
  return cleaned || "new_level";
}

function sanitizeLevelNamespace(value) {
  return String(value || "")
    .trim()
    .replace(/[^\w.]+/g, "_")
    .replace(/^_+|_+$/g, "");
}

function setLevelNameInputs(qualifiedName) {
  levelNamespaceInput.value = editableLevelNamespace(qualifiedName);
  levelNameInput.value = editableLevelName(qualifiedName);
}

function editableLevelNamespace(value) {
  const raw = String(value || "").trim();
  const parts = raw.split(".").filter(Boolean);
  return parts.length > 1 ? parts.slice(0, -1).join(".") : "";
}

function editableLevelName(value) {
  const raw = String(value || "").trim();
  const parts = raw.split(".").filter(Boolean);
  return parts.length ? parts[parts.length - 1] : raw;
}

function qualifiedLevelName(namespace, name) {
  const levelName = editableLevelName(name);
  const levelsName = sanitizeLevelNamespace(namespace);
  return levelsName ? `${levelsName}.${levelName}` : levelName;
}

function nextLevelName(name) {
  const match = name.match(/^(.*?)(\d+)$/);
  if (!match) {
    return `${name}_2`;
  }
  return `${match[1]}${Number(match[2]) + 1}`;
}

function levelRows() {
  return levelSourceData().rows;
}

function levelSourceData(source = activePreviewSource(), exportData = previewExport || extractPreviewExport(latestHtml)) {
  const charEntries = sourceCharEntries(source, exportData);
  const allocator = createLevelLegendAllocator(charEntries);
  const visualObjects = visualObjectNameSet(exportData);
  const rows = [];
  const regions = levelRegions();
  for (const [regionIndex, region] of regions.entries()) {
    if (regionIndex > 0) {
      rows.push("");
    }
    for (let y = region.y; y < region.y + region.height; y += 1) {
      const row = [];
      for (let x = region.x; x < region.x + region.width; x += 1) {
        row.push(charForSourceCell(level.cells[y * level.width + x], charEntries, allocator, exportData, visualObjects));
      }
      rows.push(row.join(""));
    }
  }
  return { rows, localLegends: allocator.localLegends };
}

function createLevelLegendAllocator(entries) {
  const usedChars = new Set(entries.map((entry) => entry.char));
  const byObjects = new Map();
  const localLegends = [];
  const candidates = "xyzabcdefghijklmnopqrstuvwABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789@$%&?!~^:;,_+-*/";

  return {
    localLegends,
    charForObjects(objects) {
      const key = objectSetKey(objects);
      if (byObjects.has(key)) {
        return byObjects.get(key);
      }
      const ch = [...candidates].find((candidate) => !usedChars.has(candidate));
      if (!ch) {
        return ".";
      }
      usedChars.add(ch);
      byObjects.set(key, ch);
      localLegends.push({ char: ch, objects: [...objects] });
      return ch;
    },
  };
}

function charForSourceCell(slots, entries, allocator, exportData = previewExport, _visualObjects = visualObjectNameSet(exportData)) {
  const objects = objectNamesForSlots(slots, exportData);
  const exact = exactCharForObjects(objects, entries);
  if (exact) {
    return exact;
  }
  const commonObjects = objects.filter((object) => commonLegendObjectNames(entries).has(object));
  if (commonObjects.length && commonObjects.length < objects.length) {
    const commonExact = exactCharForObjects(commonObjects, entries);
    if (commonExact) {
      return commonExact;
    }
    return allocator.charForObjects(commonObjects);
  }
  return objects.length ? allocator.charForObjects(objects) : ".";
}

function objectNamesForSlots(slots, exportData = previewExport) {
  return layersForSlots(slots, exportData).map((layer) => layer.object);
}

function commonLegendObjectNames(entries) {
  return new Set(entries
    .filter((entry) => entry.objects.length === 1)
    .map((entry) => entry.objects[0]));
}

function exactCharForObjects(objects, entries) {
  const key = objectSetKey(objects);
  return entries.find((entry) => objectSetKey(entry.objects) === key)?.char || "";
}

function objectSetKey(objects) {
  return [...objects].sort().join("\u0000");
}

function levelRegions() {
  return normalizedLevelRegions(level.regions, level.width, level.height);
}

function defaultLevelRegions(width, height) {
  return [{
    index: 0,
    x: 0,
    y: 0,
    width: Math.max(0, Number(width) || 0),
    height: Math.max(0, Number(height) || 0),
  }];
}

function normalizedLevelRegions(regions, width, height) {
  const boardWidth = Math.max(0, Number(width) || 0);
  const boardHeight = Math.max(0, Number(height) || 0);
  const normalized = (Array.isArray(regions) ? regions : [])
    .map((region, index) => ({
      index: Number.isInteger(region?.index) ? region.index : index,
      x: Math.max(0, Math.trunc(Number(region?.x) || 0)),
      y: Math.max(0, Math.trunc(Number(region?.y) || 0)),
      width: Math.max(0, Math.trunc(Number(region?.width) || 0)),
      height: Math.max(0, Math.trunc(Number(region?.height) || 0)),
    }))
    .map((region) => ({
      ...region,
      width: Math.min(region.width, Math.max(0, boardWidth - region.x)),
      height: Math.min(region.height, Math.max(0, boardHeight - region.y)),
    }))
    .filter((region) => region.width > 0 && region.height > 0)
    .sort((left, right) => left.index - right.index);
  return normalized.length ? normalized : defaultLevelRegions(boardWidth, boardHeight);
}

function resizeLevelRegions(regions, edge, width, height) {
  const normalized = normalizedLevelRegions(regions, level.width, level.height).map((region) => ({ ...region }));
  if (!normalized.length) {
    return defaultLevelRegions(width, height);
  }
  if (edge === "top" || edge === "bottom") {
    for (const region of normalized) {
      region.height += 1;
    }
  } else if (edge === "left") {
    normalized[0].width += 1;
    for (let index = 1; index < normalized.length; index += 1) {
      normalized[index].x += 1;
    }
  } else if (edge === "right") {
    normalized[normalized.length - 1].width += 1;
  }
  return normalizedLevelRegions(normalized, width, height);
}

function sourceCharEntries(source, exportData = previewExport) {
  const entries = [];
  const domains = schemaDomains(source);
  const knownObjects = new Set(engineObjects(exportData).map((object) => object.name));

  for (const blockName of ["objects", "display_objects"]) {
    for (const line of blockLines(source, blockName)) {
      const schemaMatch = line.match(/^\s*(@?[A-Za-z][\w]*):([A-Za-z][\w]*)\s+(\S+)\s*$/);
      if (schemaMatch && !/^\d+$/.test(schemaMatch[3])) {
        const [, baseName, schemaName, symbols] = schemaMatch;
        const values = domains.get(schemaName) || [...symbols];
        [...symbols].forEach((char, index) => {
          const objectName = `${baseName}:${values[index] || char}`;
          knownObjects.add(objectName);
          entries.push({ char, objects: [objectName] });
        });
        continue;
      }

      const objectMatch = line.match(/^\s*(@?[A-Za-z][\w:]*)\s+(\S+)\s*$/);
      if (objectMatch && objectMatch[2].length === 1 && !/[{}=\d]/.test(objectMatch[2])) {
        const [, objectName, char] = objectMatch;
        knownObjects.add(objectName);
        entries.push({ char, objects: [objectName] });
      }
    }
  }

  for (const row of sourceCommonLegendRows(source)) {
    const entry = legendEntryFromRow(row, knownObjects);
    if (entry) {
      entries.push(entry);
    }
  }

  if (!entries.some((entry) => entry.objects.length === 0)) {
    entries.unshift({ char: ".", objects: [] });
  }

  return entries
    .filter((entry) => entry.char.length === 1)
    .sort((left, right) => right.objects.length - left.objects.length);
}

function sourceCommonLegendRows(source) {
  const lines = sourceLinesWithOffsets(source);
  const rawLines = lines.map((line) => line.raw);
  const levelRanges = sourceLevelLocalRanges(source);
  const rows = [];

  for (let index = 0; index < lines.length; index += 1) {
    if (isOffsetInRanges(lines[index].start, levelRanges)) {
      continue;
    }

    const section = sectionHeaderAtForWasm(rawLines, index);
    if (section?.block === "legend") {
      const result = collectSectionLegendRows(lines, rawLines, index + 3, levelRanges);
      rows.push(...result.rows);
      index = result.endIndex;
      continue;
    }

    const code = levelScannerCode(lines[index].raw);
    if (!code) {
      continue;
    }
    if (/^legend(?:\s*\{)?\s*$/.test(code)) {
      const result = collectLegendBlockRows(lines, index + 1, levelRanges);
      rows.push(...result.rows);
      index = result.endIndex;
      continue;
    }

    const directive = code.match(/^legend\s+(.+)$/);
    if (directive) {
      rows.push(directive[1]);
    }
  }

  return rows;
}

function collectSectionLegendRows(lines, rawLines, startIndex, levelRanges) {
  const rows = [];
  let endIndex = startIndex - 1;
  for (let index = startIndex; index < lines.length; index += 1) {
    if (sectionHeaderAtForWasm(rawLines, index)) {
      break;
    }
    if (isOffsetInRanges(lines[index].start, levelRanges)) {
      continue;
    }
    const code = levelScannerCode(lines[index].raw);
    const tokens = splitLevelTokens(code);
    if (code && sectionBoundaryForWasm("legend", tokens)) {
      break;
    }
    if (isLegendRowForWasm(tokens)) {
      rows.push(code);
    }
    endIndex = index;
  }
  return { rows, endIndex };
}

function collectLegendBlockRows(lines, startIndex, levelRanges) {
  const rows = [];
  let endIndex = startIndex - 1;
  for (let index = startIndex; index < lines.length; index += 1) {
    const code = levelScannerCode(lines[index].raw);
    if (code === "}" || code === "end") {
      endIndex = index;
      break;
    }
    if (!isOffsetInRanges(lines[index].start, levelRanges)) {
      rows.push(code);
    }
    endIndex = index;
  }
  return { rows, endIndex };
}

function sourceLevelLocalRanges(source) {
  const lines = sourceLinesWithOffsets(source);
  const ranges = [];
  for (const levelsRange of findLevelsRanges(source)) {
    let index = lines.findIndex((line) => line.absoluteEnd >= levelsRange.bodyStart);
    if (index < 0) {
      continue;
    }
    while (index < lines.length && lines[index].start <= levelsRange.bodyEnd) {
      if (lines[index].start < levelsRange.bodyStart) {
        index += 1;
        continue;
      }
      const code = levelScannerCode(lines[index].raw);
      const tokens = splitLevelTokens(code);
      if (!code) {
        index += 1;
        continue;
      }
      if (tokens[0] === "legend") {
        const result = collectLegendBlockRows(lines, index + 1, []);
        index = Math.max(index + 1, result.endIndex + 1);
        continue;
      }
      if (isLevelsSectionBoundary(tokens) || code === "}" || code === "end") {
        break;
      }

      let entry = null;
      if (tokens[0] === "level") {
        const nameTokens = tokens.at(-1) === "{" ? tokens.slice(1, -1) : tokens.slice(1);
        entry = tokens.at(-1) === "{"
          ? bracedLevelEntry(source, lines, index, levelNameFromTokens(nameTokens), levelsRange.bodyEnd)
          : unbracedLevelEntry(lines, index, index + 1, levelNameFromTokens(nameTokens), levelsRange.bodyEnd);
      } else if (tokens.length === 1 && tokens[0] === "{") {
        entry = bracedLevelEntry(source, lines, index, "", levelsRange.bodyEnd);
      } else if (tokens.at(-1) === "{" && tokens[0] !== "legend") {
        entry = bracedLevelEntry(source, lines, index, levelNameFromTokens(tokens.slice(0, -1)), levelsRange.bodyEnd);
      }

      if (!entry) {
        index += 1;
        continue;
      }
      ranges.push({ start: entry.start, end: entry.end });
      index = Math.max(index + 1, entry.nextIndex);
    }
  }
  return ranges;
}

function isOffsetInRanges(offset, ranges) {
  return ranges.some((range) => offset >= range.start && offset <= range.end);
}

function legendEntryFromRow(row, knownObjects) {
  const legendMatch = String(row || "").match(/^\s*(\S)\s*=\s*(.+?)\s*$/);
  if (!legendMatch) {
    return null;
  }
  const [, char, expression] = legendMatch;
  const trimmed = expression.trim();
  const parts = trimmed.split(/\s+/);
  const objects = trimmed === "empty"
    ? []
    : parts.filter((part) => knownObjects.has(part));
  return { char, objects };
}

function schemaDomains(source) {
  const domains = new Map();
  for (const line of source.split("\n")) {
    const match = line.match(/^\s*([A-Za-z][\w]*)\s*=\s+([A-Za-z][\w]*(?:\s+[A-Za-z][\w]*)*)\s*$/);
    if (match) {
      domains.set(match[1], match[2].trim().split(/\s+/));
    }
  }
  return domains;
}

function insertLevel(source, name, levelData, namespace = "") {
  const range = findLevelsInsertionRange(source, namespace);
  if (!range) {
    return "";
  }
  const levelIndent = levelInsertionIndent(source, range);
  const levelSource = levelDefinitionSource(name, levelData, levelIndent, { leadingBlank: true });
  return `${source.slice(0, range.bodyEnd).trimEnd()}\n${levelSource}\n${source.slice(range.bodyEnd)}`;
}

function replaceLevelByName(source, name, levelData, namespace = "") {
  const ranges = findLevelsRanges(source);
  const requestedName = qualifiedLevelName(namespace, name);
  const requestedNamespace = sanitizeLevelNamespace(namespace);
  for (const range of ranges) {
    if (requestedNamespace && sanitizeLevelNamespace(range.namespace) !== requestedNamespace) {
      continue;
    }
    const entry = findLevelDefinitions(source, range)
      .find((candidate) => sourceTitleMatches(candidate.name, requestedName, range.namespace));
    if (!entry) {
      continue;
    }
    const indent = levelDefinitionIndent(source, entry);
    const lifecycle = levelLifecycleSourceData(source, entry);
    const replacement = levelDefinitionSource(name, levelData, indent, { leadingBlank: false, lifecycle });
    const replacementEnd = source[entry.end] === "}" ? entry.end + 1 : entry.end;
    return {
      source: `${source.slice(0, entry.start)}${replacement}${source.slice(replacementEnd)}`,
    };
  }
  return null;
}

function levelDefinitionSource(name, levelData, levelIndent, options = {}) {
  const { rows, localLegends } = normalizeLevelSourceData(levelData);
  const lifecycle = options.lifecycle || {};
  const startLifecycleLines = Array.isArray(lifecycle.start) ? lifecycle.start : [];
  const clearLifecycleLines = Array.isArray(lifecycle.clear) ? lifecycle.clear : [];
  const rowIndent = levelIndent;
  const hasRegionBreak = rows.some((row) => row.trim() === "");
  const hasLocalLegends = localLegends.length > 0;
  const hasLifecycle = startLifecycleLines.length > 0 || clearLifecycleLines.length > 0;
  const lines = hasRegionBreak || hasLocalLegends || hasLifecycle
    ? [
      `${levelIndent}level ${name} {`,
      ...startLifecycleLines.map((line) => `${rowIndent}${line}`),
      ...levelLegendSourceLines(localLegends, rowIndent),
      ...rows.map((row) => `${rowIndent}${row}`),
      ...clearLifecycleLines.map((line) => `${rowIndent}${line}`),
      `${levelIndent}}`,
    ]
    : [
      `${levelIndent}level ${name}`,
      ...rows.map((row) => `${rowIndent}${row}`),
    ];
  return `${options.leadingBlank ? "\n" : ""}${lines.join("\n")}`;
}

function levelLifecycleSourceData(source, entry) {
  const lines = sourceLinesWithOffsets(source.slice(entry.start, entry.end)).map((line) => line.raw);
  if (lines.length <= 1) {
    return { start: [], clear: [] };
  }
  const start = [];
  const clear = [];
  let sawMapRow = false;
  let index = 1;
  while (index < lines.length) {
    const code = levelScannerCode(lines[index]);
    if (!code) {
      index += 1;
      continue;
    }
    const normalized = braceNormalizedLineForSectionForWasm(code);
    const tokens = splitLevelTokens(normalized);
    if (isLevelLifecycleHeader(tokens)) {
      const block = collectLevelBodySourceBlock(lines, index);
      (tokens[0] === "on_level_start" ? start : clear).push(...block.lines);
      index = block.nextIndex;
      continue;
    }
    if (isLevelEventSugarCode(code)) {
      (sawMapRow ? clear : start).push(code);
      index += 1;
      continue;
    }
    if (startsLevelBodyBlock(tokens, normalized)) {
      index = skipLevelBodySourceBlock(lines, index);
      continue;
    }
    sawMapRow = true;
    index += 1;
  }
  return { start, clear };
}

function isLevelLifecycleHeader(tokens) {
  return tokens.length === 1 && (tokens[0] === "on_level_start" || tokens[0] === "on_level_clear");
}

function isLevelEventSugarCode(code) {
  const tokens = splitLevelTokens(code);
  return code.startsWith("message ")
    || tokens[0] === "wait"
    || (tokens[0] === "sfx" && tokens.length === 2);
}

function collectLevelBodySourceBlock(lines, startIndex) {
  const blockLines = [levelScannerCode(lines[startIndex])];
  let nestedDepth = 0;
  let index = startIndex + 1;
  while (index < lines.length) {
    const code = levelScannerCode(lines[index]);
    if (code) {
      const normalized = braceNormalizedLineForSectionForWasm(code);
      const tokens = splitLevelTokens(normalized);
      blockLines.push(code);
      if (normalized === "end" || normalized === "}") {
        if (nestedDepth === 0) {
          return { lines: blockLines, nextIndex: index + 1 };
        }
        nestedDepth -= 1;
      } else if (startsInlineBlockForWasm(tokens, normalized)) {
        nestedDepth += 1;
      }
    }
    index += 1;
  }
  return { lines: blockLines, nextIndex: index };
}

function skipLevelBodySourceBlock(lines, startIndex) {
  let nestedDepth = 0;
  let index = startIndex + 1;
  while (index < lines.length) {
    const code = levelScannerCode(lines[index]);
    if (code) {
      const normalized = braceNormalizedLineForSectionForWasm(code);
      const tokens = splitLevelTokens(normalized);
      if (normalized === "end" || normalized === "}") {
        if (nestedDepth === 0) {
          return index + 1;
        }
        nestedDepth -= 1;
      } else if (startsInlineBlockForWasm(tokens, normalized)) {
        nestedDepth += 1;
      }
    }
    index += 1;
  }
  return index;
}

function normalizeLevelSourceData(levelData) {
  if (Array.isArray(levelData)) {
    return { rows: levelData, localLegends: [] };
  }
  return {
    rows: Array.isArray(levelData?.rows) ? levelData.rows : [],
    localLegends: Array.isArray(levelData?.localLegends) ? levelData.localLegends : [],
  };
}

function levelLegendSourceLines(localLegends, indent) {
  if (!localLegends.length) {
    return [];
  }
  return [
    `${indent}legend {`,
    ...localLegends.map((entry) => `${indent}${entry.char} = ${entry.objects.join(" ")}`),
    `${indent}}`,
  ];
}

function levelDefinitionIndent(source, entry) {
  const lines = sourceLinesWithOffsets(source);
  const line = lines.find((candidate) => entry.start >= candidate.start && entry.start <= candidate.end);
  return line ? lineIndent(line.raw) : "\t";
}

function sourceTitleMatches(existing, title, namespace = "") {
  const existingTitle = String(existing || "").trim();
  const requested = editableLevelName(title);
  const requestedNamespace = sanitizeLevelNamespace(editableLevelNamespace(title) || namespace);
  const existingNamespace = sanitizeLevelNamespace(editableLevelNamespace(existingTitle) || namespace);
  return existingTitle === requested
    || existingTitle.endsWith(`.${requested}`)
    || (
      editableLevelName(existingTitle) === requested
      && (!requestedNamespace || !existingNamespace || requestedNamespace === existingNamespace)
    );
}

function findLevelsInsertionRange(source, namespace = "") {
  const ranges = findLevelsRanges(source);
  if (!ranges.length) {
    return null;
  }
  const requestedNamespace = sanitizeLevelNamespace(namespace);
  const matchingRanges = requestedNamespace
    ? ranges.filter((range) => sanitizeLevelNamespace(range.namespace) === requestedNamespace)
    : ranges;
  if (requestedNamespace && !matchingRanges.length) {
    return null;
  }
  const activePosition = activeDocument()?.id === activePreviewDocument()?.id
    ? sourceEditor.selectionStart
    : -1;
  return matchingRanges.find((range) => activePosition >= range.bodyStart && activePosition <= range.bodyEnd)
    || matchingRanges.at(-1)
    || ranges.at(-1);
}

function levelInsertionIndent(source, range) {
  const existing = findLevelDefinitions(source, range)[0];
  if (existing) {
    const lines = sourceLinesWithOffsets(source);
    const line = lines.find((candidate) => existing.start >= candidate.start && existing.start <= candidate.end);
    if (line) {
      return lineIndent(line.raw);
    }
  }
  return range.indent || "\t";
}

function lineIndent(line) {
  return String(line || "").match(/^[\t ]*/)?.[0] || "";
}

function findNamedBlock(source, name) {
  const pattern = new RegExp(`(^|\\n)([\\t ]*)${name}\\s*\\{`, "m");
  const match = pattern.exec(source);
  if (!match) {
    return null;
  }
  const openIndex = source.indexOf("{", match.index + match[0].lastIndexOf(name));
  const closeIndex = findMatchingBrace(source, openIndex);
  if (closeIndex < 0) {
    return null;
  }
  return {
    indent: match[2] || "",
    bodyStart: openIndex + 1,
    bodyEnd: closeIndex,
  };
}

function findMatchingBrace(source, openIndex) {
  let depth = 0;
  for (let index = openIndex; index < source.length; index += 1) {
    if (source[index] === "{") {
      depth += 1;
    } else if (source[index] === "}") {
      depth -= 1;
      if (depth === 0) {
        return index;
      }
    }
  }
  return -1;
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

runButton.addEventListener("click", renderPreview);
clearPreviewLogButton.addEventListener("click", clearPreviewLog);
saveButton.addEventListener("click", () => {
  saveCurrentDocument(true).catch((error) => {
    console.error(error);
    setEditorStatus("Save failed", "is-error");
    saveButton.disabled = false;
  });
});
sourceBackButton?.addEventListener("click", goSourceNavigationBack);
sourceForwardButton?.addEventListener("click", goSourceNavigationForward);
document.addEventListener("keydown", handleSaveShortcut);
document.addEventListener("keydown", handleExplorerToggleShortcut);
document.addEventListener("click", (event) => {
  if (fileActionsMenu?.hidden) {
    return;
  }
  if (event.target.closest("#fileActionsMenu, #fileActionsButton")) {
    return;
  }
  setFileActionsMenuOpen(false);
});
document.addEventListener("keydown", (event) => {
  if (event.key === "Escape") {
    setFileActionsMenuOpen(false);
  }
});
newDocumentButton.addEventListener("click", createNewFile);
fileActionsButton?.addEventListener("click", (event) => {
  event.stopPropagation();
  setFileActionsMenuOpen(fileActionsMenu.hidden);
});
newFolderButton.addEventListener("click", () => {
  setFileActionsMenuOpen(false);
  createNewFolder();
});
importButton.addEventListener("click", () => {
  setFileActionsMenuOpen(false);
  importFileInput.click();
});
importFolderButton.addEventListener("click", () => {
  setFileActionsMenuOpen(false);
  importFolderInput.click();
});
document.addEventListener("click", (event) => {
  const button = event.target.closest("[data-open-project]");
  if (!button) {
    return;
  }
  event.preventDefault();
  setFileActionsMenuOpen(false);
  openProjectFromDesktop().catch((error) => {
    console.error(error);
    setEditorStatus("Open failed", "is-error");
    setOpenProjectButtonsDisabled(false);
  });
});
downloadButton.addEventListener("click", downloadHtml);
themeToggleButton?.addEventListener("click", toggleEditorTheme);
importFileInput.addEventListener("change", () => {
  importFiles(importFileInput.files).catch((error) => {
    console.error(error);
    setEditorStatus("Import failed", "is-error");
  });
  importFileInput.value = "";
});
importFolderInput.addEventListener("change", () => {
  importFiles(importFolderInput.files).catch((error) => {
    console.error(error);
    setEditorStatus("Import failed", "is-error");
  });
  importFolderInput.value = "";
});
documentTabs?.addEventListener("click", (event) => {
  const closeButton = event.target.closest("[data-close-tab]");
  if (closeButton) {
    event.stopPropagation();
    closeDocumentTab(closeButton.dataset.closeTab);
    return;
  }
  const tab = event.target.closest("[data-document-tab]");
  if (!tab || tab.dataset.documentTab === activeFileId) {
    return;
  }
  activateDocumentTab(tab.dataset.documentTab);
});
documentTabs?.addEventListener("scroll", updateDocumentTabScrollState, { passive: true });
documentTabs?.addEventListener("wheel", (event) => {
  const maxScroll = Math.max(0, documentTabs.scrollWidth - documentTabs.clientWidth);
  if (maxScroll <= 1) {
    return;
  }
  const delta = normalizedDocumentTabWheelDelta(event);
  if (!delta) {
    return;
  }
  event.preventDefault();
  documentTabs.scrollLeft = Math.max(0, Math.min(maxScroll, documentTabs.scrollLeft + delta));
  updateDocumentTabScrollState();
}, { passive: false });
documentTabs?.addEventListener("keydown", (event) => {
  if (event.altKey || event.ctrlKey || event.metaKey) {
    return;
  }
  if (event.key === "ArrowLeft") {
    event.preventDefault();
    moveDocumentTabFocus(-1);
  } else if (event.key === "ArrowRight") {
    event.preventDefault();
    moveDocumentTabFocus(1);
  } else if (event.key === "Home") {
    event.preventDefault();
    activateDocumentTab(openTabIds[0]);
  } else if (event.key === "End") {
    event.preventDefault();
    activateDocumentTab(openTabIds[openTabIds.length - 1]);
  }
});
window.addEventListener("resize", updateDocumentTabScrollState);
if (window.ResizeObserver && documentTabs) {
  new ResizeObserver(updateDocumentTabScrollState).observe(documentTabs);
}
documentList.addEventListener("click", (event) => {
  const actionButton = event.target.closest("[data-tree-action]");
  if (actionButton && documentList.contains(actionButton)) {
    event.preventDefault();
    event.stopPropagation();
    const row = actionButton.closest(".tree-row");
    const node = treeNodeFromRow(row);
    if (!node) {
      return;
    }
    selectedTreeId = node.id;
    if (actionButton.dataset.treeAction === "rename") {
      startRenameEntry(node.id);
    } else if (actionButton.dataset.treeAction === "delete") {
      deleteTreeNode(node.id);
    }
    return;
  }

  const row = event.target.closest(".tree-row");
  if (!row) {
    return;
  }
  if (row.dataset.nodeId) {
    const folder = findNode(fileTree, row.dataset.nodeId);
    if (folder?.kind === "folder") {
      if (event.target.closest(".tree-chevron, .tree-icon")) {
        folder.expanded = folder.expanded === false;
      }
      loadFolderPreview(folder);
    }
    return;
  }
  if (row.dataset.fileId) {
    persistCurrentDocument();
    saveDocumentStore(false);
    activeFileId = row.dataset.fileId;
    selectedTreeId = activeFileId;
    selectedFolderId = findParentFolder(fileTree, activeFileId)?.id || "";
    syncDocumentsFromTree();
    loadEmbeddedDocument(activeDocumentIndex());
  }
});
documentList.addEventListener("keydown", (event) => {
  if (!["Enter", " ", "ArrowRight", "ArrowLeft"].includes(event.key)) {
    return;
  }
  const row = event.target.closest(".tree-row");
  if (!row || event.target.closest("input, button")) {
    return;
  }
  event.preventDefault();
  if (row.dataset.nodeId && ["ArrowRight", "ArrowLeft"].includes(event.key)) {
    const folder = findNode(fileTree, row.dataset.nodeId);
    if (folder?.kind === "folder") {
      folder.expanded = event.key === "ArrowRight";
      loadFolderPreview(folder);
    }
    return;
  }
  row.click();
});
documentList.addEventListener("dragstart", (event) => {
  const row = event.target.closest(".tree-row");
  if (!row?.dataset.dragId || row.classList.contains("draft-row")) {
    event.preventDefault();
    return;
  }
  draggedNodeId = row.dataset.dragId;
  row.classList.add("is-dragging");
  event.dataTransfer.effectAllowed = "move";
  event.dataTransfer.setData("text/plain", draggedNodeId);
});
documentList.addEventListener("dragover", (event) => {
  const fileCount = event.dataTransfer?.files?.length || 0;
  const targetFolderId = dropFolderIdForEvent(event);
  if (!fileCount && !canDropNodeOnFolder(draggedNodeId, targetFolderId)) {
    return;
  }
  event.preventDefault();
  event.dataTransfer.dropEffect = fileCount ? "copy" : "move";
  markDropTarget(targetFolderId);
});
documentList.addEventListener("dragleave", (event) => {
  if (!documentList.contains(event.relatedTarget)) {
    clearDropTargets();
  }
});
documentList.addEventListener("drop", (event) => {
  event.preventDefault();
  const files = event.dataTransfer?.files;
  const targetFolderId = dropFolderIdForEvent(event);
  clearDropTargets();
  if (files?.length) {
    const targetFolder = targetFolderId ? findNode(fileTree, targetFolderId) : fileTree;
    if (targetFolder?.kind === "folder") {
      importFilesIntoFolder(files, targetFolder).catch((error) => {
        console.error(error);
        setEditorStatus("Import failed", "is-error");
      });
    }
    return;
  }
  if (moveNodeToFolder(draggedNodeId, targetFolderId)) {
    setEditorStatus("Moved", "is-ok");
  }
});
documentList.addEventListener("dragend", () => {
  draggedNodeId = "";
  clearDropTargets();
  documentList.querySelectorAll(".is-dragging").forEach((row) => row.classList.remove("is-dragging"));
});
initializePhysicalWorkPanes();
paneToggleButtons.forEach((button) => {
  button.addEventListener("click", () => togglePaneVisibility(button.dataset.paneToggle));
});
workbench.addEventListener("click", (event) => {
  const button = event.target.closest("[data-pane-close]");
  if (!button || !workbench.contains(button)) {
    return;
  }
  closeWorkPane(button.dataset.paneClose);
});
workbench.addEventListener("pointerdown", handleWorkPaneFocus);
workbench.addEventListener("focusin", handleWorkPaneFocus);
workbench.addEventListener("dragstart", (event) => {
  const handle = event.target.closest("[data-pane-drag-handle]");
  if (!handle || !workbench.contains(handle)) {
    return;
  }
  startWorkPaneDrag(event);
});
workbench.addEventListener("dragend", (event) => {
  if (event.target.closest("[data-pane-drag-handle]")) {
    stopWorkPaneDrag();
  }
});
workbench.addEventListener("dragover", handleWorkPaneDragOver);
workbench.addEventListener("drop", handleWorkPaneDrop);
workbench.addEventListener("dragleave", (event) => {
  if (draggingWorkPaneId && !workbench.contains(event.relatedTarget)) {
    clearWorkPaneDropState({ keepDragSource: true });
  }
});
window.addEventListener("message", (event) => {
  if (event.data?.type === "PuzzleStudioPreviewLayout") {
    return;
  }
  if (event.data?.type === "PuzzleStudioPreviewState") {
    applyPreviewTheme(event.data.theme || previewExport?.theme || null);
    const inLevelMode = !levelBuilder.hidden || !solverPanel.hidden;
    const screenHasPuzzle = event.data.screenHasPuzzle !== false;
    const levelIndex = inLevelMode
      ? activeLevelIndex
      : setActiveLevelIndex(event.data.levelIndex);
    latestPreviewState = {
      levelIndex,
      rawScene: event.data.rawScene,
      scene: event.data.scene,
      inputs: event.data.inputs || [],
      screen: event.data.screen || "",
      screenHasPuzzle,
    };
    if (inLevelMode) {
      if (!levelBuilder.hidden && levelPlaytestActive && pendingPreviewKeyStateSync > 0) {
        pendingPreviewKeyStateSync = Math.max(0, pendingPreviewKeyStateSync - 1);
      }
      if (screenHasPuzzle && event.data.scene && (levelPlaytestActive || !solverPanel.hidden)) {
        const displayCells = sceneCellsToSlots(event.data.scene, []);
        levelDisplayCells = displayCells.length === level.cells.length ? displayCells : null;
        renderLevelBoard();
      }
      if (levelSolutionPreview) {
        updateSolutionControls();
      }
    }
    return;
  }
  if (event.data?.type === "PuzzleStudioPreviewLog") {
    appendPreviewLog(event.data.level, event.data.message);
    return;
  }
  if (event.data?.type === "PuzzleStudioSolveProgress") {
    handleLevelSolveProgress(event.data);
    return;
  }
  if (event.data?.type === "PuzzleStudioSolveResult") {
    handleLevelSolveResult(event.data);
  }
});
window.addEventListener("resize", syncPreviewViewportScale);
window.addEventListener("resize", syncLevelBoardScale);
window.addEventListener("resize", syncSolverBoardScale);
if (window.ResizeObserver && previewFrameWrap) {
  const previewWrapObserver = new ResizeObserver(() => schedulePreviewViewportSync(2));
  previewWrapObserver.observe(previewFrameWrap);
}
if (window.ResizeObserver && levelBoardViewport) {
  const levelWrapObserver = new ResizeObserver(syncLevelBoardScale);
  const levelWrap = levelBoardViewport.closest(".level-board-wrap");
  if (levelWrap) {
    levelWrapObserver.observe(levelWrap);
  }
  if (levelBuilder) {
    levelWrapObserver.observe(levelBuilder);
  }
}
if (window.ResizeObserver && solverBoardViewport) {
  const solverWrapObserver = new ResizeObserver(syncSolverBoardScale);
  const solverWrap = solverBoardViewport.closest(".solver-board-wrap");
  if (solverWrap) {
    solverWrapObserver.observe(solverWrap);
  }
  if (solverPanel) {
    solverWrapObserver.observe(solverPanel);
  }
}
paneSplitter.addEventListener("pointerdown", startPaneResize);
previewLogSplitter?.addEventListener("pointerdown", startPreviewLogResize);
explorerSplitter.addEventListener("pointerdown", startExplorerResize);
document.addEventListener("pointermove", resizePanes);
document.addEventListener("pointermove", resizeExplorer);
document.addEventListener("pointermove", resizePreviewLog);
document.addEventListener("pointerup", stopActiveResize);
document.addEventListener("pointercancel", stopActiveResize);
paneSplitter.addEventListener("lostpointercapture", stopPaneResize);
previewLogSplitter?.addEventListener("lostpointercapture", stopPreviewLogResize);
explorerSplitter.addEventListener("lostpointercapture", stopExplorerResize);
window.addEventListener("blur", () => stopActiveResize());
playModeButton.addEventListener("click", () => {
  openPreviewModePane("play");
});
editModeButton.addEventListener("click", () => {
  ensurePreviewTargetsActiveDocument();
  openPreviewModePane(currentLevelPaneMode);
  syncSourceFromPreviewPane(currentLevelPaneMode);
});
solverModeButton.addEventListener("click", () => {
  ensurePreviewTargetsActiveDocument();
  openPreviewModePane("solver");
  syncSourceFromPreviewPane("solver");
});
for (const button of levelPaneModeButtons) {
  button.addEventListener("click", () => {
    const mode = button.dataset.levelPaneMode;
    if (!["edit", "level3d"].includes(mode)) {
      return;
    }
    ensurePreviewTargetsActiveDocument();
    openPreviewModePane(mode);
    syncSourceFromPreviewPane(mode);
  });
}
spriteModeButton.addEventListener("click", () => {
  openPreviewModePane("sprite");
  syncSourceFromPreviewPane("sprite");
});
sprite3dModeButton?.addEventListener("click", () => {
  openPreviewModePane("sprite3d");
  syncSourceFromPreviewPane("sprite3d");
});
for (const button of spritePaneModeButtons) {
  button.addEventListener("click", () => {
    const mode = button.dataset.spritePaneMode;
    if (!["sprite", "sprite3d"].includes(mode)) {
      return;
    }
    openPreviewModePane(mode);
    syncSourceFromPreviewPane(mode);
  });
}
soundsTopbarButton.addEventListener("click", () => {
  openPreviewModePane("sounds");
  syncSourceFromPreviewPane("sounds");
});
psImportTopbarButton?.addEventListener("click", () => {
  openPreviewModePane("psimport");
});
docsTopbarButton?.addEventListener("click", () => {
  openPreviewModePane("docs");
  docsSearchInput?.focus();
});
psImportSourceInput?.addEventListener("input", () => schedulePuzzleScriptImportConversion());
psImportConvertButton?.addEventListener("click", () => {
  convertPuzzleScriptImport().catch((error) => {
    console.error(error);
    setPuzzleScriptImportStatus(error.message || String(error), "is-error");
  });
});
psImportCopyButton?.addEventListener("click", () => {
  copyPuzzleScriptImportOutput().catch((error) => {
    console.error(error);
    setPuzzleScriptImportStatus("Copy failed", "is-error");
  });
});
psImportAddFileButton?.addEventListener("click", () => {
  addPuzzleScriptImportFile().catch((error) => {
    console.error(error);
    setPuzzleScriptImportStatus(error.message || String(error), "is-error");
  });
});
levelPaletteCollapseButton.addEventListener("click", () => {
  level.paletteCollapsed = !level.paletteCollapsed;
  renderLevelPalette();
});
levelPlaytestButton?.addEventListener("click", toggleLevelPlaytest);
levelBoard.addEventListener("pointerdown", startLevelPaint);
levelBoard.addEventListener("pointermove", continueLevelPaint);
levelBoard.addEventListener("pointerup", stopLevelPaint);
levelBoard.addEventListener("pointercancel", stopLevelPaint);
levelBoard.addEventListener("keydown", (event) => {
  if (handleSolutionKey(event)) {
    return;
  }
  if (!levelPlaytestActive && (event.key === "Enter" || event.key === " ") && latestPreviewState?.screenHasPuzzle !== false) {
    if (paintLevelCellFromElement(event.target)) {
      event.preventDefault();
      event.stopPropagation();
      return;
    }
  }
  if (!levelPlaytestActive) {
    return;
  }
  sendPreviewKey(event);
  event.preventDefault();
  event.stopPropagation();
});
solverBoard.addEventListener("keydown", (event) => {
  handleSolutionKey(event);
});
document.addEventListener("keydown", (event) => {
  if ((levelBuilder.hidden && solverPanel.hidden) || ["INPUT", "TEXTAREA", "SELECT"].includes(event.target.tagName)) {
    return;
  }
  if (handleSolutionKey(event)) {
    return;
  }
  if (levelBuilder.hidden) {
    return;
  }
  if (!levelPlaytestActive) {
    return;
  }
  sendPreviewKey(event);
  event.preventDefault();
});
levelEdgeButtons.forEach((button) => {
  button.addEventListener("click", () => addLevelEdge(button.dataset.levelEdge));
});
levelNamespaceInput.addEventListener("input", renderLevelSourcePreview);
levelNameInput.addEventListener("input", renderLevelSourcePreview);
copyLevelButton.addEventListener("click", copyLevelToClipboard);
addLevelButton.addEventListener("click", addLevelToSource);
updateLevelButton.addEventListener("click", updateLevelInSource);
solveLevelButton.addEventListener("click", solveLevel);
solutionPrevButton.addEventListener("click", () => setSolutionStep((levelSolutionPreview?.index || 0) - 1));
solutionNextButton.addEventListener("click", () => setSolutionStep((levelSolutionPreview?.index || 0) + 1));
solutionPlayButton.addEventListener("click", toggleSolutionPlayback);
solutionSpeedSelect.addEventListener("change", changeSolutionPlaybackSpeed);
solutionResetButton.addEventListener("click", resetSolutionPreview);
solutionExportButton.addEventListener("click", exportSolution);
solutionSeekInput.addEventListener("input", seekSolutionStep);
solutionSeekInput.addEventListener("change", seekSolutionStep);

bindSourceEditorEvents();
bindSourceEditorPopoverEvents();

applyPaneVisibility();

loadSource().catch((error) => {
  setPreviewDocumentLoaded(false);
  setPreviewFrameHtml(errorDocument(error));
  setEditorStatus("Load error", "is-error");
});
