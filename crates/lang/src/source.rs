use crate::DiagnosticReport;
use crate::syntax::is_puzzle_lifecycle_block;

pub(crate) fn logical_lines(source: &str) -> Result<Vec<String>, DiagnosticReport> {
    let mut lines = Vec::new();
    let mut preserve_level_blanks = false;
    let mut level_brace_depth = 0i32;
    let mut level_end_depth = None::<usize>;
    let raw_lines = source
        .lines()
        .map(|raw_line| strip_line_comment(raw_line).trim().to_string())
        .collect::<Vec<_>>();
    let raw_lines = expand_structural_sugar(&raw_lines)?;

    for index in 0..raw_lines.len() {
        let line = raw_lines[index].as_str();
        if line.is_empty() {
            if preserve_level_blanks {
                lines.push(String::new());
            }
            continue;
        }

        let tokens = split_header_tokens(line);
        if is_levels_header(&tokens) {
            preserve_level_blanks = true;
            level_end_depth = Some(1);
        } else if (is_levels_header(&tokens) || matches!(tokens.as_slice(), ["level", ..]))
            && line.ends_with('{')
        {
            preserve_level_blanks = true;
            level_brace_depth = 0;
        }
        if let Some(depth) = &mut level_end_depth {
            if !is_levels_header(&tokens) {
                if line.ends_with('{') {
                    *depth += 1;
                }
                if line == "}" {
                    *depth = depth.saturating_sub(1);
                }
            }
        }
        if preserve_level_blanks {
            level_brace_depth += line.chars().filter(|ch| *ch == '{').count() as i32;
            level_brace_depth -= line.chars().filter(|ch| *ch == '}').count() as i32;
        }
        lines.push(line.to_string());
        if preserve_level_blanks && level_brace_depth <= 0 && level_end_depth.is_none() {
            preserve_level_blanks = false;
        }
        if level_end_depth == Some(0) {
            preserve_level_blanks = false;
            level_end_depth = None;
        }
    }
    let lines = normalize_brace_blocks(&lines)?;
    Ok(lines)
}

pub(crate) fn strip_line_comment(line: &str) -> &str {
    let mut in_string = false;
    let mut escaped = false;
    let mut previous = None;
    for (index, ch) in line.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
        } else if ch == '"' {
            in_string = true;
        } else if previous == Some('/') && ch == '/' {
            return &line[..index - 1];
        }
        previous = Some(ch);
    }
    line
}

fn expand_structural_sugar(lines: &[String]) -> Result<Vec<String>, DiagnosticReport> {
    let mut expanded = Vec::new();
    let mut block_stack = Vec::<String>::new();

    for line in lines {
        if line.is_empty() {
            expanded.push(String::new());
            continue;
        }

        let split_semicolons = !block_stack.iter().any(|block| ascii_sensitive_block(block));
        if !split_semicolons && ascii_row_contains_brace(line) {
            return Err(parse_error(line, "ASCII rows cannot contain braces"));
        }
        for piece in split_structural_line(line, split_semicolons)? {
            update_structural_block_stack(&piece, &mut block_stack);
            expanded.push(piece);
        }
    }

    Ok(expanded)
}

fn ascii_sensitive_block(block: &str) -> bool {
    matches!(block, "levels" | "levels3" | "sprites" | "sprites3" | "map")
}

fn ascii_row_contains_brace(line: &str) -> bool {
    line.contains(['{', '}']) && !line.trim_end().ends_with('{') && line.trim() != "}"
}

fn update_structural_block_stack(line: &str, stack: &mut Vec<String>) {
    if line == "}" {
        stack.pop();
        return;
    }
    if !line.ends_with('{') {
        return;
    }
    let tokens = split_header_tokens(line);
    if let Some(first) = tokens.first() {
        stack.push((*first).to_string());
    }
}

fn split_structural_line(
    line: &str,
    split_semicolons: bool,
) -> Result<Vec<String>, DiagnosticReport> {
    let mut pieces = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut escaped = false;
    let mut square_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut inline_brace_depth = 0usize;

    for (index, ch) in line.char_indices() {
        if in_string {
            current.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        if ch == '"' {
            in_string = true;
            current.push(ch);
            continue;
        }

        if inline_brace_depth > 0 {
            current.push(ch);
            if ch == '{' {
                inline_brace_depth += 1;
            } else if ch == '}' {
                inline_brace_depth = inline_brace_depth.saturating_sub(1);
            }
            continue;
        }

        match ch {
            '[' => {
                square_depth += 1;
                current.push(ch);
            }
            ']' => {
                square_depth = square_depth.saturating_sub(1);
                current.push(ch);
            }
            '(' => {
                paren_depth += 1;
                current.push(ch);
            }
            ')' => {
                paren_depth = paren_depth.saturating_sub(1);
                current.push(ch);
            }
            '{' if square_depth == 0 && paren_depth == 0 => {
                if is_inline_selector_brace(line, index) {
                    inline_brace_depth = 1;
                    current.push(ch);
                    continue;
                }
                push_trimmed_piece(&mut pieces, &current);
                current.clear();
                if let Some(last) = pieces.last_mut() {
                    last.push_str(" {");
                } else {
                    pieces.push("{".to_string());
                }
            }
            '}' if square_depth == 0 && paren_depth == 0 => {
                if !current.trim().is_empty() {
                    push_trimmed_piece(&mut pieces, &current);
                    current.clear();
                }
                pieces.push("}".to_string());
            }
            ';' if split_semicolons && square_depth == 0 && paren_depth == 0 => {
                push_trimmed_piece(&mut pieces, &current);
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    if in_string {
        return Err(parse_error(line, "string literal is missing closing quote"));
    }
    if inline_brace_depth > 0 {
        return Err(parse_error(
            line,
            "inline selector scratch is missing closing brace",
        ));
    }
    push_trimmed_piece(&mut pieces, &current);
    Ok(pieces)
}

fn push_trimmed_piece(pieces: &mut Vec<String>, piece: &str) {
    let trimmed = piece.trim();
    if !trimmed.is_empty() {
        pieces.push(trimmed.to_string());
    }
}

fn is_inline_selector_brace(line: &str, index: usize) -> bool {
    let before = line[..index].chars().next_back();
    let after = line[index + 1..].chars().next();
    before.is_some_and(is_selector_token_char) && after.is_some_and(|ch| !ch.is_whitespace())
}

fn is_selector_token_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '@' | ':' | '*')
}

fn normalize_brace_blocks(lines: &[String]) -> Result<Vec<String>, DiagnosticReport> {
    let mut normalized = Vec::new();
    let mut levels_brace_depth = 0i32;

    for line in lines {
        if line == "}" {
            normalized.push("}".to_string());
            if levels_brace_depth > 0 {
                levels_brace_depth -= 1;
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix('}') {
            let rest = rest.trim_start();
            match rest {
                "else" => normalized.push("else".to_string()),
                "else {" => normalized.push("else".to_string()),
                "else{" => normalized.push("else".to_string()),
                rest if rest.starts_with("->") => {
                    normalized.push("}".to_string());
                    normalized.push(rest.to_string());
                }
                _ => {
                    return Err(parse_error(
                        line,
                        "closing brace must be alone or followed by else or ->",
                    ));
                }
            }
            continue;
        }

        if line == "else {" {
            normalized.push("else".to_string());
            continue;
        }

        if line == "{" {
            normalized.push("{".to_string());
            continue;
        }

        if let Some(header) = line.strip_suffix('{') {
            let header = header.trim_end();
            if header.is_empty() {
                return Err(parse_error(
                    line,
                    "opening brace must follow a block header",
                ));
            }
            let header_tokens = split_header_tokens(header);
            let is_levels_header = is_levels_header(&header_tokens);
            let preserve_level_header = levels_brace_depth > 0
                && !starts_inline_block(&header_tokens, header)
                || matches!(header_tokens.as_slice(), ["level", ..]);
            if header.ends_with("->") {
                normalized.push(format!("{header} {{"));
                continue;
            }
            if preserve_level_header {
                normalized.push(format!("{header} {{"));
                if levels_brace_depth > 0 || is_levels_header {
                    levels_brace_depth += 1;
                }
                continue;
            }
            normalized.push(format!("{header} {{"));
            if levels_brace_depth > 0 || is_levels_header {
                levels_brace_depth += 1;
            }
            continue;
        }

        let structural_view = strip_inline_scratch_blocks(line)?;
        if structural_view.contains('{') || structural_view.contains('}') {
            return Err(parse_error(line, "braces must be used at block boundaries"));
        }

        normalized.push(line.clone());
    }

    Ok(normalized)
}

fn is_levels_header(tokens: &[&str]) -> bool {
    matches!(
        tokens,
        ["levels"] | ["levels", "of", _] | ["levels", _, "of", _]
    )
}

fn starts_inline_block(tokens: &[&str], line: &str) -> bool {
    if matches!(tokens, [lifecycle] if is_puzzle_lifecycle_block(lifecycle)) {
        return true;
    }
    matches!(
        tokens,
        ["map", ..]
            | ["on_display"]
            | ["scratch"]
            | ["groups"]
            | ["layers"]
            | ["collision_layers"]
            | ["legend"]
            | ["win_conditions", ..]
            | ["lose_conditions", ..]
            | ["sprites", ..]
            | ["colors"]
            | ["palettes"]
            | ["shapes"]
            | ["theme", ..]
            | ["assets"]
            | ["screen"]
            | ["layout"]
            | ["rule", ..]
            | ["rules"]
            | ["main"]
            | ["transitions"]
            | ["levels", ..]
            | ["level", ..]
            | ["state"]
            | ["keys"]
            | ["resources"]
            | ["on_scene_start"]
            | ["input", ..]
            | ["action", ..]
            | ["if", ..]
            | ["row"]
            | ["column"]
            | ["box"]
            | ["for", ..]
            | ["level_menu"]
            | ["fix", ..]
            | ["repeat"]
            | ["once"]
            | ["once_all"]
            | ["once_per_level"]
            | ["display"]
    ) || matches!(tokens, ["button", ..] if line.trim_end().ends_with(" with"))
}

fn strip_inline_scratch_blocks(line: &str) -> Result<String, DiagnosticReport> {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '{' {
            out.push(ch);
            continue;
        }
        let mut closed = false;
        for inner in chars.by_ref() {
            if inner == '{' {
                return Err(parse_error(line, "nested inline braces are not supported"));
            }
            if inner == '}' {
                closed = true;
                break;
            }
        }
        if !closed {
            out.push('{');
        }
    }
    Ok(out)
}

#[cfg(test)]
pub(crate) fn split_tokens(line: &str) -> Vec<&str> {
    line.split_whitespace().collect()
}

pub(crate) fn split_header_tokens(line: &str) -> Vec<&str> {
    puzzle_authoring::split_header_tokens(line)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SourceScope {
    Puzzle,
    Sounds,
    Assets,
    Scene,
    SceneLayout,
    SceneState,
    SceneKeys,
    SceneTransitions,
    LevelMenu,
    Tags,
    Group,
    Layers,
    Scratch,
    Keys,
    Legend,
    Levels,
    Level,
    UnbracedLevel,
    Visuals,
    VisualShapeTable,
    VisualShapeEntry,
    VisualColorTable,
    VisualPaletteTable,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SourceLineRole {
    Normal,
    Raw,
    PlainAssignmentLeft,
    PlainAfterKeywordAssignmentLeft,
    PlainFirstToken,
}

#[derive(Clone, Debug)]
pub(crate) struct SourceToken {
    pub(crate) text: String,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct SourceContextLine {
    pub(crate) tokens: Vec<String>,
    pub(crate) token_spans: Vec<SourceToken>,
    pub(crate) scope: Option<SourceScope>,
    pub(crate) start: usize,
    pub(crate) content: String,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct SourceContext {
    raw: Vec<(usize, usize)>,
    plain: Vec<(usize, usize)>,
    pub(crate) lines: Vec<SourceContextLine>,
}

impl SourceContext {
    pub(crate) fn raw_range_starting_at(&self, start: usize) -> Option<usize> {
        self.raw
            .iter()
            .find_map(|(range_start, range_end)| (*range_start == start).then_some(*range_end))
    }

    pub(crate) fn is_plain_range(&self, start: usize, end: usize) -> bool {
        self.plain
            .iter()
            .any(|(range_start, range_end)| start >= *range_start && end <= *range_end)
    }
}

pub(crate) fn scan_source_context(source: &str) -> SourceContext {
    let mut context = SourceContext::default();
    let mut block_stack = Vec::<SourceScope>::new();
    let mut offset = 0usize;

    for line in source.split_inclusive('\n') {
        let line_end = offset + line.len();
        let content_end = line_end - usize::from(line.ends_with('\n'));
        let content = &source[offset..content_end];
        let raw = strip_line_comment(content);
        let trimmed = raw.trim();
        let tokens = source_context_tokens(trimmed);
        let current = block_stack.last().copied();

        if trimmed.is_empty() {
            close_blank_line(&mut block_stack);
        }

        if !trimmed.is_empty() && trimmed != "}" {
            match source_line_role(current, trimmed, &tokens) {
                SourceLineRole::Normal => {}
                SourceLineRole::Raw => context.raw.push((offset, content_end)),
                SourceLineRole::PlainAssignmentLeft => {
                    add_assignment_left_range(content, offset, 0, &mut context.plain);
                }
                SourceLineRole::PlainAfterKeywordAssignmentLeft => {
                    let keyword_len = leading_token_len(content).unwrap_or(0);
                    add_assignment_left_range(content, offset, keyword_len, &mut context.plain);
                }
                SourceLineRole::PlainFirstToken => {
                    add_first_token_range(content, offset, &mut context.plain);
                }
            }
        }

        if trimmed == "}" {
            close_block_line(&mut block_stack);
        } else if source_opens_block(trimmed, &tokens, current)
            && let Some(opened) = opening_scope(trimmed, &tokens, current)
        {
            block_stack.push(opened);
        }

        context.lines.push(SourceContextLine {
            tokens: tokens.iter().map(|token| (*token).to_string()).collect(),
            token_spans: source_line_tokens(raw, offset),
            scope: current,
            start: offset,
            content: content.to_string(),
        });

        offset = line_end;
    }

    context
}

fn source_line_role(
    current: Option<SourceScope>,
    trimmed: &str,
    tokens: &[&str],
) -> SourceLineRole {
    match current {
        Some(SourceScope::Legend) => SourceLineRole::PlainAssignmentLeft,
        Some(SourceScope::Level) if starts_level_legend(tokens) => {
            SourceLineRole::PlainAfterKeywordAssignmentLeft
        }
        Some(SourceScope::Level | SourceScope::UnbracedLevel) => SourceLineRole::Raw,
        Some(SourceScope::VisualShapeEntry) => SourceLineRole::Raw,
        Some(SourceScope::VisualShapeTable) if trimmed.ends_with('{') => {
            SourceLineRole::PlainFirstToken
        }
        Some(SourceScope::VisualColorTable) => SourceLineRole::PlainAssignmentLeft,
        Some(SourceScope::VisualPaletteTable) => SourceLineRole::PlainAssignmentLeft,
        _ => SourceLineRole::Normal,
    }
}

fn starts_level_legend(tokens: &[&str]) -> bool {
    tokens.first().copied() == Some("legend")
}

fn source_context_tokens(line: &str) -> Vec<&str> {
    line.split(|ch: char| ch.is_whitespace() || matches!(ch, '{' | '}' | ',' | ';'))
        .filter(|token| !token.is_empty())
        .collect()
}

pub(crate) fn source_line_tokens(line: &str, line_offset: usize) -> Vec<SourceToken> {
    let mut tokens = Vec::new();
    let mut start = None;
    for (index, ch) in line.char_indices() {
        if ch.is_whitespace() {
            if let Some(token_start) = start.take() {
                tokens.push(SourceToken {
                    text: line[token_start..index].to_string(),
                    start: line_offset + token_start,
                    end: line_offset + index,
                });
            }
        } else if start.is_none() {
            start = Some(index);
        }
    }
    if let Some(token_start) = start {
        tokens.push(SourceToken {
            text: line[token_start..].to_string(),
            start: line_offset + token_start,
            end: line_offset + line.len(),
        });
    }
    tokens
}

fn close_blank_line(block_stack: &mut Vec<SourceScope>) {
    if block_stack.last() == Some(&SourceScope::UnbracedLevel) {
        block_stack.pop();
    }
}

fn close_block_line(block_stack: &mut Vec<SourceScope>) {
    if block_stack.last() == Some(&SourceScope::UnbracedLevel) {
        block_stack.pop();
        if block_stack.last() == Some(&SourceScope::Levels) {
            block_stack.pop();
        }
        return;
    }
    block_stack.pop();
}

fn source_opens_block(line: &str, tokens: &[&str], current: Option<SourceScope>) -> bool {
    if is_scene_scope(current) {
        match tokens {
            ["state"] | ["keys"] | ["inputs"] | ["rules"] | ["on_scene_start"] => {
                return true;
            }
            ["layout", ..] => return true,
            ["row", ..]
            | ["column", ..]
            | ["box", ..]
            | ["for", ..]
            | ["level_menu", ..]
            | ["puzzle", ..]
            | ["puzzle3", ..]
            | ["routine", ..]
            | ["input", ..]
            | ["action", ..]
            | ["if", ..] => return line.ends_with('{'),
            _ => {}
        }
    }
    if current == Some(SourceScope::Levels) && !tokens.is_empty() {
        return true;
    }
    line.ends_with('{')
        || matches!(
            tokens,
            ["sounds"]
                | ["assets"]
                | ["groups"]
                | ["legend"]
                | ["levels", ..]
                | ["levels3", ..]
                | ["tags"]
                | ["layers"]
                | ["collision_layers"]
                | ["scratch"]
                | ["keys"]
                | ["inputs"]
                | ["resources"]
                | ["sprites", ..]
                | ["sprites3", ..]
                | ["colors"]
                | ["palettes"]
                | ["shapes"]
                | ["render"]
                | ["camera"]
                | ["scene", ..]
                | ["puzzle", ..]
                | ["puzzle3", ..]
                | ["layout", ..]
                | ["state"]
                | ["rules"]
                | ["on_scene_start"]
                | ["input", ..]
                | ["action", ..]
                | ["if", ..]
                | ["row", ..]
                | ["column", ..]
                | ["box", ..]
                | ["for", ..]
                | ["level_menu", ..]
        )
}

fn opening_scope(line: &str, tokens: &[&str], current: Option<SourceScope>) -> Option<SourceScope> {
    if is_scene_scope(current) {
        match tokens {
            ["layout", ..]
            | ["row", ..]
            | ["column", ..]
            | ["box", ..]
            | ["for", ..]
            | ["puzzle", ..]
            | ["puzzle3", ..] => {
                return Some(SourceScope::SceneLayout);
            }
            ["state"] => return Some(SourceScope::SceneState),
            ["keys"] | ["inputs"] => return Some(SourceScope::SceneKeys),
            ["rules"]
            | ["on_scene_start"]
            | ["routine", ..]
            | ["input", ..]
            | ["action", ..]
            | ["if", ..] => {
                return Some(SourceScope::SceneTransitions);
            }
            ["level_menu", ..] => return Some(SourceScope::LevelMenu),
            _ => {}
        }
        if line.ends_with('{') {
            return if current == Some(SourceScope::SceneLayout) && line.contains("->") {
                Some(SourceScope::SceneTransitions)
            } else {
                current
            };
        }
    }
    if matches!(
        current,
        Some(SourceScope::VisualShapeTable | SourceScope::VisualShapeEntry)
    ) && line.ends_with('{')
    {
        return Some(SourceScope::VisualShapeEntry);
    }
    if current == Some(SourceScope::Levels) {
        if matches!(tokens, ["legend"]) {
            return Some(SourceScope::Legend);
        }
        if matches!(tokens, ["{"]) || (matches!(tokens, ["level", ..]) && line.ends_with('{')) {
            return Some(SourceScope::Level);
        }
    }
    if current == Some(SourceScope::Levels) && !tokens.is_empty() {
        return Some(SourceScope::UnbracedLevel);
    }
    match tokens {
        ["sounds"] => Some(SourceScope::Sounds),
        ["assets"] => Some(SourceScope::Assets),
        ["scene", ..] => Some(SourceScope::Scene),
        ["puzzle", ..] | ["puzzle3", ..] => Some(SourceScope::Puzzle),
        ["level", ..] => Some(SourceScope::Level),
        ["shapes"] => Some(SourceScope::VisualShapeTable),
        ["colors"] => Some(SourceScope::VisualColorTable),
        ["palettes"] => Some(SourceScope::VisualPaletteTable),
        [first, ..] => source_scope_for_name(first),
        [] => line.ends_with('{').then_some(SourceScope::Other),
    }
    .or_else(|| line.ends_with('{').then_some(SourceScope::Other))
}

fn is_scene_scope(scope: Option<SourceScope>) -> bool {
    matches!(
        scope,
        Some(
            SourceScope::Scene
                | SourceScope::SceneLayout
                | SourceScope::SceneState
                | SourceScope::SceneKeys
                | SourceScope::SceneTransitions
                | SourceScope::LevelMenu
        )
    )
}

fn source_scope_for_name(name: &str) -> Option<SourceScope> {
    match name {
        "sounds" => Some(SourceScope::Sounds),
        "assets" => Some(SourceScope::Assets),
        "puzzle" => Some(SourceScope::Puzzle),
        "tags" => Some(SourceScope::Tags),
        "layers" | "collision_layers" => Some(SourceScope::Layers),
        "groups" => Some(SourceScope::Group),
        "scratch" => Some(SourceScope::Scratch),
        "keys" | "inputs" => Some(SourceScope::Keys),
        "resources" => Some(SourceScope::Other),
        "legend" => Some(SourceScope::Legend),
        "levels" | "levels3" => Some(SourceScope::Levels),
        "level" => Some(SourceScope::Level),
        "sprites" | "sprites3" => Some(SourceScope::Visuals),
        "render" | "camera" => Some(SourceScope::Other),
        "rules" => Some(SourceScope::Other),
        _ => None,
    }
}

fn add_assignment_left_range(
    line: &str,
    absolute_start: usize,
    search_start: usize,
    ranges: &mut Vec<(usize, usize)>,
) {
    let Some(eq_offset) = line[search_start..].find('=') else {
        return;
    };
    let left_start = search_start
        + line[search_start..search_start + eq_offset]
            .find(|ch: char| !ch.is_whitespace())
            .unwrap_or(eq_offset);
    let left_end = search_start
        + line[search_start..search_start + eq_offset]
            .trim_end()
            .len();
    if left_start < left_end {
        ranges.push((absolute_start + left_start, absolute_start + left_end));
    }
}

fn add_first_token_range(line: &str, absolute_start: usize, ranges: &mut Vec<(usize, usize)>) {
    let Some(start) = line.find(|ch: char| !ch.is_whitespace()) else {
        return;
    };
    let end = start + leading_token_len(&line[start..]).unwrap_or(0);
    if start < end {
        ranges.push((absolute_start + start, absolute_start + end));
    }
}

fn leading_token_len(line: &str) -> Option<usize> {
    let start = line.find(|ch: char| !ch.is_whitespace())?;
    let token = line[start..]
        .find(|ch: char| ch.is_whitespace() || matches!(ch, '{' | '}' | ',' | ';'))
        .unwrap_or(line[start..].len());
    Some(start + token)
}

fn parse_error(line: &str, message: &str) -> DiagnosticReport {
    DiagnosticReport::error_at_line(message, line)
}

#[cfg(test)]
mod tests {
    use super::{scan_source_context, split_header_tokens, split_tokens};

    #[test]
    fn split_tokens_preserves_block_openers() {
        assert_eq!(split_tokens("level first {"), vec!["level", "first", "{"]);
        assert_eq!(split_tokens("levels {"), vec!["levels", "{"]);
        assert_eq!(split_tokens("{"), vec!["{"]);
    }

    #[test]
    fn split_header_tokens_removes_only_trailing_block_opener() {
        assert_eq!(split_header_tokens("level first {"), vec!["level", "first"]);
        assert_eq!(split_header_tokens("levels {"), vec!["levels"]);
        assert_eq!(split_header_tokens("{"), vec!["{"]);
    }

    #[test]
    fn source_context_preserves_token_spans_before_comments() {
        let source = "scene title {\n  button start -> goto playing // comment\n}\n";
        let context = scan_source_context(source);
        let button_line = context
            .lines
            .iter()
            .find(|line| line.content.contains("button"))
            .unwrap();

        let start = button_line
            .token_spans
            .iter()
            .find(|token| token.text == "start")
            .unwrap();
        assert_eq!(&source[start.start..start.end], "start");

        assert!(
            !button_line
                .token_spans
                .iter()
                .any(|token| token.text == "//" || token.text == "comment")
        );
    }
}
