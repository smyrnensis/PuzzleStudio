#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";

const args = process.argv.slice(2);
const enforce = args.includes("--enforce");
const repoRoot = findRepoRoot(process.cwd());
const failures = [];
const warnings = [];

const tauriConfig = readJson(path.join(repoRoot, "src-tauri/tauri.conf.json"));
const frontendDist = tauriConfig?.build?.frontendDist;
if (frontendDist !== "../crates/html_editor/static") {
  failures.push(
    `src-tauri/tauri.conf.json build.frontendDist must remain "../crates/html_editor/static"; got ${JSON.stringify(frontendDist)}`,
  );
}

const tauriCargo = fs.readFileSync(path.join(repoRoot, "src-tauri/Cargo.toml"), "utf8");
const directDeps = dependencyDetails(tauriCargo, "dependencies");
const htmlEditor = directDeps.get("html-editor");
if (htmlEditor) {
  const defaultFeaturesDisabled = /default-features\s*=\s*false/.test(htmlEditor.raw);
  const featureList = dependencyFeatureList(htmlEditor.raw);
  const forbiddenFeatures = featureList.filter((feature) =>
    ["default", "embedded-assets", "native-preview"].includes(feature),
  );
  if (!defaultFeaturesDisabled || forbiddenFeatures.length > 0) {
    const message = [
      "src-tauri may depend on html-editor only as a service/docs/sound-tools dependency:",
      "set default-features = false and do not enable embedded-assets or native-preview",
    ].join(" ");
    (enforce ? failures : warnings).push(message);
  }
}

if (directDeps.has("html-play")) {
  const message =
    "src-tauri directly depends on html-play; desktop should consume editor service behavior, not game runtime/export asset embedding";
  (enforce ? failures : warnings).push(message);
}

for (const hit of includeMacroHits(path.join(repoRoot, "src-tauri"))) {
  failures.push(`desktop-owned code must not embed web assets with ${hit.macro}: ${hit.relative}`);
}

if (!fs.existsSync(path.join(repoRoot, "crates/html_editor/static/editor.html"))) {
  failures.push("expected Tauri frontendDist editor asset is missing: crates/html_editor/static/editor.html");
}

for (const asset of [
  "crates/html_editor/static/renderer.css",
  "crates/html_editor/static/renderer.js",
  "crates/html_editor/static/puzzle3_visual_core.js",
  "crates/html_editor/static/wasm_game/puzzle_wasm_game.js",
  "crates/html_editor/static/wasm_game/puzzle_wasm_game_bg.wasm",
]) {
  if (!fs.existsSync(path.join(repoRoot, asset))) {
    failures.push(`expected Tauri frontendDist game runtime asset is missing: ${asset}`);
  }
}

const staticAssetCheck = spawnSync("tools/sync_static_assets.sh", ["--check"], {
  cwd: repoRoot,
  encoding: "utf8",
});
if (staticAssetCheck.status !== 0) {
  failures.push(
    staticAssetCheck.stderr.trim() ||
      "generated editor distribution assets failed their canonical-source check",
  );
}

if (!hasIncludeMacros(path.join(repoRoot, "crates/html_editor/src/lib.rs"))) {
  failures.push(
    "html-editor no longer appears to own embedded editor assets; move export/server ownership deliberately instead of erasing it while changing desktop",
  );
}

if (!hasIncludeMacros(path.join(repoRoot, "crates/html_play/src/lib.rs"))) {
  failures.push(
    "html-play no longer appears to own embedded runtime assets; standalone export ownership must be moved deliberately if this changed",
  );
}

for (const warning of warnings) {
  console.warn(`warning: ${warning}`);
}

if (failures.length > 0) {
  for (const failure of failures) {
    console.error(`error: ${failure}`);
  }
  process.exit(1);
}

console.log(
  enforce
    ? "desktop asset boundary enforcement passed"
    : "desktop asset boundary report passed; use --enforce before shipping boundary changes",
);

function findRepoRoot(start) {
  let current = start;
  while (true) {
    if (fs.existsSync(path.join(current, "Cargo.toml")) && fs.existsSync(path.join(current, "src-tauri"))) {
      return current;
    }
    const parent = path.dirname(current);
    if (parent === current) {
      failUsage("could not find repository root");
    }
    current = parent;
  }
}

function failUsage(message) {
  console.error(`usage error: ${message}`);
  process.exit(2);
}

function readJson(file) {
  try {
    return JSON.parse(fs.readFileSync(file, "utf8"));
  } catch (error) {
    failures.push(`failed to read JSON ${path.relative(repoRoot, file)}: ${error.message}`);
    return null;
  }
}

function dependencyDetails(toml, sectionName) {
  const details = new Map();
  let active = false;
  for (const rawLine of toml.split(/\r?\n/)) {
    const line = rawLine.trim();
    const section = line.match(/^\[([^\]]+)\]$/);
    if (section) {
      const name = section[1];
      active = name === sectionName;
      const tableDependency = name.match(new RegExp(`^${escapeRegExp(sectionName)}\\.([^\\.]+)$`));
      if (tableDependency) {
        details.set(tableDependency[1].replace(/^"|"$/g, ""), { raw: "" });
      }
      continue;
    }
    if (!active || line.startsWith("#") || line === "") {
      continue;
    }
    const dependency = line.match(/^([A-Za-z0-9_-]+)\s*=/);
    if (dependency) {
      details.set(dependency[1], { raw: line });
    }
  }
  return details;
}

function dependencyFeatureList(raw) {
  const features = raw.match(/features\s*=\s*\[([^\]]*)\]/);
  if (!features) {
    return [];
  }
  return features[1]
    .split(",")
    .map((value) => value.trim().replace(/^"|"$/g, ""))
    .filter(Boolean);
}

function includeMacroHits(root) {
  const hits = [];
  for (const file of walk(root)) {
    if (!file.endsWith(".rs")) {
      continue;
    }
    const source = fs.readFileSync(file, "utf8");
    for (const macro of ["include_str!", "include_bytes!"]) {
      if (source.includes(macro)) {
        hits.push({
          file,
          relative: path.relative(repoRoot, file),
          macro,
        });
      }
    }
  }
  return hits;
}

function hasIncludeMacros(file) {
  if (!fs.existsSync(file)) {
    return false;
  }
  const source = fs.readFileSync(file, "utf8");
  return source.includes("include_str!") || source.includes("include_bytes!");
}

function* walk(root) {
  const entries = fs.readdirSync(root, { withFileTypes: true });
  for (const entry of entries) {
    const file = path.join(root, entry.name);
    if (entry.isDirectory()) {
      if (entry.name === "target" || entry.name === ".git") {
        continue;
      }
      yield* walk(file);
    } else if (entry.isFile()) {
      yield file;
    }
  }
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
