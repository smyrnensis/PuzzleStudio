use wasm_bindgen::prelude::*;

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
pub fn highlight_source_html(source: &str) -> String {
    puzzle_lang::highlight_source(source).html
}

#[wasm_bindgen]
pub fn highlight_source_json(source: &str, include_outline: bool) -> String {
    if include_outline {
        let highlighted = puzzle_lang::highlight_source_with_outline(source);
        return highlighted_source_json(
            &highlighted.highlighted,
            Some(highlighted.outline.as_slice()),
        );
    }
    let highlighted = puzzle_lang::highlight_source(source);
    highlighted_source_json(&highlighted, None)
}

fn highlighted_source_json(
    highlighted: &puzzle_lang::HighlightedSource,
    outline: Option<&[puzzle_lang::SourceOutlineItem]>,
) -> String {
    let mut out = String::from("{");
    out.push_str("\"parsed\":");
    out.push_str(if highlighted.parsed { "true" } else { "false" });
    out.push(',');
    push_json_pair(&mut out, "html", &highlighted.html);
    if let Some(items) = outline {
        out.push_str(",\"outline\":{\"items\":[");
        for (index, item) in items.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            push_source_outline_item_json(&mut out, item);
        }
        out.push_str("]}");
    }
    out.push('}');
    out
}

#[wasm_bindgen]
pub fn source_outline_json(source: &str) -> String {
    puzzle_lang::source_outline_json(source)
}

#[wasm_bindgen]
pub fn translate_puzzlescript(source: &str) -> Result<String, JsValue> {
    puzzle_lang::translate_puzzlescript_to_canonical(source)
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

#[wasm_bindgen]
pub fn suggest_source_completions(source: &str, cursor_offset: usize) -> String {
    puzzle_lang::completion_list_json(&puzzle_lang::suggest_source_completions(
        source,
        cursor_offset,
    ))
}

#[wasm_bindgen]
pub fn resolve_source_target(source: &str, cursor_offset: usize) -> String {
    let target = puzzle_lang::resolve_source_target(source, cursor_offset);
    puzzle_lang::source_target_json(target.as_ref())
}

#[wasm_bindgen]
pub fn source_entries_json(source: &str) -> String {
    puzzle_lang::source_entries_json(source)
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

fn push_source_outline_item_json(out: &mut String, item: &puzzle_lang::SourceOutlineItem) {
    out.push('{');
    push_json_pair(out, "id", &item.id);
    out.push(',');
    push_json_pair(out, "kind", &item.kind);
    out.push(',');
    push_json_pair(out, "label", &item.label);
    out.push(',');
    push_json_number(out, "start", item.start);
    out.push(',');
    push_json_number(out, "end", item.end);
    out.push(',');
    push_json_number(out, "depth", item.depth);
    out.push_str(",\"parent\":");
    match item.parent.as_deref() {
        Some(parent) => push_json_string(out, parent),
        None => out.push_str("null"),
    }
    out.push('}');
}

fn push_json_pair(out: &mut String, key: &str, value: &str) {
    push_json_string(out, key);
    out.push(':');
    push_json_string(out, value);
}

fn push_json_number(out: &mut String, key: &str, value: usize) {
    push_json_string(out, key);
    out.push(':');
    out.push_str(&value.to_string());
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
    use super::{compile_preview, diagnostic_report_json, highlight_source_json};

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
    fn highlight_source_json_includes_outline_only_when_requested() {
        let source = r#"
puzzle board {
  rules {
  }
}
"#;

        let without_outline = highlight_source_json(source, false);
        assert!(without_outline.contains(r#""html":"#));
        assert!(!without_outline.contains(r#""outline":"#));

        let with_outline = highlight_source_json(source, true);
        assert!(with_outline.contains(r#""html":"#));
        assert!(with_outline.contains(r#""outline":{"items":["#));
        assert!(with_outline.contains(r#""label":"puzzle board""#));
        assert!(with_outline.contains(r#""label":"rules""#));
    }
}
