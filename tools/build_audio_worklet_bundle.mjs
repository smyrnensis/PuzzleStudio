import fs from "node:fs";

const [bindingsPath, wasmPath, wrapperPath, outputPath] = process.argv.slice(2);
if (!bindingsPath || !wasmPath || !wrapperPath || !outputPath) {
  throw new Error(
    "usage: node build_audio_worklet_bundle.mjs <bindings.js> <module.wasm> <wrapper.js> <output.js>",
  );
}

const bindings = fs.readFileSync(bindingsPath, "utf8");
const eagerDecoder = `let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();`;
if (bindings.split(eagerDecoder).length !== 2) {
  throw new Error("wasm-bindgen AudioWorklet bindings must contain exactly one eager TextDecoder");
}
const workletBindings = bindings.replace(
  eagerDecoder,
  `let cachedTextDecoder =
    typeof TextDecoder === "undefined"
      ? null
      : new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
if (cachedTextDecoder) cachedTextDecoder.decode();`,
).replace(
  "function decodeText(ptr, len) {\n",
  `function decodeText(ptr, len) {
    if (!cachedTextDecoder) {
        throw new Error("AudioWorkletGlobalScope cannot decode a Rust diagnostic because TextDecoder is unavailable.");
    }
`,
);
const wasmBytes = fs.readFileSync(wasmPath);
const embeddedWasmBytes = `new Uint8Array([${wasmBytes.join(",")}])`;
const placeholder = "__PUZZLE_AUDIO_WORKLET_WASM_BYTES__";
const wrapperSource = fs.readFileSync(wrapperPath, "utf8");
if (wrapperSource.split(placeholder).length !== 2) {
  throw new Error("AudioWorklet wrapper must contain exactly one WASM placeholder.");
}
const wrapper = wrapperSource.replace(placeholder, embeddedWasmBytes);
if (wrapper.includes(placeholder)) {
  throw new Error("AudioWorklet WASM placeholder was not replaced exactly once.");
}
fs.writeFileSync(outputPath, `${workletBindings.trimEnd()}\n${wrapper.trimEnd()}\n`);
