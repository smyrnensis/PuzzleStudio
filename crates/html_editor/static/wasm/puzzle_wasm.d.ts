/* tslint:disable */
/* eslint-disable */

export class WasmCompiledCoreRuntime {
    free(): void;
    [Symbol.dispose](): void;
    current_state(): string;
    current_state_hash(): string;
    constructor(engine_json: string);
    restore_saved_state(handle: number): void;
    save_current_state(): number;
    set_state(state_json: string): void;
    transition_current_outcome(program_key: string, level_index: number, input: number): string;
}

export function compile_preview(source: string, puzzle_path: string, game_css: string, game_visuals_js: string): string;

export function export_html(source: string, puzzle_path: string, game_css: string, game_visuals_js: string, game_runtime_module_js: string, game_runtime_wasm_base64: string): string;

export function generate_visuals_js(source: string, base_visuals_js: string): string;

export function highlight_source_html(source: string): string;

export function highlight_source_json(source: string, include_outline: boolean): string;

export function resolve_source_target(source: string, cursor_offset: number): string;

export function solve_request_json(request_json: string): string;

export function solve_solver_task_json(request_json: string): string;

export function solve_solver_task_json_with_progress(request_json: string, on_observation: Function): string;

export function solve_state(source: string, puzzle_path: string, state_json: string, max_depth: number, max_nodes: number, max_ms: number): string;

export function solver_task_initial_display_state_json(request_json: string): string;

export function source_entries_json(source: string): string;

export function source_outline_json(source: string): string;

export function suggest_source_completions(source: string, cursor_offset: number): string;

export function translate_puzzlescript(source: string): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly compile_preview: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number, number, number];
    readonly export_html: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number) => [number, number, number, number];
    readonly generate_visuals_js: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly highlight_source_html: (a: number, b: number) => [number, number];
    readonly highlight_source_json: (a: number, b: number, c: number) => [number, number];
    readonly resolve_source_target: (a: number, b: number, c: number) => [number, number];
    readonly solve_request_json: (a: number, b: number) => [number, number, number, number];
    readonly solve_solver_task_json: (a: number, b: number) => [number, number, number, number];
    readonly solve_solver_task_json_with_progress: (a: number, b: number, c: any) => [number, number, number, number];
    readonly solve_state: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number) => [number, number, number, number];
    readonly solver_task_initial_display_state_json: (a: number, b: number) => [number, number, number, number];
    readonly source_entries_json: (a: number, b: number) => [number, number];
    readonly source_outline_json: (a: number, b: number) => [number, number];
    readonly suggest_source_completions: (a: number, b: number, c: number) => [number, number];
    readonly translate_puzzlescript: (a: number, b: number) => [number, number, number, number];
    readonly __wbg_wasmcompiledcoreruntime_free: (a: number, b: number) => void;
    readonly wasmcompiledcoreruntime_current_state: (a: number) => [number, number, number, number];
    readonly wasmcompiledcoreruntime_current_state_hash: (a: number) => [number, number, number, number];
    readonly wasmcompiledcoreruntime_new: (a: number, b: number) => [number, number, number];
    readonly wasmcompiledcoreruntime_restore_saved_state: (a: number, b: number) => [number, number];
    readonly wasmcompiledcoreruntime_save_current_state: (a: number) => [number, number, number];
    readonly wasmcompiledcoreruntime_set_state: (a: number, b: number, c: number) => [number, number];
    readonly wasmcompiledcoreruntime_transition_current_outcome: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_start: () => void;
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
