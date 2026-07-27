/* tslint:disable */
/* eslint-disable */

export function startStandalonePlayer(export_json: string, canvas_selector: string): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly startStandalonePlayer: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly __wasm_bindgen_func_elem_169697: (a: number, b: number, c: number, d: number) => void;
    readonly __wasm_bindgen_func_elem_12814: (a: number, b: number, c: number, d: number) => void;
    readonly __wasm_bindgen_func_elem_12812: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_157474: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_12812_4: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_12812_5: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_2805: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_12812_7: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_12812_8: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_2805_9: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_12812_10: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_12812_11: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_12812_12: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_12813: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_12811: (a: number, b: number) => void;
    readonly __wasm_bindgen_func_elem_2804: (a: number, b: number) => void;
    readonly __wbindgen_export: (a: number, b: number) => number;
    readonly __wbindgen_export2: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_export3: (a: number) => void;
    readonly __wbindgen_export4: (a: number, b: number, c: number) => void;
    readonly __wbindgen_export5: (a: number, b: number) => void;
    readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
