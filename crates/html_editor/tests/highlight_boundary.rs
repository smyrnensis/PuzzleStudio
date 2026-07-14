const CODEMIRROR_SOURCE: &str = include_str!("../web/src/editor_codemirror.js");
const EDITOR_SOURCE: &str = include_str!("../static/editor_source.js");

fn function_body<'a>(source: &'a str, signature: &str, next_signature: &str) -> &'a str {
    source
        .split(signature)
        .nth(1)
        .and_then(|tail| tail.split(next_signature).next())
        .expect("function body")
}

#[test]
fn viewport_highlight_updates_only_the_returned_decoration_range() {
    assert!(CODEMIRROR_SOURCE.contains("let next = decorations.map(transaction.changes);"));
    assert!(CODEMIRROR_SOURCE.contains("next = next.update({"));
    assert!(CODEMIRROR_SOURCE.contains("filterFrom: replacement.from,"));
    assert!(CODEMIRROR_SOURCE.contains("filterTo: replacement.to,"));
    assert!(
        CODEMIRROR_SOURCE
            .contains("filter: (from, to) => to <= replacement.from || from >= replacement.to,")
    );
    assert!(!CODEMIRROR_SOURCE.contains("Decoration.set(replacement.decorations, true)"));
}

#[test]
fn repeated_viewport_queries_reuse_the_exact_source_offset_map() {
    assert!(CODEMIRROR_SOURCE.contains("const offsetsForSource = (source) => {"));
    assert!(CODEMIRROR_SOURCE.contains("if (offsetMapSource !== expected || offsetMaps === null)"));
    assert!(CODEMIRROR_SOURCE.contains("offsetsForSource(expected),"));
    assert!(CODEMIRROR_SOURCE.contains("const offsets = offsetsForSource(expected);"));
}

#[test]
fn active_highlight_path_projects_rust_spans_without_recognizing_source_syntax() {
    let refresh = function_body(
        EDITOR_SOURCE,
        "async function refreshSourceHighlight() {",
        "function scheduleSourceOutlineRefresh(",
    );
    assert!(refresh.contains("window.PuzzleStudioHost.highlight("));
    assert!(refresh.contains("sourceEditor.sourceEditorPort.highlightViewportRange()"));
    assert!(refresh.contains("sourceEditor.sourceEditorPort.applyHighlightRange("));
    for forbidden in [
        "split(/",
        "match(/",
        "exec(",
        "split_whitespace",
        "stripSourceStructureLineComment",
        "sourceLineHasStructuralBrace",
    ] {
        assert!(
            !refresh.contains(forbidden),
            "active JS highlight path must consume Rust spans, not recognize source via {forbidden}"
        );
    }

    let decorations = function_body(
        CODEMIRROR_SOURCE,
        "function highlightDecorations(",
        "function createState(",
    );
    assert!(decorations.contains("for (const span of payload.spans)"));
    assert!(decorations.contains("sourceHighlightClasses[String(span?.kind || \"\")]"));
    assert!(!decorations.contains("source.split"));
    assert!(!decorations.contains("source.match"));
}
