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

export function activate_source_analysis(source: string): number;

export function activate_source_analysis_with_profile(source: string, source_profile: string): number;

export function active_source_analysis_entries_json(revision: number): string;

export function active_source_analysis_highlight_range_json(revision: number, range_start_utf16: number, range_end_utf16: number, include_outline: boolean): string;

export function active_source_analysis_import_at_json(revision: number, document_path: string, cursor_utf16_offset: number): string;

export function active_source_analysis_json(revision: number): string;

/**
 * Returns a typed object-ID buffer for one integrated level state. Pass `-1` for
 * the composite state, otherwise pass the authored ASCII layer index.
 */
export function active_source_analysis_level_editor_level_slots(revision: number, level_index: number, authored_layer: number): Uint32Array;

/**
 * Returns level-editor metadata for the active source snapshot. Board cells and
 * sprite payloads deliberately travel through their own on-demand exports.
 */
export function active_source_analysis_level_editor_manifest_json(revision: number): string;

/**
 * Returns one renderer-ready sprite payload by canonical object ID.
 */
export function active_source_analysis_level_editor_sprite_json(revision: number, object_id: number): string;

export function active_source_analysis_mutate_sprite(revision: number, request_json: string): string;

export function active_source_analysis_outline_json(revision: number): string;

export function active_source_analysis_resolve_source_target(revision: number, cursor_utf16_offset: number): string;

export function active_source_analysis_suggest_source_completions(revision: number, cursor_utf16_offset: number): string;

export function apply_source_analysis_edit(revision: number, start_utf16: number, end_utf16: number, insert: string): string;

export function compile_preview(source: string, puzzle_path: string, game_css: string, game_visuals_js: string): string;

export function compile_solver_rules_json(source: string, puzzle_path: string): string;

export function compile_workspace_preview(entry_path: string, documents_json: string, game_css: string, game_visuals_js: string): string;

export function compile_workspace_solver_rules_json(entry_path: string, documents_json: string): string;

export function editor_solver_cache_policy_json(): string;

export function expand_workspace_entry_source(entry_path: string, documents_json: string): string;

export function export_html(source: string, puzzle_path: string, game_css: string, game_visuals_js: string, player_runtime_module_js: string, player_runtime_wasm_base64: string): string;

export function export_workspace_html(entry_path: string, documents_json: string, game_css: string, game_visuals_js: string, player_runtime_module_js: string, player_runtime_wasm_base64: string): string;

export function generate_visuals_js(source: string, base_visuals_js: string): string;

export function solve_request_json(request_json: string): string;

export function solve_solver_task_json(request_json: string): string;

export function solve_solver_task_json_with_progress(request_json: string, on_observation: Function): string;

export function solve_state(source: string, puzzle_path: string, state_json: string, max_depth: number, max_nodes: number, max_ms: number): string;

export function solver_task_initial_display_state_json(request_json: string): string;

export function translate_puzzlescript(source: string): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly activate_source_analysis: (a: number, b: number) => number;
    readonly activate_source_analysis_with_profile: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly active_source_analysis_entries_json: (a: number) => [number, number, number, number];
    readonly active_source_analysis_highlight_range_json: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly active_source_analysis_import_at_json: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly active_source_analysis_json: (a: number) => [number, number, number, number];
    readonly active_source_analysis_level_editor_level_slots: (a: number, b: number, c: number) => [number, number, number, number];
    readonly active_source_analysis_level_editor_manifest_json: (a: number) => [number, number, number, number];
    readonly active_source_analysis_level_editor_sprite_json: (a: number, b: number) => [number, number, number, number];
    readonly active_source_analysis_mutate_sprite: (a: number, b: number, c: number) => [number, number, number, number];
    readonly active_source_analysis_outline_json: (a: number) => [number, number, number, number];
    readonly active_source_analysis_resolve_source_target: (a: number, b: number) => [number, number, number, number];
    readonly active_source_analysis_suggest_source_completions: (a: number, b: number) => [number, number, number, number];
    readonly apply_source_analysis_edit: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly compile_preview: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number, number, number];
    readonly compile_solver_rules_json: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly compile_workspace_preview: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number, number, number];
    readonly compile_workspace_solver_rules_json: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly editor_solver_cache_policy_json: () => [number, number];
    readonly expand_workspace_entry_source: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly export_html: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number) => [number, number, number, number];
    readonly export_workspace_html: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number) => [number, number, number, number];
    readonly generate_visuals_js: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly solve_request_json: (a: number, b: number) => [number, number, number, number];
    readonly solve_solver_task_json: (a: number, b: number) => [number, number, number, number];
    readonly solve_solver_task_json_with_progress: (a: number, b: number, c: any) => [number, number, number, number];
    readonly solve_state: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number) => [number, number, number, number];
    readonly solver_task_initial_display_state_json: (a: number, b: number) => [number, number, number, number];
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
