/* tslint:disable */
/* eslint-disable */

export class WasmPuzzle3Runtime {
    free(): void;
    [Symbol.dispose](): void;
    current_cells(): string;
    is_current_complete(): boolean;
    constructor(source: string, puzzle_path: string);
    restore_saved_state(handle: number): void;
    save_current_state(): number;
    set_state(state_json: string): void;
    transition_current_outcome(program_key: string, input: number): string;
}

export class WasmStandaloneSession {
    free(): void;
    [Symbol.dispose](): void;
    apply_command_name(command_name: string): void;
    apply_input_name(input_name: string): void;
    clear_progress_save(): void;
    static fromExport(export_json: string): WasmStandaloneSession;
    mark_progress_save_written(): void;
    constructor(source: string, puzzle_path: string);
    progress_save(): string;
    request_json(method: string, url: string): string;
    restore_progress_save(save_json: string): void;
    snapshot(): string;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_wasmpuzzle3runtime_free: (a: number, b: number) => void;
    readonly __wbg_wasmstandalonesession_free: (a: number, b: number) => void;
    readonly wasmpuzzle3runtime_current_cells: (a: number) => [number, number, number, number];
    readonly wasmpuzzle3runtime_is_current_complete: (a: number) => [number, number, number];
    readonly wasmpuzzle3runtime_new: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly wasmpuzzle3runtime_restore_saved_state: (a: number, b: number) => [number, number];
    readonly wasmpuzzle3runtime_save_current_state: (a: number) => [number, number, number];
    readonly wasmpuzzle3runtime_set_state: (a: number, b: number, c: number) => [number, number];
    readonly wasmpuzzle3runtime_transition_current_outcome: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly wasmstandalonesession_apply_command_name: (a: number, b: number, c: number) => [number, number];
    readonly wasmstandalonesession_apply_input_name: (a: number, b: number, c: number) => [number, number];
    readonly wasmstandalonesession_clear_progress_save: (a: number) => void;
    readonly wasmstandalonesession_fromExport: (a: number, b: number) => [number, number, number];
    readonly wasmstandalonesession_mark_progress_save_written: (a: number) => void;
    readonly wasmstandalonesession_new: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly wasmstandalonesession_progress_save: (a: number) => [number, number];
    readonly wasmstandalonesession_request_json: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly wasmstandalonesession_restore_progress_save: (a: number, b: number, c: number) => [number, number];
    readonly wasmstandalonesession_snapshot: (a: number) => [number, number];
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
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
