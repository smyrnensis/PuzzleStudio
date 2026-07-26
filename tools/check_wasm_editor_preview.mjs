import { readFile } from "node:fs/promises";

import init, {
  WasmSolverService,
  WasmWorkspaceSession,
  activate_source_analysis,
  active_source_analysis_highlight_range_json,
  active_source_analysis_level_editor_level_slots,
  active_source_analysis_level_editor_manifest_json,
  active_source_analysis_level_editor_visual_json,
  compile_preview,
} from "../crates/html_editor/static/wasm/puzzle_wasm.js";

const wasm = await readFile("crates/html_editor/static/wasm/puzzle_wasm_bg.wasm");
await init({ module_or_path: wasm });

const ownerDimensionHighlightSource = `puzzle board {
dimension = 3
layers {
ground = Floor
}
rules {
}
levels {
legend {
_ = Floor
}
level "stacked" {
___
-
___
}
}
}
`;
const ownerDimensionRevision = activate_source_analysis(ownerDimensionHighlightSource);
const ownerDimensionHighlight = JSON.parse(active_source_analysis_highlight_range_json(
  ownerDimensionRevision,
  0,
  ownerDimensionHighlightSource.length,
  false,
));
const invalidOwnerDimensionText = ownerDimensionHighlight.spans
  .filter((span) => span.kind === "level-cell-invalid")
  .map((span) => ownerDimensionHighlightSource.slice(span.start, span.end));
if (invalidOwnerDimensionText.includes("_") || invalidOwnerDimensionText.includes("-")) {
  throw new Error(`owner-declared 3D highlighting rejected declared cells or slice separators: ${invalidOwnerDimensionText}`);
}

const source = `
const title = "Editor Preview Contract"

puzzle board {
  layers {
    actor = Player
  }
  keys {
    d ArrowRight -> right
  }
  rules {
    input right [ Player | no actor ] -> [ | Player ]
  }
  win_conditions {
    some Player
  }
}

levels main of board {
  legend {
    . = empty
    P = Player
  }
  level "one" {
    P.
  }
}
`;

const build = JSON.parse(compile_preview(source, "game.puzzle", "", ""));
if (
  typeof build.html !== "string"
  || build.documentMetadata?.title !== "Editor Preview Contract"
  || build.models?.board?.kind !== "puzzle2d"
  || !build.models.board.engine
) {
  throw new Error(`top-level preview compiler returned an invalid typed build: ${JSON.stringify(build)}`);
}
const html = build.html;
const runtimeExportLiteral = html.match(
  /window\.PuzzleRuntimeExportJson\s*=\s*("(?:\\.|[^"\\])*")/,
)?.[1];
if (!runtimeExportLiteral) {
  throw new Error("generated editor preview is missing its standalone runtime export");
}
const runtimeExport = JSON.parse(JSON.parse(runtimeExportLiteral));
const runtimeExportKeys = Object.keys(runtimeExport).sort();
const expectedRuntimeExportKeys = [
  "progressStorage",
  "runtimeLoadedDocument",
  "version",
  "visualImages",
];
if (
  JSON.stringify(runtimeExportKeys) !== JSON.stringify(expectedRuntimeExportKeys)
  || runtimeExport.version !== 4
  || !runtimeExport.runtimeLoadedDocument
  || !runtimeExport.visualImages
  || !runtimeExport.progressStorage
) {
  throw new Error(
    `editor preview does not expose the complete standalone runtime contract: ${JSON.stringify(runtimeExport)}`,
  );
}
if (html.includes("PuzzleEditorPreviewExportJson")) {
  throw new Error("generated editor preview republished editor metadata through HTML");
}
const workspaceDocuments = [{ path: "game.puzzle", source }];
const workspace = new WasmWorkspaceSession(workspaceDocuments);
const workspaceBuild = JSON.parse(workspace.compile_preview("game.puzzle", "", ""));
if (
  !workspaceBuild.html.includes('editorPreview\\":true')
  || workspaceBuild.documentMetadata?.title !== build.documentMetadata.title
  || workspaceBuild.models?.board?.kind !== "puzzle2d"
) {
  throw new Error(`typed workspace preview returned an invalid build: ${JSON.stringify(workspaceBuild)}`);
}
const workspaceManifest = workspace.presentation_manifest("game.puzzle");
if (
  workspaceManifest.themeName !== "clean"
  || !Array.isArray(workspaceManifest.cssPaths)
  || !Array.isArray(workspaceManifest.scriptPaths)
  || !Array.isArray(workspaceManifest.filePaths)
  || !Array.isArray(workspaceManifest.visualImageAssets)
) {
  throw new Error(`typed workspace manifest is invalid: ${JSON.stringify(workspaceManifest)}`);
}
workspace.replace_documents(workspaceDocuments);
if (workspace.revision() !== 2 || JSON.parse(workspace.index_json()).revision !== 2) {
  throw new Error("workspace replacement did not advance its authoritative revision");
}
const resourceWorkspace = new WasmWorkspaceSession([
  { path: "games/game.puzzle", source: 'import board = "../models/board.puzzle"\n' },
  { path: "models/board.puzzle", source: `puzzle main {
layers {
actor = Player
}
visuals {
Player {
image = "images/player.png"
}
}
rules {
}
levels {
legend {
P = Player
}
level "start" {
P
}
}
}
` },
]);
const resourceManifest = resourceWorkspace.presentation_manifest("games/game.puzzle");
if (
  resourceManifest.visualImageAssets[0]?.path !== "models/images/player.png"
) {
  throw new Error(`imported resource path was not canonicalized: ${JSON.stringify(resourceManifest)}`);
}
let invalidWorkspaceDocumentsError = "";
try {
  new WasmWorkspaceSession([{ path: "game.puzzle" }]);
} catch (error) {
  invalidWorkspaceDocumentsError = String(error);
}
if (!invalidWorkspaceDocumentsError.includes("missing field")) {
  throw new Error(`typed workspace input accepted a missing source: ${invalidWorkspaceDocumentsError}`);
}
let missingImportDiagnostic = null;
try {
  new WasmWorkspaceSession([
    { path: "game.puzzle", source: "// heading\nimport missing = \"missing.puzzle\"\n" },
  ]).compile_preview("game.puzzle", "", "");
} catch (error) {
  missingImportDiagnostic = error?.diagnostics?.[0] || null;
}
if (
  missingImportDiagnostic?.file !== "game.puzzle"
  || missingImportDiagnostic?.line !== 2
) {
  throw new Error(`workspace import diagnostic lost its source origin: ${JSON.stringify(missingImportDiagnostic)}`);
}
const importedInvalidSource = source
  .replace('const title = "Editor Preview Contract"\n', "")
  .replace(
    "input right [ Player | no actor ] -> [ | Player ]",
    "unknown_imported_statement",
  );
const importedInvalidLine = importedInvalidSource
  .split(/\r?\n/)
  .findIndex((line) => line === "    unknown_imported_statement") + 1;
let importedCompileDiagnostic = null;
try {
  new WasmWorkspaceSession(
    [
      { path: "game.puzzle", source: "import part = \"parts/game.puzzle\"\n" },
      { path: "parts/game.puzzle", source: importedInvalidSource },
    ],
  ).compile_preview("game.puzzle", "", "");
} catch (error) {
  importedCompileDiagnostic = error?.diagnostics?.[0] || null;
}
if (
  importedCompileDiagnostic?.file !== "parts/game.puzzle"
  || importedCompileDiagnostic?.line !== importedInvalidLine
) {
  throw new Error(`workspace compile diagnostic lost its imported origin: ${JSON.stringify(importedCompileDiagnostic)}`);
}
const solverService = new WasmSolverService();
const preparedSolver = solverService.prepare_source(source, "game.puzzle", Date.now());
if (preparedSolver.modelKind !== "2d" || preparedSolver.levelCount !== 1 || !preparedSolver.artifactId) {
  throw new Error(`typed solver preparation returned an invalid handle: ${JSON.stringify(preparedSolver)}`);
}
solverService.pin_artifact(preparedSolver.artifactId, Date.now());
const solverSearch = solverService.start(preparedSolver.artifactId, {
  levelIndex: 0,
  state: {
    kind: "2d",
    width: 2,
    height: 1,
    layerCount: 1,
    slots: [1, 0],
    variables: [],
    levelFiredRules: [],
  },
  materializeLevelStart: true,
  maxDepth: 4,
  maxStoredNodes: 16,
}, Date.now());
const solverAdvance = solverService.advance(solverSearch, 4, Date.now());
if (solverAdvance.status !== "solved" || solverAdvance.result?.result !== "solved") {
  throw new Error(`typed solver search did not finish in Rust: ${JSON.stringify(solverAdvance)}`);
}
const solvedState = solverAdvance.result.steps[0]?.state;
if (
  !Array.isArray(solvedState?.slots)
  || !Array.isArray(solvedState?.variables)
  || !Array.isArray(solvedState?.levelFiredRules)
  || Object.hasOwn(solvedState, "slotMarks")
  || Object.hasOwn(solvedState, "cellMarks")
  || Object.hasOwn(solvedState, "mark")
) {
  throw new Error(`typed solver state violated the committed-state contract: ${JSON.stringify(solvedState)}`);
}
const required = [
  'editorPreview\\":true',
  "window.PuzzleRuntimeWasmLoader",
  "PuzzleStudioRuntimeAssetRequest",
  "PuzzleStudioRuntimeAssetResponse",
  "Editor preview requires its WASM session runtime; /api requests are unavailable in the preview iframe.",
];

for (const token of required) {
  if (!html.includes(token)) {
    throw new Error(`generated editor preview is missing required runtime contract: ${token}`);
  }
}

const puzzle3Source = `
const title = "3D Editor Preview Contract"

puzzle preview {
  dimension = 3
  layers {
    Player
  }
  rules {
  }
  layout {
    puzzle
  }
}

levels default of preview {
  legend {
    P = Player
  }
  level "one" {
    P
  }
}

`;
const puzzle3Build = JSON.parse(compile_preview(
  puzzle3Source,
  "spec_3d_preview_contract.puzzle",
  "",
  "",
));
const puzzle3Html = puzzle3Build.html;
if (
  puzzle3Build.models?.preview?.kind !== "puzzle3d"
  || !puzzle3Build.models.preview.fixture?.objects
) {
  throw new Error(`spatial preview typed build is invalid: ${JSON.stringify(puzzle3Build)}`);
}
for (const token of [
  "window.Puzzle3DFrameFixtures = JSON.parse(",
  "WasmStandaloneSession",
  "window.Puzzle3Component",
  "window.PuzzleRuntimeWasmLoader",
]) {
  if (!puzzle3Html.includes(token)) {
    throw new Error(
      `generated spatial editor preview is missing required runtime contract: ${token}\n${puzzle3Html.slice(0, 800)}`,
    );
  }
}

const editorSource = `
puzzle default {
layers {
Player Box Wall
}
rules {
move
}
}
visuals {
Player {
#fff
0
}
}
levels {
legend {
P = Player
B = Box
. = empty
}
level "start"
P
}
`;
const revision = activate_source_analysis(editorSource);
const manifest = active_source_analysis_level_editor_manifest_json(revision);
if (manifest.includes("slots") || manifest.includes("\"visuals\"")) {
  throw new Error("level editor manifest must not transfer level cells or full visual definitions");
}
if (!manifest.includes('"id":1,"layer":0,"name":"Player"')) {
  throw new Error(`level editor manifest lost canonical object identity: ${manifest}`);
}
const slots = active_source_analysis_level_editor_level_slots(revision, 0, -1);
if (!(slots instanceof Uint32Array) || slots.length !== 1 || slots[0] !== 1) {
  throw new Error(`level editor slots must be a typed canonical-ID buffer: ${slots}`);
}
const visual = active_source_analysis_level_editor_visual_json(revision, 1);
if (!visual.includes('"colors":{"0":"#fff"}')) {
  throw new Error(`level editor visual payload is not renderer-ready: ${visual}`);
}
let fullCompileError = null;
try {
  compile_preview(editorSource, "game.puzzle", "", "");
} catch (error) {
  fullCompileError = typeof error === "object" ? JSON.stringify(error) : String(error);
}
if (!fullCompileError?.includes("unknown routine call: move")) {
  throw new Error(`level editor integration must exclude rules while full compile reports them: ${fullCompileError}`);
}
