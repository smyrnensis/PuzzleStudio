/* tslint:disable */
/* eslint-disable */

export function startStandalonePlayer(export_json: string, canvas_selector: string): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly startStandalonePlayer: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly __wasm_bindgen_func_elem_169709: (a: number, b: number, c: number, d: number) => void;
    readonly __wasm_bindgen_func_elem_12827: (a: number, b: number, c: number, d: number) => void;
    readonly __wasm_bindgen_func_elem_12825: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_157486: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_12825_4: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_12825_5: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_2803: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_12825_7: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_12825_8: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_2803_9: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_12825_10: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_12825_11: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_12825_12: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_12826: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_12824: (a: number, b: number) => void;
    readonly __wasm_bindgen_func_elem_2802: (a: number, b: number) => void;
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
