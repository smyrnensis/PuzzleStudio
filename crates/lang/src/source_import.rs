use crate::normalize_virtual_import_path;
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceImportReference {
    pub range: SourceImportRange,
    pub path_range: SourceImportRange,
    pub raw_path: String,
    pub resolved_path: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceImportRange {
    pub start: usize,
    pub end: usize,
}

pub(crate) fn source_import_reference_at(
    source: &str,
    document_path: &str,
    cursor: usize,
) -> Option<SourceImportReference> {
    let cursor = cursor.min(source.len());
    let line_start = source[..cursor].rfind('\n').map_or(0, |index| index + 1);
    let line_end = source[cursor..]
        .find('\n')
        .map_or(source.len(), |offset| cursor + offset);
    let line = &source[line_start..line_end];
    let code = crate::source::strip_line_comment(line);
    let tokens = crate::source::split_header_tokens(code.trim());
    if !matches!(tokens.as_slice(), ["import", _]) {
        return None;
    }
    let token = tokens[1];
    let raw_path = token.strip_prefix('"')?.strip_suffix('"')?;
    let quote_offset = code.find(token)?;
    let start = line_start + quote_offset;
    let end = start + token.len();
    if cursor < start || cursor > end {
        return None;
    }
    let base = Path::new(document_path)
        .parent()
        .unwrap_or_else(|| Path::new(""));
    let resolved = normalize_virtual_import_path(&base.join(raw_path));
    Some(SourceImportReference {
        range: SourceImportRange { start, end },
        path_range: SourceImportRange {
            start: start + 1,
            end: end - 1,
        },
        raw_path: raw_path.to_string(),
        resolved_path: resolved.to_string_lossy().into_owned(),
    })
}
