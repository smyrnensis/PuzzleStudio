import { readFile } from "node:fs/promises";

import init, {
  WasmSolverService,
  activate_source_analysis,
  activate_source_analysis_with_profile,
  active_source_analysis_highlight_range_json,
  active_source_analysis_level_editor_level_slots,
  active_source_analysis_level_editor_manifest_json,
  active_source_analysis_level_editor_sprite_json,
  compile_preview,
  compile_workspace_preview,
  workspace_presentation_manifest,
} from "../crates/html_editor/static/wasm/puzzle_wasm.js";

const wasm = await readFile("crates/html_editor/static/wasm/puzzle_wasm_bg.wasm");
await init({ module_or_path: wasm });

const profiledHighlightSource = `levels {
legend {
_ = Floor
}
level "stacked" {
___
-
___
}
}
`;
const profiledRevision = activate_source_analysis_with_profile(
  profiledHighlightSource,
  "puzzle3d",
);
const profiledHighlight = JSON.parse(active_source_analysis_highlight_range_json(
  profiledRevision,
  0,
  profiledHighlightSource.length,
  false,
));
const invalidProfiledText = profiledHighlight.spans
  .filter((span) => span.kind === "level-cell-invalid")
  .map((span) => profiledHighlightSource.slice(span.start, span.end));
if (invalidProfiledText.includes("_") || invalidProfiledText.includes("-")) {
  throw new Error(`profiled level highlighting rejected declared cells or slice separators: ${invalidProfiledText}`);
}

const source = `
title = "Editor Preview Contract"

puzzle board {
  slots {
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

const html = compile_preview(source, "game.puzzle", "", "");
const runtimeExportLiteral = html.match(
  /window\.PuzzleRuntimeExportJson\s*=\s*("(?:\\.|[^"\\])*")/,
)?.[1];
if (!runtimeExportLiteral) {
  throw new Error("generated editor preview is missing its standalone runtime export");
}
const runtimeExport = JSON.parse(JSON.parse(runtimeExportLiteral));
if (
  !runtimeExport.runtimeLoadedDocument
  || Object.keys(runtimeExport).some((key) => key !== "runtimeLoadedDocument")
) {
  throw new Error(`editor preview leaked editor metadata into its standalone runtime export: ${Object.keys(runtimeExport)}`);
}
const editorPreviewExportLiteral = html.match(
  /window\.PuzzleEditorPreviewExportJson\s*=\s*("(?:\\.|[^"\\])*")/,
)?.[1];
if (!editorPreviewExportLiteral) {
  throw new Error("generated editor preview is missing its editor metadata export");
}
const editorPreviewExport = JSON.parse(JSON.parse(editorPreviewExportLiteral));
if (!editorPreviewExport.engine || editorPreviewExport.runtimeLoadedDocument) {
  throw new Error("editor preview metadata does not have an editor-only contract");
}
const workspaceDocuments = [{ path: "game.puzzle", source }];
const workspaceHtml = compile_workspace_preview("game.puzzle", workspaceDocuments, "", "");
if (!workspaceHtml.includes('editorPreview\\":true')) {
  throw new Error("typed workspace preview did not compile editor HTML");
}
const workspaceManifest = workspace_presentation_manifest("game.puzzle", workspaceDocuments);
if (
  workspaceManifest.themeName !== "clean"
  || !Array.isArray(workspaceManifest.cssPaths)
  || !Array.isArray(workspaceManifest.scriptPaths)
  || !Array.isArray(workspaceManifest.filePaths)
  || !Array.isArray(workspaceManifest.spriteImagePaths)
) {
  throw new Error(`typed workspace manifest is invalid: ${JSON.stringify(workspaceManifest)}`);
}
let invalidWorkspaceDocumentsError = "";
try {
  compile_workspace_preview(
    "game.puzzle",
    [{ path: "game.puzzle" }],
    "",
    "",
  );
} catch (error) {
  invalidWorkspaceDocumentsError = String(error);
}
if (!invalidWorkspaceDocumentsError.includes("missing field")) {
  throw new Error(`typed workspace input accepted a missing source: ${invalidWorkspaceDocumentsError}`);
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
    slotMarks: [[], []],
    cellMarks: [[], []],
    variables: [],
    levelFiredRules: [],
  },
  materializeLevelStart: true,
  maxDepth: 4,
  maxNodes: 16,
}, Date.now());
const solverAdvance = solverService.advance(solverSearch, 4, Date.now());
if (solverAdvance.status !== "solved" || solverAdvance.result?.result !== "solved") {
  throw new Error(`typed solver search did not finish in Rust: ${JSON.stringify(solverAdvance)}`);
}
const solvedState = solverAdvance.result.steps[0]?.state;
if (
  !Array.isArray(solvedState?.slotMarks)
  || !Array.isArray(solvedState?.cellMarks)
  || Object.hasOwn(solvedState, "mark")
) {
  throw new Error(`typed solver state lost mark ownership: ${JSON.stringify(solvedState)}`);
}
const required = [
  'editorPreview\\":true',
  "window.PuzzleEditorPreviewExportJson",
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
title = "3D Editor Preview Contract"

puzzle preview {
  dimension = 3
  slots {
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
const puzzle3Html = compile_preview(
  puzzle3Source,
  "spec_3d_preview_contract.puzzle3",
  "",
  "",
);
for (const token of [
  "window.Puzzle3DFrameFixture = JSON.parse(",
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
slots {
Player Box Wall
}
rules {
move
}
}
sprites {
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
if (manifest.includes("slots") || manifest.includes("\"sprites\"")) {
  throw new Error("level editor manifest must not transfer level cells or full sprite definitions");
}
if (!manifest.includes('"id":1,"layer":0,"name":"Player"')) {
  throw new Error(`level editor manifest lost canonical object identity: ${manifest}`);
}
const slots = active_source_analysis_level_editor_level_slots(revision, 0, -1);
if (!(slots instanceof Uint32Array) || slots.length !== 1 || slots[0] !== 1) {
  throw new Error(`level editor slots must be a typed canonical-ID buffer: ${slots}`);
}
const sprite = active_source_analysis_level_editor_sprite_json(revision, 1);
if (!sprite.includes('"colors":{"0":"#fff"}')) {
  throw new Error(`level editor sprite payload is not renderer-ready: ${sprite}`);
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
