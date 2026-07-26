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
    readonly visualImageAssets: ReadonlyArray<{
        readonly id: string;
        readonly path: string;
        readonly format: "png" | "jpeg";
    }>;
}



/**
 * Editor-owned sound audition session.
 *
 * Authoring recipes cross this editor-only contract directly into Rust
 * synthesis. Only resolved assets and device commands reach WebAudio.
 */
export class WasmEditorAudio {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Drains every queued browser audio event in arrival order.
     *
     * Voice failures are contained to their voice by `AudioRuntime`; a
     * device-scoped failure alone may change output capability. Diagnostics
     * are returned as JSON so the editor can present every settled failure
     * without interpreting browser audio state.
     */
    audio_feedback_event(now_ms: number): string;
    configure(sfx_seed: string, sfx_type: string, sfx_volume: number, music_seed: string, music_height: number, music_bars: number, music_bpm: number, music_volume: number, now_ms: number): void;
    export_music_wav(): Uint8Array;
    export_sfx_wav(): Uint8Array;
    music_progress(now_ms: number): number;
    constructor();
    pause_music(now_ms: number): void;
    play_music(progress: number, now_ms: number): void;
    play_sfx(now_ms: number): void;
    resume_music(now_ms: number): void;
    /**
     * Connects asynchronous Web Audio events to this Rust-owned audition
     * session. The callback is a wakeup only; typed feedback remains owned by
     * `BrowserAudioBackend` until `audio_feedback_event` drains it.
     */
    set_audio_feedback_wakeup(callback: Function): void;
    set_visible(visible: boolean, now_ms: number): void;
    stop(now_ms: number): void;
    stop_music(now_ms: number): void;
    unlock(now_ms: number): Promise<void>;
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

<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
export function activate_source_analysis_with_profile(source: string, source_profile: string): number;
=======
export class WasmWorkspaceSession {
    free(): void;
    [Symbol.dispose](): void;
    compile_preview(entry_path: string, game_css: string, game_visuals_js: string): string;
    export_html(entry_path: string, game_css: string, game_visuals_js: string, player_runtime_module_js: string, player_runtime_wasm_base64: string): string;
    index_json(): string;
    constructor(documents: ReadonlyArray<WorkspaceSourceDocument>);
    presentation_manifest(entry_path: string): WorkspacePresentationManifest;
    replace_documents(documents: ReadonlyArray<WorkspaceSourceDocument>): void;
    revision(): number;
    source_analysis_json(path: string): string;
}

export function activate_source_analysis(source: string): number;
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544

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
 * visual payloads deliberately travel through their own on-demand exports.
 */
export function active_source_analysis_level_editor_manifest_json(revision: number): string;

/**
 * Returns one renderer-ready visual payload by canonical object ID.
 */
export function active_source_analysis_level_editor_visual_json(revision: number, object_id: number): string;

export function active_source_analysis_level_source_request(revision: number, request_json: string): string;

export function active_source_analysis_mutate_visual(revision: number, request_json: string): string;

export function active_source_analysis_outline_json(revision: number): string;

export function active_source_analysis_resolve_source_target(revision: number, cursor_utf16_offset: number): string;

export function active_source_analysis_sound_request(revision: number, request_json: string): string;

export function active_source_analysis_suggest_source_completions(revision: number, cursor_utf16_offset: number): string;

export function apply_source_analysis_edit(revision: number, start_utf16: number, end_utf16: number, insert: string): string;

export function compile_preview(source: string, puzzle_path: string, game_css: string, game_visuals_js: string): string;

<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
export function compile_workspace_preview(entry_path: string, documents: ReadonlyArray<WorkspaceSourceDocument>, game_css: string, game_visuals_js: string): string;

export function editor_audio_random_music_preset(seed: string): any;

export function editor_audio_random_sfx_preset(seed: string, type_target: string): any;

export function editor_audio_sfx_types(): Array<any>;

=======
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
export function export_html(source: string, puzzle_path: string, game_css: string, game_visuals_js: string, player_runtime_module_js: string, player_runtime_wasm_base64: string): string;

export function generate_visuals_js(source: string, base_visuals_js: string): string;

export function translate_puzzlescript(source: string): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_wasmeditoraudio_free: (a: number, b: number) => void;
    readonly __wbg_wasmsolverservice_free: (a: number, b: number) => void;
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
    readonly activate_source_analysis_with_profile: (a: number, b: number, c: number, d: number) => [number, number, number];
=======
    readonly __wbg_wasmworkspacesession_free: (a: number, b: number) => void;
    readonly activate_source_analysis: (a: number, b: number) => number;
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
    readonly active_source_analysis_entries_json: (a: number) => [number, number, number, number];
    readonly active_source_analysis_highlight_range_json: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly active_source_analysis_import_at_json: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly active_source_analysis_json: (a: number) => [number, number, number, number];
    readonly active_source_analysis_level_editor_level_slots: (a: number, b: number, c: number) => [number, number, number, number];
    readonly active_source_analysis_level_editor_manifest_json: (a: number) => [number, number, number, number];
    readonly active_source_analysis_level_editor_visual_json: (a: number, b: number) => [number, number, number, number];
    readonly active_source_analysis_level_source_request: (a: number, b: number, c: number) => [number, number, number, number];
    readonly active_source_analysis_mutate_visual: (a: number, b: number, c: number) => [number, number, number, number];
    readonly active_source_analysis_outline_json: (a: number) => [number, number, number, number];
    readonly active_source_analysis_resolve_source_target: (a: number, b: number) => [number, number, number, number];
    readonly active_source_analysis_sound_request: (a: number, b: number, c: number) => [number, number, number, number];
    readonly active_source_analysis_suggest_source_completions: (a: number, b: number) => [number, number, number, number];
    readonly apply_source_analysis_edit: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly compile_preview: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number, number, number];
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
    readonly compile_workspace_preview: (a: number, b: number, c: any, d: number, e: number, f: number, g: number) => [number, number, number, number];
    readonly editor_audio_random_music_preset: (a: number, b: number) => [number, number, number];
    readonly editor_audio_random_sfx_preset: (a: number, b: number, c: number, d: number) => [number, number, number];
=======
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
    readonly export_html: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number) => [number, number, number, number];
    readonly generate_visuals_js: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly translate_puzzlescript: (a: number, b: number) => [number, number, number, number];
    readonly wasmeditoraudio_audio_feedback_event: (a: number, b: number) => [number, number, number, number];
    readonly wasmeditoraudio_configure: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number, m: number) => [number, number];
    readonly wasmeditoraudio_export_music_wav: (a: number) => [number, number, number, number];
    readonly wasmeditoraudio_export_sfx_wav: (a: number) => [number, number, number, number];
    readonly wasmeditoraudio_music_progress: (a: number, b: number) => [number, number, number];
    readonly wasmeditoraudio_new: () => [number, number, number];
    readonly wasmeditoraudio_pause_music: (a: number, b: number) => [number, number];
    readonly wasmeditoraudio_play_music: (a: number, b: number, c: number) => [number, number];
    readonly wasmeditoraudio_play_sfx: (a: number, b: number) => [number, number];
    readonly wasmeditoraudio_resume_music: (a: number, b: number) => [number, number];
    readonly wasmeditoraudio_set_audio_feedback_wakeup: (a: number, b: any) => void;
    readonly wasmeditoraudio_set_visible: (a: number, b: number, c: number) => [number, number];
    readonly wasmeditoraudio_stop: (a: number, b: number) => [number, number];
    readonly wasmeditoraudio_stop_music: (a: number, b: number) => [number, number];
    readonly wasmeditoraudio_unlock: (a: number, b: number) => any;
    readonly wasmsolverservice_advance: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly wasmsolverservice_cancel: (a: number, b: number, c: number) => [number, number];
    readonly wasmsolverservice_materialize_state: (a: number, b: number, c: number, d: number, e: any, f: number, g: number) => [number, number, number];
    readonly wasmsolverservice_new: () => number;
    readonly wasmsolverservice_pin_artifact: (a: number, b: number, c: number, d: number) => [number, number];
    readonly wasmsolverservice_prepare_source: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number];
    readonly wasmsolverservice_prepare_workspace: (a: number, b: number, c: number, d: any, e: number) => [number, number, number];
    readonly wasmsolverservice_start: (a: number, b: number, c: number, d: any, e: number) => [number, number, number];
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
    readonly workspace_presentation_manifest: (a: number, b: number, c: any) => [number, number, number];
    readonly editor_audio_sfx_types: () => any;
    readonly wasm_bindgen__convert__closures_____invoke__h5b8ed1e5fec17ec2: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen__convert__closures_____invoke__h709bd5d4c0b96c84: (a: number, b: number, c: any, d: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h44b7ab73cd046207: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h44b7ab73cd046207_2: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h4681ec085113b112: (a: number, b: number) => void;
=======
    readonly wasmworkspacesession_compile_preview: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number, number, number];
    readonly wasmworkspacesession_export_html: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number) => [number, number, number, number];
    readonly wasmworkspacesession_index_json: (a: number) => [number, number, number, number];
    readonly wasmworkspacesession_new: (a: any) => [number, number, number];
    readonly wasmworkspacesession_presentation_manifest: (a: number, b: number, c: number) => [number, number, number];
    readonly wasmworkspacesession_replace_documents: (a: number, b: any) => [number, number];
    readonly wasmworkspacesession_revision: (a: number) => number;
    readonly wasmworkspacesession_source_analysis_json: (a: number, b: number, c: number) => [number, number, number, number];
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_destroy_closure: (a: number, b: number) => void;
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
