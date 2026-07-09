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
    pub html: String,
    pub parsed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HighlightedSourceWithOutline {
    pub highlighted: HighlightedSource,
    pub outline: Vec<SourceOutlineItem>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HighlightKind {
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
}

impl HighlightKind {
    fn class_name(self) -> &'static str {
        match self {
            HighlightKind::Keyword => "syntax-keyword",
            HighlightKind::Literal => "syntax-literal",
            HighlightKind::Binding => "syntax-binding",
            HighlightKind::Effect => "syntax-effect",
            HighlightKind::Emission => "syntax-emission",
            HighlightKind::Object => "syntax-object",
            HighlightKind::Input => "syntax-input",
            HighlightKind::State => "syntax-state",
            HighlightKind::Group => "syntax-group",
            HighlightKind::Mark => "syntax-mark",
            HighlightKind::Variant => "syntax-variant",
            HighlightKind::Condition => "syntax-condition",
            HighlightKind::Scene => "syntax-scene",
            HighlightKind::Theme => "syntax-theme",
            HighlightKind::Asset => "syntax-asset",
            HighlightKind::Color => "syntax-color",
            HighlightKind::Number => "syntax-number",
            HighlightKind::String => "syntax-string",
            HighlightKind::Comment => "syntax-comment",
            HighlightKind::Operator => "syntax-operator",
            HighlightKind::Arrow => "syntax-arrow",
            HighlightKind::Brace0 => "syntax-brace-depth-0",
            HighlightKind::Brace1 => "syntax-brace-depth-1",
            HighlightKind::Brace2 => "syntax-brace-depth-2",
            HighlightKind::Brace3 => "syntax-brace-depth-3",
            HighlightKind::Brace4 => "syntax-brace-depth-4",
            HighlightKind::Brace5 => "syntax-brace-depth-5",
            HighlightKind::InvalidBrace => "syntax-brace-invalid",
            HighlightKind::LevelCell => "syntax-level-cell",
            HighlightKind::InvalidLevelCell => "syntax-level-cell-invalid",
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
        html: highlight_html(source, document),
        parsed: true,
    }
}

fn highlight_html(source: &str, document: &SurfaceDocument) -> String {
    let mut out = String::with_capacity(source.len().saturating_add(source.len() / 8));
    let semantic_ranges = crate::surface_document_semantic_tokens(document);
    let brace_ranges = scan_brace_ranges(source);
    let highlight_ranges = &document.highlight_ranges;
    let mut chars = source.char_indices().peekable();

    while let Some((index, ch)) = chars.next() {
        if let Some(range) =
            level_ascii_range_starting_at(&highlight_ranges.level_ascii_ranges, index)
        {
            let kind = if range.known {
                HighlightKind::LevelCell
            } else {
                HighlightKind::InvalidLevelCell
            };
            push_span(&mut out, kind, &source[range.span.start..range.span.end]);
            skip_until(&mut chars, range.span.end);
            continue;
        }

        if let Some(range) =
            visual_ascii_color_range_starting_at(&highlight_ranges.visual_ascii_color_ranges, index)
        {
            push_colored_text_span(
                &mut out,
                &range.color,
                &source[range.span.start..range.span.end],
                range.transparent,
            );
            skip_until(&mut chars, range.span.end);
            continue;
        }

        if let Some(range) =
            visual_named_color_range_starting_at(&highlight_ranges.visual_named_color_ranges, index)
        {
            push_color_text_span(
                &mut out,
                &range.color,
                &source[range.span.start..range.span.end],
            );
            skip_until(&mut chars, range.span.end);
            continue;
        }

        if let Some(range) =
            visual_separator_range_starting_at(&highlight_ranges.visual_separator_ranges, index)
        {
            push_span(
                &mut out,
                HighlightKind::Arrow,
                &source[range.start..range.end],
            );
            skip_until(&mut chars, range.end);
            continue;
        }

        if let Some(end) = highlight_ranges.raw_range_starting_at(index) {
            if let Some(next_start) =
                next_raw_embedded_highlight_start(index, end, highlight_ranges)
                && next_start > index
            {
                escape_html_into(&mut out, &source[index..next_start]);
                skip_until(&mut chars, next_start);
                continue;
            }
            escape_html_into(&mut out, &source[index..end]);
            skip_until(&mut chars, end);
            continue;
        }

        if highlight_ranges.is_plain_range(index, index + ch.len_utf8()) {
            escape_char_into(&mut out, ch);
            continue;
        }

        if ch == '/' && chars.peek().is_some_and(|(_, next)| *next == '/') {
            let end = source[index..]
                .find('\n')
                .map(|offset| index + offset)
                .unwrap_or(source.len());
            push_span(&mut out, HighlightKind::Comment, &source[index..end]);
            if end < source.len() {
                out.push('\n');
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
            if !push_quoted_semantic_inner_span(&mut out, token, index, quote, &semantic_ranges) {
                let kind = semantic_kind_at(&semantic_ranges, index, end)
                    .and_then(highlight_kind_for_semantic)
                    .unwrap_or(HighlightKind::String);
                push_span(&mut out, kind, token);
            }
            continue;
        }

        if let Some(end) = hex_color_end(source, index, ch) {
            push_color_span(&mut out, &source[index..end]);
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
                        .then_some((token.end, HighlightKind::Number))
                })
                .unwrap_or((lexical_end, HighlightKind::Number));
            if highlight_ranges.is_plain_range(index, end) {
                escape_html_into(&mut out, &source[index..end]);
            } else {
                push_span(&mut out, kind, &source[index..end]);
            }
            skip_until(&mut chars, end);
            continue;
        }

        if is_word_start_at(source, index, ch) {
            let end = consume_word(source, index);
            let token = &source[index..end];
            if highlight_ranges.is_plain_range(index, end) {
                escape_html_into(&mut out, token);
            } else {
                push_word(&mut out, token, index, &semantic_ranges);
            }
            skip_until(&mut chars, end);
            continue;
        }

        if source[index..].starts_with("->") {
            push_span(&mut out, HighlightKind::Arrow, &source[index..index + 2]);
            skip_until(&mut chars, index + 2);
            continue;
        }

        if is_direction_glyph_token(source, index, ch) {
            push_span(
                &mut out,
                HighlightKind::Literal,
                &source[index..index + ch.len_utf8()],
            );
            continue;
        }

        if is_operator_char(ch) {
            let end = consume_while(source, index, is_operator_char);
            push_operator_run(&mut out, source, index, end, &brace_ranges);
            skip_until(&mut chars, end);
            continue;
        }

        escape_char_into(&mut out, ch);
    }

    if source.ends_with('\n') {
        out.push(' ');
    }
    out
}

fn scan_brace_ranges(source: &str) -> HashMap<usize, HighlightKind> {
    let mut ranges = HashMap::<usize, HighlightKind>::new();
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
        ranges.insert(open_index, HighlightKind::InvalidBrace);
    }

    ranges
}

fn scan_brace_line(
    source: &str,
    line_start: usize,
    content_end: usize,
    block_stack: &mut Vec<(usize, usize)>,
    ranges: &mut HashMap<usize, HighlightKind>,
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
                    ranges.insert(index, HighlightKind::InvalidBrace);
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

fn brace_highlight_kind(depth: usize) -> HighlightKind {
    match depth % 6 {
        0 => HighlightKind::Brace0,
        1 => HighlightKind::Brace1,
        2 => HighlightKind::Brace2,
        3 => HighlightKind::Brace3,
        4 => HighlightKind::Brace4,
        _ => HighlightKind::Brace5,
    }
}

fn push_word(out: &mut String, token: &str, token_start: usize, semantic_ranges: &[SemanticToken]) {
    let parts = split_highlight_word(token);
    for part in &parts {
        if let Some(separator) = part.separator_before {
            push_span(out, HighlightKind::Operator, separator);
        }
        let absolute_start = token_start + part.start;
        let absolute_end = token_start + part.end;
        let text = &token[part.start..part.end];
        if let Some(kind) = semantic_kind_at(semantic_ranges, absolute_start, absolute_end)
            .and_then(highlight_kind_for_semantic)
        {
            push_span(out, kind, text);
        } else {
            escape_html_into(out, text);
        };
    }
    if let Some(last) = parts.last()
        && last.end < token.len()
    {
        escape_html_into(out, &token[last.end..]);
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

fn highlight_kind_for_semantic(kind: SemanticKind) -> Option<HighlightKind> {
    match kind {
        SemanticKind::Keyword => Some(HighlightKind::Keyword),
        SemanticKind::Literal => Some(HighlightKind::Literal),
        SemanticKind::Binding => Some(HighlightKind::Binding),
        SemanticKind::Effect => Some(HighlightKind::Effect),
        SemanticKind::Emission => Some(HighlightKind::Emission),
        SemanticKind::Object => Some(HighlightKind::Object),
        SemanticKind::Input => Some(HighlightKind::Input),
        SemanticKind::State => Some(HighlightKind::State),
        SemanticKind::Group => Some(HighlightKind::Group),
        SemanticKind::Mark => Some(HighlightKind::Mark),
        SemanticKind::Variant => Some(HighlightKind::Variant),
        SemanticKind::Condition => Some(HighlightKind::Condition),
        SemanticKind::Scene => Some(HighlightKind::Scene),
        SemanticKind::Theme => Some(HighlightKind::Theme),
        SemanticKind::Asset => Some(HighlightKind::Asset),
        SemanticKind::Setting => Some(HighlightKind::Keyword),
        SemanticKind::Color => Some(HighlightKind::Color),
        SemanticKind::Number => Some(HighlightKind::Number),
        SemanticKind::String => Some(HighlightKind::String),
    }
}

fn push_quoted_semantic_inner_span(
    out: &mut String,
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
    if kind == HighlightKind::String {
        return false;
    }
    escape_html_into(out, &token[..quote_len]);
    push_span(out, kind, &token[quote_len..token.len() - quote_len]);
    escape_html_into(out, &token[token.len() - quote_len..]);
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
    ch == '@' || ch == '_' || ch.is_ascii_alphabetic()
}

fn is_word_start_at(source: &str, index: usize, ch: char) -> bool {
    is_word_start(ch) || (ch == '*' && source[index + ch.len_utf8()..].starts_with(':'))
}

fn is_word_continue(ch: char) -> bool {
    ch == '@'
        || ch == '_'
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
    out: &mut String,
    source: &str,
    start: usize,
    end: usize,
    brace_ranges: &HashMap<usize, HighlightKind>,
) {
    let mut plain_start = start;
    for (offset, ch) in source[start..end].char_indices() {
        let index = start + offset;
        if let Some(kind) = brace_ranges.get(&index).copied() {
            if plain_start < index {
                push_span(out, HighlightKind::Operator, &source[plain_start..index]);
            }
            let display_kind = if kind != HighlightKind::InvalidBrace
                && is_inline_selector_mark_brace(source, index, ch)
            {
                HighlightKind::Mark
            } else {
                kind
            };
            push_span(out, display_kind, &source[index..index + ch.len_utf8()]);
            plain_start = index + ch.len_utf8();
            continue;
        }
        if !is_direction_glyph_token(source, index, ch) {
            continue;
        }
        if plain_start < index {
            push_span(out, HighlightKind::Operator, &source[plain_start..index]);
        }
        push_span(
            out,
            HighlightKind::Literal,
            &source[index..index + ch.len_utf8()],
        );
        plain_start = index + ch.len_utf8();
    }
    if plain_start < end {
        push_span(out, HighlightKind::Operator, &source[plain_start..end]);
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

fn skip_until(chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>, end: usize) {
    while chars.peek().is_some_and(|(index, _)| *index < end) {
        chars.next();
    }
}

fn push_span(out: &mut String, kind: HighlightKind, text: &str) {
    out.push_str("<span class=\"");
    out.push_str(kind.class_name());
    out.push_str("\">");
    escape_html_into(out, text);
    out.push_str("</span>");
}

fn push_color_span(out: &mut String, color: &str) {
    push_color_text_span(out, color, color);
}

fn push_color_text_span(out: &mut String, color: &str, text: &str) {
    out.push_str("<span class=\"");
    out.push_str(HighlightKind::Color.class_name());
    out.push_str("\" style=\"--syntax-color-token: ");
    out.push_str(color);
    out.push_str("\">");
    escape_html_into(out, text);
    out.push_str("</span>");
}

fn push_colored_text_span(out: &mut String, color: &str, text: &str, transparent: bool) {
    out.push_str("<span class=\"syntax-sprite-pixel");
    if transparent {
        out.push_str(" is-transparent");
    }
    out.push_str("\" style=\"--syntax-sprite-pixel-color: ");
    out.push_str(color);
    out.push_str("\">");
    escape_html_into(out, text);
    out.push_str("</span>");
}

fn escape_html_into(out: &mut String, text: &str) {
    for ch in text.chars() {
        escape_char_into(out, ch);
    }
}

fn escape_char_into(out: &mut String, ch: char) {
    match ch {
        '&' => out.push_str("&amp;"),
        '<' => out.push_str("&lt;"),
        '>' => out.push_str("&gt;"),
        '"' => out.push_str("&quot;"),
        _ => out.push(ch),
    }
}

#[cfg(test)]
mod tests {
    use super::{highlight_source, highlight_source_with_document};

    #[test]
    fn highlight_keeps_surface_projection_lenient_for_unowned_headers() {
        let highlighted =
            highlight_source("puzzle board {\n__invalid_unowned_surface_node__ {\n}\n}\n");

        assert!(highlighted.parsed);
        assert!(
            highlighted
                .html
                .contains("__invalid_unowned_surface_node__")
        );
    }

    #[test]
    fn renders_parser_semantic_tokens_without_local_symbol_classification() {
        let source = r#"
title = highlight_symbols

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
P
}
"#;
        crate::parse_game2d(source).unwrap();
        let highlighted = highlight_source(source);

        assert!(highlighted.parsed);
        assert!(highlighted.html.contains("syntax-keyword\">puzzle"));
        assert!(highlighted.html.contains("syntax-object\">Player"));
        assert!(highlighted.html.contains("syntax-arrow\">-&gt;</span>"));
    }

    #[test]
    fn selectors_render_parser_semantic_kinds_without_tag_slots() {
        let highlighted = highlight_source(
            r#"
title = selector_parser_colors

puzzle board {
layers {
each A:directions
}
groups {
movers = A:left
}
rules {
once [ movers | A:directions ] -> [ A:left | movers ]
}
levels {
legend {
. = empty
L = A:left
}
level "start" {
.
}
}
}
"#,
        );

        assert!(highlighted.parsed);
        assert!(!highlighted.html.contains("syntax-tag-"));
        assert!(highlighted.html.contains("syntax-group\">directions"));
        assert!(highlighted.html.contains("syntax-variant\">left"));
        assert!(!highlighted.html.contains("syntax-object\">directions"));
    }

    #[test]
    fn teneten_group_rhs_flows_from_surface_document_to_highlight_html() {
        let source = r#"
title = group_rhs_highlight

puzzle board {
tags {
D = F B
}
layers {
You:D Crate Ball Wall Fly Headlong TimeMachine:D
}
groups {
player = You:D
object = player Crate Ball Wall Fly Headlong TimeMachine:D
}
rules {
}
on_level_start {
}
levels {
legend {
. = empty
Y = You:F
}
level "start" {
.
}
}
}
"#;
        crate::parse_game2d(source).unwrap();
        let document = crate::parse_surface_document(source);
        let tokens = crate::surface_document_semantic_tokens(&document);
        let object_line_start = source
            .find("object = player Crate Ball Wall Fly Headlong TimeMachine:D")
            .unwrap();
        let structural_keyword_starts = [
            ("layers", source.find("layers {").unwrap()),
            ("groups", source.find("groups {").unwrap()),
            ("rules", source.find("rules {").unwrap()),
            ("on_level_start", source.find("on_level_start {").unwrap()),
            ("levels", source.find("levels {").unwrap()),
            ("legend", source.find("legend {").unwrap()),
        ];

        for (text, kind) in [
            ("layers", crate::SemanticKind::Keyword),
            ("groups", crate::SemanticKind::Keyword),
            ("rules", crate::SemanticKind::Keyword),
            ("on_level_start", crate::SemanticKind::Keyword),
            ("levels", crate::SemanticKind::Keyword),
            ("legend", crate::SemanticKind::Keyword),
            ("object", crate::SemanticKind::Group),
            ("player", crate::SemanticKind::Group),
            ("Crate", crate::SemanticKind::Object),
            ("Ball", crate::SemanticKind::Object),
            ("Wall", crate::SemanticKind::Object),
            ("Fly", crate::SemanticKind::Object),
            ("Headlong", crate::SemanticKind::Object),
            ("TimeMachine", crate::SemanticKind::Object),
            ("D", crate::SemanticKind::Group),
        ] {
            let search_start = structural_keyword_starts
                .iter()
                .find_map(|(keyword, start)| (*keyword == text).then_some(*start))
                .unwrap_or(object_line_start);
            let start = search_start + source[search_start..].find(text).unwrap();
            assert!(
                tokens.iter().any(|token| {
                    token.start == start && token.end == start + text.len() && token.kind == kind
                }),
                "missing parser surface semantic token for {text} as {kind:?}"
            );
        }

        let highlighted = highlight_source_with_document(source, &document);
        assert!(highlighted.parsed);
        assert!(highlighted.html.contains(
            "<span class=\"syntax-group\">object</span> <span class=\"syntax-operator\">=</span> <span class=\"syntax-group\">player</span> <span class=\"syntax-object\">Crate</span>"
        ));
        assert!(highlighted.html.contains(
            "<span class=\"syntax-object\">TimeMachine</span><span class=\"syntax-operator\">:</span><span class=\"syntax-group\">D</span>"
        ));
        for keyword in [
            "layers",
            "groups",
            "rules",
            "on_level_start",
            "levels",
            "legend",
        ] {
            assert!(
                highlighted
                    .html
                    .contains(&format!("<span class=\"syntax-keyword\">{keyword}</span>")),
                "missing highlighted structural keyword {keyword}"
            );
        }
    }

    #[test]
    fn renders_authoring_schema_surface_roles_from_universal_nodes() {
        let source = r##"
title = universal_node_highlight

theme {
preset = "clean"
background_color = #112233
}

puzzle main {
render {
tween = true
tween_duration = 90ms
}
}

sounds {
sfx clear { seed = 17551700; type = puzzlescript }
}

input_buffer {
queue_during_wait = false
}

assets {
"game.css"
}

puzzle board {
tags {
D = F
}
layers {
Ink:D
}
sprites {
Ink:F
colors #fff
shape __ps_shape_Ink_F
0
}
rules {
}
levels {
legend {
. = empty
I = Ink:F
}
level "start" {
I
}
}
render {
cell_size = 64
grid {
type = "all_cells"
}
}
}

puzzle3 board3 {
render {
shade = true
camera {
yaw = 90
interactive_look = true
}
grid {
type = "occupied_cells"
}
pixelate {
enabled = true
scale = 4
smoothing = false
}
}
}
"##;
        let highlighted = highlight_source(source);

        for keyword in [
            "theme",
            "tween",
            "sounds",
            "sfx",
            "input_buffer",
            "assets",
            "render",
            "grid",
            "preset",
            "background_color",
            "tween",
            "tween_duration",
            "seed",
            "type",
            "queue_during_wait",
            "cell_size",
            "type",
            "shade",
            "camera",
            "yaw",
            "interactive_look",
            "pixelate",
            "enabled",
            "scale",
            "smoothing",
        ] {
            assert!(
                highlighted
                    .html
                    .contains(&format!("<span class=\"syntax-keyword\">{keyword}</span>")),
                "missing schema-projected keyword/setting highlight for {keyword}"
            );
        }

        for (class, text) in [
            ("syntax-theme", "clean"),
            ("syntax-asset", "clear"),
            ("syntax-string", "puzzlescript"),
            ("syntax-object", "Ink"),
            ("syntax-group", "F"),
            ("syntax-asset", "__ps_shape_Ink_F"),
            ("syntax-literal", "false"),
            ("syntax-literal", "all_cells"),
            ("syntax-literal", "occupied_cells"),
            ("syntax-literal", "true"),
            ("syntax-number", "90ms"),
            ("syntax-number", "17551700"),
            ("syntax-number", "90"),
            ("syntax-number", "4"),
            ("syntax-number", "64"),
        ] {
            assert!(
                highlighted
                    .html
                    .contains(&format!("<span class=\"{class}\">{text}</span>")),
                "missing schema-projected {class} highlight for {text}"
            );
        }
        assert!(
            highlighted
                .html
                .contains("<span class=\"syntax-string\">&quot;game.css&quot;</span>")
        );
        assert!(
            highlighted
                .html
                .contains("syntax-color\" style=\"--syntax-color-token: #112233\">#112233")
        );
        assert!(
            highlighted
                .html
                .contains("syntax-color\" style=\"--syntax-color-token: #fff\">#fff")
        );
    }

    #[test]
    fn renders_parser_owned_source_tree_headers_from_universal_blocks() {
        let highlighted = highlight_source(
            r#"
puzzle board {
tags {
N = 0
}
layers {
Count:N
Moment:directions
}
sprites {
Count:0
transparent
Moment:directions
transparent
}
rules {
routine BallPush once {
[ Count:0 ] -> [ Count:0 ]
}
}
levels {
legend {
. = empty
C = Count:0
}
level "start"
C
}
}
"#,
        );

        assert!(
            highlighted.html.contains(
                "<span class=\"syntax-keyword\">routine</span> <span class=\"syntax-effect\">BallPush</span> <span class=\"syntax-keyword\">once</span>"
            ),
            "routine header spans must come from the parser-owned source-tree header surface"
        );
        assert!(
            highlighted.html.contains(
                "<span class=\"syntax-asset\">Count</span><span class=\"syntax-operator\">:</span><span class=\"syntax-group\">0</span>"
            ),
            "visual selector tags must be projected from selector structure, including digit-start values"
        );
        assert!(
            highlighted.html.contains(
                "<span class=\"syntax-asset\">Moment</span><span class=\"syntax-operator\">:</span><span class=\"syntax-group\">directions</span>"
            ),
            "visual selector tags must use the same selector-part path for named tag sets"
        );
    }

    #[test]
    fn highlight_renderer_consumes_surface_document_not_source_scanner() {
        let source = include_str!("highlight.rs");
        let forbidden_fragments = [
            ["scan_source", "_context"],
            ["Source", "Context"],
            ["Source", "ContextLine"],
            ["Source", "Scope::"],
            ["strip_line", "_comment"],
            ["scan_level", "_ascii"],
            ["scan_", "visual"],
            ["visual_", "highlight"],
            ["VisualSprite", "PixelScan"],
            ["LevelAscii", "Scan"],
        ];
        for parts in forbidden_fragments {
            let forbidden = parts.concat();
            assert!(
                !source.contains(&forbidden),
                "highlight.rs must render SurfaceDocument products, not recover source grammar via {forbidden}"
            );
        }
    }

    #[test]
    fn highlight_with_outline_reuses_same_surface_document() {
        let source = include_str!("highlight.rs");
        let function_start = source
            .find("pub fn highlight_source_with_outline")
            .expect("highlight_source_with_outline function");
        let function_end = source[function_start..]
            .find("fn highlight_source_with_document")
            .expect("highlight_source_with_document function");
        let function = &source[function_start..function_start + function_end];
        let required = "source_outline_from_document(&document)";
        assert!(
            function.contains(required),
            "highlight_source_with_outline must derive outline from the same SurfaceDocument"
        );
        let forbidden_fragments: &[&[&str]] = &[
            &["outline: crate::source_", "outline(source)"],
            &["let highlighted = highlight_", "source(source);"],
        ];
        for parts in forbidden_fragments {
            let forbidden = parts.concat();
            assert!(
                !function.contains(&forbidden),
                "highlight_source_with_outline must not trigger a second surface parse via {forbidden}"
            );
        }
    }

    #[test]
    fn renders_braces_by_depth_and_marks_unmatched_braces() {
        let highlighted = highlight_source(
            r#"
puzzle board {
rules {
if { flag } -> score = 1
}
}
}
scene menu {
"{ignored string}"
// {ignored comment}
layout {
"#,
        );

        assert!(highlighted.html.contains("syntax-brace-depth-0\">{</span>"));
        assert!(highlighted.html.contains("syntax-brace-depth-1\">{</span>"));
        assert!(highlighted.html.contains("syntax-brace-depth-2\">{</span>"));
        assert!(highlighted.html.contains("syntax-brace-invalid\">}</span>"));
        assert!(highlighted.html.contains("syntax-brace-invalid\">{</span>"));
    }

    #[test]
    fn renders_level_cells_from_legend_context() {
        let highlighted = highlight_source(
            r#"
title = level_cell_highlight

puzzle board {
layers {
actor = Player
}
rules {
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
"#,
        );

        assert!(highlighted.html.contains("syntax-level-cell\">P</span>"));
        assert!(
            highlighted
                .html
                .contains("syntax-level-cell-invalid\">X</span>")
        );
    }

    #[test]
    fn renders_visual_color_tokens_and_sprite_pixels() {
        let highlighted = highlight_source(
            r##"
title = sprite_pixel_highlight

puzzle default {
layers {
objects = Box
}
sprites {
Box {
#fff #000
01
>
10
}

Background
#90ee90 #008000
500ms
11111
01111
>
10111
11111
>

sprite {
selector = Box
colors = #123456 #abcdef
shape =
01
>
10
}
}
rules {
}
levels {
legend {
. = empty
B = Box
}
level "start"
B
}
}
"##,
        );

        assert!(
            highlighted
                .html
                .contains("syntax-color\" style=\"--syntax-color-token: #fff\">#fff")
        );
        assert!(highlighted.html.contains(
            "syntax-sprite-pixel\" style=\"--syntax-sprite-pixel-color: #fff\">0</span>"
        ));
        assert!(highlighted.html.contains(
            "syntax-sprite-pixel\" style=\"--syntax-sprite-pixel-color: #000\">1</span>"
        ));
        assert!(highlighted.html.contains(
            "syntax-sprite-pixel\" style=\"--syntax-sprite-pixel-color: #123456\">0</span>"
        ));
        assert!(highlighted.html.contains(
            "syntax-sprite-pixel\" style=\"--syntax-sprite-pixel-color: #abcdef\">1</span>"
        ));
        assert!(highlighted.html.contains(
            "syntax-sprite-pixel\" style=\"--syntax-sprite-pixel-color: #90ee90\">0</span>"
        ));
        assert!(highlighted.html.contains(
            "syntax-sprite-pixel\" style=\"--syntax-sprite-pixel-color: #008000\">1</span>"
        ));
        assert!(
            highlighted
                .html
                .contains("<span class=\"syntax-arrow\">&gt;</span>")
        );
    }

    #[test]
    fn renders_hex_colors_independently_of_semantic_types() {
        let highlighted =
            highlight_source("theme {\npreset = \"clean\"\nbackground_color = #112233\n}\n");

        assert!(
            highlighted
                .html
                .contains("syntax-color\" style=\"--syntax-color-token: #112233\">#112233")
        );
    }

    #[test]
    fn appends_space_for_trailing_newline_to_keep_empty_final_line_visible() {
        let highlighted = highlight_source("title = trailing_newline\n");

        assert!(highlighted.html.ends_with(' '));
    }
}
