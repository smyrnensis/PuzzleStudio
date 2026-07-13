use std::collections::HashMap;

use crate::SourceOutlineItem;
use crate::semantic::{SemanticKind, SemanticToken};
use crate::source_outline::source_outline_from_document;
use crate::{
    SourceSpan, SurfaceAsciiRange, SurfaceDocument, SurfaceHighlightRanges,
    SurfaceVisualAsciiColorRange, SurfaceVisualNamedColorRange,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HighlightedSource {
    pub spans: Vec<SourceHighlightSpan>,
    pub parsed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HighlightedSourceWithOutline {
    pub highlighted: HighlightedSource,
    pub outline: Vec<SourceOutlineItem>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceHighlightSpan {
    pub start: usize,
    pub end: usize,
    pub kind: SourceHighlightKind,
    pub color: Option<String>,
    pub transparent: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceHighlightKind {
    Keyword,
    Literal,
    Binding,
    Effect,
    Emission,
    Object,
    Input,
    State,
    Group,
    Mark,
    Variant,
    Condition,
    Scene,
    Theme,
    Asset,
    Color,
    Number,
    String,
    Comment,
    Operator,
    Arrow,
    Brace0,
    Brace1,
    Brace2,
    Brace3,
    Brace4,
    Brace5,
    InvalidBrace,
    LevelCell,
    InvalidLevelCell,
    SpritePixel,
}

impl SourceHighlightKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SourceHighlightKind::Keyword => "keyword",
            SourceHighlightKind::Literal => "literal",
            SourceHighlightKind::Binding => "binding",
            SourceHighlightKind::Effect => "effect",
            SourceHighlightKind::Emission => "emission",
            SourceHighlightKind::Object => "object",
            SourceHighlightKind::Input => "input",
            SourceHighlightKind::State => "state",
            SourceHighlightKind::Group => "group",
            SourceHighlightKind::Mark => "mark",
            SourceHighlightKind::Variant => "variant",
            SourceHighlightKind::Condition => "condition",
            SourceHighlightKind::Scene => "scene",
            SourceHighlightKind::Theme => "theme",
            SourceHighlightKind::Asset => "asset",
            SourceHighlightKind::Color => "color",
            SourceHighlightKind::Number => "number",
            SourceHighlightKind::String => "string",
            SourceHighlightKind::Comment => "comment",
            SourceHighlightKind::Operator => "operator",
            SourceHighlightKind::Arrow => "arrow",
            SourceHighlightKind::Brace0 => "brace-depth-0",
            SourceHighlightKind::Brace1 => "brace-depth-1",
            SourceHighlightKind::Brace2 => "brace-depth-2",
            SourceHighlightKind::Brace3 => "brace-depth-3",
            SourceHighlightKind::Brace4 => "brace-depth-4",
            SourceHighlightKind::Brace5 => "brace-depth-5",
            SourceHighlightKind::InvalidBrace => "brace-invalid",
            SourceHighlightKind::LevelCell => "level-cell",
            SourceHighlightKind::InvalidLevelCell => "level-cell-invalid",
            SourceHighlightKind::SpritePixel => "sprite-pixel",
        }
    }
}

pub fn highlight_source(source: &str) -> HighlightedSource {
    let document = crate::parse_surface_document(source);
    highlight_source_with_document(source, &document)
}

pub fn highlight_source_with_outline(source: &str) -> HighlightedSourceWithOutline {
    let document = crate::parse_surface_document(source);
    HighlightedSourceWithOutline {
        highlighted: highlight_source_with_document(source, &document),
        outline: source_outline_from_document(&document),
    }
}

pub(crate) fn highlight_source_with_document(
    source: &str,
    document: &SurfaceDocument,
) -> HighlightedSource {
    HighlightedSource {
        spans: highlight_spans(source, document, 0, source.len()),
        parsed: true,
    }
}

pub(crate) fn highlight_source_range_with_document(
    source: &str,
    document: &SurfaceDocument,
    range_start: usize,
    range_end: usize,
) -> HighlightedSource {
    HighlightedSource {
        spans: highlight_spans(source, document, range_start, range_end),
        parsed: true,
    }
}

fn highlight_spans(
    source: &str,
    document: &SurfaceDocument,
    range_start: usize,
    range_end: usize,
) -> Vec<SourceHighlightSpan> {
    let mut spans = Vec::new();
    let scan_start = source[..range_start]
        .rfind('\n')
        .map_or(0, |index| index + 1);
    let scan_end = source[range_end..]
        .find('\n')
        .map_or(source.len(), |offset| range_end + offset);
    let semantic_ranges = semantic_tokens_intersecting(document, scan_start, scan_end);
    let brace_ranges = scan_brace_ranges(source);
    let highlight_ranges = highlight_ranges_intersecting(document, scan_start, scan_end);
    let mut chars = source[scan_start..scan_end]
        .char_indices()
        .map(|(index, ch)| (scan_start + index, ch))
        .peekable();

    while let Some((index, ch)) = chars.next() {
        if let Some(range) =
            level_ascii_range_starting_at(&highlight_ranges.level_ascii_ranges, index)
        {
            let kind = if range.known {
                SourceHighlightKind::LevelCell
            } else {
                SourceHighlightKind::InvalidLevelCell
            };
            push_span(&mut spans, range.span.start, range.span.end, kind);
            skip_until(&mut chars, range.span.end);
            continue;
        }

        if let Some(range) =
            visual_ascii_color_range_starting_at(&highlight_ranges.visual_ascii_color_ranges, index)
        {
            push_colored_span(
                &mut spans,
                range.span.start,
                range.span.end,
                &range.color,
                range.transparent,
            );
            skip_until(&mut chars, range.span.end);
            continue;
        }

        if let Some(range) =
            visual_named_color_range_starting_at(&highlight_ranges.visual_named_color_ranges, index)
        {
            push_color_span(&mut spans, range.span.start, range.span.end, &range.color);
            skip_until(&mut chars, range.span.end);
            continue;
        }

        if let Some(range) =
            visual_separator_range_starting_at(&highlight_ranges.visual_separator_ranges, index)
        {
            push_span(
                &mut spans,
                range.start,
                range.end,
                SourceHighlightKind::Arrow,
            );
            skip_until(&mut chars, range.end);
            continue;
        }

        if let Some(end) = highlight_ranges.raw_range_starting_at(index) {
            if let Some(next_start) =
                next_raw_embedded_highlight_start(index, end, &highlight_ranges, &semantic_ranges)
                && next_start > index
            {
                skip_until(&mut chars, next_start);
                continue;
            }
            if next_raw_embedded_highlight_start(index, end, &highlight_ranges, &semantic_ranges)
                != Some(index)
            {
                skip_until(&mut chars, end);
                continue;
            }
        }

        if highlight_ranges.is_plain_range(index, index + ch.len_utf8()) {
            continue;
        }

        if ch == '/' && chars.peek().is_some_and(|(_, next)| *next == '/') {
            let end = source[index..]
                .find('\n')
                .map(|offset| index + offset)
                .unwrap_or(source.len());
            push_span(&mut spans, index, end, SourceHighlightKind::Comment);
            if end < source.len() {
                while chars
                    .peek()
                    .is_some_and(|(next_index, _)| *next_index <= end)
                {
                    chars.next();
                }
            } else {
                chars.by_ref().for_each(drop);
            }
            continue;
        }

        if ch == '"' || ch == '\'' {
            let quote = ch;
            let mut end = index + ch.len_utf8();
            let mut escaped = false;
            for (next_index, next_ch) in chars.by_ref() {
                end = next_index + next_ch.len_utf8();
                if escaped {
                    escaped = false;
                } else if next_ch == '\\' {
                    escaped = true;
                } else if next_ch == quote {
                    break;
                } else if next_ch == '\n' {
                    break;
                }
            }
            let token = &source[index..end];
            if !push_quoted_semantic_inner_span(&mut spans, token, index, quote, &semantic_ranges) {
                let kind = semantic_kind_at(&semantic_ranges, index, end)
                    .and_then(highlight_kind_for_semantic)
                    .unwrap_or(SourceHighlightKind::String);
                push_span(&mut spans, index, end, kind);
            }
            continue;
        }

        if let Some(end) = hex_color_end(source, index, ch) {
            push_color_span(&mut spans, index, end, &source[index..end]);
            skip_until(&mut chars, end);
            continue;
        }

        if is_number_start(source, index, ch) {
            let lexical_end = consume_while(source, index, |value| {
                value.is_ascii_digit() || matches!(value, '.' | '_' | '-')
            });
            let (end, kind) = semantic_token_starting_at(&semantic_ranges, index)
                .and_then(|token| {
                    (token.kind == SemanticKind::Number && token.end > lexical_end)
                        .then_some((token.end, SourceHighlightKind::Number))
                })
                .unwrap_or((lexical_end, SourceHighlightKind::Number));
            if highlight_ranges.is_plain_range(index, end) {
            } else {
                push_span(&mut spans, index, end, kind);
            }
            skip_until(&mut chars, end);
            continue;
        }

        if is_word_start_at(source, index, ch) {
            let end = consume_word(source, index);
            let token = &source[index..end];
            if highlight_ranges.is_plain_range(index, end) {
            } else {
                push_word(&mut spans, token, index, &semantic_ranges);
            }
            skip_until(&mut chars, end);
            continue;
        }

        if source[index..].starts_with("->") {
            push_span(&mut spans, index, index + 2, SourceHighlightKind::Arrow);
            skip_until(&mut chars, index + 2);
            continue;
        }

        if is_direction_glyph_token(source, index, ch) {
            push_span(
                &mut spans,
                index,
                index + ch.len_utf8(),
                SourceHighlightKind::Literal,
            );
            continue;
        }

        if is_operator_char(ch) {
            let end = consume_while(source, index, is_operator_char);
            push_operator_run(&mut spans, source, index, end, &brace_ranges);
            skip_until(&mut chars, end);
            continue;
        }
    }
    normalize_highlight_spans(spans)
        .into_iter()
        .filter(|span| span.end > range_start && span.start < range_end)
        .collect()
}

fn semantic_tokens_intersecting(
    document: &SurfaceDocument,
    start: usize,
    end: usize,
) -> Vec<SemanticToken> {
    document
        .semantic_tokens
        .iter()
        .filter(|token| token.span.end > start && token.span.start < end)
        .map(|token| SemanticToken {
            start: token.span.start,
            end: token.span.end,
            kind: crate::project_surface_semantic_kind(token.kind),
        })
        .collect()
}

fn highlight_ranges_intersecting(
    document: &SurfaceDocument,
    start: usize,
    end: usize,
) -> SurfaceHighlightRanges {
    let ranges = &document.highlight_ranges;
    SurfaceHighlightRanges {
        raw_ranges: ranges
            .raw_ranges
            .iter()
            .copied()
            .filter(|range| range.end > start && range.start < end)
            .collect(),
        plain_ranges: ranges
            .plain_ranges
            .iter()
            .copied()
            .filter(|range| range.end > start && range.start < end)
            .collect(),
        level_ascii_ranges: ranges
            .level_ascii_ranges
            .iter()
            .cloned()
            .filter(|range| range.span.end > start && range.span.start < end)
            .collect(),
        visual_ascii_color_ranges: ranges
            .visual_ascii_color_ranges
            .iter()
            .cloned()
            .filter(|range| range.span.end > start && range.span.start < end)
            .collect(),
        visual_named_color_ranges: ranges
            .visual_named_color_ranges
            .iter()
            .cloned()
            .filter(|range| range.span.end > start && range.span.start < end)
            .collect(),
        visual_separator_ranges: ranges
            .visual_separator_ranges
            .iter()
            .copied()
            .filter(|range| range.end > start && range.start < end)
            .collect(),
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
        normalized.push(span);
    }
    normalized
}

fn scan_brace_ranges(source: &str) -> HashMap<usize, SourceHighlightKind> {
    let mut ranges = HashMap::<usize, SourceHighlightKind>::new();
    let mut block_stack = Vec::<(usize, usize)>::new();
    let mut line_start = 0usize;

    for line in source.split_inclusive('\n') {
        let line_end = line_start + line.len();
        let content_end = line_end - usize::from(line.ends_with('\n'));
        scan_brace_line(
            source,
            line_start,
            content_end,
            &mut block_stack,
            &mut ranges,
        );
        line_start = line_end;
    }

    if line_start < source.len() {
        scan_brace_line(
            source,
            line_start,
            source.len(),
            &mut block_stack,
            &mut ranges,
        );
    }

    for (open_index, _) in block_stack {
        ranges.insert(open_index, SourceHighlightKind::InvalidBrace);
    }

    ranges
}

fn scan_brace_line(
    source: &str,
    line_start: usize,
    content_end: usize,
    block_stack: &mut Vec<(usize, usize)>,
    ranges: &mut HashMap<usize, SourceHighlightKind>,
) {
    let line = &source[line_start..content_end];
    let code_end = line_code_end(line);
    let code = &line[..code_end];
    let trimmed = code.trim();
    if trimmed.is_empty() {
        return;
    }

    let braces = line_code_braces(source, line_start, code_end);
    if braces.is_empty() {
        return;
    }

    let mut brace_index = 0usize;
    while brace_index < braces.len() {
        let (index, ch) = braces[brace_index];
        match ch {
            '{' => {
                let depth = block_stack.len();
                block_stack.push((index, depth));
            }
            '}' => {
                if let Some((open_index, depth)) = block_stack.pop() {
                    let kind = brace_highlight_kind(depth);
                    ranges.insert(open_index, kind);
                    ranges.insert(index, kind);
                } else {
                    ranges.insert(index, SourceHighlightKind::InvalidBrace);
                }
            }
            _ => {}
        }
        brace_index += 1;
    }
}

fn line_code_end(line: &str) -> usize {
    let mut quote = None::<char>;
    let mut escaped = false;
    let mut previous = None;
    for (index, ch) in line.char_indices() {
        if let Some(active) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == active {
                quote = None;
            }
        } else if ch == '"' || ch == '\'' {
            quote = Some(ch);
        } else if previous == Some('/') && ch == '/' {
            return index - 1;
        }
        previous = Some(ch);
    }
    line.len()
}

fn line_code_braces(source: &str, line_start: usize, code_end: usize) -> Vec<(usize, char)> {
    let mut braces = Vec::new();
    let mut chars = source[line_start..line_start + code_end].char_indices();
    while let Some((offset, ch)) = chars.next() {
        if ch == '"' || ch == '\'' {
            let quote = ch;
            let mut escaped = false;
            for (_, next_ch) in chars.by_ref() {
                if escaped {
                    escaped = false;
                } else if next_ch == '\\' {
                    escaped = true;
                } else if next_ch == quote {
                    break;
                }
            }
            continue;
        }
        if ch == '{' || ch == '}' {
            braces.push((line_start + offset, ch));
        }
    }
    braces
}

fn brace_highlight_kind(depth: usize) -> SourceHighlightKind {
    match depth % 6 {
        0 => SourceHighlightKind::Brace0,
        1 => SourceHighlightKind::Brace1,
        2 => SourceHighlightKind::Brace2,
        3 => SourceHighlightKind::Brace3,
        4 => SourceHighlightKind::Brace4,
        _ => SourceHighlightKind::Brace5,
    }
}

fn push_word(
    spans: &mut Vec<SourceHighlightSpan>,
    token: &str,
    token_start: usize,
    semantic_ranges: &[SemanticToken],
) {
    let parts = split_highlight_word(token);
    for part in &parts {
        if let Some(separator) = part.separator_before {
            let separator_start = token_start + part.start - separator.len();
            push_span(
                spans,
                separator_start,
                token_start + part.start,
                SourceHighlightKind::Operator,
            );
        }
        let absolute_start = token_start + part.start;
        let absolute_end = token_start + part.end;
        if let Some(kind) = semantic_kind_at(semantic_ranges, absolute_start, absolute_end)
            .and_then(highlight_kind_for_semantic)
        {
            push_span(spans, absolute_start, absolute_end, kind);
        };
    }
}

#[derive(Clone, Copy, Debug)]
struct HighlightWordPart {
    start: usize,
    end: usize,
    separator_before: Option<&'static str>,
}

fn split_highlight_word(token: &str) -> Vec<HighlightWordPart> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut separator_before = None;
    for (index, ch) in token.char_indices() {
        let separator = match ch {
            ':' => Some(":"),
            '.' => Some("."),
            '#' => Some("#"),
            _ => None,
        };
        let Some(separator) = separator else {
            continue;
        };
        if start < index {
            parts.push(HighlightWordPart {
                start,
                end: index,
                separator_before,
            });
        }
        start = index + ch.len_utf8();
        separator_before = Some(separator);
    }
    if start < token.len() {
        parts.push(HighlightWordPart {
            start,
            end: token.len(),
            separator_before,
        });
    }
    if parts.is_empty() {
        parts.push(HighlightWordPart {
            start: 0,
            end: token.len(),
            separator_before: None,
        });
    }
    parts
}

fn semantic_kind_at(ranges: &[SemanticToken], start: usize, end: usize) -> Option<SemanticKind> {
    if let Some(kind) = ranges
        .iter()
        .rev()
        .find(|range| range.start == start && range.end == end)
        .map(|range| range.kind)
    {
        return Some(kind);
    }
    ranges
        .iter()
        .rev()
        .find(|range| range.start <= start && end <= range.end)
        .map(|range| range.kind)
}

fn semantic_token_starting_at(ranges: &[SemanticToken], start: usize) -> Option<SemanticToken> {
    ranges
        .iter()
        .rev()
        .find(|range| range.start == start)
        .copied()
}

fn highlight_kind_for_semantic(kind: SemanticKind) -> Option<SourceHighlightKind> {
    match kind {
        SemanticKind::Keyword => Some(SourceHighlightKind::Keyword),
        SemanticKind::Literal => Some(SourceHighlightKind::Literal),
        SemanticKind::Binding => Some(SourceHighlightKind::Binding),
        SemanticKind::Effect => Some(SourceHighlightKind::Effect),
        SemanticKind::Emission => Some(SourceHighlightKind::Emission),
        SemanticKind::Object => Some(SourceHighlightKind::Object),
        SemanticKind::Input => Some(SourceHighlightKind::Input),
        SemanticKind::State => Some(SourceHighlightKind::State),
        SemanticKind::Group => Some(SourceHighlightKind::Group),
        SemanticKind::Mark => Some(SourceHighlightKind::Mark),
        SemanticKind::Variant => Some(SourceHighlightKind::Variant),
        SemanticKind::Condition => Some(SourceHighlightKind::Condition),
        SemanticKind::Scene => Some(SourceHighlightKind::Scene),
        SemanticKind::Theme => Some(SourceHighlightKind::Theme),
        SemanticKind::Asset => Some(SourceHighlightKind::Asset),
        SemanticKind::Setting => Some(SourceHighlightKind::Keyword),
        SemanticKind::Color => Some(SourceHighlightKind::Color),
        SemanticKind::Number => Some(SourceHighlightKind::Number),
        SemanticKind::String => Some(SourceHighlightKind::String),
    }
}

fn push_quoted_semantic_inner_span(
    spans: &mut Vec<SourceHighlightSpan>,
    token: &str,
    token_start: usize,
    quote: char,
    semantic_ranges: &[SemanticToken],
) -> bool {
    let quote_len = quote.len_utf8();
    if token.len() < quote_len * 2 || !token.starts_with(quote) || !token.ends_with(quote) {
        return false;
    }
    let inner_start = token_start + quote_len;
    let inner_end = token_start + token.len() - quote_len;
    let Some(kind) = semantic_kind_at(semantic_ranges, inner_start, inner_end)
        .and_then(highlight_kind_for_semantic)
    else {
        return false;
    };
    if kind == SourceHighlightKind::String {
        return false;
    }
    push_span(spans, token_start, token_start + token.len(), kind);
    true
}

fn visual_ascii_color_range_starting_at(
    ranges: &[SurfaceVisualAsciiColorRange],
    start: usize,
) -> Option<&SurfaceVisualAsciiColorRange> {
    ranges.iter().find(|range| range.span.start == start)
}

fn visual_named_color_range_starting_at(
    ranges: &[SurfaceVisualNamedColorRange],
    start: usize,
) -> Option<&SurfaceVisualNamedColorRange> {
    ranges.iter().find(|range| range.span.start == start)
}

fn visual_separator_range_starting_at(ranges: &[SourceSpan], start: usize) -> Option<&SourceSpan> {
    ranges.iter().find(|range| range.start == start)
}

fn level_ascii_range_starting_at(
    ranges: &[SurfaceAsciiRange],
    start: usize,
) -> Option<&SurfaceAsciiRange> {
    ranges.iter().find(|range| range.span.start == start)
}

fn next_raw_embedded_highlight_start(
    raw_start: usize,
    raw_end: usize,
    ranges: &SurfaceHighlightRanges,
    semantic_ranges: &[SemanticToken],
) -> Option<usize> {
    ranges
        .level_ascii_ranges
        .iter()
        .map(|range| range.span.start)
        .chain(
            ranges
                .visual_ascii_color_ranges
                .iter()
                .map(|range| range.span.start),
        )
        .chain(
            ranges
                .visual_named_color_ranges
                .iter()
                .map(|range| range.span.start),
        )
        .chain(
            ranges
                .visual_separator_ranges
                .iter()
                .map(|range| range.start),
        )
        .chain(semantic_ranges.iter().map(|token| token.start))
        .filter(|start| *start >= raw_start && *start < raw_end)
        .min()
}

fn hex_color_end(source: &str, index: usize, ch: char) -> Option<usize> {
    if ch != '#' {
        return None;
    }
    let mut digit_count = 0;
    let mut end = index + ch.len_utf8();
    for (offset, next) in source[end..].char_indices() {
        if !next.is_ascii_hexdigit() {
            break;
        }
        if digit_count == 8 {
            return None;
        }
        digit_count += 1;
        end = index + ch.len_utf8() + offset + next.len_utf8();
    }
    if !matches!(digit_count, 3 | 4 | 6 | 8) {
        return None;
    }
    if source[end..]
        .chars()
        .next()
        .is_some_and(|next| next == '_' || next.is_ascii_alphanumeric())
    {
        return None;
    }
    Some(end)
}

fn is_number_start(source: &str, index: usize, ch: char) -> bool {
    if ch.is_ascii_digit() {
        return true;
    }
    ch == '-'
        && source[index + ch.len_utf8()..]
            .chars()
            .next()
            .is_some_and(|next| next.is_ascii_digit())
}

fn is_word_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_word_start_at(source: &str, index: usize, ch: char) -> bool {
    is_word_start(ch) || (ch == '*' && source[index + ch.len_utf8()..].starts_with(':'))
}

fn is_word_continue(ch: char) -> bool {
    ch == '_'
        || ch == ':'
        || ch == '.'
        || ch == '#'
        || ch == '-'
        || ch == '*'
        || ch.is_ascii_alphanumeric()
}

fn consume_word(source: &str, start: usize) -> usize {
    let mut end = start;
    for (index, ch) in source[start..].char_indices() {
        let absolute = start + index;
        if ch == '-' && source[absolute..].starts_with("->") {
            break;
        }
        if !is_word_continue(ch)
            && !is_qualified_direction_glyph_continue(source, start, absolute, ch)
        {
            break;
        }
        end = absolute + ch.len_utf8();
    }
    end
}

fn is_qualified_direction_glyph_continue(
    source: &str,
    token_start: usize,
    index: usize,
    ch: char,
) -> bool {
    matches!(ch, '>' | '<' | '^' | 'v') && source[token_start..index].ends_with(':')
}

fn is_operator_char(ch: char) -> bool {
    matches!(
        ch,
        '[' | ']' | '{' | '}' | '(' | ')' | '|' | ';' | ',' | '=' | '!' | '<' | '>' | '+' | '*'
    )
}

fn is_direction_glyph_token(source: &str, index: usize, ch: char) -> bool {
    if !matches!(ch, '<' | '>' | '^') {
        return false;
    }
    let before = source[..index].chars().next_back();
    let after = source[index + ch.len_utf8()..].chars().next();
    is_direction_glyph_boundary(before) && is_direction_glyph_boundary(after)
}

fn is_direction_glyph_boundary(ch: Option<char>) -> bool {
    ch.is_none_or(|ch| {
        ch.is_whitespace() || matches!(ch, '[' | ']' | '(' | ')' | '{' | '}' | '|' | ';' | ',')
    })
}

fn push_operator_run(
    spans: &mut Vec<SourceHighlightSpan>,
    source: &str,
    start: usize,
    end: usize,
    brace_ranges: &HashMap<usize, SourceHighlightKind>,
) {
    let mut plain_start = start;
    for (offset, ch) in source[start..end].char_indices() {
        let index = start + offset;
        if let Some(kind) = brace_ranges.get(&index).copied() {
            if plain_start < index {
                push_span(spans, plain_start, index, SourceHighlightKind::Operator);
            }
            let display_kind = if kind != SourceHighlightKind::InvalidBrace
                && is_inline_selector_mark_brace(source, index, ch)
            {
                SourceHighlightKind::Mark
            } else {
                kind
            };
            push_span(spans, index, index + ch.len_utf8(), display_kind);
            plain_start = index + ch.len_utf8();
            continue;
        }
        if !is_direction_glyph_token(source, index, ch) {
            continue;
        }
        if plain_start < index {
            push_span(spans, plain_start, index, SourceHighlightKind::Operator);
        }
        push_span(
            spans,
            index,
            index + ch.len_utf8(),
            SourceHighlightKind::Literal,
        );
        plain_start = index + ch.len_utf8();
    }
    if plain_start < end {
        push_span(spans, plain_start, end, SourceHighlightKind::Operator);
    }
}

fn is_inline_selector_mark_brace(source: &str, index: usize, brace: char) -> bool {
    match brace {
        '{' => {
            is_inline_selector_mark_open(source, index)
                && inline_selector_mark_close(source, index).is_some()
        }
        '}' => matching_inline_selector_mark_open(source, index).is_some(),
        _ => false,
    }
}

fn is_inline_selector_mark_open(source: &str, index: usize) -> bool {
    let before = source[..index].chars().next_back();
    let after = source[index + 1..].chars().next();
    before.is_some_and(is_selector_token_char) && after.is_some_and(|ch| !ch.is_whitespace())
}

fn inline_selector_mark_close(source: &str, open: usize) -> Option<usize> {
    let line_end = source[open + 1..]
        .find('\n')
        .map(|offset| open + 1 + offset)
        .unwrap_or(source.len());
    for (offset, ch) in source[open + 1..line_end].char_indices() {
        let index = open + 1 + offset;
        match ch {
            '}' => return Some(index),
            '[' | ']' | '|' | ';' | ',' | '(' | ')' | '{' => return None,
            _ => {}
        }
    }
    None
}

fn matching_inline_selector_mark_open(source: &str, close: usize) -> Option<usize> {
    let line_start = source[..close]
        .rfind('\n')
        .map(|index| index + 1)
        .unwrap_or(0);
    for (offset, ch) in source[line_start..close].char_indices().rev() {
        let index = line_start + offset;
        match ch {
            '{' if is_inline_selector_mark_open(source, index) => return Some(index),
            '[' | ']' | '|' | ';' | ',' | '(' | ')' | '}' => return None,
            _ => {}
        }
    }
    None
}

fn is_selector_token_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '@' | ':' | '*')
}

fn consume_while(source: &str, start: usize, predicate: impl Fn(char) -> bool) -> usize {
    let mut end = start;
    for (index, ch) in source[start..].char_indices() {
        if !predicate(ch) {
            break;
        }
        end = start + index + ch.len_utf8();
    }
    end
}

fn skip_until(chars: &mut std::iter::Peekable<impl Iterator<Item = (usize, char)>>, end: usize) {
    while chars.peek().is_some_and(|(index, _)| *index < end) {
        chars.next();
    }
}

fn push_span(
    spans: &mut Vec<SourceHighlightSpan>,
    start: usize,
    end: usize,
    kind: SourceHighlightKind,
) {
    spans.push(SourceHighlightSpan {
        start,
        end,
        kind,
        color: None,
        transparent: false,
    });
}

fn push_color_span(spans: &mut Vec<SourceHighlightSpan>, start: usize, end: usize, color: &str) {
    spans.push(SourceHighlightSpan {
        start,
        end,
        kind: SourceHighlightKind::Color,
        color: Some(color.to_string()),
        transparent: false,
    });
}

fn push_colored_span(
    spans: &mut Vec<SourceHighlightSpan>,
    start: usize,
    end: usize,
    color: &str,
    transparent: bool,
) {
    spans.push(SourceHighlightSpan {
        start,
        end,
        kind: SourceHighlightKind::SpritePixel,
        color: Some(color.to_string()),
        transparent,
    });
}

#[cfg(test)]
mod tests {
    use super::{
        HighlightedSource, SourceHighlightKind, SourceHighlightSpan, highlight_source,
        highlight_source_range_with_document, highlight_source_with_document,
    };

    fn spans_for_text<'a>(
        source: &'a str,
        highlighted: &'a HighlightedSource,
        kind: SourceHighlightKind,
        text: &str,
    ) -> Vec<&'a SourceHighlightSpan> {
        highlighted
            .spans
            .iter()
            .filter(|span| span.kind == kind && &source[span.start..span.end] == text)
            .collect()
    }

    fn assert_span(
        source: &str,
        highlighted: &HighlightedSource,
        kind: SourceHighlightKind,
        text: &str,
    ) {
        assert!(
            !spans_for_text(source, highlighted, kind, text).is_empty(),
            "missing {kind:?} highlight for {text:?}"
        );
    }

    #[test]
    fn at_prefix_is_plain_and_the_name_uses_normal_object_highlight() {
        let source = r#"puzzle board {
layers {
objects = @Box
}
rules {
[ @Box ] -> [ @Box ]
}
}
"#;
        let highlighted = highlight_source(source);

        assert_span(source, &highlighted, SourceHighlightKind::Object, "Box");
        assert!(spans_for_text(source, &highlighted, SourceHighlightKind::Mark, "@Box").is_empty());
        let prefix = source.find("@Box").expect("at-prefixed object");
        assert!(
            highlighted
                .spans
                .iter()
                .all(|span| !(span.start <= prefix && prefix < span.end)),
            "the @ prefix must not receive a semantic highlight"
        );
    }

    #[test]
    fn highlighting_is_a_typed_utf8_span_product() {
        let source = r##"title = "Demo"
theme {
preset = "clean"
background_color = #112233
}
puzzle board {
layers {
actor = Player
}
rules {
once [ Player ] -> [ Player ]
}
}
levels {
legend {
. = empty
P = Player
}
level "start"
PX
}
"##;
        let highlighted = highlight_source(source);

        assert!(highlighted.parsed);
        assert_span(source, &highlighted, SourceHighlightKind::Keyword, "title");
        assert_span(
            source,
            &highlighted,
            SourceHighlightKind::String,
            "\"Demo\"",
        );
        assert_span(
            source,
            &highlighted,
            SourceHighlightKind::Theme,
            "\"clean\"",
        );
        assert_span(source, &highlighted, SourceHighlightKind::Object, "Player");
        assert_span(source, &highlighted, SourceHighlightKind::Arrow, "->");
        assert_span(source, &highlighted, SourceHighlightKind::LevelCell, "P");
        assert_span(
            source,
            &highlighted,
            SourceHighlightKind::InvalidLevelCell,
            "X",
        );
        let color = spans_for_text(source, &highlighted, SourceHighlightKind::Color, "#112233");
        assert_eq!(color.len(), 1);
        assert_eq!(color[0].color.as_deref(), Some("#112233"));

        for pair in highlighted.spans.windows(2) {
            assert!(
                pair[0].end <= pair[1].start,
                "highlight spans must not overlap"
            );
        }
        for span in &highlighted.spans {
            assert!(span.start < span.end && span.end <= source.len());
            assert!(source.is_char_boundary(span.start));
            assert!(source.is_char_boundary(span.end));
        }
    }

    #[test]
    fn named_level_pack_legend_punctuation_highlights_as_valid_cells() {
        let source = r#"levels microban of sokoban {
legend {
, = Floor
# = Wall
}
level "microban 1" {
##
..
,,
}
}
"#;
        let highlighted = highlight_source(source);

        for row in ["##", "..", ",,"] {
            assert_span(source, &highlighted, SourceHighlightKind::LevelCell, row);
        }
        assert!(
            highlighted
                .spans
                .iter()
                .all(|span| span.kind != SourceHighlightKind::InvalidLevelCell),
            "reserved empty and legend-backed punctuation must not be marked as invalid level cells"
        );
    }

    #[test]
    fn selector_parts_keep_parser_owned_semantic_kinds() {
        let source = r#"puzzle board {
layers {
each A:directions
}
groups {
movers = A:left
}
rules {
once [ movers | A:directions ] -> [ A:left | movers ]
}
}
"#;
        let highlighted = highlight_source(source);

        assert_span(
            source,
            &highlighted,
            SourceHighlightKind::Group,
            "directions",
        );
        assert_span(source, &highlighted, SourceHighlightKind::Variant, "left");
        assert!(
            spans_for_text(
                source,
                &highlighted,
                SourceHighlightKind::Object,
                "directions"
            )
            .is_empty()
        );
    }

    #[test]
    fn braces_encode_depth_and_invalid_state() {
        let source = "puzzle board {\nrules {\nif { flag } -> score = 1\n}\n}\n}\nscene menu {\n";
        let highlighted = highlight_source(source);

        assert_span(source, &highlighted, SourceHighlightKind::Brace0, "{");
        assert_span(source, &highlighted, SourceHighlightKind::Brace1, "{");
        assert_span(source, &highlighted, SourceHighlightKind::Brace2, "{");
        assert!(
            spans_for_text(source, &highlighted, SourceHighlightKind::InvalidBrace, "}").len() == 1
        );
        assert_span(source, &highlighted, SourceHighlightKind::InvalidBrace, "{");
    }

    #[test]
    fn all_on_condition_highlights_keywords_and_object_selectors() {
        let source = r#"puzzle board {
layers {
floor = Goal
actor = Box
}
win_conditions {
all Box on Goal
}
rules {
}
}
"#;
        let highlighted = highlight_source(source);

        assert_span(source, &highlighted, SourceHighlightKind::Keyword, "all");
        assert_span(source, &highlighted, SourceHighlightKind::Keyword, "on");
        assert_span(source, &highlighted, SourceHighlightKind::Object, "Box");
        assert_span(source, &highlighted, SourceHighlightKind::Object, "Goal");
    }

    #[test]
    fn visual_pixels_carry_color_and_transparency_as_data() {
        let source = r##"puzzle default {
layers {
objects = Box
}
sprites {
Box {
#fff #000
01
}
}
rules {
}
}
"##;
        let highlighted = highlight_source(source);
        let zero = spans_for_text(source, &highlighted, SourceHighlightKind::SpritePixel, "0");
        let one = spans_for_text(source, &highlighted, SourceHighlightKind::SpritePixel, "1");

        assert!(
            zero.iter()
                .any(|span| span.color.as_deref() == Some("#fff"))
        );
        assert!(one.iter().any(|span| span.color.as_deref() == Some("#000")));
        assert!(zero.iter().all(|span| !span.transparent));
    }

    #[test]
    fn unicode_offsets_remain_utf8_byte_offsets() {
        let source = "title = 星😀\n// 注釈😀\n";
        let highlighted = highlight_source(source);
        let comment_start = source.find("//").expect("comment start");
        let comment = highlighted
            .spans
            .iter()
            .find(|span| span.kind == SourceHighlightKind::Comment)
            .expect("comment highlight");

        assert_eq!(comment.start, comment_start);
        assert_eq!(comment.end, source.trim_end().len());
        assert_eq!(&source[comment.start..comment.end], "// 注釈😀");
    }

    #[test]
    fn adjacent_identical_highlights_are_normalized_into_runs() {
        let source = r##"puzzle default {
layers {
objects = Box
}
sprites {
Box {
#fff
00000
}
}
rules {
}
}
"##;
        let highlighted = highlight_source(source);
        let pixels = spans_for_text(
            source,
            &highlighted,
            SourceHighlightKind::SpritePixel,
            "00000",
        );

        assert_eq!(pixels.len(), 1);
        assert_eq!(pixels[0].color.as_deref(), Some("#fff"));
        assert!(!pixels[0].transparent);
    }

    #[test]
    fn range_highlighting_matches_full_highlighting_for_intersecting_spans() {
        let source = r##"title = "Demo"
puzzle board {
sprites {
Box {
#fff #000
00110
}
}
rules {
[ Box ] -> [ Box ]
}
}
// 注釈😀
"##;
        let document = crate::parse_surface_document(source);
        let full = highlight_source_with_document(source, &document);
        let start = source.find("#fff").expect("range start");
        let end = source.find("rules").expect("range end");
        let ranged = highlight_source_range_with_document(source, &document, start, end);
        let expected = full
            .spans
            .into_iter()
            .filter(|span| span.end > start && span.start < end)
            .collect::<Vec<_>>();

        assert_eq!(ranged.spans, expected);
    }

    #[test]
    fn highlighting_and_outline_share_one_surface_document() {
        let source = "puzzle board {\nrules {\n}\n}\n";
        let document = crate::parse_surface_document(source);
        let highlighted = highlight_source_with_document(source, &document);
        assert!(highlighted.parsed);

        let implementation = include_str!("highlight.rs");
        let production = implementation
            .split("#[cfg(test)]")
            .next()
            .expect("highlight production source");
        assert!(!production.contains("<span"));
        assert!(!production.contains("escape_html"));
        assert!(!production.contains("syntax-"));
    }
}
