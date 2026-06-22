/* tslint:disable */
/* eslint-disable */
export const memory: WebAssembly.Memory;
export const __wbg_wasmpuzzle3runtime_free: (a: number, b: number) => void;
export const __wbg_wasmstandalonesession_free: (a: number, b: number) => void;
export const wasmpuzzle3runtime_current_cells: (a: number) => [number, number, number, number];
export const wasmpuzzle3runtime_is_current_complete: (a: number) => [number, number, number];
export const wasmpuzzle3runtime_new: (a: number, b: number, c: number, d: number) => [number, number, number];
export const wasmpuzzle3runtime_restore_saved_state: (a: number, b: number) => [number, number];
export const wasmpuzzle3runtime_save_current_state: (a: number) => [number, number, number];
export const wasmpuzzle3runtime_set_state: (a: number, b: number, c: number) => [number, number];
export const wasmpuzzle3runtime_transition_current_outcome: (a: number, b: number, c: number, d: number) => [number, number, number, number];
export const wasmstandalonesession_apply_command_name: (a: number, b: number, c: number) => [number, number];
export const wasmstandalonesession_apply_input_name: (a: number, b: number, c: number) => [number, number];
export const wasmstandalonesession_clear_progress_save: (a: number) => void;
export const wasmstandalonesession_fromExport: (a: number, b: number) => [number, number, number];
export const wasmstandalonesession_mark_progress_save_written: (a: number) => void;
export const wasmstandalonesession_new: (a: number, b: number, c: number, d: number) => [number, number, number];
export const wasmstandalonesession_progress_save: (a: number) => [number, number];
export const wasmstandalonesession_request_json: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
export const wasmstandalonesession_restore_progress_save: (a: number, b: number, c: number) => [number, number];
export const wasmstandalonesession_snapshot: (a: number) => [number, number];
export const __wbindgen_externrefs: WebAssembly.Table;
export const __externref_table_dealloc: (a: number) => void;
export const __wbindgen_free: (a: number, b: number, c: number) => void;
export const __wbindgen_malloc: (a: number, b: number) => number;
export const __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
export const __wbindgen_start: () => void;
