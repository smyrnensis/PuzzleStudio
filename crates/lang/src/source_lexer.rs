//! Lossless context-free tokens emitted by the parser frontend lexer.
//!
//! This module recognizes source spelling once, while `SurfaceSourceScan` is
//! scanning a changed line. Editor products may map these typed facts, but must
//! never recover them from source text.

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SourceLexicalFact {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) kind: SourceLexicalKind,
    pub(crate) brace_disposition: Option<super::SourceBraceDisposition>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum SourceLexicalKind {
    Word,
    Number,
    String,
    Comment,
    Color(String),
    Arrow,
    Operator,
    Brace(SourceBraceKind),
    Plain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum SourceBraceKind {
    Open,
    Close,
}

pub(crate) fn scan_source_line_lexical_facts(
    line: &str,
    line_offset: usize,
) -> Vec<SourceLexicalFact> {
    let mut facts = Vec::new();
    let mut index = 0usize;

    while index < line.len() {
        let ch = line[index..].chars().next().expect("character at boundary");
        let ch_end = index + ch.len_utf8();

        if ch.is_whitespace() {
            index = ch_end;
            continue;
        }

        if line[index..].starts_with("//") {
            push(
                &mut facts,
                line_offset + index..line_offset + line.len(),
                SourceLexicalKind::Comment,
            );
            break;
        }

        if ch == '"' {
            let end = quoted_end(line, index);
            push(
                &mut facts,
                line_offset + index..line_offset + end,
                SourceLexicalKind::String,
            );
            index = end;
            continue;
        }

        if let Some(end) = hex_color_end(line, index) {
            push(
                &mut facts,
                line_offset + index..line_offset + end,
                SourceLexicalKind::Color(line[index..end].to_string()),
            );
            index = end;
            continue;
        }

        if is_number_start(line, index, ch) {
            let end = consume_while(line, index, |value| {
                value.is_ascii_digit() || matches!(value, '.' | '_' | '-')
            });
            push(
                &mut facts,
                line_offset + index..line_offset + end,
                SourceLexicalKind::Number,
            );
            index = end;
            continue;
        }

        if is_word_start_at(line, index, ch) {
            let end = consume_word(line, index);
            push_word_parts(&mut facts, line, line_offset, index, end);
            index = end;
            continue;
        }

        if line[index..].starts_with("->") {
            push(
                &mut facts,
                line_offset + index..line_offset + index + 2,
                SourceLexicalKind::Arrow,
            );
            index += 2;
            continue;
        }

        if matches!(ch, '{' | '}') {
            push(
                &mut facts,
                line_offset + index..line_offset + ch_end,
                SourceLexicalKind::Brace(if ch == '{' {
                    SourceBraceKind::Open
                } else {
                    SourceBraceKind::Close
                }),
            );
            index = ch_end;
            continue;
        }

        if is_operator_char(ch) {
            let end = consume_while(line, index, |value| {
                is_operator_char(value) && !matches!(value, '{' | '}')
            });
            push(
                &mut facts,
                line_offset + index..line_offset + end,
                SourceLexicalKind::Operator,
            );
            index = end;
            continue;
        }

        push(
            &mut facts,
            line_offset + index..line_offset + ch_end,
            SourceLexicalKind::Plain,
        );
        index = ch_end;
    }

    facts
}

pub(crate) fn shift_lexical_facts(facts: &mut [SourceLexicalFact], threshold: usize, delta: i64) {
    for fact in facts {
        if fact.start >= threshold {
            fact.start = shift_offset(fact.start, delta);
            fact.end = shift_offset(fact.end, delta);
        }
    }
}

fn push(facts: &mut Vec<SourceLexicalFact>, span: std::ops::Range<usize>, kind: SourceLexicalKind) {
    if !span.is_empty() {
        facts.push(SourceLexicalFact {
            start: span.start,
            end: span.end,
            kind,
            brace_disposition: None,
        });
    }
}

fn quoted_end(line: &str, start: usize) -> usize {
    let mut escaped = false;
    for (offset, ch) in line[start + 1..].char_indices() {
        let end = start + 1 + offset + ch.len_utf8();
        if escaped {
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            return end;
        }
    }
    line.len()
}

fn hex_color_end(line: &str, start: usize) -> Option<usize> {
    if !line[start..].starts_with('#') {
        return None;
    }
    let digit_start = start + 1;
    let mut end = digit_start;
    let mut count = 0usize;
    for (offset, ch) in line[digit_start..].char_indices() {
        if !ch.is_ascii_hexdigit() || count == 8 {
            break;
        }
        count += 1;
        end = digit_start + offset + ch.len_utf8();
    }
    if !matches!(count, 3 | 4 | 6 | 8) {
        return None;
    }
    if line[end..]
        .chars()
        .next()
        .is_some_and(|next| next == '_' || next.is_ascii_alphanumeric())
    {
        return None;
    }
    Some(end)
}

fn is_number_start(line: &str, index: usize, ch: char) -> bool {
    ch.is_ascii_digit()
        || (ch == '-'
            && line[index + 1..]
                .chars()
                .next()
                .is_some_and(|next| next.is_ascii_digit()))
}

fn is_word_start_at(line: &str, index: usize, ch: char) -> bool {
    ch == '_'
        || ch.is_ascii_alphabetic()
        || (ch == '*' && line[index + ch.len_utf8()..].starts_with(':'))
}

fn consume_word(line: &str, start: usize) -> usize {
    let mut end = start;
    for (offset, ch) in line[start..].char_indices() {
        let index = start + offset;
        if ch == '-' && line[index..].starts_with("->") {
            break;
        }
        let qualified_glyph =
            matches!(ch, '>' | '<' | '^' | 'v') && line[start..index].ends_with(':');
        if !(ch == '_'
            || matches!(ch, ':' | '.' | '#' | '-' | '*')
            || ch.is_ascii_alphanumeric()
            || qualified_glyph)
        {
            break;
        }
        end = index + ch.len_utf8();
    }
    end
}

fn push_word_parts(
    facts: &mut Vec<SourceLexicalFact>,
    line: &str,
    line_offset: usize,
    start: usize,
    end: usize,
) {
    let mut part_start = start;
    for (relative, ch) in line[start..end].char_indices() {
        if !matches!(ch, ':' | '.' | '#') {
            continue;
        }
        let separator = start + relative;
        push(
            facts,
            line_offset + part_start..line_offset + separator,
            SourceLexicalKind::Word,
        );
        push(
            facts,
            line_offset + separator..line_offset + separator + ch.len_utf8(),
            SourceLexicalKind::Operator,
        );
        part_start = separator + ch.len_utf8();
    }
    push(
        facts,
        line_offset + part_start..line_offset + end,
        SourceLexicalKind::Word,
    );
}

fn is_operator_char(ch: char) -> bool {
    matches!(
        ch,
        '[' | ']' | '(' | ')' | '|' | ';' | ',' | '=' | '!' | '<' | '>' | '+' | '*'
    )
}

fn consume_while(line: &str, start: usize, predicate: impl Fn(char) -> bool) -> usize {
    let mut end = start;
    for (offset, ch) in line[start..].char_indices() {
        if !predicate(ch) {
            break;
        }
        end = start + offset + ch.len_utf8();
    }
    end
}

fn shift_offset(value: usize, delta: i64) -> usize {
    usize::try_from(value as i64 + delta).expect("incremental lexical offset underflow")
}

#[cfg(test)]
mod tests {
    #[test]
    fn only_the_incremental_source_scanner_may_create_lexical_facts() {
        let source_scanner = include_str!("source.rs");
        assert_eq!(
            source_scanner
                .matches("scan_source_line_lexical_facts")
                .count(),
            1
        );
        for consumer in [
            include_str!("highlight.rs"),
            include_str!("source_lexical_product.rs"),
            include_str!("source_analysis.rs"),
            include_str!("source_outline.rs"),
        ] {
            assert!(!consumer.contains("scan_source_line_lexical_facts"));
        }
    }
}
