/* tslint:disable */
/* eslint-disable */

<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
export function startStandalonePlayer(export_json: string, canvas_selector: string): void;
=======
export class WasmStandaloneSession {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    confirm_progress_save_cleared(): void;
    confirm_progress_save_written(request_id: number): void;
    dispatch(action_json: string): string;
    static fromExport(export_json: string): WasmStandaloneSession;
    progress_save_request(): string | undefined;
    resolve_scene_presentation(scene_name: string, state_json: string): string;
    restore_progress_save(save_json: string): void;
    set_current_state(state_json: string, level_index: number, materialize_level_start: boolean): void;
    set_progress_persistence_enabled(enabled: boolean): void;
    snapshot(): string;
}
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544

export function hydrate_render_scene_images(render_scene_json: string, image_assets_json: string): string;

export function project_renderer_state(runtime_export_json: string, state_json: string, level_index: number): string;

export function resolve_render_frame(render_scene_json: string, elapsed_ms: number): string;

export function resolve_render_moment(render_scene_json: string, render_moment_json: string): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
    readonly startStandalonePlayer: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly __wasm_bindgen_func_elem_169658: (a: number, b: number, c: number, d: number) => void;
    readonly __wasm_bindgen_func_elem_12782: (a: number, b: number, c: number, d: number) => void;
    readonly __wasm_bindgen_func_elem_12780: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_157446: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_12780_4: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_12780_5: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_2767: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_12780_7: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_12780_8: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_2767_9: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_12780_10: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_12780_11: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_12780_12: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_12781: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_12779: (a: number, b: number) => void;
    readonly __wasm_bindgen_func_elem_2766: (a: number, b: number) => void;
    readonly __wbindgen_export: (a: number, b: number) => number;
    readonly __wbindgen_export2: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_export3: (a: number) => void;
    readonly __wbindgen_export4: (a: number, b: number, c: number) => void;
    readonly __wbindgen_export5: (a: number, b: number) => void;
    readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
=======
    readonly __wbg_wasmstandalonesession_free: (a: number, b: number) => void;
    readonly hydrate_render_scene_images: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly project_renderer_state: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly resolve_render_frame: (a: number, b: number, c: number) => [number, number, number, number];
    readonly resolve_render_moment: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly wasmstandalonesession_confirm_progress_save_cleared: (a: number) => void;
    readonly wasmstandalonesession_confirm_progress_save_written: (a: number, b: number) => [number, number];
    readonly wasmstandalonesession_dispatch: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmstandalonesession_fromExport: (a: number, b: number) => [number, number, number];
    readonly wasmstandalonesession_progress_save_request: (a: number) => [number, number];
    readonly wasmstandalonesession_resolve_scene_presentation: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly wasmstandalonesession_restore_progress_save: (a: number, b: number, c: number) => [number, number];
    readonly wasmstandalonesession_set_current_state: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly wasmstandalonesession_set_progress_persistence_enabled: (a: number, b: number) => void;
    readonly wasmstandalonesession_snapshot: (a: number) => [number, number];
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_start: () => void;
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
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
