use std::cell::RefCell;

use wasm_bindgen::prelude::*;

type SourceAnalysisRevision = u32;

thread_local! {
    static SOURCE_ANALYSES: RefCell<SourceAnalysisStore> =
        RefCell::new(SourceAnalysisStore::default());
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
    fn activate(&mut self, source: &str) -> SourceAnalysisRevision {
        if let Some(active) = &self.active {
            if active.analysis.source() == source {
                return active.revision;
            }
        }
        let revision = self.next_revision.max(1);
        self.next_revision = revision
            .checked_add(1)
            .expect("source analysis revision counter exhausted");
        self.active = Some(ActiveSourceAnalysis {
            revision,
            analysis: puzzle_lang::analyze_source(source),
        });
        revision
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
    SOURCE_ANALYSES.with(|store| store.borrow_mut().activate(source))
}

#[wasm_bindgen]
pub fn active_source_analysis_json(revision: SourceAnalysisRevision) -> Result<String, JsValue> {
    with_source_analysis(revision, puzzle_lang::SourceAnalysis::analysis_json)
        .map_err(source_analysis_error_js_value)
}

#[wasm_bindgen]
pub fn active_source_analysis_highlight_json(
    revision: SourceAnalysisRevision,
    include_outline: bool,
) -> Result<String, JsValue> {
    with_source_analysis(revision, |analysis| {
        analysis.highlight_json(include_outline)
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
pub fn active_source_analysis_entries_json(
    revision: SourceAnalysisRevision,
) -> Result<String, JsValue> {
    with_source_analysis(revision, puzzle_lang::SourceAnalysis::entries_json)
        .map_err(source_analysis_error_js_value)
}

/// Returns level-editor metadata for the active source snapshot. Board cells and
/// sprite payloads deliberately travel through their own on-demand exports.
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

/// Returns one renderer-ready sprite payload by canonical object ID.
#[wasm_bindgen]
pub fn active_source_analysis_level_editor_sprite_json(
    revision: SourceAnalysisRevision,
    object_id: u16,
) -> Result<String, JsValue> {
    with_source_analysis(revision, |analysis| {
        analysis.level_editor_sprite_payload_json(object_id)
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

#[wasm_bindgen]
pub fn export_html(
    source: &str,
    puzzle_path: &str,
    game_css: &str,
    game_visuals_js: &str,
    game_runtime_module_js: &str,
    game_runtime_wasm_base64: &str,
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
        game_runtime_module_js,
        game_runtime_wasm_base64,
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

#[wasm_bindgen]
pub fn solve_state(
    source: &str,
    puzzle_path: &str,
    state_json: &str,
    max_depth: u32,
    max_nodes: u32,
    max_ms: u32,
) -> Result<String, JsValue> {
    html_play::solve_state_json_from_source(
        source,
        puzzle_path,
        state_json,
        max_depth,
        max_nodes as usize,
        u64::from(max_ms),
    )
    .map_err(|error| JsValue::from_str(&error))
}

#[wasm_bindgen]
pub fn solve_request_json(request_json: &str) -> Result<String, JsValue> {
    html_play::solve_request_json(request_json).map_err(|error| JsValue::from_str(&error))
}

#[wasm_bindgen]
pub fn solve_solver_task_json(request_json: &str) -> Result<String, JsValue> {
    html_play::solve_solver_task_json(request_json).map_err(|error| JsValue::from_str(&error))
}

#[wasm_bindgen]
pub fn solver_task_initial_display_state_json(request_json: &str) -> Result<String, JsValue> {
    html_play::solver_task_initial_display_state_json(request_json)
        .map_err(|error| JsValue::from_str(&error))
}

#[wasm_bindgen]
pub fn solve_solver_task_json_with_progress(
    request_json: &str,
    on_observation: &js_sys::Function,
) -> Result<String, JsValue> {
    html_play::solve_solver_task_json_with_progress(request_json, |observation_json| {
        let _ = on_observation.call1(&JsValue::NULL, &JsValue::from_str(observation_json));
    })
    .map_err(|error| JsValue::from_str(&error))
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
        activate_source_analysis, active_source_analysis_entries_json, active_source_analysis_json,
        active_source_analysis_outline_json, active_source_analysis_suggest_source_completions,
        compile_preview, diagnostic_report_json, utf8_offset_from_utf16, utf16_offset_from_utf8,
        with_source_analysis,
    };

    #[test]
    fn compile_preview_accepts_display_object_single_color_sprite() {
        let source = r##"
title display_object_single_color_preview

puzzle default {
layers {
@display_floor = @Floor
}
sprites {
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
    fn active_source_analysis_reuses_exact_source_and_rejects_stale_revisions() {
        let source = "puzzle Demo {\n  sounds {\n    \n  }\n}\n";
        let cursor = source.find("    ").unwrap() + 4;
        let revision = activate_source_analysis(source);
        assert_eq!(activate_source_analysis(source), revision);

        let analysis = active_source_analysis_json(revision).expect("analysis json");
        assert!(analysis.contains(r#""version":1"#));
        assert!(analysis.contains(r#""entries":"#));

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
}
