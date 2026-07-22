use std::cell::RefCell;

use serde::{Serialize, de::DeserializeOwned};
use wasm_bindgen::prelude::*;

type SourceAnalysisRevision = u32;

#[wasm_bindgen(typescript_custom_section)]
const WORKSPACE_SOURCE_TYPES: &str = r#"
export interface WorkspaceSourceDocument {
    readonly path: string;
    readonly source: string;
}

export interface WorkspacePresentationManifest {
    readonly themeName: string | null;
    readonly cssPaths: string[];
    readonly scriptPaths: string[];
    readonly filePaths: string[];
    readonly visualImagePaths: string[];
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ReadonlyArray<WorkspaceSourceDocument>")]
    pub type WorkspaceSourceDocuments;

    #[wasm_bindgen(typescript_type = "WorkspacePresentationManifest")]
    pub type WorkspacePresentationManifestJs;
}

thread_local! {
    static SOURCE_ANALYSES: RefCell<SourceAnalysisStore> =
        RefCell::new(SourceAnalysisStore::default());
}

#[wasm_bindgen]
pub struct WasmSolverService {
    inner: puzzle_solver_runtime::SolverService,
}

#[wasm_bindgen]
impl WasmSolverService {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: puzzle_solver_runtime::SolverService::new(),
        }
    }

    pub fn prepare_workspace(
        &mut self,
        entry_path: &str,
        documents: WorkspaceSourceDocuments,
        now_ms: f64,
    ) -> Result<JsValue, JsValue> {
        let documents = decode_js_value::<Vec<puzzle_lang::WorkspaceSourceDocument>>(
            documents.into(),
            "solver workspace documents",
        )?;
        let prepared = self
            .inner
            .prepare_workspace(entry_path, documents, solver_now_ms(now_ms)?)
            .map_err(|error| JsValue::from_str(&error))?;
        encode_js_value(&prepared, "prepared solver artifact")
    }

    pub fn prepare_source(
        &mut self,
        source: &str,
        puzzle_path: &str,
        now_ms: f64,
    ) -> Result<JsValue, JsValue> {
        if puzzle_path.trim().is_empty() {
            return Err(JsValue::from_str(
                "solver source preparation requires an explicit puzzle path",
            ));
        }
        let prepared = self
            .inner
            .prepare_workspace(
                puzzle_path,
                vec![puzzle_lang::WorkspaceSourceDocument {
                    path: puzzle_path.to_string(),
                    source: source.to_string(),
                }],
                solver_now_ms(now_ms)?,
            )
            .map_err(|error| JsValue::from_str(&error))?;
        encode_js_value(&prepared, "prepared solver artifact")
    }

    pub fn pin_artifact(
        &mut self,
        artifact_id: Option<String>,
        now_ms: f64,
    ) -> Result<(), JsValue> {
        self.inner
            .pin_artifact(artifact_id.as_deref(), solver_now_ms(now_ms)?)
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn start(
        &mut self,
        artifact_id: &str,
        request: JsValue,
        now_ms: f64,
    ) -> Result<u32, JsValue> {
        let request = decode_js_value::<puzzle_runtime_contract::SolverSearchRequest>(
            request,
            "solver search request",
        )?;
        self.inner
            .start(artifact_id, request, solver_now_ms(now_ms)?)
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn advance(
        &mut self,
        search_id: u32,
        max_expanded_nodes: u32,
        now_ms: f64,
    ) -> Result<JsValue, JsValue> {
        let response = self
            .inner
            .advance_nodes(
                search_id,
                max_expanded_nodes as usize,
                solver_now_ms(now_ms)?,
            )
            .map_err(|error| JsValue::from_str(&error))?;
        encode_js_value(&response, "solver advance response")
    }

    pub fn cancel(&mut self, search_id: u32, now_ms: f64) -> Result<(), JsValue> {
        self.inner
            .cancel(search_id, solver_now_ms(now_ms)?)
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn materialize_state(
        &mut self,
        artifact_id: &str,
        level_index: usize,
        state: JsValue,
        materialize_level_start: bool,
        now_ms: f64,
    ) -> Result<JsValue, JsValue> {
        let state =
            decode_js_value::<puzzle_runtime_contract::SolverStateSnapshot>(state, "solver state")?;
        let state = self
            .inner
            .materialize_state(
                artifact_id,
                level_index,
                state,
                materialize_level_start,
                solver_now_ms(now_ms)?,
            )
            .map_err(|error| JsValue::from_str(&error))?;
        encode_js_value(&state, "materialized solver state")
    }
}

impl Default for WasmSolverService {
    fn default() -> Self {
        Self::new()
    }
}

fn solver_now_ms(value: f64) -> Result<u64, JsValue> {
    if !value.is_finite() || value < 0.0 || value > u64::MAX as f64 {
        return Err(JsValue::from_str("solver timestamp is invalid"));
    }
    Ok(value as u64)
}

fn decode_js_value<T: DeserializeOwned>(value: JsValue, label: &str) -> Result<T, JsValue> {
    serde_wasm_bindgen::from_value(value)
        .map_err(|error| JsValue::from_str(&format!("{label} is invalid: {error}")))
}

fn encode_js_value<T: Serialize>(value: &T, label: &str) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(value)
        .map_err(|error| JsValue::from_str(&format!("{label} could not be encoded: {error}")))
}

#[derive(Default)]
struct SourceAnalysisStore {
    next_revision: SourceAnalysisRevision,
    active: Option<ActiveSourceAnalysis>,
}

struct ActiveSourceAnalysis {
    revision: SourceAnalysisRevision,
    analysis: puzzle_lang::SourceAnalysis,
}

impl SourceAnalysisStore {
    fn allocate_revision(&mut self) -> SourceAnalysisRevision {
        let revision = self.next_revision.max(1);
        self.next_revision = revision
            .checked_add(1)
            .expect("source analysis revision counter exhausted");
        revision
    }

    fn activate(
        &mut self,
        source: &str,
        source_profile: Option<puzzle_lang::PuzzleSourceProfile>,
    ) -> SourceAnalysisRevision {
        if let Some(active) = &self.active {
            if active.analysis.source() == source
                && active.analysis.source_profile() == source_profile
            {
                return active.revision;
            }
        }
        let revision = self.allocate_revision();
        self.active = Some(ActiveSourceAnalysis {
            revision,
            analysis: puzzle_lang::SourceAnalysis::new_for_profile(source, source_profile),
        });
        revision
    }

    fn apply_edit(
        &mut self,
        revision: SourceAnalysisRevision,
        start_utf16: usize,
        end_utf16: usize,
        insert: &str,
    ) -> Result<
        (
            SourceAnalysisRevision,
            puzzle_lang::SourceAnalysisEditResult,
        ),
        String,
    > {
        let Some(active) = &self.active else {
            return Err(
                "source analysis is not active; activate the source before editing it".to_string(),
            );
        };
        if active.revision != revision {
            return Err(format!(
                "source analysis revision `{revision}` is stale; active revision is `{}`",
                active.revision
            ));
        }
        let start = utf8_offset_from_utf16(active.analysis.source(), start_utf16);
        let end = utf8_offset_from_utf16(active.analysis.source(), end_utf16);
        let next_revision = self.allocate_revision();
        let active = self.active.as_mut().expect("active analysis checked above");
        let result = active
            .analysis
            .apply_edit(puzzle_lang::SourceAnalysisEdit { start, end }, insert)?;
        active.revision = next_revision;
        Ok((next_revision, result))
    }

    fn with_analysis<T>(
        &self,
        revision: SourceAnalysisRevision,
        f: impl FnOnce(&puzzle_lang::SourceAnalysis) -> T,
    ) -> Result<T, String> {
        let Some(active) = &self.active else {
            return Err(
                "source analysis is not active; activate the source before querying it".to_string(),
            );
        };
        if active.revision != revision {
            return Err(format!(
                "source analysis revision `{revision}` is stale; active revision is `{}`",
                active.revision
            ));
        }
        Ok(f(&active.analysis))
    }
}

fn source_analysis_error_js_value(message: String) -> JsValue {
    JsValue::from_str(&message)
}

fn with_source_analysis<T>(
    revision: SourceAnalysisRevision,
    f: impl FnOnce(&puzzle_lang::SourceAnalysis) -> T,
) -> Result<T, String> {
    SOURCE_ANALYSES.with(|store| store.borrow().with_analysis(revision, f))
}

fn utf8_offset_from_utf16(source: &str, utf16_offset: usize) -> usize {
    let mut consumed = 0;
    for (byte_offset, ch) in source.char_indices() {
        if consumed >= utf16_offset {
            return byte_offset;
        }
        let next = consumed + ch.len_utf16();
        if next > utf16_offset {
            return byte_offset;
        }
        consumed = next;
    }
    source.len()
}

fn utf16_offset_from_utf8(source: &str, byte_offset: usize) -> usize {
    source
        .char_indices()
        .take_while(|(index, _)| *index < byte_offset.min(source.len()))
        .map(|(_, ch)| ch.len_utf16())
        .sum()
}

fn source_target_with_utf16_offsets(
    source: &str,
    mut target: puzzle_lang::SourceTarget,
) -> puzzle_lang::SourceTarget {
    target.start = utf16_offset_from_utf8(source, target.start);
    target.end = utf16_offset_from_utf8(source, target.end);
    target.body_start = target
        .body_start
        .map(|offset| utf16_offset_from_utf8(source, offset));
    target.body_end = target
        .body_end
        .map(|offset| utf16_offset_from_utf8(source, offset));
    target
}

#[wasm_bindgen]
pub fn activate_source_analysis(source: &str) -> SourceAnalysisRevision {
    SOURCE_ANALYSES.with(|store| store.borrow_mut().activate(source, None))
}

#[wasm_bindgen]
pub fn activate_source_analysis_with_profile(
    source: &str,
    source_profile: &str,
) -> Result<SourceAnalysisRevision, JsValue> {
    let profile = match source_profile {
        "puzzle2d" => puzzle_lang::PuzzleSourceProfile::Puzzle2d,
        "puzzle3d" => puzzle_lang::PuzzleSourceProfile::Puzzle3d,
        _ => {
            return Err(JsValue::from_str(
                "source analysis profile must be `puzzle2d` or `puzzle3d`",
            ));
        }
    };
    Ok(SOURCE_ANALYSES.with(|store| store.borrow_mut().activate(source, Some(profile))))
}

#[wasm_bindgen]
pub fn apply_source_analysis_edit(
    revision: SourceAnalysisRevision,
    start_utf16: usize,
    end_utf16: usize,
    insert: &str,
) -> Result<String, JsValue> {
    let (revision, result) = SOURCE_ANALYSES
        .with(|store| {
            store
                .borrow_mut()
                .apply_edit(revision, start_utf16, end_utf16, insert)
        })
        .map_err(source_analysis_error_js_value)?;
    Ok(format!(
        "{{\"revision\":{revision},\"rescannedLines\":{},\"totalLines\":{},\"parserCatalogReused\":{}}}",
        result.rescanned_lines,
        result.total_lines,
        if result.parser_catalog_reused {
            "true"
        } else {
            "false"
        }
    ))
}

#[wasm_bindgen]
pub fn active_source_analysis_json(revision: SourceAnalysisRevision) -> Result<String, JsValue> {
    with_source_analysis(revision, puzzle_lang::SourceAnalysis::analysis_json)
        .map_err(source_analysis_error_js_value)
}

#[wasm_bindgen]
pub fn active_source_analysis_highlight_range_json(
    revision: SourceAnalysisRevision,
    range_start_utf16: usize,
    range_end_utf16: usize,
    include_outline: bool,
) -> Result<String, JsValue> {
    with_source_analysis(revision, |analysis| {
        let source = analysis.source();
        let range_start = utf8_offset_from_utf16(source, range_start_utf16);
        let range_end = utf8_offset_from_utf16(source, range_end_utf16);
        analysis.highlight_range_json(range_start, range_end, include_outline)
    })
    .map_err(source_analysis_error_js_value)
}

#[wasm_bindgen]
pub fn active_source_analysis_outline_json(
    revision: SourceAnalysisRevision,
) -> Result<String, JsValue> {
    with_source_analysis(revision, puzzle_lang::SourceAnalysis::outline_json)
        .map_err(source_analysis_error_js_value)
}

#[wasm_bindgen]
pub fn active_source_analysis_suggest_source_completions(
    revision: SourceAnalysisRevision,
    cursor_utf16_offset: usize,
) -> Result<String, JsValue> {
    with_source_analysis(revision, |analysis| {
        let source = analysis.source();
        let cursor_offset = utf8_offset_from_utf16(source, cursor_utf16_offset);
        let mut completions = analysis.completion_list(cursor_offset);
        completions.replace_start = utf16_offset_from_utf8(source, completions.replace_start);
        completions.replace_end = utf16_offset_from_utf8(source, completions.replace_end);
        puzzle_lang::completion_list_json(&completions)
    })
    .map_err(source_analysis_error_js_value)
}

#[wasm_bindgen]
pub fn active_source_analysis_resolve_source_target(
    revision: SourceAnalysisRevision,
    cursor_utf16_offset: usize,
) -> Result<String, JsValue> {
    with_source_analysis(revision, |analysis| {
        let source = analysis.source();
        let cursor_offset = utf8_offset_from_utf16(source, cursor_utf16_offset);
        let target = analysis
            .resolve_target(cursor_offset)
            .map(|target| source_target_with_utf16_offsets(source, target));
        puzzle_lang::source_target_json(target.as_ref())
    })
    .map_err(source_analysis_error_js_value)
}

#[wasm_bindgen]
pub fn active_source_analysis_mutate_visual(
    revision: SourceAnalysisRevision,
    request_json: &str,
) -> Result<String, JsValue> {
    with_source_analysis(revision, |analysis| {
        puzzle_lang::mutate_visual_source(analysis.source(), request_json).map(|result| {
            let start = utf16_offset_from_utf8(&result.source, result.start);
            let end = utf16_offset_from_utf8(&result.source, result.end);
            serde_json::json!({
                "source": result.source,
                "start": start,
                "end": end,
                "name": result.name,
            })
            .to_string()
        })
    })
    .map_err(source_analysis_error_js_value)?
    .map_err(source_analysis_error_js_value)
}

#[wasm_bindgen]
pub fn active_source_analysis_import_at_json(
    revision: SourceAnalysisRevision,
    document_path: &str,
    cursor_utf16_offset: usize,
) -> Result<String, JsValue> {
    with_source_analysis(revision, |analysis| {
        let source = analysis.source();
        let cursor = utf8_offset_from_utf16(source, cursor_utf16_offset);
        let reference = analysis
            .import_reference_at(document_path, cursor)
            .map(|reference| {
                serde_json::json!({
                    "range": {
                        "start": utf16_offset_from_utf8(source, reference.range.start),
                        "end": utf16_offset_from_utf8(source, reference.range.end),
                    },
                    "pathRange": {
                        "start": utf16_offset_from_utf8(source, reference.path_range.start),
                        "end": utf16_offset_from_utf8(source, reference.path_range.end),
                    },
                    "rawPath": reference.raw_path,
                    "resolvedPath": reference.resolved_path,
                })
            });
        serde_json::json!({ "version": 1, "reference": reference }).to_string()
    })
    .map_err(source_analysis_error_js_value)
}

#[wasm_bindgen]
pub fn active_source_analysis_entries_json(
    revision: SourceAnalysisRevision,
) -> Result<String, JsValue> {
    with_source_analysis(revision, puzzle_lang::SourceAnalysis::entries_json)
        .map_err(source_analysis_error_js_value)
}

/// Returns level-editor metadata for the active source snapshot. Board cells and
/// visual payloads deliberately travel through their own on-demand exports.
#[wasm_bindgen]
pub fn active_source_analysis_level_editor_manifest_json(
    revision: SourceAnalysisRevision,
) -> Result<String, JsValue> {
    with_source_analysis(
        revision,
        puzzle_lang::SourceAnalysis::level_editor_manifest_json,
    )
    .map_err(source_analysis_error_js_value)?
    .map_err(source_analysis_error_js_value)
}

/// Returns a typed object-ID buffer for one integrated level state. Pass `-1` for
/// the composite state, otherwise pass the authored ASCII layer index.
#[wasm_bindgen]
pub fn active_source_analysis_level_editor_level_slots(
    revision: SourceAnalysisRevision,
    level_index: usize,
    authored_layer: i32,
) -> Result<Vec<u32>, JsValue> {
    let authored_layer = match authored_layer {
        -1 => None,
        value if value >= 0 => Some(value as usize),
        value => {
            return Err(source_analysis_error_js_value(format!(
                "level editor authored layer must be -1 or non-negative, got {value}"
            )));
        }
    };
    with_source_analysis(revision, |analysis| {
        analysis.level_editor_level_slots(level_index, authored_layer)
    })
    .map_err(source_analysis_error_js_value)?
    .map_err(source_analysis_error_js_value)
}

/// Returns one renderer-ready visual payload by canonical object ID.
#[wasm_bindgen]
pub fn active_source_analysis_level_editor_visual_json(
    revision: SourceAnalysisRevision,
    object_id: u16,
) -> Result<String, JsValue> {
    with_source_analysis(revision, |analysis| {
        analysis.level_editor_visual_payload_json(object_id)
    })
    .map_err(source_analysis_error_js_value)?
    .map_err(source_analysis_error_js_value)
}

#[wasm_bindgen]
pub fn compile_preview(
    source: &str,
    puzzle_path: &str,
    game_css: &str,
    game_visuals_js: &str,
) -> Result<String, JsValue> {
    let path = if puzzle_path.trim().is_empty() {
        "game.puzzle"
    } else {
        puzzle_path
    };
    html_play::export_editor_preview_html_from_source(source, path, game_css, game_visuals_js)
        .map_err(|error| diagnostic_report_js_value(&error))
}

fn expand_workspace_entry(
    entry_path: &str,
    documents: WorkspaceSourceDocuments,
) -> Result<String, JsValue> {
    let documents = decode_js_value::<Vec<puzzle_lang::WorkspaceSourceDocument>>(
        documents.into(),
        "workspace source documents",
    )?;
    puzzle_lang::expand_game_imports_from_documents(entry_path, &documents)
        .map_err(|error| diagnostic_report_js_value(&error))
}

#[wasm_bindgen]
pub fn workspace_presentation_manifest(
    entry_path: &str,
    documents: WorkspaceSourceDocuments,
) -> Result<WorkspacePresentationManifestJs, JsValue> {
    let documents = decode_js_value::<Vec<puzzle_lang::WorkspaceSourceDocument>>(
        documents.into(),
        "workspace source documents",
    )?;
    let manifest = puzzle_lang::workspace_presentation_manifest(entry_path, &documents)
        .map_err(|error| diagnostic_report_js_value(&error))?;
    encode_js_value(&manifest, "workspace presentation manifest")
        .map(WorkspacePresentationManifestJs::from)
}

#[wasm_bindgen]
pub fn compile_workspace_preview(
    entry_path: &str,
    documents: WorkspaceSourceDocuments,
    game_css: &str,
    game_visuals_js: &str,
) -> Result<String, JsValue> {
    let documents = decode_js_value::<Vec<puzzle_lang::WorkspaceSourceDocument>>(
        documents.into(),
        "workspace source documents",
    )?;
    compile_workspace_preview_from_documents(entry_path, &documents, game_css, game_visuals_js)
        .map_err(|error| diagnostic_report_js_value(&error))
}

fn compile_workspace_preview_from_documents(
    entry_path: &str,
    documents: &[puzzle_lang::WorkspaceSourceDocument],
    game_css: &str,
    game_visuals_js: &str,
) -> Result<String, puzzle_lang::DiagnosticReport> {
    let expanded =
        puzzle_lang::expand_game_imports_from_documents_with_origins(entry_path, documents)?;
    html_play::export_editor_preview_html_from_source(
        &expanded.source,
        entry_path,
        game_css,
        game_visuals_js,
    )
    .map_err(|error| expanded.remap_diagnostic_report(error))
}

#[wasm_bindgen]
pub fn export_html(
    source: &str,
    puzzle_path: &str,
    game_css: &str,
    game_visuals_js: &str,
    player_runtime_module_js: &str,
    player_runtime_wasm_base64: &str,
) -> Result<String, JsValue> {
    let path = if puzzle_path.trim().is_empty() {
        "game.puzzle"
    } else {
        puzzle_path
    };
    html_play::export_html_from_source_with_embedded_wasm(
        source,
        path,
        game_css,
        game_visuals_js,
        player_runtime_module_js,
        player_runtime_wasm_base64,
    )
    .map_err(|error| diagnostic_report_js_value(&error))
}

#[wasm_bindgen]
pub fn export_workspace_html(
    entry_path: &str,
    documents: WorkspaceSourceDocuments,
    game_css: &str,
    game_visuals_js: &str,
    player_runtime_module_js: &str,
    player_runtime_wasm_base64: &str,
) -> Result<String, JsValue> {
    let source = expand_workspace_entry(entry_path, documents)?;
    html_play::export_html_from_source_with_embedded_wasm(
        &source,
        entry_path,
        game_css,
        game_visuals_js,
        player_runtime_module_js,
        player_runtime_wasm_base64,
    )
    .map_err(|error| diagnostic_report_js_value(&error))
}

#[wasm_bindgen]
pub fn generate_visuals_js(source: &str, base_visuals_js: &str) -> Result<String, JsValue> {
    html_play::export_visuals_js_from_source(source, base_visuals_js)
        .map_err(|error| JsValue::from_str(&error))
}

#[wasm_bindgen]
pub fn translate_puzzlescript(source: &str) -> Result<String, JsValue> {
    puzzle_lang::translate_puzzlescript_to_canonical(source)
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

fn diagnostic_report_js_value(report: &puzzle_lang::DiagnosticReport) -> JsValue {
    let payload = js_sys::Object::new();
    let diagnostics = js_sys::Array::new();
    for diagnostic in report.diagnostics() {
        diagnostics.push(&diagnostic_js_value(diagnostic));
    }
    let _ = js_sys::Reflect::set(
        &payload,
        &JsValue::from_str("diagnostics"),
        diagnostics.as_ref(),
    );
    payload.into()
}

#[cfg(test)]
fn diagnostic_report_json(report: &puzzle_lang::DiagnosticReport) -> String {
    let mut body = String::new();
    push_diagnostics_json(&mut body, report.diagnostics());
    body
}

#[cfg(test)]
fn push_diagnostics_json(out: &mut String, diagnostics: &[puzzle_lang::Diagnostic]) {
    out.push_str("{\"diagnostics\":[");
    for (index, diagnostic) in diagnostics.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_diagnostic_json(out, diagnostic);
    }
    out.push_str("]}");
}

#[cfg(test)]
fn push_diagnostic_json(out: &mut String, diagnostic: &puzzle_lang::Diagnostic) {
    let span = diagnostic.primary_span.as_ref();
    out.push('{');
    push_json_pair(out, "severity", diagnostic.severity.as_str());
    out.push(',');
    push_json_pair(out, "code", diagnostic.code);
    out.push(',');
    push_json_pair(
        out,
        "file",
        span.and_then(|span| span.file.as_deref()).unwrap_or(""),
    );
    out.push(',');
    push_json_option_number(out, "line", span.and_then(|span| span.line));
    out.push(',');
    push_json_option_number(out, "column", span.and_then(|span| span.column));
    out.push(',');
    push_json_option_string(
        out,
        "sourceLine",
        span.and_then(|span| span.source_line.as_deref()),
    );
    out.push(',');
    push_json_pair(out, "message", &diagnostic.message);
    out.push('}');
}

#[cfg(test)]
fn push_json_pair(out: &mut String, key: &str, value: &str) {
    push_json_string(out, key);
    out.push(':');
    push_json_string(out, value);
}

#[cfg(test)]
fn push_json_option_number(out: &mut String, key: &str, value: Option<usize>) {
    push_json_string(out, key);
    out.push(':');
    match value {
        Some(value) => out.push_str(&value.to_string()),
        None => out.push_str("null"),
    }
}

#[cfg(test)]
fn push_json_option_string(out: &mut String, key: &str, value: Option<&str>) {
    push_json_string(out, key);
    out.push(':');
    match value {
        Some(value) => push_json_string(out, value),
        None => out.push_str("null"),
    }
}

#[cfg(test)]
fn push_json_string(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out.push('"');
}

fn diagnostic_js_value(diagnostic: &puzzle_lang::Diagnostic) -> JsValue {
    let payload = js_sys::Object::new();
    let span = diagnostic.primary_span.as_ref();
    set_js_string(&payload, "severity", diagnostic.severity.as_str());
    set_js_string(&payload, "code", diagnostic.code);
    set_js_string(
        &payload,
        "file",
        span.and_then(|span| span.file.as_deref()).unwrap_or(""),
    );
    set_js_optional_number(&payload, "line", span.and_then(|span| span.line));
    set_js_optional_number(&payload, "column", span.and_then(|span| span.column));
    set_js_optional_string(
        &payload,
        "sourceLine",
        span.and_then(|span| span.source_line.as_deref()),
    );
    set_js_string(&payload, "message", &diagnostic.message);
    payload.into()
}

fn set_js_string(payload: &js_sys::Object, key: &str, value: &str) {
    let _ = js_sys::Reflect::set(payload, &JsValue::from_str(key), &JsValue::from_str(value));
}

fn set_js_optional_number(payload: &js_sys::Object, key: &str, value: Option<usize>) {
    let value = value
        .map(|value| JsValue::from_f64(value as f64))
        .unwrap_or(JsValue::NULL);
    let _ = js_sys::Reflect::set(payload, &JsValue::from_str(key), &value);
}

fn set_js_optional_string(payload: &js_sys::Object, key: &str, value: Option<&str>) {
    let value = value.map(JsValue::from_str).unwrap_or(JsValue::NULL);
    let _ = js_sys::Reflect::set(payload, &JsValue::from_str(key), &value);
}

#[cfg(test)]
mod tests {
    use super::{
        activate_source_analysis, active_source_analysis_entries_json,
        active_source_analysis_highlight_range_json, active_source_analysis_json,
        active_source_analysis_outline_json, active_source_analysis_suggest_source_completions,
        apply_source_analysis_edit, compile_preview, compile_workspace_preview_from_documents,
        diagnostic_report_json, utf8_offset_from_utf16, utf16_offset_from_utf8,
        with_source_analysis,
    };

    fn invalid_workspace_game(statement: &str) -> String {
        format!(
            r#"title = "Diagnostic origin"

puzzle main {{
layers {{
base = Floor
}}
visuals {{
}}
rules {{
{statement}
}}
levels {{
legend {{
. = empty
}}
level "first"
.
}}
}}
"#
        )
    }

    #[test]
    fn compile_preview_accepts_at_prefixed_object_single_color_visual() {
        let source = r##"
title at_prefixed_object_single_color_preview

puzzle default {
layers {
@floor_slot = @Floor
}
visuals {
@Floor
#eeeeee
}
rules {

}
levels {
legend {
. = empty
}
level "start"
.
}
}
"##;

        let html = compile_preview(source, "game.puzzle", "", "").expect("compile preview");

        assert!(html.contains("<!doctype html>"));
        assert!(html.contains("#eeeeee"));
        assert!(html.contains("PuzzleStudioPreviewState"));
        assert!(html.contains("PuzzleRuntimeWasmLoader"));
        assert!(html.contains("ui-tap"));
        assert!(html.contains("buildSelectLayers"));
    }

    #[test]
    fn diagnostic_report_json_preserves_line_for_editor_links() {
        let report = puzzle_lang::DiagnosticReport::error_at_source_line_number(
            "`action` statements were removed",
            "action jump",
            9,
        );

        let json = diagnostic_report_json(&report);

        assert!(json.contains(r#""diagnostics":["#));
        assert!(json.contains(r#""line":9"#));
        assert!(json.contains(r#""sourceLine":"action jump""#));
        assert!(json.contains(r#""message":"`action` statements were removed""#));
    }

    #[test]
    fn workspace_preview_diagnostic_points_to_imported_document_line() {
        let imported_source = invalid_workspace_game("unknown_imported_statement");
        let expected_line = imported_source
            .lines()
            .position(|line| line == "unknown_imported_statement")
            .expect("invalid imported statement")
            + 1;
        let documents = vec![
            puzzle_lang::WorkspaceSourceDocument {
                path: "games/demo/game.puzzle".to_string(),
                source: "import \"parts/game.puzzle\"\n".to_string(),
            },
            puzzle_lang::WorkspaceSourceDocument {
                path: "games/demo/parts/game.puzzle".to_string(),
                source: imported_source,
            },
        ];

        let report =
            compile_workspace_preview_from_documents("games/demo/game.puzzle", &documents, "", "")
                .expect_err("invalid imported source should fail preview compile");
        let span = report.diagnostics()[0]
            .primary_span
            .as_ref()
            .expect("imported diagnostic span");

        assert_eq!(span.file.as_deref(), Some("games/demo/parts/game.puzzle"));
        assert_eq!(span.line, Some(expected_line));
    }

    #[test]
    fn workspace_preview_diagnostic_remaps_entry_line_after_import_expansion() {
        let game_source = invalid_workspace_game("unknown_entry_statement");
        let source = format!("import \"padding.puzzle\"\n{game_source}");
        let expected_line = source
            .lines()
            .position(|line| line == "unknown_entry_statement")
            .expect("invalid entry statement")
            + 1;
        let documents = vec![
            puzzle_lang::WorkspaceSourceDocument {
                path: "game.puzzle".to_string(),
                source,
            },
            puzzle_lang::WorkspaceSourceDocument {
                path: "padding.puzzle".to_string(),
                source: "// first imported line\n// second imported line\n".to_string(),
            },
        ];

        let report = compile_workspace_preview_from_documents("game.puzzle", &documents, "", "")
            .expect_err("invalid entry source should fail preview compile");
        let span = report.diagnostics()[0]
            .primary_span
            .as_ref()
            .expect("entry diagnostic span");

        assert_eq!(span.file.as_deref(), Some("game.puzzle"));
        assert_eq!(span.line, Some(expected_line));
    }

    #[test]
    fn workspace_preview_import_error_points_to_import_statement() {
        let documents = vec![puzzle_lang::WorkspaceSourceDocument {
            path: "game.puzzle".to_string(),
            source: "// heading\nimport \"missing.puzzle\"\n".to_string(),
        }];

        let report = compile_workspace_preview_from_documents("game.puzzle", &documents, "", "")
            .expect_err("missing import should fail preview compile");
        let span = report.diagnostics()[0]
            .primary_span
            .as_ref()
            .expect("import diagnostic span");

        assert_eq!(span.file.as_deref(), Some("game.puzzle"));
        assert_eq!(span.line, Some(2));
        assert_eq!(
            span.source_line.as_deref(),
            Some("import \"missing.puzzle\"")
        );
    }

    #[test]
    fn active_source_analysis_reuses_exact_source_and_rejects_stale_revisions() {
        let source = "puzzle Demo {\n  sounds {\n    \n  }\n}\n";
        let cursor = source.find("    ").unwrap() + 4;
        let revision = activate_source_analysis(source);
        assert_eq!(activate_source_analysis(source), revision);

        let analysis = active_source_analysis_json(revision).expect("analysis json");
        assert!(analysis.contains(r#""version":2"#));
        assert!(analysis.contains(r#""entries":"#));

        let highlight = active_source_analysis_highlight_range_json(
            revision,
            0,
            source.encode_utf16().count(),
            false,
        )
        .expect("highlight spans");
        assert!(highlight.contains(r#""version":3"#));
        assert!(highlight.contains(r#""offsetEncoding":"utf8""#));
        assert!(highlight.contains(&format!(r#""range":{{"start":0,"end":{}}}"#, source.len())));
        assert!(highlight.contains(r#""spans":["#));
        assert!(!highlight.contains(r#""html""#));

        let completions = active_source_analysis_suggest_source_completions(revision, cursor)
            .expect("completions");
        assert!(completions.contains(r#""label":"sfx""#));
        assert!(completions.contains(r#""label":"music""#));

        let entries = active_source_analysis_entries_json(revision).expect("entries");
        assert!(entries.contains(r#""entries":"#));

        let outline = active_source_analysis_outline_json(revision).expect("outline");
        assert!(outline.contains(r#""items":"#));

        let next_revision = activate_source_analysis("puzzle Other {}\n");
        assert_ne!(next_revision, revision);
        assert!(
            with_source_analysis(revision, puzzle_lang::SourceAnalysis::analysis_json).is_err()
        );
    }

    #[test]
    fn active_source_analysis_boundary_uses_browser_utf16_offsets() {
        let source = "title = \"😀\"\npuzzle Demo {\n  sounds {\n    \n  }\n}\n";
        let cursor_byte = source.find("    ").unwrap() + 4;
        let cursor_utf16 = source[..cursor_byte].encode_utf16().count();
        let revision = activate_source_analysis(source);

        assert_eq!(utf8_offset_from_utf16(source, cursor_utf16), cursor_byte);
        assert_eq!(utf16_offset_from_utf8(source, cursor_byte), cursor_utf16);

        let completions = active_source_analysis_suggest_source_completions(revision, cursor_utf16)
            .expect("source completions");
        assert!(completions.contains(&format!(r#""replaceStart":{cursor_utf16}"#)));
    }

    #[test]
    fn active_source_analysis_applies_utf16_edits_to_the_existing_session() {
        let source = "puzzle Demo {\n}\n// note\n";
        let revision = activate_source_analysis(source);
        let cursor_byte = source.find("note").unwrap() + "note".len();
        let cursor_utf16 = source[..cursor_byte].encode_utf16().count();

        let update = apply_source_analysis_edit(revision, cursor_utf16, cursor_utf16, "😀")
            .expect("incremental update");
        assert!(update.contains(r#""rescannedLines":1"#));
        assert!(update.contains(r#""parserCatalogReused":true"#));
        let next_revision = update
            .strip_prefix("{\"revision\":")
            .and_then(|tail| tail.split(',').next())
            .and_then(|value| value.parse::<u32>().ok())
            .expect("updated revision");

        assert!(
            with_source_analysis(revision, puzzle_lang::SourceAnalysis::analysis_json).is_err()
        );
        let highlight = active_source_analysis_highlight_range_json(
            next_revision,
            0,
            source.encode_utf16().count() + 2,
            false,
        )
        .expect("updated highlight");
        assert!(highlight.contains(&format!(r#""sourceLength":{}"#, source.len() + 4)));
    }
}
