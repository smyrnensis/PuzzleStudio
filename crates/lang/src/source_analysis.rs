use crate::completion::{
    CompletionList, completion_context_requires_symbols, completion_list_from_context,
    completion_list_json,
};
use crate::highlight::{
    HighlightedSource, HighlightedSourceWithOutline, highlight_source_range_with_document,
    highlight_source_with_document,
};
use crate::level_editor_source::{
    level_editor_level_slots, level_editor_manifest_json, level_editor_visual_payload_json,
};
use crate::source_folding::{SourceFoldRange, source_fold_ranges_from_document};
use crate::source_outline::SourceOutlineItem;
use crate::source_target::{
    SourceTarget, resolve_source_entries_from_document, resolve_source_target_from_entries,
    source_entries_json_from_entries, source_target_json,
};
use crate::surface::SurfaceDocument;
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
    owner_dimension: Option<crate::ModelDimension>,
    snapshot: OnceCell<crate::ParseSnapshot>,
    highlighted_source: OnceCell<HighlightedSource>,
    entries: OnceCell<Vec<SourceTarget>>,
    outline: OnceCell<Vec<SourceOutlineItem>>,
    folds: OnceCell<Vec<SourceFoldRange>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceAnalysisEdit {
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceAnalysisEditResult {
    pub rescanned_lines: usize,
    pub total_lines: usize,
    pub parser_catalog_reused: bool,
}

impl Clone for SourceAnalysis {
    fn clone(&self) -> Self {
        // Cached parser products are implementation detail; cloning preserves the
        // source snapshot while keeping each analysis cache independently lazy.
        Self::new_with_owner_dimension(&self.source, self.owner_dimension)
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
        Self::new_with_owner_dimension(source, None)
    }

    /// Builds analysis for a fragment in one explicitly identified puzzle owner.
    pub fn new_with_owner_dimension(
        source: &str,
        owner_dimension: Option<crate::ModelDimension>,
    ) -> Self {
        Self {
            source: source.to_string(),
            owner_dimension,
            snapshot: OnceCell::new(),
            highlighted_source: OnceCell::new(),
            entries: OnceCell::new(),
            outline: OnceCell::new(),
            folds: OnceCell::new(),
        }
    }

    /// Returns the source snapshot this analysis was built from.
    pub fn source(&self) -> &str {
        &self.source
    }

    fn snapshot(&self) -> &crate::ParseSnapshot {
        self.snapshot
            .get_or_init(|| crate::ParseSnapshot::parse(&self.source, self.owner_dimension))
    }

    fn document(&self) -> &SurfaceDocument {
        self.snapshot().document()
    }

    pub fn sound_source_request(
        &self,
        request: crate::SoundSourceRequest,
    ) -> Result<crate::SoundSourceResponse, String> {
        crate::source_sound_edit::sound_source_request(
            &self.source,
            self.document(),
            self.snapshot().parser_recognition(),
            request,
        )
    }

    pub fn level_source_request(
        &self,
        request: crate::LevelSourceRequest,
    ) -> Result<crate::LevelSourceResponse, String> {
        crate::source_level_edit::level_source_request(&self.source, self.document(), request)
    }

    fn highlighted_source(&self) -> &HighlightedSource {
        self.highlighted_source
            .get_or_init(|| highlight_source_with_document(self.document()))
    }

    fn entries(&self) -> &[SourceTarget] {
        self.entries
            .get_or_init(|| resolve_source_entries_from_document(self.document()))
            .as_slice()
    }

    fn outline(&self) -> &[SourceOutlineItem] {
        self.outline
            .get_or_init(|| {
                crate::source::outline_product::build_surface_outline_items(self.document())
            })
            .as_slice()
    }

    fn folds(&self) -> &[SourceFoldRange] {
        self.folds
            .get_or_init(|| source_fold_ranges_from_document(&self.source, self.document()))
            .as_slice()
    }

    fn level_editor_integration(&self) -> Result<&crate::LevelEditorIntegration, String> {
        self.snapshot().level_editor_integration()
    }

    /// Produces highlighting and outline from this analysis document.
    pub fn highlighted(&self) -> HighlightedSourceWithOutline {
        HighlightedSourceWithOutline {
            highlighted: self.highlighted_source().clone(),
            outline: self.outline().to_vec(),
        }
    }

    /// Projects highlighting without constructing the lazy outline product.
    pub fn highlight(&self) -> HighlightedSource {
        self.highlighted_source().clone()
    }

    /// Projects the revision-local parser-owned outline product.
    pub fn outline_items(&self) -> Vec<SourceOutlineItem> {
        self.outline().to_vec()
    }

    /// Produces completions from this analysis document.
    pub fn completion_list(&self, cursor_offset: usize) -> CompletionList {
        let context =
            surface_completion_context_for_document(&self.source, cursor_offset, self.document());
        let symbols = if completion_context_requires_symbols(&context) {
            self.document().completion_symbols.clone()
        } else {
            Default::default()
        };
        completion_list_from_context(&self.source, cursor_offset, context, symbols)
    }

    /// Resolves the source target at `cursor_offset` from this analysis document.
    pub fn resolve_target(&self, cursor_offset: usize) -> Option<SourceTarget> {
        resolve_source_target_from_entries(
            self.document(),
            self.entries(),
            cursor_offset.min(self.source.len()),
        )
    }

    /// Applies one UTF-8 source edit while preserving this document session.
    pub fn apply_edit(
        &mut self,
        edit: SourceAnalysisEdit,
        insert: &str,
    ) -> Result<SourceAnalysisEditResult, String> {
        if edit.start > edit.end
            || edit.end > self.source.len()
            || !self.source.is_char_boundary(edit.start)
            || !self.source.is_char_boundary(edit.end)
        {
            return Err("source analysis edit range is not a valid UTF-8 boundary".to_string());
        }

        let old_source = self.source.clone();
        self.snapshot();
        self.source.replace_range(edit.start..edit.end, insert);
        let snapshot = self
            .snapshot
            .get_mut()
            .expect("parse snapshot initialized before edit");
        let (rescanned_lines, parser_catalog_reused) = snapshot.apply_edit(
            &old_source,
            &self.source,
            self.owner_dimension,
            edit.start,
            edit.end,
            insert.len(),
        );
        let total_lines = snapshot.line_count();

        self.highlighted_source.take();
        self.entries.take();
        self.outline.take();
        self.folds.take();
        Ok(SourceAnalysisEditResult {
            rescanned_lines,
            total_lines,
            parser_catalog_reused,
        })
    }

    /// Emits the shared editor analysis JSON for this exact source snapshot.
    pub fn analysis_json(&self) -> String {
        source_analysis_json(self)
    }

    /// Emits highlight JSON from this analysis document.
    pub fn highlight_json(&self, include_outline: bool) -> String {
        self.highlight_range_json(0, self.source.len(), include_outline)
    }

    /// Emits highlight JSON for spans intersecting one UTF-8 byte range.
    pub fn highlight_range_json(
        &self,
        range_start: usize,
        range_end: usize,
        include_outline: bool,
    ) -> String {
        let start = range_start.min(self.source.len());
        let end = range_end.max(start).min(self.source.len());
        assert!(self.source.is_char_boundary(start));
        assert!(self.source.is_char_boundary(end));
        if start == 0 && end == self.source.len() {
            return highlighted_source_json(
                self.source.len(),
                self.highlighted_source(),
                start,
                end,
                include_outline.then(|| (self.outline(), self.folds())),
            );
        }
        let highlighted = highlight_source_range_with_document(self.document(), start, end);
        highlighted_source_json(
            self.source.len(),
            &highlighted,
            start,
            end,
            include_outline.then(|| (self.outline(), self.folds())),
        )
    }

    /// Emits source outline JSON from the cached structure document.
    pub fn outline_json(&self) -> String {
        source_outline_items_json(self.outline(), self.folds(), self.source.len())
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

    pub fn import_reference_at(
        &self,
        document_path: &str,
        cursor_offset: usize,
    ) -> Option<crate::SourceImportReference> {
        crate::source_import::source_import_reference_at(
            &self.document().imports,
            document_path,
            cursor_offset,
        )
    }

    pub fn imports(&self) -> &[crate::SourceImportDeclaration] {
        &self.document().imports
    }

    pub(crate) fn strict_document_parts(
        &self,
    ) -> Result<crate::DocumentSourceParts, crate::DiagnosticReport> {
        self.snapshot().strict_document_parts()
    }

    /// Emits source entries JSON from the entries captured by this analysis.
    pub fn entries_json(&self) -> String {
        let mut out = String::from("{\"version\":1,\"entries\":");
        push_entries_json_value(&mut out, self.entries());
        out.push('}');
        out
    }

    /// Emits level-editor metadata without transferring per-cell state or every visual.
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

    /// Returns one renderer-ready visual payload on demand.
    pub fn level_editor_visual_payload_json(&self, object_id: u16) -> Result<String, String> {
        level_editor_visual_payload_json(self.level_editor_integration()?, object_id)
    }
}

/// Builds a parser-owned source analysis for one exact source snapshot.
pub fn analyze_source(source: &str) -> SourceAnalysis {
    SourceAnalysis::new(source)
}

/// Builds parser-owned analysis for a model-less fragment in an explicit owner dimension.
/// Any `puzzle` declaration in the source remains authoritative for its own dimension.
pub fn analyze_source_for_owner_dimension(
    source: &str,
    owner_dimension: crate::ModelDimension,
) -> SourceAnalysis {
    SourceAnalysis::new_with_owner_dimension(source, Some(owner_dimension))
}

/// Builds and emits shared editor analysis JSON for one exact source snapshot.
pub fn analyze_source_json(source: &str) -> String {
    analyze_source(source).analysis_json()
}

fn source_analysis_json(analysis: &SourceAnalysis) -> String {
    let highlighted = analysis.highlighted();
    let mut out = String::from("{\"version\":2");
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

fn source_outline_items_json(
    items: &[SourceOutlineItem],
    folds: &[SourceFoldRange],
    source_length: usize,
) -> String {
    let mut out = String::from("{\"version\":1,\"offsetEncoding\":\"utf8\",\"sourceLength\":");
    out.push_str(&source_length.to_string());
    out.push_str(",\"items\":[");
    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_source_outline_item_json(&mut out, item);
    }
    out.push_str("],\"folds\":[");
    push_source_fold_ranges_json(&mut out, folds);
    out.push_str("]}");
    out
}

fn highlighted_source_json(
    source_length: usize,
    highlighted: &HighlightedSource,
    range_start: usize,
    range_end: usize,
    outline: Option<(&[SourceOutlineItem], &[SourceFoldRange])>,
) -> String {
    let mut out = String::from("{\"version\":3,\"offsetEncoding\":\"utf8\",");
    out.push_str("\"sourceLength\":");
    out.push_str(&source_length.to_string());
    out.push_str(",\"range\":{");
    push_json_number(&mut out, "start", range_start);
    out.push(',');
    push_json_number(&mut out, "end", range_end);
    out.push_str("},");
    out.push_str("\"parsed\":");
    out.push_str(if highlighted.parsed { "true" } else { "false" });
    out.push_str(",\"spans\":");
    push_highlight_spans_json(
        &mut out,
        highlighted
            .spans
            .iter()
            .filter(|span| span.end > range_start && span.start < range_end),
    );
    if let Some((outline, folds)) = outline {
        out.push_str(",\"outline\":{\"items\":[");
        for (index, item) in outline.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            push_source_outline_item_json(&mut out, item);
        }
        out.push_str("],\"version\":1,\"offsetEncoding\":\"utf8\",\"sourceLength\":");
        out.push_str(&source_length.to_string());
        out.push_str(",\"folds\":[");
        push_source_fold_ranges_json(&mut out, folds);
        out.push_str("]}");
    }
    out.push('}');
    out
}

fn push_source_fold_ranges_json(out: &mut String, folds: &[SourceFoldRange]) {
    for (index, range) in folds.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_number(out, "from", range.from);
        out.push(',');
        push_json_number(out, "to", range.to);
        out.push('}');
    }
}

fn push_highlight_json_value(out: &mut String, highlighted: &crate::HighlightedSource) {
    out.push('{');
    out.push_str("\"offsetEncoding\":\"utf8\",");
    out.push_str("\"parsed\":");
    out.push_str(if highlighted.parsed { "true" } else { "false" });
    out.push_str(",\"spans\":");
    push_highlight_spans_json(out, &highlighted.spans);
    out.push('}');
}

fn push_highlight_spans_json<'a>(
    out: &mut String,
    spans: impl IntoIterator<Item = &'a crate::SourceHighlightSpan>,
) {
    out.push('[');
    for (index, span) in spans.into_iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_number(out, "start", span.start);
        out.push(',');
        push_json_number(out, "end", span.end);
        out.push(',');
        push_json_string(out, "kind", span.kind.as_str());
        out.push_str(",\"color\":");
        match &span.color {
            Some(color) => push_json_string_value(out, color.as_str()),
            None => out.push_str("null"),
        }
        out.push(',');
        push_json_bool(out, "transparent", span.transparent);
        out.push('}');
    }
    out.push(']');
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

fn push_json_bool(out: &mut String, key: &str, value: bool) {
    push_json_string_value(out, key);
    out.push(':');
    out.push_str(if value { "true" } else { "false" });
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
    use super::{SourceAnalysisEdit, analyze_source};
    use crate::{SourceHighlightKind, SourceTargetKind};

    #[test]
    fn highlight_only_query_does_not_construct_the_lazy_outline_product() {
        let source = "puzzle board {\nrules {\n}\n}\n";
        let analysis = analyze_source(source);

        assert!(analysis.outline.get().is_none());
        let _ = analysis.highlight_range_json(0, source.len(), false);
        assert!(
            analysis.outline.get().is_none(),
            "viewport highlighting must not walk or cache the outline tree"
        );
        let first = analysis.outline_items();
        assert!(analysis.outline.get().is_some());
        assert_eq!(analysis.outline_items(), first);
    }

    #[test]
    fn puzzle3_analysis_highlights_level_slice_separator_as_valid_structure() {
        let source = r#"puzzle board {
dimension = 3
layers {
ground = Floor
}
levels {
legend {
_ = Floor
}
level "stacked" {
___
-
___
}
}
}
"#;
        let separator = source.find("\n-\n").expect("slice separator") + 1;
        let highlighted = analyze_source(source).highlight();

        assert!(
            highlighted.spans.iter().any(|span| {
                span.start == separator
                    && span.end == separator + 1
                    && span.kind == SourceHighlightKind::LevelSeparator
            }),
            "{:?}",
            highlighted.spans
        );
        assert!(highlighted.spans.iter().all(|span| {
            span.start != separator || span.kind != SourceHighlightKind::InvalidLevelCell
        }));
    }

    #[test]
    fn legend_char_highlighting_uses_the_parser_role_for_every_lexical_spelling() {
        let source = r#"puzzle board {
layers {
alpha = Alpha
numeric = Numeric
dash = Dash
}
levels {
legend {
A = Alpha
1 = Numeric
- = Dash
. = empty
}
level "one" {
A1-.
}
}
}
"#;
        let highlighted = analyze_source(source).highlight();

        for row in ["A = Alpha", "1 = Numeric", "- = Dash", ". = empty"] {
            let start = source.find(row).expect("legend row");
            let end = start + row.chars().next().expect("legend char").len_utf8();
            assert!(
                highlighted.spans.iter().any(|span| {
                    span.start == start
                        && span.end == end
                        && span.kind == SourceHighlightKind::Literal
                }),
                "row={row:?} spans={:?}",
                highlighted.spans
            );
        }
    }

    #[test]
    fn puzzle2d_analysis_keeps_undeclared_dash_level_cell_invalid() {
        let source =
            "puzzle default {\nlayers {\nactor = Box\n}\n}\nlevels {\nlevel \"dash\" {\n-\n}\n}\n";
        let separator = source.find('-').expect("dash cell");
        let highlighted = analyze_source(source).highlight();

        assert!(highlighted.spans.iter().any(|span| {
            span.start == separator
                && span.end == separator + 1
                && span.kind == SourceHighlightKind::InvalidLevelCell
        }));
    }

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
        let source = "puzzle Demo {\n  level \"Start\" {\n    .\n  }\n}\n";
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
        assert!(json.contains("\"version\":2"));
        assert!(json.contains("\"highlight\":"));
        assert!(json.contains("\"offsetEncoding\":\"utf8\""));
        assert!(json.contains("\"spans\":["));
        assert!(!json.contains("\"html\""));
        assert!(json.contains("\"outline\":{\"items\":["));
        assert!(json.contains("\"entries\":["));
        assert!(json.contains("\"kind\":\"level\""));
    }

    #[test]
    fn highlight_range_json_returns_only_intersecting_normalized_spans() {
        let source = "const title = \"Demo\"\n// visible\nconst author = \"Elsewhere\"\n";
        let range_start = source.find("// visible").expect("range start");
        let range_end = range_start + "// visible".len();
        let json = analyze_source(source).highlight_range_json(range_start, range_end, false);

        assert!(json.contains("\"version\":3"));
        assert!(json.contains(&format!(
            "\"sourceLength\":{},\"range\":{{\"start\":{},\"end\":{}}}",
            source.len(),
            range_start,
            range_end
        )));
        assert!(json.contains("\"kind\":\"comment\""));
        assert!(!json.contains("\"kind\":\"string\""));
        assert!(!json.contains("\"outline\""));
    }

    #[test]
    fn level_editor_contract_uses_parser_products_without_compiling_routines() {
        let source = r#"
const title = move_requires_explicit_routine

puzzle default {
layers {
actor = Box
marker = Marker
}
rules {
move
}
}
visuals {
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
            !json.contains("visuals"),
            "manifest must not transfer visual definitions: {json}"
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
        let visual = analysis.level_editor_visual_payload_json(1).unwrap();
        assert!(
            visual.contains(r##""colors":{"0":"#fff"}"##),
            "unexpected visual: {visual}"
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
    fn level_editor_manifest_keeps_levels_when_a_visual_is_invalid() {
        let source = r#"
puzzle default {
layers {
actor = Box
}
rules {
move
}
}
visuals {
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
            .expect("visual diagnostics must not stop the level editor session");

        assert!(
            manifest.contains("solid visual requires exactly one color"),
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
        assert!(!new_body.contains("source_fold_ranges_from_document"));
    }

    #[test]
    fn source_outline_contract_includes_parser_owned_utf8_fold_ranges() {
        let source = "puzzle démo {\n  rules {\n    move\n  }\n}\n";
        let payload = analyze_source(source).outline_json();
        let outer_from = source.find('{').unwrap() + 1;
        let outer_to = source.rfind('}').unwrap();
        assert!(payload.contains("\"offsetEncoding\":\"utf8\""), "{payload}");
        assert!(payload.contains("\"version\":1"), "{payload}");
        assert!(
            payload.contains(&format!("\"sourceLength\":{}", source.len())),
            "{payload}"
        );
        assert!(
            payload.contains(&format!("\"from\":{outer_from},\"to\":{outer_to}")),
            "{payload}"
        );
    }

    #[test]
    fn source_analysis_comment_edit_rescans_only_the_changed_suffix_line() {
        let source = "puzzle board {\n  rules {\n  }\n}\n// note\n";
        let mut analysis = analyze_source(source);
        let _ = analysis.highlight_json(true);
        let insert_at = source.find("note").expect("comment") + "note".len();

        let result = analysis
            .apply_edit(
                SourceAnalysisEdit {
                    start: insert_at,
                    end: insert_at,
                },
                "😀",
            )
            .expect("incremental edit");

        assert_eq!(result.rescanned_lines, 1);
        assert!(result.total_lines > result.rescanned_lines);
        assert!(result.parser_catalog_reused);
        let mut expected = source.to_string();
        expected.insert_str(insert_at, "😀");
        assert_eq!(analysis.source(), expected);
        assert_eq!(
            analysis.highlight_json(true),
            analyze_source(&expected).highlight_json(true)
        );
    }

    #[test]
    fn source_analysis_trivia_edit_keeps_parser_spans_on_the_current_revision() {
        let source =
            "puzzle board {\n  input right\n  layers { actors = Player }\n  rules {\n  }\n}\n";
        let mut analysis = analyze_source(source);
        let _ = analysis.highlight_json(true);

        let result = analysis
            .apply_edit(SourceAnalysisEdit { start: 0, end: 0 }, " ")
            .expect("incremental trivia edit");

        let expected = format!(" {source}");
        assert_eq!(analysis.source(), expected);
        assert_eq!(
            analysis.highlight_json(true),
            analyze_source(&expected).highlight_json(true),
            "incremental parser spans must describe the same revision as a fresh parser product"
        );
        assert!(!result.parser_catalog_reused);
    }

    #[test]
    fn source_analysis_structural_edit_invalidates_the_dependent_suffix() {
        let source = "puzzle board {\n  rules {\n  }\n}\nscene menu {\n}\n";
        let mut analysis = analyze_source(source);
        let _ = analysis.analysis_json();
        let brace = source.find("rules {").expect("rules") + "rules ".len();

        let result = analysis
            .apply_edit(
                SourceAnalysisEdit {
                    start: brace,
                    end: brace + 1,
                },
                "",
            )
            .expect("structural edit");

        assert!(!result.parser_catalog_reused);
        assert_eq!(result.rescanned_lines, 3);
        assert!(result.rescanned_lines < result.total_lines);
        let expected = source.replacen("rules {", "rules ", 1);
        assert_eq!(
            analysis.analysis_json(),
            analyze_source(&expected).analysis_json()
        );
    }

    #[test]
    fn source_analysis_preserves_unavailable_catalog_diagnostics() {
        let source = "const title = \"unfinished\npuzzle board {\n}\n";
        let analysis = analyze_source(source);
        let diagnostics = &analysis.snapshot().document().diagnostics;

        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("string literal is missing closing quote")
        }));
        assert!(
            crate::parse_surface_compile_document(source)
                .unwrap_err()
                .to_string()
                .contains("string literal is missing closing quote")
        );
    }

    #[test]
    fn source_analysis_owns_one_shared_surface_document() {
        let implementation = include_str!("source_analysis.rs");
        let production = implementation
            .split("#[cfg(test)]")
            .next()
            .expect("production source");

        assert_eq!(
            production.matches("OnceCell<crate::ParseSnapshot>").count(),
            1
        );
        assert!(!production.contains("OnceCell<SurfaceDocument>"));
        assert!(!production.contains("OnceCell<SurfaceSourceScan>"));
        assert!(!production.contains("OnceCell<Option<Catalog>>"));
        assert!(!production.contains("parse_surface_completion_context_document"));
        assert!(!production.contains("parse_surface_source_target_document"));
        assert!(!production.contains("parse_surface_completion_symbols_document"));
    }
}
