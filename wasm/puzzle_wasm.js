/* @ts-self-types="./puzzle_wasm.d.ts" */

/**
 * Editor-owned sound audition session.
 *
 * Authoring recipes cross this editor-only contract directly into Rust
 * synthesis. Only resolved assets and device commands reach WebAudio.
 */
export class WasmEditorAudio {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmEditorAudioFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmeditoraudio_free(ptr, 0);
    }
    /**
     * Drains every queued browser audio event in arrival order.
     *
     * Voice failures are contained to their voice by `AudioRuntime`; a
     * device-scoped failure alone may change output capability. Diagnostics
     * are returned as JSON so the editor can present every settled failure
     * without interpreting browser audio state.
     * @param {number} now_ms
     * @returns {string}
     */
    audio_feedback_event(now_ms) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ret = wasm.wasmeditoraudio_audio_feedback_event(this.__wbg_ptr, now_ms);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {string} sfx_seed
     * @param {string} sfx_type
     * @param {number} sfx_volume
     * @param {string} music_backend
     * @param {string} music_seed
     * @param {string} music_slots_json
     * @param {number} music_height
     * @param {number} music_bars
     * @param {number} music_bpm
     * @param {number} music_volume
     * @param {number} now_ms
     */
    configure(sfx_seed, sfx_type, sfx_volume, music_backend, music_seed, music_slots_json, music_height, music_bars, music_bpm, music_volume, now_ms) {
        const ptr0 = passStringToWasm0(sfx_seed, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(sfx_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(music_backend, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ptr3 = passStringToWasm0(music_seed, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len3 = WASM_VECTOR_LEN;
        const ptr4 = passStringToWasm0(music_slots_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len4 = WASM_VECTOR_LEN;
        const ret = wasm.wasmeditoraudio_configure(this.__wbg_ptr, ptr0, len0, ptr1, len1, sfx_volume, ptr2, len2, ptr3, len3, ptr4, len4, music_height, music_bars, music_bpm, music_volume, now_ms);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Configures the isolated Abundant Music preview with its complete
     * generator-neutral arrangement and Rust-owned probabilistic timbres.
     * @param {string} arrangement_json
     * @param {string} music_slots_json
     * @param {number} music_height
     * @param {number} music_volume
     * @param {number} now_ms
     */
    configure_abundant_music(arrangement_json, music_slots_json, music_height, music_volume, now_ms) {
        const ptr0 = passStringToWasm0(arrangement_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(music_slots_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmeditoraudio_configure_abundant_music(this.__wbg_ptr, ptr0, len0, ptr1, len1, music_height, music_volume, now_ms);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * @returns {any}
     */
    export_music_pcm() {
        const ret = wasm.wasmeditoraudio_export_music_pcm(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * @returns {Uint8Array}
     */
    export_music_wav() {
        const ret = wasm.wasmeditoraudio_export_music_wav(this.__wbg_ptr);
        if (ret[3]) {
            throw takeFromExternrefTable0(ret[2]);
        }
        var v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        return v1;
    }
    /**
     * @returns {Uint8Array}
     */
    export_sfx_wav() {
        const ret = wasm.wasmeditoraudio_export_sfx_wav(this.__wbg_ptr);
        if (ret[3]) {
            throw takeFromExternrefTable0(ret[2]);
        }
        var v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        return v1;
    }
    /**
     * @param {number} now_ms
     * @returns {number}
     */
    music_progress(now_ms) {
        const ret = wasm.wasmeditoraudio_music_progress(this.__wbg_ptr, now_ms);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0];
    }
    constructor() {
        const ret = wasm.wasmeditoraudio_new();
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        this.__wbg_ptr = ret[0];
        WasmEditorAudioFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * @param {number} now_ms
     */
    pause_music(now_ms) {
        const ret = wasm.wasmeditoraudio_pause_music(this.__wbg_ptr, now_ms);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * @param {number} progress
     * @param {number} now_ms
     */
    play_music(progress, now_ms) {
        const ret = wasm.wasmeditoraudio_play_music(this.__wbg_ptr, progress, now_ms);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * @param {number} now_ms
     */
    play_sfx(now_ms) {
        const ret = wasm.wasmeditoraudio_play_sfx(this.__wbg_ptr, now_ms);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * @returns {Promise<void>}
     */
    prepare_music() {
        const ret = wasm.wasmeditoraudio_prepare_music(this.__wbg_ptr);
        return ret;
    }
    /**
     * @param {number} now_ms
     */
    resume_music(now_ms) {
        const ret = wasm.wasmeditoraudio_resume_music(this.__wbg_ptr, now_ms);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Connects asynchronous Web Audio events to this Rust-owned audition
     * session. The callback is a wakeup only; typed feedback remains owned by
     * `BrowserAudioBackend` until `audio_feedback_event` drains it.
     * @param {Function} callback
     */
    set_audio_feedback_wakeup(callback) {
        wasm.wasmeditoraudio_set_audio_feedback_wakeup(this.__wbg_ptr, callback);
    }
    /**
     * @param {boolean} visible
     * @param {number} now_ms
     */
    set_visible(visible, now_ms) {
        const ret = wasm.wasmeditoraudio_set_visible(this.__wbg_ptr, visible, now_ms);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * @param {number} now_ms
     */
    stop(now_ms) {
        const ret = wasm.wasmeditoraudio_stop(this.__wbg_ptr, now_ms);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * @param {number} now_ms
     */
    stop_music(now_ms) {
        const ret = wasm.wasmeditoraudio_stop_music(this.__wbg_ptr, now_ms);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * @param {number} now_ms
     * @returns {Promise<void>}
     */
    unlock(now_ms) {
        const ret = wasm.wasmeditoraudio_unlock(this.__wbg_ptr, now_ms);
        return ret;
    }
}
if (Symbol.dispose) WasmEditorAudio.prototype[Symbol.dispose] = WasmEditorAudio.prototype.free;

export class WasmEditorLevelApplication {
    static __wrap(ptr) {
        const obj = Object.create(WasmEditorLevelApplication.prototype);
        obj.__wbg_ptr = ptr;
        WasmEditorLevelApplicationFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmEditorLevelApplicationFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmeditorlevelapplication_free(ptr, 0);
    }
    /**
     * @param {any} request
     * @returns {any}
     */
    handle(request) {
        const ret = wasm.wasmeditorlevelapplication_handle(this.__wbg_ptr, request);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
}
if (Symbol.dispose) WasmEditorLevelApplication.prototype[Symbol.dispose] = WasmEditorLevelApplication.prototype.free;

export class WasmEditorWorkspaceSession {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmEditorWorkspaceSessionFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmeditorworkspacesession_free(ptr, 0);
    }
    /**
     * @param {any} command
     * @returns {any}
     */
    dispatch(command) {
        const ret = wasm.wasmeditorworkspacesession_dispatch(this.__wbg_ptr, command);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * @param {any} document_ids
     */
    constructor(document_ids) {
        const ret = wasm.wasmeditorworkspacesession_new(document_ids);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        this.__wbg_ptr = ret[0];
        WasmEditorWorkspaceSessionFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
}
if (Symbol.dispose) WasmEditorWorkspaceSession.prototype[Symbol.dispose] = WasmEditorWorkspaceSession.prototype.free;

/**
 * One compiled document held across the asynchronous acquisition of the
 * standalone runtime artifact selected from that document's capabilities.
 */
export class WasmPreparedWebExport {
    static __wrap(ptr) {
        const obj = Object.create(WasmPreparedWebExport.prototype);
        obj.__wbg_ptr = ptr;
        WasmPreparedWebExportFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmPreparedWebExportFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmpreparedwebexport_free(ptr, 0);
    }
    /**
     * @param {string} player_runtime_module_source
     * @param {string} player_runtime_wasm_base64
     * @param {string} player_renderer_module_js
     * @param {string} player_audio_worklet_js
     * @returns {string}
     */
    export_web_bundle(player_runtime_module_source, player_runtime_wasm_base64, player_renderer_module_js, player_audio_worklet_js) {
        let deferred6_0;
        let deferred6_1;
        try {
            const ptr0 = passStringToWasm0(player_runtime_module_source, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(player_runtime_wasm_base64, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ptr2 = passStringToWasm0(player_renderer_module_js, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len2 = WASM_VECTOR_LEN;
            const ptr3 = passStringToWasm0(player_audio_worklet_js, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len3 = WASM_VECTOR_LEN;
            const ret = wasm.wasmpreparedwebexport_export_web_bundle(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3);
            var ptr5 = ret[0];
            var len5 = ret[1];
            if (ret[3]) {
                ptr5 = 0; len5 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred6_0 = ptr5;
            deferred6_1 = len5;
            return getStringFromWasm0(ptr5, len5);
        } finally {
            wasm.__wbindgen_free(deferred6_0, deferred6_1, 1);
        }
    }
    /**
     * @returns {StandaloneRendererArtifact}
     */
    renderer_artifact() {
        const ret = wasm.wasmpreparedwebexport_renderer_artifact(this.__wbg_ptr);
        return ret;
    }
}
if (Symbol.dispose) WasmPreparedWebExport.prototype[Symbol.dispose] = WasmPreparedWebExport.prototype.free;

export class WasmSolverService {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmSolverServiceFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmsolverservice_free(ptr, 0);
    }
    /**
     * @param {number} search_id
     * @param {number} max_expanded_nodes
     * @param {number} now_ms
     * @returns {any}
     */
    advance(search_id, max_expanded_nodes, now_ms) {
        const ret = wasm.wasmsolverservice_advance(this.__wbg_ptr, search_id, max_expanded_nodes, now_ms);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * @param {number} search_id
     * @param {number} now_ms
     */
    cancel(search_id, now_ms) {
        const ret = wasm.wasmsolverservice_cancel(this.__wbg_ptr, search_id, now_ms);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * @param {number} search_id
     * @param {string} candidate_id
     * @param {number} now_ms
     * @returns {any}
     */
    materialize_candidate(search_id, candidate_id, now_ms) {
        const ptr0 = passStringToWasm0(candidate_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmsolverservice_materialize_candidate(this.__wbg_ptr, search_id, ptr0, len0, now_ms);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * @param {string} artifact_id
     * @param {number} level_index
     * @param {any} state
     * @param {boolean} materialize_level_start
     * @param {number} now_ms
     * @returns {any}
     */
    materialize_state(artifact_id, level_index, state, materialize_level_start, now_ms) {
        const ptr0 = passStringToWasm0(artifact_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmsolverservice_materialize_state(this.__wbg_ptr, ptr0, len0, level_index, state, materialize_level_start, now_ms);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    constructor() {
        const ret = wasm.wasmsolverservice_new();
        this.__wbg_ptr = ret;
        WasmSolverServiceFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * @param {string | null | undefined} artifact_id
     * @param {number} now_ms
     */
    pin_artifact(artifact_id, now_ms) {
        var ptr0 = isLikeNone(artifact_id) ? 0 : passStringToWasm0(artifact_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmsolverservice_pin_artifact(this.__wbg_ptr, ptr0, len0, now_ms);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * @param {string} source
     * @param {string} puzzle_path
     * @param {string} model_name
     * @param {number} now_ms
     * @returns {any}
     */
    prepare_source(source, puzzle_path, model_name, now_ms) {
        const ptr0 = passStringToWasm0(source, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(puzzle_path, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(model_name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ret = wasm.wasmsolverservice_prepare_source(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2, now_ms);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * @param {string} entry_path
     * @param {string} model_name
     * @param {ReadonlyArray<WorkspaceSourceDocument>} documents
     * @param {number} now_ms
     * @returns {any}
     */
    prepare_workspace(entry_path, model_name, documents, now_ms) {
        const ptr0 = passStringToWasm0(entry_path, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(model_name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmsolverservice_prepare_workspace(this.__wbg_ptr, ptr0, len0, ptr1, len1, documents, now_ms);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * @param {string} artifact_id
     * @param {any} request
     * @param {number} now_ms
     * @returns {number}
     */
    start(artifact_id, request, now_ms) {
        const ptr0 = passStringToWasm0(artifact_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmsolverservice_start(this.__wbg_ptr, ptr0, len0, request, now_ms);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] >>> 0;
    }
}
if (Symbol.dispose) WasmSolverService.prototype[Symbol.dispose] = WasmSolverService.prototype.free;

export class WasmWorkspaceSession {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmWorkspaceSessionFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmworkspacesession_free(ptr, 0);
    }
    /**
     * @param {string} entry_path
     * @param {string} audio_file_documents_json
     * @returns {string}
     */
    compile_preview(entry_path, audio_file_documents_json) {
        let deferred4_0;
        let deferred4_1;
        try {
            const ptr0 = passStringToWasm0(entry_path, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(audio_file_documents_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ret = wasm.wasmworkspacesession_compile_preview(this.__wbg_ptr, ptr0, len0, ptr1, len1);
            var ptr3 = ret[0];
            var len3 = ret[1];
            if (ret[3]) {
                ptr3 = 0; len3 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred4_0 = ptr3;
            deferred4_1 = len3;
            return getStringFromWasm0(ptr3, len3);
        } finally {
            wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
        }
    }
    /**
     * @returns {string}
     */
    index_json() {
        let deferred2_0;
        let deferred2_1;
        try {
            const ret = wasm.wasmworkspacesession_index_json(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {ReadonlyArray<WorkspaceSourceDocument>} documents
     */
    constructor(documents) {
        const ret = wasm.wasmworkspacesession_new(documents);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        this.__wbg_ptr = ret[0];
        WasmWorkspaceSessionFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * @param {string} entry_path
     * @param {string} audio_file_documents_json
     * @returns {WasmPreparedWebExport}
     */
    prepare_web_export(entry_path, audio_file_documents_json) {
        const ptr0 = passStringToWasm0(entry_path, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(audio_file_documents_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmworkspacesession_prepare_web_export(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return WasmPreparedWebExport.__wrap(ret[0]);
    }
    /**
     * @param {string} entry_path
     * @returns {WorkspacePresentationManifest}
     */
    presentation_manifest(entry_path) {
        const ptr0 = passStringToWasm0(entry_path, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmworkspacesession_presentation_manifest(this.__wbg_ptr, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * @param {ReadonlyArray<WorkspaceSourceDocument>} documents
     */
    replace_documents(documents) {
        const ret = wasm.wasmworkspacesession_replace_documents(this.__wbg_ptr, documents);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * @returns {number}
     */
    revision() {
        const ret = wasm.wasmworkspacesession_revision(this.__wbg_ptr);
        return ret;
    }
    /**
     * @param {string} path
     * @returns {string}
     */
    source_analysis_json(path) {
        let deferred3_0;
        let deferred3_1;
        try {
            const ptr0 = passStringToWasm0(path, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmworkspacesession_source_analysis_json(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
}
if (Symbol.dispose) WasmWorkspaceSession.prototype[Symbol.dispose] = WasmWorkspaceSession.prototype.free;

/**
 * @param {string} source
 * @returns {number}
 */
export function activate_source_analysis(source) {
    const ptr0 = passStringToWasm0(source, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.activate_source_analysis(ptr0, len0);
    return ret >>> 0;
}

/**
 * @param {number} revision
 * @returns {string}
 */
export function active_source_analysis_entries_json(revision) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ret = wasm.active_source_analysis_entries_json(revision);
        var ptr1 = ret[0];
        var len1 = ret[1];
        if (ret[3]) {
            ptr1 = 0; len1 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred2_0 = ptr1;
        deferred2_1 = len1;
        return getStringFromWasm0(ptr1, len1);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * @param {number} revision
 * @param {number} range_start_utf16
 * @param {number} range_end_utf16
 * @param {boolean} include_outline
 * @returns {string}
 */
export function active_source_analysis_highlight_range_json(revision, range_start_utf16, range_end_utf16, include_outline) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ret = wasm.active_source_analysis_highlight_range_json(revision, range_start_utf16, range_end_utf16, include_outline);
        var ptr1 = ret[0];
        var len1 = ret[1];
        if (ret[3]) {
            ptr1 = 0; len1 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred2_0 = ptr1;
        deferred2_1 = len1;
        return getStringFromWasm0(ptr1, len1);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * @param {number} revision
 * @returns {string}
 */
export function active_source_analysis_json(revision) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ret = wasm.active_source_analysis_json(revision);
        var ptr1 = ret[0];
        var len1 = ret[1];
        if (ret[3]) {
            ptr1 = 0; len1 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred2_0 = ptr1;
        deferred2_1 = len1;
        return getStringFromWasm0(ptr1, len1);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * Returns source-free static renderer resources for one selected authoring model.
 * @param {number} revision
 * @param {string} model_name
 * @returns {string}
 */
export function active_source_analysis_level_editor_authoring_model_projection_json(revision, model_name) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(model_name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.active_source_analysis_level_editor_authoring_model_projection_json(revision, ptr0, len0);
        var ptr2 = ret[0];
        var len2 = ret[1];
        if (ret[3]) {
            ptr2 = 0; len2 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * Returns a typed object-ID buffer for one integrated level state. Pass `-1` for
 * the composite state, otherwise pass the authored ASCII layer index.
 * @param {number} revision
 * @param {string} model_name
 * @param {number} level_index
 * @param {number} authored_layer
 * @returns {Uint32Array}
 */
export function active_source_analysis_level_editor_level_slots(revision, model_name, level_index, authored_layer) {
    const ptr0 = passStringToWasm0(model_name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.active_source_analysis_level_editor_level_slots(revision, ptr0, len0, level_index, authored_layer);
    if (ret[3]) {
        throw takeFromExternrefTable0(ret[2]);
    }
    var v2 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
    return v2;
}

/**
 * Returns level-editor metadata for the active source snapshot. Board cells and
 * visual payloads deliberately travel through their own on-demand exports.
 * @param {number} revision
 * @param {string} model_name
 * @returns {string}
 */
export function active_source_analysis_level_editor_manifest_json(revision, model_name) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(model_name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.active_source_analysis_level_editor_manifest_json(revision, ptr0, len0);
        var ptr2 = ret[0];
        var len2 = ret[1];
        if (ret[3]) {
            ptr2 = 0; len2 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * Returns one renderer-ready visual payload by canonical object ID.
 * @param {number} revision
 * @param {string} model_name
 * @param {number} object_id
 * @returns {string}
 */
export function active_source_analysis_level_editor_visual_json(revision, model_name, object_id) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(model_name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.active_source_analysis_level_editor_visual_json(revision, ptr0, len0, object_id);
        var ptr2 = ret[0];
        var len2 = ret[1];
        if (ret[3]) {
            ptr2 = 0; len2 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * @param {number} revision
 * @param {string} request_json
 * @returns {string}
 */
export function active_source_analysis_level_source_request(revision, request_json) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(request_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.active_source_analysis_level_source_request(revision, ptr0, len0);
        var ptr2 = ret[0];
        var len2 = ret[1];
        if (ret[3]) {
            ptr2 = 0; len2 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * @param {number} revision
 * @param {string} request_json
 * @returns {string}
 */
export function active_source_analysis_mutate_visual(revision, request_json) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(request_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.active_source_analysis_mutate_visual(revision, ptr0, len0);
        var ptr2 = ret[0];
        var len2 = ret[1];
        if (ret[3]) {
            ptr2 = 0; len2 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * @param {number} revision
 * @returns {string}
 */
export function active_source_analysis_outline_json(revision) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ret = wasm.active_source_analysis_outline_json(revision);
        var ptr1 = ret[0];
        var len1 = ret[1];
        if (ret[3]) {
            ptr1 = 0; len1 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred2_0 = ptr1;
        deferred2_1 = len1;
        return getStringFromWasm0(ptr1, len1);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * @param {number} revision
 * @param {number} cursor_utf16_offset
 * @returns {string}
 */
export function active_source_analysis_resolve_source_target(revision, cursor_utf16_offset) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ret = wasm.active_source_analysis_resolve_source_target(revision, cursor_utf16_offset);
        var ptr1 = ret[0];
        var len1 = ret[1];
        if (ret[3]) {
            ptr1 = 0; len1 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred2_0 = ptr1;
        deferred2_1 = len1;
        return getStringFromWasm0(ptr1, len1);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * @param {number} revision
 * @param {string} request_json
 * @returns {string}
 */
export function active_source_analysis_sound_request(revision, request_json) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(request_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.active_source_analysis_sound_request(revision, ptr0, len0);
        var ptr2 = ret[0];
        var len2 = ret[1];
        if (ret[3]) {
            ptr2 = 0; len2 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * @param {number} revision
 * @param {number} cursor_utf16_offset
 * @returns {string}
 */
export function active_source_analysis_suggest_source_completions(revision, cursor_utf16_offset) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ret = wasm.active_source_analysis_suggest_source_completions(revision, cursor_utf16_offset);
        var ptr1 = ret[0];
        var len1 = ret[1];
        if (ret[3]) {
            ptr1 = 0; len1 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred2_0 = ptr1;
        deferred2_1 = len1;
        return getStringFromWasm0(ptr1, len1);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * @param {number} revision
 * @param {number} start_utf16
 * @param {number} end_utf16
 * @param {string} insert
 * @returns {string}
 */
export function apply_source_analysis_edit(revision, start_utf16, end_utf16, insert) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(insert, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.apply_source_analysis_edit(revision, start_utf16, end_utf16, ptr0, len0);
        var ptr2 = ret[0];
        var len2 = ret[1];
        if (ret[3]) {
            ptr2 = 0; len2 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * @param {string} source
 * @param {string} puzzle_path
 * @returns {string}
 */
export function compile_preview(source, puzzle_path) {
    let deferred4_0;
    let deferred4_1;
    try {
        const ptr0 = passStringToWasm0(source, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(puzzle_path, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.compile_preview(ptr0, len0, ptr1, len1);
        var ptr3 = ret[0];
        var len3 = ret[1];
        if (ret[3]) {
            ptr3 = 0; len3 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred4_0 = ptr3;
        deferred4_1 = len3;
        return getStringFromWasm0(ptr3, len3);
    } finally {
        wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
    }
}

/**
 * Projects the selectable music authoring backends for the editor Sound pane.
 *
 * `declarable` marks the single backend that a `sounds { music ... }`
 * declaration means. The pane must not offer to write a non-declarable
 * backend into authored source.
 * @returns {any}
 */
export function editor_audio_music_backends() {
    const ret = wasm.editor_audio_music_backends();
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @param {string} seed
 * @returns {any}
 */
export function editor_audio_random_music_preset(seed) {
    const ptr0 = passStringToWasm0(seed, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.editor_audio_random_music_preset(ptr0, len0);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @param {string} seed
 * @param {string} type_target
 * @returns {any}
 */
export function editor_audio_random_sfx_preset(seed, type_target) {
    const ptr0 = passStringToWasm0(seed, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(type_target, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ret = wasm.editor_audio_random_sfx_preset(ptr0, len0, ptr1, len1);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @returns {Array<any>}
 */
export function editor_audio_sfx_types() {
    const ret = wasm.editor_audio_sfx_types();
    return ret;
}

export function initialize_wasm_diagnostics() {
    wasm.initialize_wasm_diagnostics();
}

/**
 * @param {number} revision
 * @param {string} model_name
 * @returns {WasmEditorLevelApplication}
 */
export function open_active_source_analysis_level_editor_application(revision, model_name) {
    const ptr0 = passStringToWasm0(model_name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.open_active_source_analysis_level_editor_application(revision, ptr0, len0);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return WasmEditorLevelApplication.__wrap(ret[0]);
}

/**
 * @param {string} source
 * @returns {string}
 */
export function translate_puzzlescript(source) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(source, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.translate_puzzlescript(ptr0, len0);
        var ptr2 = ret[0];
        var len2 = ret[1];
        if (ret[3]) {
            ptr2 = 0; len2 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}
function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg_Error_92b29b0548f8b746: function(arg0, arg1) {
            const ret = Error(getStringFromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_Number_9a4e0ecb0fa16705: function(arg0) {
            const ret = Number(arg0);
            return ret;
        },
        __wbg_String_8564e559799eccda: function(arg0, arg1) {
            const ret = String(arg1);
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_bigint_get_as_i64_d968e41184ae354f: function(arg0, arg1) {
            const v = arg1;
            const ret = typeof(v) === 'bigint' ? v : undefined;
            getDataViewMemory0().setBigInt64(arg0 + 8 * 1, isLikeNone(ret) ? BigInt(0) : ret, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
        },
        __wbg___wbindgen_boolean_get_fa956cfa2d1bd751: function(arg0) {
            const v = arg0;
            const ret = typeof(v) === 'boolean' ? v : undefined;
            return isLikeNone(ret) ? 0xFFFFFF : ret ? 1 : 0;
        },
        __wbg___wbindgen_debug_string_c25d447a39f5578f: function(arg0, arg1) {
            const ret = debugString(arg1);
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_in_aca499c5de7ff5e5: function(arg0, arg1) {
            const ret = arg0 in arg1;
            return ret;
        },
        __wbg___wbindgen_is_bigint_2f76dc55065b4273: function(arg0) {
            const ret = typeof(arg0) === 'bigint';
            return ret;
        },
        __wbg___wbindgen_is_function_1ff95bcc5517c252: function(arg0) {
            const ret = typeof(arg0) === 'function';
            return ret;
        },
        __wbg___wbindgen_is_object_a27215656b807791: function(arg0) {
            const val = arg0;
            const ret = typeof(val) === 'object' && val !== null;
            return ret;
        },
        __wbg___wbindgen_is_string_ea5e6cc2e4141dfe: function(arg0) {
            const ret = typeof(arg0) === 'string';
            return ret;
        },
        __wbg___wbindgen_is_undefined_c05833b95a3cf397: function(arg0) {
            const ret = arg0 === undefined;
            return ret;
        },
        __wbg___wbindgen_jsval_eq_e659fcf7b0e32763: function(arg0, arg1) {
            const ret = arg0 === arg1;
            return ret;
        },
        __wbg___wbindgen_jsval_loose_eq_db4c3b15f63fc170: function(arg0, arg1) {
            const ret = arg0 == arg1;
            return ret;
        },
        __wbg___wbindgen_number_get_394265ed1e1b84ee: function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'number' ? obj : undefined;
            getDataViewMemory0().setFloat64(arg0 + 8 * 1, isLikeNone(ret) ? 0 : ret, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
        },
        __wbg___wbindgen_string_get_b0ca35b86a603356: function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'string' ? obj : undefined;
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_throw_344f42d3211c4765: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbg__wbg_cb_unref_fffb441def202758: function(arg0) {
            arg0._wbg_cb_unref();
        },
        __wbg_addModule_f3bc9edcd955efdd: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.addModule(getStringFromWasm0(arg1, arg2));
            return ret;
        }, arguments); },
        __wbg_audioWorklet_d7d0902bffef7227: function() { return handleError(function (arg0) {
            const ret = arg0.audioWorklet;
            return ret;
        }, arguments); },
        __wbg_call_8a2dd23819f8a60a: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.call(arg1);
            return ret;
        }, arguments); },
        __wbg_call_a6e5c5dce5018821: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.call(arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_clearTimeout_8f80437be2324e09: function(arg0, arg1) {
            arg0.clearTimeout(arg1);
        },
        __wbg_close_3423cc7dafc477bb: function(arg0) {
            arg0.close();
        },
        __wbg_close_886ba3a68ebe947b: function() { return handleError(function (arg0) {
            const ret = arg0.close();
            return ret;
        }, arguments); },
        __wbg_connect_d4827f2479dd71e1: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.connect(arg1);
            return ret;
        }, arguments); },
        __wbg_copyToChannel_aa9f5237dd25a018: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            arg0.copyToChannel(getArrayF32FromWasm0(arg1, arg2), arg3);
        }, arguments); },
        __wbg_createBufferSource_b29772e399f58437: function() { return handleError(function (arg0) {
            const ret = arg0.createBufferSource();
            return ret;
        }, arguments); },
        __wbg_createBuffer_531b11bced993e64: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = arg0.createBuffer(arg1 >>> 0, arg2 >>> 0, arg3);
            return ret;
        }, arguments); },
        __wbg_createGain_73f2f9566ffdfb20: function() { return handleError(function (arg0) {
            const ret = arg0.createGain();
            return ret;
        }, arguments); },
        __wbg_data_328de4280640da92: function(arg0) {
            const ret = arg0.data;
            return ret;
        },
        __wbg_destination_961d9432c10b46a1: function(arg0) {
            const ret = arg0.destination;
            return ret;
        },
        __wbg_disconnect_d4b2e1ad9db3c319: function() { return handleError(function (arg0) {
            arg0.disconnect();
        }, arguments); },
        __wbg_done_89b2b13e91a60321: function(arg0) {
            const ret = arg0.done;
            return ret;
        },
        __wbg_entries_015dc610cd81ede0: function(arg0) {
            const ret = Object.entries(arg0);
            return ret;
        },
        __wbg_fromCodePoint_a952b701bbb83fa4: function() { return handleError(function (arg0) {
            const ret = String.fromCodePoint(arg0 >>> 0);
            return ret;
        }, arguments); },
        __wbg_gain_93a64bac2c882dd8: function(arg0) {
            const ret = arg0.gain;
            return ret;
        },
        __wbg_get_507a50627bffa49b: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return ret;
        },
        __wbg_get_78f252d074a84d0b: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.get(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_get_c7eb1f358a7654df: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.get(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_get_unchecked_6e0ad6d2a41b06f6: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return ret;
        },
        __wbg_get_with_ref_key_6412cf3094599694: function(arg0, arg1) {
            const ret = arg0[arg1];
            return ret;
        },
        __wbg_instanceof_ArrayBuffer_4480b9e0068a8adb: function(arg0) {
            let result;
            try {
                result = arg0 instanceof ArrayBuffer;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Map_e5b5e3db98422fcc: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Map;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Uint8Array_309b927aaf7a3fc7: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Uint8Array;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Window_05ba1ee4f6781663: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Window;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_isArray_0677c962b281d01a: function(arg0) {
            const ret = Array.isArray(arg0);
            return ret;
        },
        __wbg_isSafeInteger_04f36e4056f1b851: function(arg0) {
            const ret = Number.isSafeInteger(arg0);
            return ret;
        },
        __wbg_iterator_6f722e4a93058b71: function() {
            const ret = Symbol.iterator;
            return ret;
        },
        __wbg_length_1f0964f4a5e2c6d8: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_length_370319915dc99107: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_new_32b398fb48b6d94a: function() {
            const ret = new Array();
            return ret;
        },
        __wbg_new_36c0fbabbe36db1c: function() { return handleError(function () {
            const ret = new lAudioContext();
            return ret;
        }, arguments); },
        __wbg_new_7796ffc7ed656783: function() {
            const ret = new Map();
            return ret;
        },
        __wbg_new_cd45aabdf6073e84: function(arg0) {
            const ret = new Uint8Array(arg0);
            return ret;
        },
        __wbg_new_da52cf8fe3429cb2: function() {
            const ret = new Object();
            return ret;
        },
        __wbg_new_from_slice_77cdfb7977362f3c: function(arg0, arg1) {
            const ret = new Uint8Array(getArrayU8FromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_new_from_slice_ddf8b82c4d6af38e: function(arg0, arg1) {
            const ret = new Float32Array(getArrayF32FromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_new_typed_1824d93f294193e5: function(arg0, arg1) {
            try {
                var state0 = {a: arg0, b: arg1};
                var cb0 = (arg0, arg1) => {
                    const a = state0.a;
                    state0.a = 0;
                    try {
                        return wasm_bindgen__convert__closures_____invoke__h03fa25002c8f256b(a, state0.b, arg0, arg1);
                    } finally {
                        state0.a = a;
                    }
                };
                const ret = new Promise(cb0);
                return ret;
            } finally {
                state0.a = 0;
            }
        },
        __wbg_new_with_options_f6a4185f6ccb78cc: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = new AudioWorkletNode(arg0, getStringFromWasm0(arg1, arg2), arg3);
            return ret;
        }, arguments); },
        __wbg_next_6dbf2c0ac8cde20f: function(arg0) {
            const ret = arg0.next;
            return ret;
        },
        __wbg_next_71f2aa1cb3d1e37e: function() { return handleError(function (arg0) {
            const ret = arg0.next();
            return ret;
        }, arguments); },
        __wbg_of_85f52f8b6491a7ca: function(arg0) {
            const ret = Array.of(arg0);
            return ret;
        },
        __wbg_parse_1c0d8a8656d7e016: function() { return handleError(function (arg0, arg1) {
            const ret = JSON.parse(getStringFromWasm0(arg0, arg1));
            return ret;
        }, arguments); },
        __wbg_port_19f04f77a6946cc0: function() { return handleError(function (arg0) {
            const ret = arg0.port;
            return ret;
        }, arguments); },
        __wbg_postMessage_b80f20949a4b4f55: function() { return handleError(function (arg0, arg1) {
            arg0.postMessage(arg1);
        }, arguments); },
        __wbg_prototypesetcall_4770620bbe4688a0: function(arg0, arg1, arg2) {
            Uint8Array.prototype.set.call(getArrayU8FromWasm0(arg0, arg1), arg2);
        },
        __wbg_push_d2ae3af0c1217ae6: function(arg0, arg1) {
            const ret = arg0.push(arg1);
            return ret;
        },
        __wbg_queueMicrotask_0ab5b2d2393e99b9: function(arg0) {
            const ret = arg0.queueMicrotask;
            return ret;
        },
        __wbg_queueMicrotask_6a09b7bc46549209: function(arg0) {
            queueMicrotask(arg0);
        },
        __wbg_resolve_2191a4dfe481c25b: function(arg0) {
            const ret = Promise.resolve(arg0);
            return ret;
        },
        __wbg_resume_e06014a2f25921a6: function() { return handleError(function (arg0) {
            const ret = arg0.resume();
            return ret;
        }, arguments); },
        __wbg_setTimeout_cfa2cf195c3738db: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.setTimeout(arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_set_575dd786d51585f8: function(arg0, arg1, arg2) {
            const ret = arg0.set(arg1, arg2);
            return ret;
        },
        __wbg_set_6be42768c690e380: function(arg0, arg1, arg2) {
            arg0[arg1] = arg2;
        },
        __wbg_set_8535240470bf2500: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = Reflect.set(arg0, arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_set_8a16b38e4805b298: function(arg0, arg1, arg2) {
            arg0[arg1 >>> 0] = arg2;
        },
        __wbg_set_buffer_abc0373627cc3560: function(arg0, arg1) {
            arg0.buffer = arg1;
        },
        __wbg_set_number_of_inputs_865fa47e916afaf4: function(arg0, arg1) {
            arg0.numberOfInputs = arg1 >>> 0;
        },
        __wbg_set_number_of_outputs_e39c7ef579a2b1f3: function(arg0, arg1) {
            arg0.numberOfOutputs = arg1 >>> 0;
        },
        __wbg_set_onended_546856f8516d6af4: function(arg0, arg1) {
            arg0.onended = arg1;
        },
        __wbg_set_onmessage_d511b70365304094: function(arg0, arg1) {
            arg0.onmessage = arg1;
        },
        __wbg_set_onprocessorerror_d7addf9f05732f83: function(arg0, arg1) {
            arg0.onprocessorerror = arg1;
        },
        __wbg_set_onstatechange_5f2d1add58307f1c: function(arg0, arg1) {
            arg0.onstatechange = arg1;
        },
        __wbg_set_output_channel_count_ac1658037522c397: function(arg0, arg1) {
            arg0.outputChannelCount = arg1;
        },
        __wbg_set_processor_options_2d8445eff840a6c3: function(arg0, arg1) {
            arg0.processorOptions = arg1;
        },
        __wbg_set_value_d75e40e4e1da7c7f: function(arg0, arg1) {
            arg0.value = arg1;
        },
        __wbg_start_ab6726fe84171a80: function() { return handleError(function (arg0) {
            arg0.start();
        }, arguments); },
        __wbg_state_3aa84b9ee90d1274: function(arg0) {
            const ret = arg0.state;
            return (__wbindgen_enum_AudioContextState.indexOf(ret) + 1 || 4) - 1;
        },
        __wbg_static_accessor_GLOBAL_4ef717fb391d88b7: function() {
            const ret = typeof global === 'undefined' ? null : global;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_GLOBAL_THIS_8d1badc68b5a74f4: function() {
            const ret = typeof globalThis === 'undefined' ? null : globalThis;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_SELF_146583524fe1469b: function() {
            const ret = typeof self === 'undefined' ? null : self;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_WINDOW_f2829a2234d7819e: function() {
            const ret = typeof window === 'undefined' ? null : window;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_stop_fda43ebb0ec951c9: function() { return handleError(function (arg0) {
            arg0.stop();
        }, arguments); },
        __wbg_then_16d107c451e9905d: function(arg0, arg1, arg2) {
            const ret = arg0.then(arg1, arg2);
            return ret;
        },
        __wbg_then_6ec10ae38b3e92f7: function(arg0, arg1) {
            const ret = arg0.then(arg1);
            return ret;
        },
        __wbg_value_a5d5488a9589444a: function(arg0) {
            const ret = arg0.value;
            return ret;
        },
        __wbindgen_cast_0000000000000001: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [Externref], shim_idx: 1046, ret: Result(Unit), inner_ret: Some(Result(Unit)) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen__convert__closures_____invoke__hd999f337e60bc82b);
            return ret;
        },
        __wbindgen_cast_0000000000000002: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("Event")], shim_idx: 211, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen__convert__closures_____invoke__h37d720ec560e4efb);
            return ret;
        },
        __wbindgen_cast_0000000000000003: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("MessageEvent")], shim_idx: 211, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen__convert__closures_____invoke__h37d720ec560e4efb_2);
            return ret;
        },
        __wbindgen_cast_0000000000000004: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [], shim_idx: 214, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen__convert__closures_____invoke__h06673932934f632a);
            return ret;
        },
        __wbindgen_cast_0000000000000005: function(arg0) {
            // Cast intrinsic for `F64 -> Externref`.
            const ret = arg0;
            return ret;
        },
        __wbindgen_cast_0000000000000006: function(arg0) {
            // Cast intrinsic for `I64 -> Externref`.
            const ret = arg0;
            return ret;
        },
        __wbindgen_cast_0000000000000007: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_cast_0000000000000008: function(arg0) {
            // Cast intrinsic for `U64 -> Externref`.
            const ret = BigInt.asUintN(64, arg0);
            return ret;
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
    };
    return {
        __proto__: null,
        "./puzzle_wasm_bg.js": import0,
    };
}

const lAudioContext = (typeof AudioContext !== 'undefined' ? AudioContext : (typeof webkitAudioContext !== 'undefined' ? webkitAudioContext : undefined));
function wasm_bindgen__convert__closures_____invoke__h06673932934f632a(arg0, arg1) {
    wasm.wasm_bindgen__convert__closures_____invoke__h06673932934f632a(arg0, arg1);
}

function wasm_bindgen__convert__closures_____invoke__h37d720ec560e4efb(arg0, arg1, arg2) {
    wasm.wasm_bindgen__convert__closures_____invoke__h37d720ec560e4efb(arg0, arg1, arg2);
}

function wasm_bindgen__convert__closures_____invoke__h37d720ec560e4efb_2(arg0, arg1, arg2) {
    wasm.wasm_bindgen__convert__closures_____invoke__h37d720ec560e4efb_2(arg0, arg1, arg2);
}

function wasm_bindgen__convert__closures_____invoke__hd999f337e60bc82b(arg0, arg1, arg2) {
    const ret = wasm.wasm_bindgen__convert__closures_____invoke__hd999f337e60bc82b(arg0, arg1, arg2);
    if (ret[1]) {
        throw takeFromExternrefTable0(ret[0]);
    }
}

function wasm_bindgen__convert__closures_____invoke__h03fa25002c8f256b(arg0, arg1, arg2, arg3) {
    wasm.wasm_bindgen__convert__closures_____invoke__h03fa25002c8f256b(arg0, arg1, arg2, arg3);
}


const __wbindgen_enum_AudioContextState = ["suspended", "running", "closed"];
const WasmEditorAudioFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmeditoraudio_free(ptr, 1));
const WasmEditorLevelApplicationFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmeditorlevelapplication_free(ptr, 1));
const WasmEditorWorkspaceSessionFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmeditorworkspacesession_free(ptr, 1));
const WasmPreparedWebExportFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmpreparedwebexport_free(ptr, 1));
const WasmSolverServiceFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmsolverservice_free(ptr, 1));
const WasmWorkspaceSessionFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmworkspacesession_free(ptr, 1));

function addToExternrefTable0(obj) {
    const idx = wasm.__externref_table_alloc();
    wasm.__wbindgen_externrefs.set(idx, obj);
    return idx;
}

const CLOSURE_DTORS = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(state => wasm.__wbindgen_destroy_closure(state.a, state.b));

function debugString(val) {
    // primitive types
    const type = typeof val;
    if (type == 'number' || type == 'boolean' || val == null) {
        return  `${val}`;
    }
    if (type == 'string') {
        return `"${val}"`;
    }
    if (type == 'symbol') {
        const description = val.description;
        if (description == null) {
            return 'Symbol';
        } else {
            return `Symbol(${description})`;
        }
    }
    if (type == 'function') {
        const name = val.name;
        if (typeof name == 'string' && name.length > 0) {
            return `Function(${name})`;
        } else {
            return 'Function';
        }
    }
    // objects
    if (Array.isArray(val)) {
        const length = val.length;
        let debug = '[';
        if (length > 0) {
            debug += debugString(val[0]);
        }
        for(let i = 1; i < length; i++) {
            debug += ', ' + debugString(val[i]);
        }
        debug += ']';
        return debug;
    }
    // Test for built-in
    const builtInMatches = /\[object ([^\]]+)\]/.exec(toString.call(val));
    let className;
    if (builtInMatches && builtInMatches.length > 1) {
        className = builtInMatches[1];
    } else {
        // Failed to match the standard '[object ClassName]'
        return toString.call(val);
    }
    if (className == 'Object') {
        // we're a user defined class or Object
        // JSON.stringify avoids problems with cycles, and is generally much
        // easier than looping through ownProperties of `val`.
        try {
            return 'Object(' + JSON.stringify(val) + ')';
        } catch (_) {
            return 'Object';
        }
    }
    // errors
    if (val instanceof Error) {
        return `${val.name}: ${val.message}\n${val.stack}`;
    }
    // TODO we could test for more things here, like `Set`s and `Map`s.
    return className;
}

function getArrayF32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getFloat32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayU32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

let cachedDataViewMemory0 = null;
function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

let cachedFloat32ArrayMemory0 = null;
function getFloat32ArrayMemory0() {
    if (cachedFloat32ArrayMemory0 === null || cachedFloat32ArrayMemory0.byteLength === 0) {
        cachedFloat32ArrayMemory0 = new Float32Array(wasm.memory.buffer);
    }
    return cachedFloat32ArrayMemory0;
}

function getStringFromWasm0(ptr, len) {
    return decodeText(ptr >>> 0, len);
}

let cachedUint32ArrayMemory0 = null;
function getUint32ArrayMemory0() {
    if (cachedUint32ArrayMemory0 === null || cachedUint32ArrayMemory0.byteLength === 0) {
        cachedUint32ArrayMemory0 = new Uint32Array(wasm.memory.buffer);
    }
    return cachedUint32ArrayMemory0;
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        const idx = addToExternrefTable0(e);
        wasm.__wbindgen_exn_store(idx);
    }
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

function makeMutClosure(arg0, arg1, f) {
    const state = { a: arg0, b: arg1, cnt: 1 };
    const real = (...args) => {

        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return f(a, state.b, ...args);
        } finally {
            state.a = a;
            real._wbg_cb_unref();
        }
    };
    real._wbg_cb_unref = () => {
        if (--state.cnt === 0) {
            wasm.__wbindgen_destroy_closure(state.a, state.b);
            state.a = 0;
            CLOSURE_DTORS.unregister(state);
        }
    };
    CLOSURE_DTORS.register(real, state, state);
    return real;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_externrefs.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasmInstance, wasm;
function __wbg_finalize_init(instance, module) {
    wasmInstance = instance;
    wasm = instance.exports;
    wasmModule = module;
    cachedDataViewMemory0 = null;
    cachedFloat32ArrayMemory0 = null;
    cachedUint32ArrayMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = module.ok && expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('puzzle_wasm_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
