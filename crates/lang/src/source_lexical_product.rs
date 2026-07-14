//! Typed highlight product derived from canonical parser facts.
//!
//! Source spelling is deliberately unavailable to this module. If a display
//! fact cannot be expressed from `SurfaceDocument`, the parser contract is
//! incomplete and must be extended at its owner.

use crate::highlight::{SourceHighlightKind, SourceHighlightSpan};
use crate::source::lexer::{SourceLexicalFact, SourceLexicalKind};
use crate::{SourceSpan, SurfaceDocument, SurfaceSemanticKind};

pub(crate) fn build_surface_highlight_spans(
    document: &SurfaceDocument,
) -> Vec<SourceHighlightSpan> {
    build_surface_highlight_spans_in_range(document, 0, usize::MAX)
}

pub(crate) fn build_surface_highlight_spans_in_range(
    document: &SurfaceDocument,
    range_start: usize,
    range_end: usize,
) -> Vec<SourceHighlightSpan> {
    if range_start >= range_end {
        return Vec::new();
    }
    let mut spans = Vec::<SourceHighlightSpan>::new();
    let display_facts = display_facts_in_range(document, range_start, range_end);
    let semantic_tokens = semantic_tokens_in_range(document, range_start, range_end);
    let raw_ranges = source_spans_in_range(
        &document.highlight_ranges.raw_ranges,
        range_start,
        range_end,
    );
    let plain_ranges = source_spans_in_range(
        &document.highlight_ranges.plain_ranges,
        range_start,
        range_end,
    );
    let first_line = document
        .lines
        .partition_point(|line| line.start + line.content.len() <= range_start);
    let last_line =
        first_line + document.lines[first_line..].partition_point(|line| line.start < range_end);

    for fact in document.lines[first_line..last_line]
        .iter()
        .flat_map(|line| &line.lexical_facts)
        .filter(|fact| fact.end > range_start && fact.start < range_end)
    {
        if spans.last().is_some_and(|span| span.end > fact.start) {
            continue;
        }
        if display_facts.iter().any(|display| {
            let span = display.span();
            span.end > fact.start && span.start < fact.end
        }) {
            continue;
        }
        if is_contained_by(fact.start, fact.end, plain_ranges) {
            continue;
        }
        if is_suppressed_raw_fact(semantic_tokens, raw_ranges, fact) {
            continue;
        }
        map_lexical_fact(document, semantic_tokens, fact, &mut spans);
    }

    append_owner_display_facts(display_facts, &mut spans);
    spans.sort_by_key(|span| (span.start, span.end));
    normalize_highlight_spans(spans)
}

fn map_lexical_fact(
    document: &SurfaceDocument,
    semantic_tokens: &[crate::surface::SurfaceSemanticToken],
    fact: &SourceLexicalFact,
    spans: &mut Vec<SourceHighlightSpan>,
) {
    match &fact.kind {
        SourceLexicalKind::Word => {
            if let Some(kind) = semantic_kind_at(semantic_tokens, fact.start, fact.end) {
                push_span(spans, fact.start, fact.end, highlight_kind(kind));
            }
        }
        SourceLexicalKind::Number => {
            let semantic = semantic_token_starting_at(semantic_tokens, fact.start)
                .filter(|token| token.kind == SurfaceSemanticKind::Number);
            push_span(
                spans,
                fact.start,
                semantic.map_or(fact.end, |token| token.span.end.max(fact.end)),
                SourceHighlightKind::Number,
            );
        }
        SourceLexicalKind::String => {
            let kind = semantic_kind_inside(semantic_tokens, fact.start, fact.end)
                .map(highlight_kind)
                .filter(|kind| *kind != SourceHighlightKind::String)
                .or_else(|| {
                    semantic_kind_at(semantic_tokens, fact.start, fact.end).map(highlight_kind)
                })
                .unwrap_or(SourceHighlightKind::String);
            push_span(spans, fact.start, fact.end, kind);
        }
        SourceLexicalKind::Comment => {
            push_span(spans, fact.start, fact.end, SourceHighlightKind::Comment);
        }
        SourceLexicalKind::Color(color) => {
            spans.push(SourceHighlightSpan {
                start: fact.start,
                end: fact.end,
                kind: SourceHighlightKind::Color,
                color: Some(color.clone()),
                transparent: false,
            });
        }
        SourceLexicalKind::Arrow => {
            push_span(spans, fact.start, fact.end, SourceHighlightKind::Arrow);
        }
        SourceLexicalKind::Operator => {
            let kind = semantic_kind_exact(semantic_tokens, fact.start, fact.end)
                .map(highlight_kind)
                .unwrap_or(SourceHighlightKind::Operator);
            push_span(spans, fact.start, fact.end, kind);
        }
        SourceLexicalKind::Brace(_) => {
            let kind = semantic_kind_exact(semantic_tokens, fact.start, fact.end)
                .map(highlight_kind)
                .unwrap_or_else(|| {
                    let disposition = fact
                        .brace_disposition
                        .expect("parser must classify every brace token");
                    if !disposition.matched_close
                        || document.unmatched_open_braces.contains(&fact.start)
                    {
                        SourceHighlightKind::InvalidBrace
                    } else {
                        brace_kind(disposition.depth)
                    }
                });
            push_span(spans, fact.start, fact.end, kind);
        }
        SourceLexicalKind::Plain => {
            if let Some(kind) = semantic_kind_at(semantic_tokens, fact.start, fact.end) {
                push_span(spans, fact.start, fact.end, highlight_kind(kind));
            }
        }
    }
}

fn display_facts_in_range(
    document: &SurfaceDocument,
    start: usize,
    end: usize,
) -> &[crate::SurfaceDisplayFact] {
    let ranges = &document.highlight_ranges.display_facts;
    let mut first = ranges.partition_point(|fact| fact.span().start < start);
    while first > 0 && ranges[first - 1].span().end > start {
        first -= 1;
    }
    let last = first + ranges[first..].partition_point(|fact| fact.span().start < end);
    &ranges[first..last]
}

fn append_owner_display_facts(
    facts: &[crate::SurfaceDisplayFact],
    spans: &mut Vec<SourceHighlightSpan>,
) {
    for fact in facts {
        match fact {
            crate::SurfaceDisplayFact::LevelCell { span, known } => push_span(
                spans,
                span.start,
                span.end,
                if *known {
                    SourceHighlightKind::LevelCell
                } else {
                    SourceHighlightKind::InvalidLevelCell
                },
            ),
            crate::SurfaceDisplayFact::SpritePixel {
                span,
                color,
                transparent,
            } => spans.push(SourceHighlightSpan {
                start: span.start,
                end: span.end,
                kind: SourceHighlightKind::SpritePixel,
                color: Some(color.clone()),
                transparent: *transparent,
            }),
            crate::SurfaceDisplayFact::Color { span, color } => spans.push(SourceHighlightSpan {
                start: span.start,
                end: span.end,
                kind: SourceHighlightKind::Color,
                color: Some(color.clone()),
                transparent: false,
            }),
            crate::SurfaceDisplayFact::Separator { span } => {
                push_span(spans, span.start, span.end, SourceHighlightKind::Arrow)
            }
        }
    }
}

fn is_suppressed_raw_fact(
    semantic_tokens: &[crate::surface::SurfaceSemanticToken],
    raw_ranges: &[SourceSpan],
    fact: &SourceLexicalFact,
) -> bool {
    is_contained_by(fact.start, fact.end, raw_ranges)
        && semantic_kind_at(semantic_tokens, fact.start, fact.end).is_none()
}

fn is_contained_by(start: usize, end: usize, ranges: &[SourceSpan]) -> bool {
    ranges
        .iter()
        .any(|range| start >= range.start && end <= range.end)
}

fn semantic_tokens_in_range(
    document: &SurfaceDocument,
    start: usize,
    end: usize,
) -> &[crate::surface::SurfaceSemanticToken] {
    let mut first = document
        .semantic_tokens
        .partition_point(|token| token.span.start < start);
    while first > 0 && document.semantic_tokens[first - 1].span.end > start {
        first -= 1;
    }
    let last =
        first + document.semantic_tokens[first..].partition_point(|token| token.span.start < end);
    &document.semantic_tokens[first..last]
}

fn source_spans_in_range(ranges: &[SourceSpan], start: usize, end: usize) -> &[SourceSpan] {
    let mut first = ranges.partition_point(|range| range.start < start);
    while first > 0 && ranges[first - 1].end > start {
        first -= 1;
    }
    let last = first + ranges[first..].partition_point(|range| range.start < end);
    &ranges[first..last]
}

fn semantic_kind_at(
    semantic_tokens: &[crate::surface::SurfaceSemanticToken],
    start: usize,
    end: usize,
) -> Option<SurfaceSemanticKind> {
    semantic_tokens
        .iter()
        .rev()
        .find(|token| token.span.start == start && token.span.end == end)
        .or_else(|| {
            semantic_tokens
                .iter()
                .rev()
                .find(|token| token.span.start <= start && end <= token.span.end)
        })
        .map(|token| token.kind)
}

fn semantic_kind_exact(
    semantic_tokens: &[crate::surface::SurfaceSemanticToken],
    start: usize,
    end: usize,
) -> Option<SurfaceSemanticKind> {
    semantic_tokens
        .iter()
        .find(|token| token.span.start == start && token.span.end == end)
        .map(|token| token.kind)
}

fn semantic_kind_inside(
    semantic_tokens: &[crate::surface::SurfaceSemanticToken],
    start: usize,
    end: usize,
) -> Option<SurfaceSemanticKind> {
    semantic_tokens
        .iter()
        .rev()
        .find(|token| start < token.span.start && token.span.end < end)
        .map(|token| token.kind)
}

fn semantic_token_starting_at(
    semantic_tokens: &[crate::surface::SurfaceSemanticToken],
    start: usize,
) -> Option<&crate::surface::SurfaceSemanticToken> {
    semantic_tokens
        .iter()
        .rev()
        .find(|token| token.span.start == start)
}

fn highlight_kind(kind: SurfaceSemanticKind) -> SourceHighlightKind {
    match kind {
        SurfaceSemanticKind::Keyword | SurfaceSemanticKind::Setting => SourceHighlightKind::Keyword,
        SurfaceSemanticKind::Literal => SourceHighlightKind::Literal,
        SurfaceSemanticKind::Binding => SourceHighlightKind::Binding,
        SurfaceSemanticKind::Effect => SourceHighlightKind::Effect,
        SurfaceSemanticKind::Emission => SourceHighlightKind::Emission,
        SurfaceSemanticKind::Object => SourceHighlightKind::Object,
        SurfaceSemanticKind::Input => SourceHighlightKind::Input,
        SurfaceSemanticKind::State => SourceHighlightKind::State,
        SurfaceSemanticKind::Group => SourceHighlightKind::Group,
        SurfaceSemanticKind::Mark => SourceHighlightKind::Mark,
        SurfaceSemanticKind::Variant => SourceHighlightKind::Variant,
        SurfaceSemanticKind::Condition => SourceHighlightKind::Condition,
        SurfaceSemanticKind::Scene => SourceHighlightKind::Scene,
        SurfaceSemanticKind::Theme => SourceHighlightKind::Theme,
        SurfaceSemanticKind::Asset => SourceHighlightKind::Asset,
        SurfaceSemanticKind::Color => SourceHighlightKind::Color,
        SurfaceSemanticKind::Number => SourceHighlightKind::Number,
        SurfaceSemanticKind::String => SourceHighlightKind::String,
    }
}

fn brace_kind(depth: usize) -> SourceHighlightKind {
    match depth % 6 {
        0 => SourceHighlightKind::Brace0,
        1 => SourceHighlightKind::Brace1,
        2 => SourceHighlightKind::Brace2,
        3 => SourceHighlightKind::Brace3,
        4 => SourceHighlightKind::Brace4,
        _ => SourceHighlightKind::Brace5,
    }
}

fn push_span(
    spans: &mut Vec<SourceHighlightSpan>,
    start: usize,
    end: usize,
    kind: SourceHighlightKind,
) {
    if start < end {
        spans.push(SourceHighlightSpan {
            start,
            end,
            kind,
            color: None,
            transparent: false,
        });
    }
}

fn normalize_highlight_spans(spans: Vec<SourceHighlightSpan>) -> Vec<SourceHighlightSpan> {
    let mut normalized = Vec::<SourceHighlightSpan>::with_capacity(spans.len());
    for span in spans {
        if let Some(previous) = normalized.last_mut()
            && previous.end == span.start
            && previous.kind == span.kind
            && previous.color == span.color
            && previous.transparent == span.transparent
        {
            previous.end = span.end;
            continue;
        }
        assert!(
            normalized
                .last()
                .is_none_or(|previous| previous.end <= span.start),
            "parser-owned highlight facts must not overlap"
        );
        normalized.push(span);
    }
    normalized
}

#[cfg(test)]
mod tests {
    use crate::highlight::{
        HighlightedSource, SourceHighlightKind, highlight_source_range_with_document,
        highlight_source_with_document,
    };

    fn highlight_source(source: &str) -> HighlightedSource {
        crate::highlight::highlight_source(source, crate::PuzzleSourceProfile::Puzzle2d)
    }

    fn has_span(
        source: &str,
        highlighted: &HighlightedSource,
        kind: SourceHighlightKind,
        text: &str,
    ) -> bool {
        highlighted
            .spans
            .iter()
            .any(|span| span.kind == kind && &source[span.start..span.end] == text)
    }

    #[test]
    fn typed_parser_facts_drive_core_highlights() {
        let source =
            "title = \"Demo\"\npuzzle board {\nrules {\n[ Box ] -> [ Box ]\n}\n}\n// note\n";
        let highlighted = highlight_source(source);
        for (kind, text) in [
            (SourceHighlightKind::Keyword, "title"),
            (SourceHighlightKind::String, "\"Demo\""),
            (SourceHighlightKind::Arrow, "->"),
            (SourceHighlightKind::Comment, "// note"),
            (SourceHighlightKind::Brace0, "{"),
        ] {
            assert!(
                has_span(source, &highlighted, kind, text),
                "missing {kind:?} for {text:?}"
            );
        }
    }

    #[test]
    fn product_has_no_source_recognizer_or_source_argument() {
        let implementation = include_str!("source_lexical_product.rs");
        let production = implementation
            .split("#[cfg(test)]")
            .next()
            .expect("highlight product production source");
        assert!(production.contains("document: &SurfaceDocument"));
        assert!(production.contains("partition_point"));
        assert!(include_str!("highlight.rs").contains("build_surface_highlight_spans_in_range"));
        for forbidden in [
            "source: &str",
            ".chars(",
            ".char_indices(",
            "split_inclusive",
            "starts_with",
            "ends_with",
            "strip_line_comment",
            "scan_brace",
            "consume_word",
            "hex_color",
            "inline_selector",
        ] {
            assert!(
                !production.contains(forbidden),
                "typed highlight product must not recognize source via {forbidden}"
            );
        }
    }

    #[test]
    fn parser_fact_boundaries_preserve_prefixes_selectors_and_invalid_braces() {
        let source = "puzzle board {\nlayers { objects = @Box }\nrules {\n[ Box:1{checked} ] -> [ @Box ]\n}\n}\n}\n";
        let highlighted = highlight_source(source);
        assert!(has_span(
            source,
            &highlighted,
            SourceHighlightKind::Object,
            "Box"
        ));
        let prefix = source.find("@Box").expect("object prefix");
        assert!(
            highlighted
                .spans
                .iter()
                .all(|span| !(span.start <= prefix && prefix < span.end))
        );
        assert!(has_span(
            source,
            &highlighted,
            SourceHighlightKind::Mark,
            "{checked}"
        ));
        assert!(has_span(
            source,
            &highlighted,
            SourceHighlightKind::InvalidBrace,
            "}"
        ));
    }

    #[test]
    fn range_projection_matches_intersecting_full_parser_facts() {
        let source =
            "title = \"Demo\"\npuzzle board {\nrules {\n[ Box ] -> [ Box ]\n}\n}\n// 注釈😀\n";
        let document = crate::parse_surface_document(source);
        let full = highlight_source_with_document(&document);
        let start = source.find("board").expect("range start");
        let end = source.find("//").expect("range end");
        let ranged = highlight_source_range_with_document(&document, start, end);
        let expected = full
            .spans
            .into_iter()
            .filter(|span| span.end > start && span.start < end)
            .collect::<Vec<_>>();
        assert_eq!(ranged.spans, expected);
    }

    #[test]
    fn owner_ranges_override_lexical_facts_with_pixel_data() {
        let source = "puzzle board {\nlayers { objects = Box }\nsprites {\nBox {\n#fff #000\n01\n}\n}\nrules {\n}\n}\n";
        let highlighted = highlight_source(source);
        let pixels = highlighted
            .spans
            .iter()
            .filter(|span| span.kind == SourceHighlightKind::SpritePixel)
            .collect::<Vec<_>>();
        assert!(
            pixels
                .iter()
                .any(|span| span.color.as_deref() == Some("#fff"))
        );
        assert!(
            pixels
                .iter()
                .any(|span| span.color.as_deref() == Some("#000"))
        );
    }
}
