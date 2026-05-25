/* tslint:disable */
/* eslint-disable */

export class WasmCoreRuntime {
    free(): void;
    [Symbol.dispose](): void;
    constructor(source: string, puzzle_path: string);
    transition_program_outcome(program_key: string, level_index: number, state_json: string, input: number): string;
}

export function compile_preview(source: string, puzzle_path: string, game_css: string, game_visuals_js: string): string;

export function generate_visuals_js(source: string, base_visuals_js: string): string;

export function highlight_source_html(source: string): string;

export function resolve_source_target(source: string, cursor_offset: number): string;

export function solve_state(source: string, puzzle_path: string, state_json: string, max_depth: number, max_nodes: number, max_ms: number): string;

export function suggest_source_completions(source: string, cursor_offset: number): string;

export function transition_program_outcome(source: string, program_key: string, level_index: number, state_json: string, input: number): string;

export function translate_puzzlescript(source: string): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_wasmcoreruntime_free: (a: number, b: number) => void;
    readonly compile_preview: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number, number, number];
    readonly generate_visuals_js: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly highlight_source_html: (a: number, b: number) => [number, number];
    readonly resolve_source_target: (a: number, b: number, c: number) => [number, number];
    readonly solve_state: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number) => [number, number, number, number];
    readonly suggest_source_completions: (a: number, b: number, c: number) => [number, number];
    readonly transition_program_outcome: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number, number, number];
    readonly translate_puzzlescript: (a: number, b: number) => [number, number, number, number];
    readonly wasmcoreruntime_new: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly wasmcoreruntime_transition_program_outcome: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number, number, number];
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
