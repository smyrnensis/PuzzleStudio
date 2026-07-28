/* tslint:disable */
/* eslint-disable */

export function dispatchEditorPreviewCommand(request_json: string): void;

export function startEditorPreview(export_json: string, canvas_selector: string): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly dispatchEditorPreviewCommand: (a: number, b: number, c: number) => void;
    readonly startEditorPreview: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly __wasm_bindgen_func_elem_172340: (a: number, b: number, c: number, d: number) => void;
    readonly __wasm_bindgen_func_elem_15417: (a: number, b: number, c: number, d: number) => void;
    readonly __wasm_bindgen_func_elem_15415: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_160121: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_15415_4: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_10811: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_15415_6: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_15415_7: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_15415_8: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_10811_9: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_15415_10: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_15415_11: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_15415_12: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_15416: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_10810: (a: number, b: number) => void;
    readonly __wasm_bindgen_func_elem_15414: (a: number, b: number) => void;
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
