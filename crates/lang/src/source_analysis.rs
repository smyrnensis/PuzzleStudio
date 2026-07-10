use crate::completion::{
    CompletionList, completion_context_requires_symbols, completion_list_from_context,
    completion_list_json,
};
use crate::highlight::{HighlightedSourceWithOutline, highlight_source_with_document};
use crate::level_editor_source::{
    level_editor_level_slots, level_editor_manifest_json, level_editor_sprite_payload_json,
};
use crate::source_outline::{SourceOutlineItem, source_outline_from_document};
use crate::source_target::{
    SourceTarget, resolve_source_entries_from_document, resolve_source_target_from_entries,
    source_entries_json_from_entries, source_target_json,
};
use crate::surface::{SurfaceCompletionSymbols, SurfaceDocument};
use crate::surface_completion::surface_completion_context_for_document;
use std::cell::OnceCell;
use std::fmt;

/// Parser-owned source analysis shared by editor-facing derived products.
///
/// `SourceAnalysis` is the stable boundary for editor source services:
/// highlighting, outline, completion, source entries, and source target lookup
/// are derived from the same parsed surface document instead of rebuilding
/// independent documents per query.
pub struct SourceAnalysis {
    source: String,
    structure_document: OnceCell<SurfaceDocument>,
    highlight_document: OnceCell<SurfaceDocument>,
    source_target_document: OnceCell<SurfaceDocument>,
    completion_symbols: OnceCell<SurfaceCompletionSymbols>,
    entries: OnceCell<Vec<SourceTarget>>,
    outline: OnceCell<Vec<SourceOutlineItem>>,
    level_editor_integration: OnceCell<Result<crate::LevelEditorIntegration, String>>,
}

impl Clone for SourceAnalysis {
    fn clone(&self) -> Self {
        // Cached parser products are implementation detail; cloning preserves the
        // source snapshot while keeping each analysis cache independently lazy.
        Self::new(&self.source)
    }
}

impl fmt::Debug for SourceAnalysis {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SourceAnalysis")
            .field("source", &self.source)
            .finish_non_exhaustive()
    }
}

impl SourceAnalysis {
    /// Builds the shared source analysis for one exact source snapshot.
    pub fn new(source: &str) -> Self {
        Self {
            source: source.to_string(),
            structure_document: OnceCell::new(),
            highlight_document: OnceCell::new(),
            source_target_document: OnceCell::new(),
            completion_symbols: OnceCell::new(),
            entries: OnceCell::new(),
            outline: OnceCell::new(),
            level_editor_integration: OnceCell::new(),
        }
    }

    /// Returns the source snapshot this analysis was built from.
    pub fn source(&self) -> &str {
        &self.source
    }

    fn structure_document(&self) -> &SurfaceDocument {
        self.structure_document
            .get_or_init(|| crate::parse_surface_completion_context_document(&self.source))
    }

    fn highlight_document(&self) -> &SurfaceDocument {
        self.highlight_document
            .get_or_init(|| crate::parse_surface_document(&self.source))
    }

    fn source_target_document(&self) -> &SurfaceDocument {
        self.source_target_document
            .get_or_init(|| crate::parse_surface_source_target_document(&self.source))
    }

    fn completion_symbols(&self) -> &SurfaceCompletionSymbols {
        self.completion_symbols.get_or_init(|| {
            crate::parse_surface_completion_symbols_document(&self.source).completion_symbols
        })
    }

    fn entries(&self) -> &[SourceTarget] {
        self.entries
            .get_or_init(|| {
                resolve_source_entries_from_document(&self.source, self.source_target_document())
            })
            .as_slice()
    }

    fn outline(&self) -> &[SourceOutlineItem] {
        self.outline
            .get_or_init(|| source_outline_from_document(self.structure_document()))
            .as_slice()
    }

    fn level_editor_integration(&self) -> Result<&crate::LevelEditorIntegration, String> {
        self.level_editor_integration
            .get_or_init(|| {
                crate::integrate_level_editor_authoring(&self.source)
                    .map_err(|report| report.to_string())
            })
            .as_ref()
            .map_err(Clone::clone)
    }

    /// Produces highlighting and outline from this analysis document.
    pub fn highlighted(&self) -> HighlightedSourceWithOutline {
        HighlightedSourceWithOutline {
            highlighted: highlight_source_with_document(&self.source, self.highlight_document()),
            outline: self.outline().to_vec(),
        }
    }

    /// Produces completions from this analysis document.
    pub fn completion_list(&self, cursor_offset: usize) -> CompletionList {
        let context = surface_completion_context_for_document(
            &self.source,
            cursor_offset,
            self.structure_document(),
        );
        let symbols = if completion_context_requires_symbols(&context) {
            self.completion_symbols().clone()
        } else {
            SurfaceCompletionSymbols::default()
        };
        completion_list_from_context(&self.source, cursor_offset, context, symbols)
    }

    /// Resolves the source target at `cursor_offset` from this analysis document.
    pub fn resolve_target(&self, cursor_offset: usize) -> Option<SourceTarget> {
        resolve_source_target_from_entries(
            &self.source,
            self.source_target_document(),
            self.entries(),
            cursor_offset.min(self.source.len()),
        )
    }

    /// Emits the shared editor analysis JSON for this exact source snapshot.
    pub fn analysis_json(&self) -> String {
        source_analysis_json(self)
    }

    /// Emits highlight JSON from this analysis document.
    pub fn highlight_json(&self, include_outline: bool) -> String {
        highlighted_source_json(&self.highlighted(), include_outline)
    }

    /// Emits source outline JSON from the cached structure document.
    pub fn outline_json(&self) -> String {
        source_outline_items_json(self.outline())
    }

    /// Emits completion JSON from this analysis document.
    pub fn completion_json(&self, cursor_offset: usize) -> String {
        completion_list_json(&self.completion_list(cursor_offset))
    }

    /// Emits source target JSON from this analysis document.
    pub fn target_json(&self, cursor_offset: usize) -> String {
        let target = self.resolve_target(cursor_offset);
        source_target_json(target.as_ref())
    }

    /// Emits source entries JSON from the entries captured by this analysis.
    pub fn entries_json(&self) -> String {
        source_entries_json_from_entries(self.entries())
    }

    /// Emits level-editor metadata without transferring per-cell state or every sprite.
    pub fn level_editor_manifest_json(&self) -> Result<String, String> {
        level_editor_manifest_json(self.level_editor_integration()?, self.entries())
    }

    /// Returns a compact object-ID slot buffer for one level state.
    pub fn level_editor_level_slots(
        &self,
        level_index: usize,
        authored_layer: Option<usize>,
    ) -> Result<Vec<u32>, String> {
        level_editor_level_slots(
            self.level_editor_integration()?,
            level_index,
            authored_layer,
        )
    }

    /// Returns one renderer-ready sprite payload on demand.
    pub fn level_editor_sprite_payload_json(&self, object_id: u16) -> Result<String, String> {
        level_editor_sprite_payload_json(self.level_editor_integration()?, object_id)
    }
}

/// Builds a parser-owned source analysis for one exact source snapshot.
pub fn analyze_source(source: &str) -> SourceAnalysis {
    SourceAnalysis::new(source)
}

/// Builds and emits shared editor analysis JSON for one exact source snapshot.
pub fn analyze_source_json(source: &str) -> String {
    analyze_source(source).analysis_json()
}

fn source_analysis_json(analysis: &SourceAnalysis) -> String {
    let highlighted = analysis.highlighted();
    let mut out = String::from("{\"version\":1");
    out.push_str(",\"sourceLength\":");
    out.push_str(&analysis.source.len().to_string());
    out.push_str(",\"highlight\":");
    push_highlight_json_value(&mut out, &highlighted.highlighted);
    out.push_str(",\"outline\":{\"items\":[");
    for (index, item) in highlighted.outline.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_source_outline_item_json(&mut out, item);
    }
    out.push_str("]},\"entries\":");
    push_entries_json_value(&mut out, analysis.entries());
    out.push('}');
    out
}

fn source_outline_items_json(items: &[SourceOutlineItem]) -> String {
    let mut out = String::from("{\"items\":[");
    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_source_outline_item_json(&mut out, item);
    }
    out.push_str("]}");
    out
}

fn highlighted_source_json(
    highlighted: &HighlightedSourceWithOutline,
    include_outline: bool,
) -> String {
    let mut out = String::from("{");
    out.push_str("\"parsed\":");
    out.push_str(if highlighted.highlighted.parsed {
        "true"
    } else {
        "false"
    });
    out.push(',');
    push_json_string(&mut out, "html", &highlighted.highlighted.html);
    if include_outline {
        out.push_str(",\"outline\":{\"items\":[");
        for (index, item) in highlighted.outline.iter().enumerate() {
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

fn push_highlight_json_value(out: &mut String, highlighted: &crate::HighlightedSource) {
    out.push('{');
    out.push_str("\"parsed\":");
    out.push_str(if highlighted.parsed { "true" } else { "false" });
    out.push(',');
    push_json_string(out, "html", &highlighted.html);
    out.push('}');
}

fn push_entries_json_value(out: &mut String, entries: &[SourceTarget]) {
    let entries_json = source_entries_json_from_entries(entries);
    let Some(entries_value) = entries_json
        .strip_prefix("{\"entries\":")
        .and_then(|value| value.strip_suffix('}'))
    else {
        panic!("source entries json must be an entries object");
    };
    out.push_str(entries_value);
}

fn push_source_outline_item_json(out: &mut String, item: &SourceOutlineItem) {
    out.push('{');
    push_json_string(out, "id", &item.id);
    out.push(',');
    push_json_string(out, "kind", &item.kind);
    out.push(',');
    push_json_string(out, "label", &item.label);
    out.push(',');
    push_json_number(out, "start", item.start);
    out.push(',');
    push_json_number(out, "end", item.end);
    out.push(',');
    push_json_number(out, "depth", item.depth);
    out.push_str(",\"parent\":");
    match &item.parent {
        Some(parent) => push_json_string_value(out, parent),
        None => out.push_str("null"),
    }
    out.push('}');
}

fn push_json_string(out: &mut String, key: &str, value: &str) {
    push_json_string_value(out, key);
    out.push(':');
    push_json_string_value(out, value);
}

fn push_json_number(out: &mut String, key: &str, value: usize) {
    push_json_string_value(out, key);
    out.push(':');
    out.push_str(&value.to_string());
}

fn push_json_string_value(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => {
                out.push_str("\\u");
                out.push_str(&format!("{:04x}", ch as u32));
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::analyze_source;
    use crate::SourceTargetKind;

    #[test]
    fn source_analysis_reuses_document_for_authoring_completion() {
        let source = "puzzle Demo {\n  sounds {\n    \n  }\n}\n";
        let cursor = source.find("    ").unwrap() + 4;
        let analysis = analyze_source(source);
        let list = analysis.completion_list(cursor);
        let labels = list
            .items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>();
        assert!(labels.contains(&"sfx"));
        assert!(labels.contains(&"music"));
    }

    #[test]
    fn source_analysis_resolves_targets_and_entries_from_analysis_document() {
        let source = "puzzle Demo {\n  level Start {\n    .\n  }\n}\n";
        let cursor = source.find("Start").expect("level name cursor");
        let analysis = analyze_source(source);

        let target = analysis.resolve_target(cursor).expect("level target");
        assert_eq!(target.kind, SourceTargetKind::Level);
        assert_eq!(target.name, "Start");

        let target_json = analysis.target_json(cursor);
        assert!(target_json.contains("\"kind\":\"level\""));
        assert!(target_json.contains("\"name\":\"Start\""));

        let entries_json = analysis.entries_json();
        assert!(entries_json.contains("\"entries\":["));
        assert!(entries_json.contains("\"kind\":\"level\""));
        assert!(entries_json.contains("\"name\":\"Start\""));
    }

    #[test]
    fn source_analysis_json_emits_shared_editor_products() {
        let source = "puzzle Demo {\n  level Start {\n    .\n  }\n}\n";
        let analysis = analyze_source(source);
        let json = analysis.analysis_json();
        assert!(json.contains("\"version\":1"));
        assert!(json.contains("\"highlight\":"));
        assert!(json.contains("\"outline\":{\"items\":["));
        assert!(json.contains("\"entries\":["));
        assert!(json.contains("\"kind\":\"level\""));
    }

    #[test]
    fn level_editor_contract_uses_parser_products_without_compiling_routines() {
        let source = r#"
title = move_requires_explicit_routine

puzzle default {
layers {
actor = Box
marker = Marker
}
rules {
move
}
}
sprites {
Box {
#fff
0
}
}
levels {
legend {
B = Box
M = Marker
. = empty
}
level "start"
B
+
M
}
"#;
        let analysis = analyze_source(source);
        let json = analysis
            .level_editor_manifest_json()
            .expect("level editor manifest must not compile routines");

        assert!(json.contains(r#""kind":"puzzle2d-level-editor""#));
        assert!(json.contains(r#""id":1,"layer":0,"name":"Box""#));
        assert!(json.contains(r#""objectIds":[1],"symbol":"B""#));
        assert!(json.contains(r#""name":"start""#));
        assert!(
            !json.contains("slots"),
            "manifest must not transfer board cells: {json}"
        );
        assert!(
            !json.contains("sprites"),
            "manifest must not transfer sprite definitions: {json}"
        );
        assert_eq!(
            analysis.level_editor_level_slots(0, None).unwrap(),
            vec![1, 2]
        );
        assert_eq!(
            analysis.level_editor_level_slots(0, Some(0)).unwrap(),
            vec![1, 0]
        );
        assert_eq!(
            analysis.level_editor_level_slots(0, Some(1)).unwrap(),
            vec![0, 2]
        );
        let sprite = analysis.level_editor_sprite_payload_json(1).unwrap();
        assert!(
            sprite.contains(r##""colors":{"0":"#fff"}"##),
            "unexpected sprite: {sprite}"
        );
    }

    #[test]
    fn level_editor_manifest_keeps_parser_legend_diagnostics_without_failing_the_session() {
        let source = "puzzle default {\nlayers {\nactor = Box\n}\n}\nlevels {\nlegend {\nX = Missing\n. = empty\n}\nlevel \"one\"\nX\n}\n";
        let manifest = analyze_source(source)
            .level_editor_manifest_json()
            .expect("invalid legend must remain visible to the level editor");

        assert!(
            manifest.contains("unknown object selector: X = Missing"),
            "unexpected parser diagnostic: {manifest}"
        );
    }

    #[test]
    fn level_editor_manifest_keeps_levels_when_a_sprite_is_invalid() {
        let source = r#"
puzzle default {
layers {
actor = Box
}
rules {
move
}
}
sprites {
Box {
unknown nope
}
}
levels {
legend {
B = Box
. = empty
}
level "one"
B
}
"#;
        let manifest = analyze_source(source)
            .level_editor_manifest_json()
            .expect("sprite diagnostics must not stop the level editor session");

        assert!(
            manifest.contains("solid sprite requires exactly one color"),
            "{manifest}"
        );
        assert!(manifest.contains(r#""name":"one""#), "{manifest}");
    }

    #[test]
    fn source_analysis_construction_is_lazy() {
        let source = include_str!("source_analysis.rs");
        let new_body = source
            .split("pub fn new(source: &str) -> Self {")
            .nth(1)
            .and_then(|tail| tail.split("    /// Returns the source snapshot").next())
            .expect("SourceAnalysis::new body");

        assert!(!new_body.contains("parse_surface_document"));
        assert!(!new_body.contains("resolve_source_entries_from_document"));
        assert!(!new_body.contains("source_outline_from_document"));
    }
}
