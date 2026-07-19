/* tslint:disable */
/* eslint-disable */

export interface WorkspaceSourceDocument {
    readonly path: string;
    readonly source: string;
}

export interface WorkspacePresentationManifest {
    readonly themeName: string | null;
    readonly cssPaths: string[];
    readonly scriptPaths: string[];
    readonly filePaths: string[];
    readonly spriteImagePaths: string[];
}



export class WasmSolverService {
    free(): void;
    [Symbol.dispose](): void;
    advance(search_id: number, max_expanded_nodes: number, now_ms: number): any;
    cancel(search_id: number, now_ms: number): void;
    materialize_state(artifact_id: string, level_index: number, state: any, materialize_level_start: boolean, now_ms: number): any;
    constructor();
    pin_artifact(artifact_id: string | null | undefined, now_ms: number): void;
    prepare_source(source: string, puzzle_path: string, now_ms: number): any;
    prepare_workspace(entry_path: string, documents: ReadonlyArray<WorkspaceSourceDocument>, now_ms: number): any;
    start(artifact_id: string, request: any, now_ms: number): number;
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

export function compile_workspace_preview(entry_path: string, documents: ReadonlyArray<WorkspaceSourceDocument>, game_css: string, game_visuals_js: string): string;

export function export_html(source: string, puzzle_path: string, game_css: string, game_visuals_js: string, player_runtime_module_js: string, player_runtime_wasm_base64: string): string;

export function export_workspace_html(entry_path: string, documents: ReadonlyArray<WorkspaceSourceDocument>, game_css: string, game_visuals_js: string, player_runtime_module_js: string, player_runtime_wasm_base64: string): string;

export function generate_visuals_js(source: string, base_visuals_js: string): string;

export function translate_puzzlescript(source: string): string;

export function workspace_presentation_manifest(entry_path: string, documents: ReadonlyArray<WorkspaceSourceDocument>): WorkspacePresentationManifest;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_wasmsolverservice_free: (a: number, b: number) => void;
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
    readonly compile_workspace_preview: (a: number, b: number, c: any, d: number, e: number, f: number, g: number) => [number, number, number, number];
    readonly export_html: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number) => [number, number, number, number];
    readonly export_workspace_html: (a: number, b: number, c: any, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number) => [number, number, number, number];
    readonly generate_visuals_js: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly translate_puzzlescript: (a: number, b: number) => [number, number, number, number];
    readonly wasmsolverservice_advance: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly wasmsolverservice_cancel: (a: number, b: number, c: number) => [number, number];
    readonly wasmsolverservice_materialize_state: (a: number, b: number, c: number, d: number, e: any, f: number, g: number) => [number, number, number];
    readonly wasmsolverservice_new: () => number;
    readonly wasmsolverservice_pin_artifact: (a: number, b: number, c: number, d: number) => [number, number];
    readonly wasmsolverservice_prepare_source: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number];
    readonly wasmsolverservice_prepare_workspace: (a: number, b: number, c: number, d: any, e: number) => [number, number, number];
    readonly wasmsolverservice_start: (a: number, b: number, c: number, d: any, e: number) => [number, number, number];
    readonly workspace_presentation_manifest: (a: number, b: number, c: any) => [number, number, number];
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
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
