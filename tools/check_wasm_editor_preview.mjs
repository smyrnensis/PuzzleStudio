import { readFile } from "node:fs/promises";

import init, {
  activate_source_analysis,
  active_source_analysis_level_editor_level_slots,
  active_source_analysis_level_editor_manifest_json,
  active_source_analysis_level_editor_sprite_json,
  compile_preview,
} from "../crates/html_editor/static/wasm/puzzle_wasm.js";

const wasm = await readFile("crates/html_editor/static/wasm/puzzle_wasm_bg.wasm");
await init({ module_or_path: wasm });

const source = `
title = "Editor Preview Contract"

puzzle board {
  layers {
    tiles = Player
  }
  empty .
  rules {
    [ Player ] -> [ Player ]
  }
}

levels default of board {
  legend P = Player
  level "one" {
    P
  }
}
`;

const html = compile_preview(source, "game.puzzle", "", "");
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

const editorSource = `
puzzle default {
layers {
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
