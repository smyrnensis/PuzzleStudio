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

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_wasmcompiledcoreruntime_free: (a: number, b: number) => void;
    readonly wasmcompiledcoreruntime_current_state: (a: number) => [number, number, number, number];
    readonly wasmcompiledcoreruntime_current_state_hash: (a: number) => [number, number, number, number];
    readonly wasmcompiledcoreruntime_new: (a: number, b: number) => [number, number, number];
    readonly wasmcompiledcoreruntime_restore_saved_state: (a: number, b: number) => [number, number];
    readonly wasmcompiledcoreruntime_save_current_state: (a: number) => [number, number, number];
    readonly wasmcompiledcoreruntime_set_state: (a: number, b: number, c: number) => [number, number];
    readonly wasmcompiledcoreruntime_transition_current_outcome: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
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
