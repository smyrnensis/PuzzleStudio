use crate::DiagnosticReport;
use crate::surface::SurfaceOptionBlock;
use crate::syntax::puzzle_lifecycle_event;

#[cfg(test)]
thread_local! {
    static CANONICAL_SCAN_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[path = "source_lexical_product.rs"]
pub(crate) mod lexical_product;

#[path = "source_lexer.rs"]
pub(crate) mod lexer;

#[path = "source_outline_product.rs"]
pub(crate) mod outline_product;

#[cfg(test)]
pub(crate) fn logical_lines(source: &str) -> Result<Vec<String>, DiagnosticReport> {
    logical_lines_with_locations(source)
        .map(|lines| lines.into_iter().map(|line| line.text).collect())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LogicalLine {
    pub(crate) text: String,
    pub(crate) line: usize,
    pub(crate) tokens: Vec<SourceToken>,
    source_span: Option<(usize, usize)>,
    structural_brace_delta: i8,
}

impl LogicalLine {
    pub(crate) fn new(text: impl Into<String>, line: usize) -> Self {
        let text = text.into();
        Self {
            structural_brace_delta: canonical_structural_brace_delta(&text),
            text,
            line,
            tokens: Vec::new(),
            source_span: None,
        }
    }

    fn with_tokens(mut self, tokens: Vec<SourceToken>) -> Self {
        self.tokens = tokens;
        self
    }

    fn with_source_span(mut self, source_span: Option<(usize, usize)>) -> Self {
        self.source_span = source_span;
        self
    }

    pub(crate) fn source_span(&self) -> Option<(usize, usize)> {
        self.source_span
    }

    pub(crate) fn with_text(&self, text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            line: self.line,
            tokens: self.tokens.clone(),
            source_span: self.source_span,
            structural_brace_delta: self.structural_brace_delta,
        }
    }

    fn with_normalized_text(&self, text: impl Into<String>) -> Self {
        Self::new(text, self.line)
            .with_tokens(self.tokens.clone())
            .with_source_span(self.source_span)
    }

    pub(crate) fn structural_brace_delta(&self) -> i32 {
        i32::from(self.structural_brace_delta)
    }

    pub(crate) fn source_start(&self) -> Option<usize> {
        self.source_span.map(|(start, _)| start)
    }

    pub(crate) fn source_end(&self) -> Option<usize> {
        self.source_span.map(|(_, end)| end)
    }
}

fn canonical_structural_brace_delta(text: &str) -> i8 {
    if text == "}" {
        -1
    } else if text == "{" || text.trim_end().ends_with('{') {
        1
    } else {
        0
    }
}

impl std::ops::Deref for LogicalLine {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.text
    }
}

impl AsRef<str> for LogicalLine {
    fn as_ref(&self) -> &str {
        &self.text
    }
}

pub(crate) fn logical_lines_with_locations(
    source: &str,
) -> Result<Vec<LogicalLine>, DiagnosticReport> {
    scan_surface_source(source).strict_logical_lines()
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

fn expand_structural_source_line(
    line: &str,
    block_stack: &mut Vec<String>,
) -> Result<Vec<String>, DiagnosticReport> {
    let row_content_owner = block_stack.last().is_some_and(|surface| {
        crate::authoring_grammar::authoring_source_block(surface)
            .and_then(|block| block.content)
            .is_some_and(|content| {
                matches!(
                    crate::authoring_grammar::authoring_content_syntax(content),
                    crate::authoring_grammar::ContentSyntax::Rows(_)
                )
            })
    });
    let opens_structural_child = line
        .find('{')
        .is_some_and(|open| is_structural_block_open_segment(&line[..open]));
    let split_semicolons = row_content_owner
        || opens_structural_child
        || !block_stack.iter().any(|block| ascii_sensitive_block(block));
    if !split_semicolons && ascii_row_contains_brace(line) {
        return Err(parse_error(line, "ASCII rows cannot contain braces"));
    }
    let pieces = split_structural_line(line, split_semicolons)?;
    for piece in &pieces {
        update_structural_block_stack(piece, block_stack);
    }
    Ok(pieces)
}

fn ascii_sensitive_block(block: &str) -> bool {
    crate::authoring_grammar::authoring_source_block(block).is_some_and(|spec| {
        matches!(
            spec.role,
            crate::authoring_grammar::AuthoringBlockRole::Visuals
                | crate::authoring_grammar::AuthoringBlockRole::LevelList
                | crate::authoring_grammar::AuthoringBlockRole::LevelEntry
        )
    }) || block == "map"
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
            if ch == '"' {
                in_string = true;
            } else if ch == '{' {
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
                if is_inline_brace_group(line, index, &current) {
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
            "inline brace group is missing closing brace",
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

fn is_inline_brace_group(line: &str, index: usize, current_segment: &str) -> bool {
    if matching_inline_brace(line, index).is_none() {
        return false;
    }
    !is_structural_block_open_segment(current_segment)
}

fn matching_inline_brace(line: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (relative, ch) in line[open..].char_indices() {
        let index = open + relative;
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn normalize_brace_block_line(
    logical_line: &LogicalLine,
    levels_brace_depth: &mut i32,
    normalized: &mut Vec<LogicalLine>,
) -> Result<(), DiagnosticReport> {
    let line = logical_line.text.as_str();
    if line == "}" {
        normalized.push(logical_line.with_normalized_text("}"));
        if *levels_brace_depth > 0 {
            *levels_brace_depth -= 1;
        }
        return Ok(());
    }

    if let Some(rest) = line.strip_prefix('}') {
        let rest = rest.trim_start();
        match rest {
            "else" => {
                normalized.push(logical_line.with_normalized_text("}"));
                normalized.push(logical_line.with_normalized_text("else"));
            }
            "else {" | "else{" => {
                normalized.push(logical_line.with_normalized_text("}"));
                normalized.push(logical_line.with_normalized_text("else {"));
            }
            rest if rest.starts_with("->") => {
                normalized.push(logical_line.with_normalized_text("}"));
                normalized.push(logical_line.with_normalized_text(rest));
            }
            _ => {
                return Err(parse_error(
                    line,
                    "closing brace must be alone or followed by else or ->",
                ));
            }
        }
        return Ok(());
    }

    if line == "else {" {
        normalized.push(logical_line.with_normalized_text("else {"));
        return Ok(());
    }

    if line == "{" {
        normalized.push(logical_line.with_normalized_text("{"));
        return Ok(());
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
        let preserve_level_header =
            *levels_brace_depth > 0 || matches!(header_tokens.as_slice(), ["level", ..]);
        if header.ends_with("->") {
            normalized.push(logical_line.with_normalized_text(format!("{header} {{")));
            return Ok(());
        }
        if preserve_level_header {
            normalized.push(logical_line.with_normalized_text(format!("{header} {{")));
            if *levels_brace_depth > 0 || is_levels_header {
                *levels_brace_depth += 1;
            }
            return Ok(());
        }
        normalized.push(logical_line.with_normalized_text(format!("{header} {{")));
        if *levels_brace_depth > 0 || is_levels_header {
            *levels_brace_depth += 1;
        }
        return Ok(());
    }

    let structural_view = strip_inline_brace_groups(line);
    if structural_view.contains('{') || structural_view.contains('}') {
        return Err(parse_error(line, "braces must be used at block boundaries"));
    }

    normalized.push(logical_line.clone());
    Ok(())
}

fn is_levels_header(tokens: &[&str]) -> bool {
    matches!(
        tokens,
        ["levels"] | ["levels", "of", _] | ["levels", _, "of", _]
    )
}

fn starts_inline_block(tokens: &[&str], line: &str) -> bool {
    if matches!(tokens, [lifecycle] if puzzle_lifecycle_event(lifecycle).is_some()) {
        return true;
    }
    if tokens
        .first()
        .is_some_and(|surface| crate::authoring_grammar::authoring_source_block(surface).is_some())
    {
        return true;
    }
    matches!(
        tokens,
        ["map", ..]
            | ["marks"]
            | ["tags"]
            | ["groups"]
            | ["layers"]
            | ["merge"]
            | ["collision_layers"]
            | ["legend"]
            | ["win_conditions", ..]
            | ["lose_conditions", ..]
            | ["palette"]
            | ["shapes"]
            | ["objects"]
            | ["render", ..]
            | ["sfx", ..]
            | ["music", ..]
            | ["camera", ..]
            | ["grid", ..]
            | ["pixelate", ..]
            | ["theme", ..]
            | ["assets"]
            | ["screen"]
            | ["layout"]
            | ["main"]
            | ["state"]
            | ["keys"]
            | ["resources"]
            | ["on_scene_start"]
            | ["input", ..]
            | ["action", ..]
            | ["if", ..]
            | ["else"]
            | ["for", ..]
            | ["repeat"]
            | ["row"]
            | ["column"]
            | ["box"]
            | ["fix", ..]
            | ["once"]
            | ["once_all"]
            | ["once_per_level"]
            | ["puzzle", ..]
    ) || matches!(tokens, ["button", ..] if line.trim_end().ends_with(" with") || line.contains("->"))
}

fn is_structural_block_open_segment(segment: &str) -> bool {
    let segment = segment.trim();
    if segment.is_empty() || segment == "else" || segment.ends_with("->") {
        return true;
    }
    let tokens = split_header_tokens(segment);
    starts_inline_block(&tokens, segment)
}

fn strip_inline_brace_groups(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut index = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    while index < line.len() {
        let ch = line[index..]
            .chars()
            .next()
            .expect("index is within string");
        if in_string {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            index += ch.len_utf8();
            continue;
        }
        if ch == '"' {
            in_string = true;
            out.push(ch);
            index += ch.len_utf8();
            continue;
        }
        if ch == '{' {
            if let Some(close) = matching_inline_brace(line, index) {
                index = close + 1;
                continue;
            }
        }
        out.push(ch);
        index += ch.len_utf8();
    }
    out
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
    Scene,
    SceneLayout,
    SceneState,
    SceneKeys,
    SceneTransitions,
    Tags,
    Group,
    Layers,
    Mark,
    Map,
    Keys,
    Legend,
    Levels,
    Level,
    UnbracedLevel,
    Condition,
    Visuals,
    VisualShapeTable,
    VisualShapeEntry,
    VisualColorTable,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SourceToken {
    pub(crate) text: String,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SourceStructuralPiece {
    pub(crate) authoring_parent: Option<crate::authoring_grammar::AuthoringKind>,
    pub(crate) source_span: Option<(usize, usize)>,
    pub(crate) product: crate::surface::ParseProduct<Vec<SourceToken>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SurfaceSourceLine {
    pub(crate) tokens: Vec<String>,
    pub(crate) token_spans: Vec<SourceToken>,
    pub(crate) structural_token_spans: Vec<SourceToken>,
    pub(crate) structural_lines: Vec<String>,
    pub(crate) structural_pieces: Vec<SourceStructuralPiece>,
    pub(crate) structural_events: Vec<SourceStructureEvent>,
    pub(crate) scope: Option<SourceScope>,
    pub(crate) start: usize,
    pub(crate) line: usize,
    pub(crate) content: String,
    pub(crate) lexical_facts: Vec<lexer::SourceLexicalFact>,
    pub(crate) option_block: Option<SurfaceOptionBlock>,
    preserve_logical_blank: bool,
    structural_diagnostic: Option<DiagnosticReport>,
    scanner_state_after: SurfaceSourceScannerState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SourceStructureEvent {
    Open {
        header: String,
        scope: SourceScope,
        role: SourceBlockRole,
        virtual_braces: bool,
        option_block: SurfaceOptionBlock,
    },
    Close,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SourceBlockRole {
    SourceTree,
    Statement,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SourceBlockStackEntry {
    scope: SourceScope,
    virtual_braces: bool,
    option_block: SurfaceOptionBlock,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SourceBraceDisposition {
    pub(crate) depth: usize,
    pub(crate) matched_close: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SourceBraceStackEntry {
    pub(crate) start: usize,
    depth: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SurfaceSourceScan {
    raw: Vec<(usize, usize)>,
    plain: Vec<(usize, usize)>,
    pub(crate) lines: Vec<SurfaceSourceLine>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct SurfaceSourceScannerState {
    block_stack: Vec<SourceBlockStackEntry>,
    structural_block_stack: Vec<String>,
    normalize_levels_brace_depth: i32,
    unbraced_visual_shape_body: bool,
    structural_brace_stack: Vec<SourceBraceStackEntry>,
}

fn active_source_option_block(stack: &[SourceBlockStackEntry]) -> Option<SurfaceOptionBlock> {
    stack
        .iter()
        .rev()
        .map(|entry| entry.option_block)
        .find(|block| matches!(block, SurfaceOptionBlock::Authoring(_)))
}

fn source_option_block_for_opening(
    tokens: &[&str],
    stack: &[SourceBlockStackEntry],
) -> SurfaceOptionBlock {
    let Some(first) = tokens.first().copied() else {
        return SurfaceOptionBlock::Other;
    };
    match first {
        "puzzle" => SurfaceOptionBlock::Puzzle2,
        surface => {
            let parent = stack
                .iter()
                .rev()
                .find_map(|entry| entry.option_block.authoring_parent_kind())
                .unwrap_or(crate::authoring_grammar::AuthoringKind::Root);
            crate::authoring_grammar::placed_authoring_kind(parent, surface)
                .map(SurfaceOptionBlock::Authoring)
                .unwrap_or(SurfaceOptionBlock::Other)
        }
    }
}

impl SurfaceSourceScan {
    pub(crate) fn raw_ranges(&self) -> &[(usize, usize)] {
        &self.raw
    }

    pub(crate) fn plain_ranges(&self) -> &[(usize, usize)] {
        &self.plain
    }

    pub(crate) fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Fingerprints the non-trivia token spellings and absolute spans consumed
    /// by parser products. Comments are excluded, but any edit that moves a
    /// parser token invalidates position-bearing semantic products.
    pub(crate) fn parser_product_fingerprint(&self) -> u64 {
        use std::hash::{Hash, Hasher};

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for line in &self.lines {
            for fact in &line.lexical_facts {
                if matches!(fact.kind, lexer::SourceLexicalKind::Comment) {
                    continue;
                }
                let start = fact.start - line.start;
                let end = fact.end - line.start;
                fact.kind.hash(&mut hasher);
                fact.start.hash(&mut hasher);
                fact.end.hash(&mut hasher);
                line.content[start..end].hash(&mut hasher);
            }
        }
        hasher.finish()
    }

    pub(crate) fn unmatched_open_braces(&self) -> &[SourceBraceStackEntry] {
        self.lines
            .last()
            .map(|line| line.scanner_state_after.structural_brace_stack.as_slice())
            .unwrap_or_default()
    }

    /// Returns the strict compiler's logical lines from this exact canonical scan.
    ///
    /// The incremental scanner remains total for editor use: malformed structural
    /// sugar is stored as a recovery diagnostic on the physical line. Strict
    /// compilation rejects that same fact here instead of rescanning the source
    /// through a second structural grammar.
    pub(crate) fn strict_logical_lines(&self) -> Result<Vec<LogicalLine>, DiagnosticReport> {
        for line in &self.lines {
            if let Some(diagnostic) = &line.structural_diagnostic {
                return Err(diagnostic.clone());
            }
        }
        Ok(self.editor_logical_lines())
    }

    /// Total logical product for editor projections. Invalid physical lines
    /// retain their scanner diagnostic and contribute no invented syntax, while
    /// successfully parsed sibling lines keep their canonical tokens.
    pub(crate) fn editor_logical_lines(&self) -> Vec<LogicalLine> {
        let mut logical_lines = Vec::new();
        for line in &self.lines {
            if line.structural_diagnostic.is_some() {
                continue;
            }
            if line.content.trim() == "}"
                && line.lexical_facts.iter().any(|fact| {
                    matches!(
                        fact.kind,
                        lexer::SourceLexicalKind::Brace(lexer::SourceBraceKind::Close)
                    ) && fact
                        .brace_disposition
                        .is_some_and(|disposition| !disposition.matched_close)
                })
            {
                continue;
            }
            if line.structural_lines.is_empty() {
                if line.preserve_logical_blank {
                    logical_lines.push(LogicalLine::new(String::new(), line.line));
                }
                continue;
            }
            logical_lines.extend(
                line.structural_lines
                    .iter()
                    .cloned()
                    .zip(line.structural_pieces.iter())
                    .map(|(text, piece)| {
                        LogicalLine::new(text, line.line)
                            .with_tokens(piece.product.value.clone())
                            .with_source_span(piece.source_span)
                    }),
            );
        }
        logical_lines
    }

    pub(crate) fn apply_edit(
        &mut self,
        old_source: &str,
        new_source: &str,
        edit_start: usize,
        edit_end: usize,
        insert_len: usize,
    ) -> usize {
        let rescan_start = old_source[..edit_start]
            .rfind('\n')
            .map_or(0, |index| index + 1);
        let prefix_line_count = self
            .lines
            .iter()
            .take_while(|line| line.start < rescan_start)
            .count();
        let mut state = self
            .lines
            .get(prefix_line_count.wrapping_sub(1))
            .map(|line| line.scanner_state_after.clone())
            .unwrap_or_default();

        let old_suffix_lines = self.lines.split_off(prefix_line_count);
        let old_raw = self.raw.clone();
        let old_plain = self.plain.clone();
        self.raw.retain(|(_, end)| *end <= rescan_start);
        self.plain.retain(|(_, end)| *end <= rescan_start);

        let mut offset = rescan_start;
        let mut rescanned = 0usize;
        let delta = insert_len as i64 - (edit_end - edit_start) as i64;
        let new_unchanged_suffix = edit_start + insert_len;
        for (relative_line_index, line) in
            new_source[rescan_start..].split_inclusive('\n').enumerate()
        {
            let line_index = prefix_line_count + relative_line_index;
            let scanned = scan_surface_source_line(line, offset, line_index, &mut state);
            self.raw.extend(scanned.raw_ranges.iter().copied());
            self.plain.extend(scanned.plain_ranges.iter().copied());
            offset += line.len();
            rescanned += 1;
            let converged_old_index = (scanned.line.start >= new_unchanged_suffix)
                .then(|| shift_offset(scanned.line.start, -delta))
                .and_then(|old_start| {
                    old_suffix_lines.iter().position(|old| {
                        let mut shifted_state = old.scanner_state_after.clone();
                        shift_structural_brace_stack(
                            &mut shifted_state.structural_brace_stack,
                            edit_end,
                            delta,
                        );
                        old.start == old_start
                            && old.content == scanned.line.content
                            && shifted_state == scanned.line.scanner_state_after
                            && old_line_has_newline(old_source, old) == line.ends_with('\n')
                    })
                });
            let new_line_number = scanned.line.line;
            self.lines.push(scanned.line);
            if let Some(old_index) = converged_old_index {
                let old_line_number = old_suffix_lines[old_index].line;
                let line_delta = new_line_number as i64 - old_line_number as i64;
                let old_reuse_start = old_suffix_lines
                    .get(old_index + 1)
                    .map_or(old_source.len(), |line| line.start);
                self.lines.extend(
                    old_suffix_lines[old_index + 1..]
                        .iter()
                        .cloned()
                        .map(|line| shift_surface_source_line(line, edit_end, delta, line_delta)),
                );
                self.raw.extend(
                    old_raw
                        .iter()
                        .copied()
                        .filter(|(start, _)| *start >= old_reuse_start)
                        .map(|(start, end)| (shift_offset(start, delta), shift_offset(end, delta))),
                );
                self.plain.extend(
                    old_plain
                        .iter()
                        .copied()
                        .filter(|(start, _)| *start >= old_reuse_start)
                        .map(|(start, end)| (shift_offset(start, delta), shift_offset(end, delta))),
                );
                return rescanned;
            }
        }
        rescanned
    }
}

fn shift_offset(value: usize, delta: i64) -> usize {
    usize::try_from(value as i64 + delta).expect("incremental source offset underflow")
}

fn shift_structural_brace_stack(stack: &mut [SourceBraceStackEntry], threshold: usize, delta: i64) {
    for entry in stack {
        if entry.start >= threshold {
            entry.start = shift_offset(entry.start, delta);
        }
    }
}

fn assign_structural_brace_dispositions(
    facts: &mut [lexer::SourceLexicalFact],
    stack: &mut Vec<SourceBraceStackEntry>,
) {
    for fact in facts {
        let lexer::SourceLexicalKind::Brace(kind) = &fact.kind else {
            continue;
        };
        let disposition = match *kind {
            lexer::SourceBraceKind::Open => {
                let depth = stack.len();
                stack.push(SourceBraceStackEntry {
                    start: fact.start,
                    depth,
                });
                SourceBraceDisposition {
                    depth,
                    matched_close: true,
                }
            }
            lexer::SourceBraceKind::Close => stack.pop().map_or(
                SourceBraceDisposition {
                    depth: 0,
                    matched_close: false,
                },
                |open| SourceBraceDisposition {
                    depth: open.depth,
                    matched_close: true,
                },
            ),
        };
        fact.brace_disposition = Some(disposition);
    }
}

fn old_line_has_newline(source: &str, line: &SurfaceSourceLine) -> bool {
    source
        .as_bytes()
        .get(line.start + line.content.len())
        .is_some_and(|byte| *byte == b'\n')
}

fn shift_surface_source_line(
    mut line: SurfaceSourceLine,
    edit_end: usize,
    offset_delta: i64,
    line_delta: i64,
) -> SurfaceSourceLine {
    line.start = shift_offset(line.start, offset_delta);
    line.line =
        usize::try_from(line.line as i64 + line_delta).expect("incremental source line underflow");
    for token in line
        .token_spans
        .iter_mut()
        .chain(line.structural_token_spans.iter_mut())
    {
        token.start = shift_offset(token.start, offset_delta);
        token.end = shift_offset(token.end, offset_delta);
    }
    for piece in &mut line.structural_pieces {
        if let Some((start, end)) = &mut piece.source_span {
            *start = shift_offset(*start, offset_delta);
            *end = shift_offset(*end, offset_delta);
        }
        for token in &mut piece.product.value {
            token.start = shift_offset(token.start, offset_delta);
            token.end = shift_offset(token.end, offset_delta);
        }
        piece
            .product
            .recognition
            .shift_offsets(edit_end, offset_delta);
    }
    lexer::shift_lexical_facts(&mut line.lexical_facts, edit_end, offset_delta);
    shift_structural_brace_stack(
        &mut line.scanner_state_after.structural_brace_stack,
        edit_end,
        offset_delta,
    );
    line
}

pub(crate) fn scan_surface_source(source: &str) -> SurfaceSourceScan {
    #[cfg(test)]
    CANONICAL_SCAN_CALLS.with(|calls| calls.set(calls.get() + 1));
    let mut context = SurfaceSourceScan::default();
    let mut state = SurfaceSourceScannerState::default();
    let mut offset = 0usize;

    for (line_index, line) in source.split_inclusive('\n').enumerate() {
        let scanned = scan_surface_source_line(line, offset, line_index, &mut state);
        context.raw.extend(scanned.raw_ranges.iter().copied());
        context.plain.extend(scanned.plain_ranges.iter().copied());
        offset += line.len();
        context.lines.push(scanned.line);
    }

    context
}

#[cfg(test)]
pub(crate) fn count_canonical_scans<T>(action: impl FnOnce() -> T) -> (T, usize) {
    CANONICAL_SCAN_CALLS.with(|calls| calls.set(0));
    let result = action();
    let canonical = CANONICAL_SCAN_CALLS.with(std::cell::Cell::get);
    (result, canonical)
}

struct ScannedSurfaceSourceLine {
    line: SurfaceSourceLine,
    raw_ranges: Vec<(usize, usize)>,
    plain_ranges: Vec<(usize, usize)>,
}

fn scan_surface_source_line(
    line: &str,
    offset: usize,
    line_index: usize,
    state: &mut SurfaceSourceScannerState,
) -> ScannedSurfaceSourceLine {
    let line_end = offset + line.len();
    let content_end = line_end - usize::from(line.ends_with('\n'));
    let content = &line[..line.len() - usize::from(line.ends_with('\n'))];
    let mut lexical_facts = lexer::scan_source_line_lexical_facts(content, offset);
    assign_structural_brace_dispositions(&mut lexical_facts, &mut state.structural_brace_stack);
    let code_end = lexical_facts
        .iter()
        .find_map(|fact| {
            matches!(fact.kind, lexer::SourceLexicalKind::Comment).then_some(fact.start - offset)
        })
        .unwrap_or(content.len());
    let raw = &content[..code_end];
    let trimmed = raw.trim();
    let tokens = surface_source_tokens(trimmed);
    let preserve_logical_blank = trimmed.is_empty()
        && state.block_stack.iter().any(|entry| {
            matches!(
                entry.scope,
                SourceScope::Levels
                    | SourceScope::Level
                    | SourceScope::UnbracedLevel
                    | SourceScope::Visuals
            )
        });
    let mut structural_events = Vec::<SourceStructureEvent>::new();
    let mut raw_ranges = Vec::new();
    let mut plain_ranges = Vec::new();

    if state
        .block_stack
        .last()
        .is_some_and(|entry| entry.virtual_braces && entry.scope == SourceScope::VisualShapeEntry)
        && starts_unbraced_visual_entry(trimmed, &tokens)
    {
        push_close_events(
            close_virtual_block(&mut state.block_stack),
            &mut structural_events,
        );
    }

    let current = state.block_stack.last().map(|entry| entry.scope);
    let option_block = active_source_option_block(&state.block_stack);
    let in_unbraced_visual_shape_body = state.unbraced_visual_shape_body
        && current == Some(SourceScope::VisualShapeTable)
        && !trimmed.is_empty()
        && trimmed != "}";

    if trimmed.is_empty() {
        push_close_events(
            close_blank_line(&mut state.block_stack),
            &mut structural_events,
        );
    }

    if !trimmed.is_empty() && trimmed != "}" {
        match source_line_role(current, trimmed, &tokens, in_unbraced_visual_shape_body) {
            SourceLineRole::Normal => {}
            SourceLineRole::Raw => raw_ranges.push((offset, content_end)),
            SourceLineRole::PlainAssignmentLeft => {
                add_assignment_left_range(content, offset, 0, &mut plain_ranges);
            }
            SourceLineRole::PlainAfterKeywordAssignmentLeft => {
                let keyword_len = leading_token_len(content).unwrap_or(0);
                add_assignment_left_range(content, offset, keyword_len, &mut plain_ranges);
            }
            SourceLineRole::PlainFirstToken => {
                add_first_token_range(content, offset, &mut plain_ranges);
            }
        }
    }

    state.unbraced_visual_shape_body =
        next_unbraced_visual_shape_body(current, trimmed, &tokens, in_unbraced_visual_shape_body);

    let (structural_lines, structural_diagnostic) = match surface_source_stack_lines(
        trimmed,
        &mut state.structural_block_stack,
        &mut state.normalize_levels_brace_depth,
    ) {
        Ok(lines) => (lines, None),
        Err(diagnostic) => (Vec::new(), Some(diagnostic)),
    };
    let mut structural_pieces = Vec::with_capacity(structural_lines.len());
    let mut structural_piece_cursor = 0usize;
    for stack_line in &structural_lines {
        let tokens = surface_source_tokens(stack_line);
        let current = state.block_stack.last().map(|entry| entry.scope);
        let mut authoring_parent = state
            .block_stack
            .iter()
            .rev()
            .find_map(|entry| entry.option_block.authoring_parent_kind());
        let (piece_tokens, source_span) = source_tokens_for_structural_piece(
            raw,
            offset,
            stack_line,
            &mut structural_piece_cursor,
        )
        .unwrap_or_default();
        let mut recognition = crate::surface::ParserRecognition::default();
        let mut schema_recognized = false;
        let mut header_owned_by_authoring = false;
        if authoring_parent.is_none() {
            let root_facts = crate::authoring_grammar::recognize_authoring_line(
                crate::authoring_grammar::AuthoringKind::Root,
                &piece_tokens,
            );
            if !root_facts.is_empty() {
                authoring_parent = Some(crate::authoring_grammar::AuthoringKind::Root);
                schema_recognized = true;
                header_owned_by_authoring = true;
                for fact in root_facts {
                    recognition.mark(
                        fact.span,
                        crate::authoring_grammar::authoring_surface_role_semantic_kind(fact.role),
                    );
                }
            }
        }
        if let Some(parent) = authoring_parent {
            if !schema_recognized {
                let facts =
                    crate::authoring_grammar::recognize_authoring_line(parent, &piece_tokens);
                header_owned_by_authoring = !facts.is_empty();
                for fact in facts {
                    recognition.mark(
                        fact.span,
                        crate::authoring_grammar::authoring_surface_role_semantic_kind(fact.role),
                    );
                }
            }
            if let Some(child) = piece_tokens.first().and_then(|token| {
                crate::authoring_grammar::placed_authoring_kind(parent, &token.text)
            }) {
                for export in crate::authoring_grammar::authoring_symbol_exports(child) {
                    let crate::authoring_grammar::AuthoringSymbolExportSource::HeaderArg(index) =
                        export.source;
                    let Some(value) = piece_tokens.get(index + 1) else {
                        continue;
                    };
                    match export.target {
                        crate::authoring_grammar::AuthoringSymbolExportTarget::Sfx => {
                            recognition
                                .completion_symbols
                                .sfx
                                .insert(value.text.clone());
                        }
                        crate::authoring_grammar::AuthoringSymbolExportTarget::Music => {
                            recognition
                                .completion_symbols
                                .music
                                .insert(value.text.clone());
                        }
                    }
                }
            }
        }
        let opened = (stack_line != "}" && source_opens_block(stack_line, &tokens, current))
            .then(|| opening_scope(stack_line, &tokens, current))
            .flatten();
        if let Some(opened) = opened
            && !header_owned_by_authoring
        {
            recognize_structural_header(stack_line, &piece_tokens, opened, &mut recognition);
        }
        recognize_owner_line(current, &piece_tokens, &mut recognition);
        structural_pieces.push(SourceStructuralPiece {
            authoring_parent,
            source_span,
            product: crate::surface::ParseProduct::new(piece_tokens, recognition),
        });
        if stack_line == "}" {
            push_close_events(
                close_block_line(&mut state.block_stack),
                &mut structural_events,
            );
        } else if let Some(opened) = opened {
            let role = source_block_role(stack_line, &tokens, current, opened);
            let virtual_braces = source_block_uses_virtual_braces(stack_line, current, opened);
            let opened_option_block = source_option_block_for_opening(&tokens, &state.block_stack);
            structural_events.push(SourceStructureEvent::Open {
                header: structural_header(stack_line),
                scope: opened,
                role,
                virtual_braces,
                option_block: opened_option_block,
            });
            state.block_stack.push(SourceBlockStackEntry {
                scope: opened,
                virtual_braces,
                option_block: opened_option_block,
            });
        }
    }

    ScannedSurfaceSourceLine {
        line: SurfaceSourceLine {
            tokens: tokens.iter().map(|token| (*token).to_string()).collect(),
            token_spans: source_line_tokens(raw, offset),
            structural_token_spans: surface_source_token_spans(raw, offset),
            structural_lines,
            structural_pieces,
            structural_events,
            scope: current,
            start: offset,
            line: line_index + 1,
            content: content.to_string(),
            lexical_facts,
            option_block,
            preserve_logical_blank,
            structural_diagnostic,
            scanner_state_after: state.clone(),
        },
        raw_ranges,
        plain_ranges,
    }
}

fn recognize_structural_header(
    line: &str,
    tokens: &[SourceToken],
    opened: SourceScope,
    recognition: &mut crate::surface::ParserRecognition,
) {
    if tokens.is_empty() {
        return;
    }
    if opened == SourceScope::UnbracedLevel && tokens[0].text != "level" {
        for token in tokens {
            recognize_token(
                recognition,
                token,
                crate::surface::SurfaceSemanticKind::Literal,
            );
        }
        return;
    }
    if opened == SourceScope::Scene && tokens[0].text == "scene" {
        recognize_token(
            recognition,
            &tokens[0],
            crate::surface::SurfaceSemanticKind::Keyword,
        );
        if let Some(name) = tokens.get(1) {
            recognize_token(
                recognition,
                name,
                crate::surface::SurfaceSemanticKind::Scene,
            );
        }
        return;
    }
    if matches!(
        tokens[0].text.as_str(),
        "win_conditions" | "lose_conditions"
    ) {
        recognize_token(
            recognition,
            &tokens[0],
            crate::surface::SurfaceSemanticKind::Condition,
        );
        for token in &tokens[1..] {
            recognize_token(
                recognition,
                token,
                crate::surface::SurfaceSemanticKind::Keyword,
            );
        }
        return;
    }
    if let Some(spans) = puzzle_authoring::rule_routine_block_header_surface_spans(line) {
        recognize_relative_span(
            recognition,
            tokens[0].start,
            spans.keyword,
            crate::surface::SurfaceSemanticKind::Keyword,
        );
        if let Some(name) = spans.name {
            recognize_relative_span(
                recognition,
                tokens[0].start,
                name,
                crate::surface::SurfaceSemanticKind::Effect,
            );
        }
        for modifier in spans.modifiers {
            recognize_relative_span(
                recognition,
                tokens[0].start,
                modifier,
                crate::surface::SurfaceSemanticKind::Keyword,
            );
        }
        return;
    }
    let structurally_owned = opened != SourceScope::Other
        || tokens
            .first()
            .is_some_and(|token| source_scope_for_name(&token.text).is_some());
    if structurally_owned {
        for (index, token) in tokens.iter().enumerate() {
            recognize_token(
                recognition,
                token,
                if index == 0 {
                    crate::surface::SurfaceSemanticKind::Keyword
                } else {
                    crate::surface::SurfaceSemanticKind::Binding
                },
            );
        }
    }
}

fn recognize_owner_line(
    owner: Option<SourceScope>,
    tokens: &[SourceToken],
    recognition: &mut crate::surface::ParserRecognition,
) {
    match owner {
        Some(SourceScope::SceneLayout) => {
            if let Some(first) = tokens.first()
                && puzzle_scene::SceneComponentKind::from_keyword(&first.text).is_some()
            {
                recognize_token(
                    recognition,
                    first,
                    crate::surface::SurfaceSemanticKind::Keyword,
                );
            }
        }
        Some(SourceScope::Scene | SourceScope::SceneTransitions) => {
            if let [step, target] = tokens
                && step.text == "step"
            {
                recognize_token(
                    recognition,
                    step,
                    crate::surface::SurfaceSemanticKind::Keyword,
                );
                recognize_token(
                    recognition,
                    target,
                    crate::surface::SurfaceSemanticKind::State,
                );
                return;
            }
            let Some(arrow) = tokens.iter().position(|token| token.text == "->") else {
                return;
            };
            for (index, token) in tokens[..arrow].iter().enumerate() {
                if index == 0 && token.text == "if" {
                    recognize_token(
                        recognition,
                        token,
                        crate::surface::SurfaceSemanticKind::Keyword,
                    );
                } else {
                    let condition = token
                        .text
                        .rsplit_once('.')
                        .map_or((0, token.text.as_str()), |(prefix, condition)| {
                            (prefix.len() + 1, condition)
                        });
                    recognition.mark(
                        crate::surface::SourceSpan {
                            start: token.start + condition.0,
                            end: token.start + condition.0 + condition.1.len(),
                        },
                        crate::surface::SurfaceSemanticKind::Condition,
                    );
                }
            }
            recognition.merge(crate::scene_effect_parser_recognition(&tokens[arrow + 1..]));
        }
        Some(SourceScope::Levels | SourceScope::UnbracedLevel) => {
            if let Some(command) = tokens.first()
                && command.text == "message"
            {
                recognize_token(
                    recognition,
                    command,
                    crate::surface::SurfaceSemanticKind::Emission,
                );
            }
        }
        _ => {}
    }
}

fn recognize_token(
    recognition: &mut crate::surface::ParserRecognition,
    token: &SourceToken,
    kind: crate::surface::SurfaceSemanticKind,
) {
    recognition.mark(
        crate::surface::SourceSpan {
            start: token.start,
            end: token.end,
        },
        kind,
    );
}

fn recognize_relative_span(
    recognition: &mut crate::surface::ParserRecognition,
    line_start: usize,
    span: std::ops::Range<usize>,
    kind: crate::surface::SurfaceSemanticKind,
) {
    recognition.mark(
        crate::surface::SourceSpan {
            start: line_start + span.start,
            end: line_start + span.end,
        },
        kind,
    );
}

fn surface_source_stack_lines(
    trimmed: &str,
    structural_block_stack: &mut Vec<String>,
    normalize_levels_brace_depth: &mut i32,
) -> Result<Vec<String>, DiagnosticReport> {
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let mut next_block_stack = structural_block_stack.clone();
    let mut next_levels_brace_depth = *normalize_levels_brace_depth;
    let mut normalized = Vec::new();
    let expanded = expand_structural_source_line(trimmed, &mut next_block_stack)?;
    for line in expanded {
        let logical_line = LogicalLine::new(line, 0);
        normalize_brace_block_line(&logical_line, &mut next_levels_brace_depth, &mut normalized)?;
    }
    *structural_block_stack = next_block_stack;
    *normalize_levels_brace_depth = next_levels_brace_depth;
    Ok(normalized
        .into_iter()
        .map(|logical_line| logical_line.text)
        .collect())
}

fn source_line_role(
    current: Option<SourceScope>,
    trimmed: &str,
    tokens: &[&str],
    in_unbraced_visual_shape_body: bool,
) -> SourceLineRole {
    if in_unbraced_visual_shape_body {
        return SourceLineRole::Raw;
    }
    match current {
        Some(SourceScope::Legend) => SourceLineRole::PlainAssignmentLeft,
        Some(SourceScope::Level) if starts_level_legend(tokens) => {
            SourceLineRole::PlainAfterKeywordAssignmentLeft
        }
        Some(SourceScope::Level | SourceScope::UnbracedLevel)
            if crate::is_level_event_sugar(trimmed, tokens) =>
        {
            SourceLineRole::Normal
        }
        Some(SourceScope::Level | SourceScope::UnbracedLevel) => SourceLineRole::Raw,
        Some(SourceScope::VisualShapeEntry) if trimmed.ends_with('{') => {
            SourceLineRole::PlainFirstToken
        }
        Some(SourceScope::VisualShapeEntry)
            if is_visual_directive_row(tokens)
                || is_visual_palette_row(tokens)
                || is_visual_duration_row(tokens) =>
        {
            SourceLineRole::Normal
        }
        Some(SourceScope::VisualShapeEntry) => SourceLineRole::Raw,
        Some(SourceScope::VisualColorTable) => SourceLineRole::PlainAssignmentLeft,
        _ => SourceLineRole::Normal,
    }
}

fn is_visual_directive_row(tokens: &[&str]) -> bool {
    crate::visual_authoring::is_visual_property_tokens(tokens)
}

fn is_visual_palette_row(tokens: &[&str]) -> bool {
    !tokens.is_empty()
        && tokens
            .iter()
            .all(|token| *token == "transparent" || token.starts_with('#'))
}

fn is_visual_duration_row(tokens: &[&str]) -> bool {
    let [value] = tokens else {
        return false;
    };
    crate::visual_authoring::is_visual_duration_token(value)
}

fn starts_unbraced_visual_entry(trimmed: &str, tokens: &[&str]) -> bool {
    !trimmed.ends_with('{') && is_unbraced_visual_entry_header(tokens)
}

fn is_unbraced_visual_entry_header(tokens: &[&str]) -> bool {
    let Some(name) = tokens.first() else {
        return false;
    };
    if !is_visual_selector_header_token(name) || is_visual_directive_row(tokens) {
        return false;
    }
    tokens
        .iter()
        .skip(1)
        .all(|token| *token == "transparent" || token.starts_with('#'))
}

fn next_unbraced_visual_shape_body(
    current: Option<SourceScope>,
    trimmed: &str,
    tokens: &[&str],
    in_unbraced_visual_shape_body: bool,
) -> bool {
    if current != Some(SourceScope::VisualShapeTable) || trimmed.is_empty() || trimmed == "}" {
        return false;
    }
    if in_unbraced_visual_shape_body {
        return true;
    }
    matches!(tokens, [name] if is_surface_source_identifier(name)) && !trimmed.ends_with('{')
}

fn is_visual_selector_header_token(value: &str) -> bool {
    puzzle_authoring::is_visual_definition_target(value)
}

fn is_surface_source_identifier(value: &str) -> bool {
    let Some(first) = value.chars().next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && value
            .chars()
            .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn starts_level_legend(tokens: &[&str]) -> bool {
    tokens.first().copied() == Some("legend")
}

fn surface_source_tokens(line: &str) -> Vec<&str> {
    line.split(|ch: char| ch.is_whitespace() || matches!(ch, '{' | '}' | ',' | ';'))
        .filter(|token| !token.is_empty())
        .collect()
}

fn source_tokens_for_structural_piece(
    source_line: &str,
    line_start: usize,
    piece: &str,
    cursor: &mut usize,
) -> Option<(Vec<SourceToken>, Option<(usize, usize)>)> {
    let mut tokens = Vec::new();
    let mut span_start = None;
    let mut span_end = None;
    for text in surface_source_tokens(piece) {
        let search = source_line.get(*cursor..)?;
        let relative = search.find(text)?;
        let start = *cursor + relative;
        let end = start + text.len();
        span_start.get_or_insert(line_start + start);
        span_end = Some(line_start + end);
        tokens.push(SourceToken {
            text: text.to_string(),
            start: line_start + start,
            end: line_start + end,
        });
        *cursor = end;
    }
    let delimiter = if piece.trim() == "}" {
        Some('}')
    } else if piece.trim_end().ends_with('{') {
        Some('{')
    } else {
        None
    };
    if let Some(delimiter) = delimiter {
        let search = source_line.get(*cursor..)?;
        let relative = search.find(delimiter)?;
        let start = *cursor + relative;
        let end = start + delimiter.len_utf8();
        span_start.get_or_insert(line_start + start);
        span_end = Some(line_start + end);
        *cursor = end;
    }
    Some((tokens, span_start.zip(span_end)))
}

fn surface_source_token_spans(line: &str, line_offset: usize) -> Vec<SourceToken> {
    let mut tokens = Vec::new();
    let mut start = None;
    for (index, ch) in line.char_indices() {
        if ch.is_whitespace() || matches!(ch, '{' | '}' | ',' | ';') {
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

fn push_close_events(count: usize, events: &mut Vec<SourceStructureEvent>) {
    events.extend((0..count).map(|_| SourceStructureEvent::Close));
}

fn structural_header(line: &str) -> String {
    line.strip_suffix('{')
        .map(str::trim)
        .unwrap_or_else(|| line.trim())
        .to_string()
}

fn close_blank_line(block_stack: &mut Vec<SourceBlockStackEntry>) -> usize {
    if block_stack.last().is_some_and(|entry| entry.virtual_braces) {
        block_stack.pop();
        return 1;
    }
    0
}

fn close_virtual_block(block_stack: &mut Vec<SourceBlockStackEntry>) -> usize {
    if block_stack.last().is_some_and(|entry| entry.virtual_braces) {
        block_stack.pop();
        return 1;
    }
    0
}

fn close_block_line(block_stack: &mut Vec<SourceBlockStackEntry>) -> usize {
    if block_stack.last().is_some_and(|entry| entry.virtual_braces) {
        let closed = block_stack.pop().expect("checked above");
        if block_stack.last().is_some_and(|entry| {
            matches!(
                (entry.scope, closed.scope),
                (SourceScope::Levels, SourceScope::UnbracedLevel)
                    | (SourceScope::Visuals, SourceScope::VisualShapeEntry)
            )
        }) {
            block_stack.pop();
            return 2;
        }
        return 1;
    }
    usize::from(block_stack.pop().is_some())
}

fn source_opens_block(line: &str, tokens: &[&str], current: Option<SourceScope>) -> bool {
    if current == Some(SourceScope::Levels) && !tokens.is_empty() {
        return true;
    }
    if current == Some(SourceScope::Visuals) && is_unbraced_visual_entry_header(tokens) {
        return true;
    }
    line.ends_with('{')
}

fn source_block_role(
    line: &str,
    tokens: &[&str],
    current: Option<SourceScope>,
    opened: SourceScope,
) -> SourceBlockRole {
    let first = tokens.first().copied().unwrap_or("");
    if is_statement_block_header(line, tokens, current, opened) || is_statement_control_flow(first)
    {
        SourceBlockRole::Statement
    } else {
        SourceBlockRole::SourceTree
    }
}

fn source_block_uses_virtual_braces(
    line: &str,
    current: Option<SourceScope>,
    opened: SourceScope,
) -> bool {
    opened == SourceScope::UnbracedLevel
        || (current == Some(SourceScope::Visuals)
            && opened == SourceScope::VisualShapeEntry
            && !line.trim_end().ends_with('{'))
}

fn is_statement_block_header(
    line: &str,
    tokens: &[&str],
    current: Option<SourceScope>,
    opened: SourceScope,
) -> bool {
    if matches!(
        puzzle_authoring::rule_statement_block_surface(line, current == Some(SourceScope::Other),),
        Some(puzzle_authoring::RuleStatementBlockSurface::Nested)
    ) {
        return true;
    }
    if structural_header(line).trim_end().ends_with("->") || line.contains("->") {
        return true;
    }
    if current == Some(SourceScope::SceneLayout) && opened == SourceScope::SceneTransitions {
        return true;
    }
    matches!(
        (current, tokens),
        (
            Some(SourceScope::SceneTransitions),
            ["input", ..] | ["action", ..] | ["if", ..]
        )
    )
}

fn is_statement_control_flow(kind: &str) -> bool {
    matches!(kind, "repeat" | "if" | "else" | "for")
}

fn opening_scope(line: &str, tokens: &[&str], current: Option<SourceScope>) -> Option<SourceScope> {
    if is_scene_scope(current) {
        match tokens {
            ["layout", ..]
            | ["row", ..]
            | ["column", ..]
            | ["box", ..]
            | ["for", ..]
            | ["puzzle", ..] => {
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
    if current == Some(SourceScope::Puzzle) && matches!(tokens, ["layout", ..]) {
        return Some(SourceScope::SceneLayout);
    }
    if matches!(
        current,
        Some(SourceScope::VisualShapeTable | SourceScope::VisualShapeEntry)
    ) && line.ends_with('{')
    {
        return Some(SourceScope::VisualShapeEntry);
    }
    if current == Some(SourceScope::Visuals) && line.ends_with('{') {
        match tokens {
            ["palette"] => return Some(SourceScope::VisualColorTable),
            ["shapes"] => return Some(SourceScope::VisualShapeTable),
            [..] => return Some(SourceScope::VisualShapeEntry),
        }
    }
    if current == Some(SourceScope::Visuals) && is_unbraced_visual_entry_header(tokens) {
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
        ["scene", ..] => Some(SourceScope::Scene),
        ["puzzle", ..] => Some(SourceScope::Puzzle),
        ["level", ..] => Some(SourceScope::Level),
        ["shapes"] => Some(SourceScope::VisualShapeTable),
        ["palette"] => Some(SourceScope::VisualColorTable),
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
        )
    )
}

fn source_scope_for_name(name: &str) -> Option<SourceScope> {
    if let Some(scope) = source_scope_for_authoring_source_block(name) {
        return Some(scope);
    }
    match name {
        "puzzle" => Some(SourceScope::Puzzle),
        "tags" => Some(SourceScope::Tags),
        "layers" => Some(SourceScope::Layers),
        "groups" => Some(SourceScope::Group),
        "marks" => Some(SourceScope::Mark),
        "map" => Some(SourceScope::Map),
        "keys" | "inputs" => Some(SourceScope::Keys),
        "resources" => Some(SourceScope::Other),
        "legend" => Some(SourceScope::Legend),
        "win_conditions" | "lose_conditions" => Some(SourceScope::Condition),
        "render" | "camera" => Some(SourceScope::Other),
        _ => None,
    }
}

fn source_scope_for_authoring_source_block(name: &str) -> Option<SourceScope> {
    let spec = crate::authoring_grammar::authoring_source_block(name)?;
    Some(match spec.role {
        crate::authoring_grammar::AuthoringBlockRole::Visuals => SourceScope::Visuals,
        crate::authoring_grammar::AuthoringBlockRole::LevelList => SourceScope::Levels,
        crate::authoring_grammar::AuthoringBlockRole::LevelEntry => SourceScope::Level,
        crate::authoring_grammar::AuthoringBlockRole::Rules => SourceScope::Other,
    })
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
    use super::{
        SourceBraceDisposition, SourceStructureEvent, lexer, logical_lines, scan_surface_source,
        split_header_tokens, split_tokens,
    };
    use crate::surface::SurfaceOptionBlock;

    #[test]
    fn canonical_scan_is_the_strict_logical_line_owner() {
        let source = r#"
title = "Demo"
puzzle board {
layers { objects = Box }
visuals {
Box {
#fff #000
01

10
}
}
rules { if exists(Box) { [ Box ] -> [ Box ]; } }
levels {
legend {
B = Box
}
level "one" {
B

B
}
}
}
"#;
        let actual = scan_surface_source(source).strict_logical_lines().unwrap();
        let texts = actual
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            texts,
            vec![
                "title = \"Demo\"",
                "puzzle board {",
                "layers {",
                "objects = Box",
                "}",
                "visuals {",
                "Box {",
                "#fff #000",
                "01",
                "",
                "10",
                "}",
                "}",
                "rules {",
                "if exists(Box) {",
                "[ Box ] -> [ Box ]",
                "}",
                "}",
                "levels {",
                "legend {",
                "B = Box",
                "}",
                "level \"one\" {",
                "B",
                "",
                "B",
                "}",
                "}",
                "}",
            ]
        );
        assert_eq!(
            actual
                .iter()
                .filter(|line| line.text.is_empty())
                .map(|line| line.line)
                .collect::<Vec<_>>(),
            vec![9, 20]
        );
    }

    #[test]
    fn structural_logical_lines_retain_delimiter_source_spans() {
        let source = "puzzle demo {\nvisuals {\nvisual Goal {\nshape = {\n0\n}\n}\n}\n}\n";
        let actual = scan_surface_source(source).strict_logical_lines().unwrap();
        let source_slices = actual
            .iter()
            .map(|line| &source[line.source_start().unwrap()..line.source_end().unwrap()])
            .collect::<Vec<_>>();

        assert_eq!(
            source_slices,
            vec![
                "puzzle demo {",
                "visuals {",
                "visual Goal {",
                "shape = {",
                "0",
                "}",
                "}",
                "}",
                "}",
            ]
        );
    }

    #[test]
    fn canonical_scan_preserves_structural_recovery_for_strict_rejection() {
        let source = "title = \"unfinished\npuzzle board {\n}\n";
        let actual = scan_surface_source(source)
            .strict_logical_lines()
            .unwrap_err();

        assert!(
            actual
                .to_string()
                .contains("string literal is missing closing quote")
        );
    }

    #[test]
    fn inline_tags_are_scanned_as_an_owner_block() {
        let source = "puzzle board { tags { kind = 1...3 } rules { for value in kind { } } }\n";
        let actual = scan_surface_source(source).strict_logical_lines().unwrap();
        let texts = actual
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            texts,
            vec![
                "puzzle board {",
                "tags {",
                "kind = 1...3",
                "}",
                "rules {",
                "for value in kind {",
                "}",
                "}",
                "}",
            ]
        );
    }

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
    fn lexer_emits_context_free_braces_and_structural_parser_assigns_dispositions() {
        let lexical = lexer::scan_source_line_lexical_facts("outer { inner } }", 0);
        let lexical_braces = lexical
            .iter()
            .filter(|fact| matches!(fact.kind, lexer::SourceLexicalKind::Brace(_)))
            .collect::<Vec<_>>();
        assert_eq!(lexical_braces.len(), 3);
        assert!(
            lexical_braces
                .iter()
                .all(|fact| fact.brace_disposition.is_none())
        );

        let scan = scan_surface_source("outer { inner } }\n");
        let dispositions = scan.lines[0]
            .lexical_facts
            .iter()
            .filter_map(|fact| fact.brace_disposition)
            .collect::<Vec<_>>();
        assert_eq!(
            dispositions,
            vec![
                SourceBraceDisposition {
                    depth: 0,
                    matched_close: true,
                },
                SourceBraceDisposition {
                    depth: 0,
                    matched_close: true,
                },
                SourceBraceDisposition {
                    depth: 0,
                    matched_close: false,
                },
            ]
        );
    }

    #[test]
    fn structural_parser_assigns_authoring_owner_once_for_all_products() {
        let scan = scan_surface_source(
            "puzzle board {\nrender {\ngrid {\ntype = occupied_cells\n}\n}\n}\n",
        );
        let type_line = scan
            .lines
            .iter()
            .find(|line| line.content.starts_with("type"))
            .expect("grid definition line");
        assert_eq!(
            type_line.option_block,
            Some(SurfaceOptionBlock::Authoring(
                crate::authoring_grammar::AuthoringKind::PuzzleRenderGridConfig,
            ))
        );
        let grid_open = scan
            .lines
            .iter()
            .find(|line| line.content.starts_with("grid"))
            .expect("grid header");
        assert!(grid_open.structural_events.iter().any(|event| {
            matches!(
                event,
                SourceStructureEvent::Open {
                    option_block: SurfaceOptionBlock::Authoring(
                        crate::authoring_grammar::AuthoringKind::PuzzleRenderGridConfig
                    ),
                    ..
                }
            )
        }));
    }

    #[test]
    fn semicolon_sugar_feeds_each_piece_to_the_normal_authoring_owner() {
        use crate::authoring_grammar::AuthoringKind;

        let scan =
            scan_surface_source("puzzle board { render { grid { type = occupied_cells; } } }\n");
        let pieces = &scan.lines[0].structural_pieces;
        let recognized = pieces
            .iter()
            .filter(|piece| !piece.product.value.is_empty())
            .map(|piece| {
                (
                    piece.authoring_parent,
                    piece
                        .product
                        .value
                        .iter()
                        .map(|token| token.text.as_str())
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(
            recognized,
            vec![
                (None, vec!["puzzle", "board"]),
                (Some(AuthoringKind::Root), vec!["render"]),
                (Some(AuthoringKind::PuzzleRenderConfig), vec!["grid"],),
                (
                    Some(AuthoringKind::PuzzleRenderGridConfig),
                    vec!["type", "=", "occupied_cells"],
                ),
            ]
        );
    }

    #[test]
    fn incremental_surface_scan_reuses_prefix_and_matches_full_scan() {
        let old = "puzzle board {\n  rules {\n    [ Player ] -> [ Player ]\n  }\n}\n// old\n";
        let new = "puzzle board {\n  rules {\n    [ Player ] -> [ Player ]\n  }\n}\n// new value\n";
        let edit_start = old.find("old").expect("edit start");
        let mut incremental = scan_surface_source(old);

        let rescanned = incremental.apply_edit(old, new, edit_start, edit_start + 3, 9);

        assert_eq!(rescanned, 1);
        assert_eq!(incremental, scan_surface_source(new));
    }

    #[test]
    fn incremental_surface_scan_recomputes_structural_suffix() {
        let old = "puzzle board {\n  rules {\n  }\n}\nscene menu {\n}\n";
        let new = "puzzle board {\n  rules\n  }\n}\nscene menu {\n}\n";
        let edit_start = old.find("rules {").expect("edit start") + "rules".len();
        let mut incremental = scan_surface_source(old);

        let rescanned = incremental.apply_edit(old, new, edit_start, edit_start + 1, 0);

        assert_eq!(rescanned, 5);
        assert_eq!(incremental, scan_surface_source(new));
    }

    #[test]
    fn incremental_surface_scan_reuses_suffix_after_state_converges() {
        let old = "puzzle board {\n  // old\n  rules {\n  }\n}\nscene menu {\n}\n";
        let new = "puzzle board {\n  // longer note\n  rules {\n  }\n}\nscene menu {\n}\n";
        let edit_start = old.find("old").expect("edit start");
        let mut incremental = scan_surface_source(old);

        let rescanned = incremental.apply_edit(old, new, edit_start, edit_start + 3, 11);

        assert_eq!(rescanned, 2);
        assert_eq!(incremental, scan_surface_source(new));
    }

    #[test]
    fn incremental_surface_scan_shifts_reused_lexical_facts() {
        let old = "title = x\npuzzle board {\n  rules {\n  }\n}\n";
        let new = "title = longer\npuzzle board {\n  rules {\n  }\n}\n";
        let edit_start = old.find('x').expect("edit start");
        let mut incremental = scan_surface_source(old);

        let rescanned = incremental.apply_edit(old, new, edit_start, edit_start + 1, 6);

        assert!(rescanned < incremental.line_count());
        assert_eq!(incremental, scan_surface_source(new));
    }

    #[test]
    fn logical_lines_preserve_else_block_braces() {
        let lines = logical_lines(
            r#"
rules {
if some([ Gate:1{checked} ]) {
[ Gate:1{checked} ] -> [ ]
} else {
[ Gate:1{checked} ] -> [ Gate:1 ]
}
}
"#,
        )
        .unwrap();

        assert!(lines.iter().any(|line| line == "else {"), "{lines:?}");
        assert!(!lines.iter().any(|line| line == "else"), "{lines:?}");
    }

    #[test]
    fn logical_lines_classify_braces_by_structural_header_not_keyword_exception() {
        let lines = logical_lines(
            r#"
scene title {
layout {
text if outer { if inner { "A }" } else { "B" } } else { "C" }
if outer { text "A" } else { text "B" }
}
}
"#,
        )
        .unwrap();

        assert!(
            lines
                .iter()
                .any(|line| line
                    == r#"text if outer { if inner { "A }" } else { "B" } } else { "C" }"#),
            "{lines:?}"
        );
        assert!(lines.iter().any(|line| line == "if outer {"), "{lines:?}");
        assert!(lines.iter().any(|line| line == r#"text "A""#), "{lines:?}");
        assert!(lines.iter().any(|line| line == "else {"), "{lines:?}");
        assert!(lines.iter().any(|line| line == r#"text "B""#), "{lines:?}");
    }

    #[test]
    fn surface_source_scan_uses_parser_structural_lines_for_scope_stack() {
        let source = r#"
puzzle board {
rules {
if some([ Gate:1{checked} ]) {
[ Gate:1{checked} ] -> [ ]
} else {
[ Gate:1{checked} ] -> [ Gate:1 ]
}
}
on_level_start {
}
}
"#;
        let context = scan_surface_source(source);
        let lifecycle_line = context
            .lines
            .iter()
            .find(|line| line.content.trim() == "on_level_start {")
            .unwrap();

        assert_eq!(lifecycle_line.scope, Some(super::SourceScope::Puzzle));
    }

    #[test]
    fn surface_source_scan_preserves_token_spans_before_comments() {
        let source = "scene title {\n  button start -> goto playing // comment\n}\n";
        let context = scan_surface_source(source);
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

    #[test]
    fn surface_source_scan_uses_parser_rule_statement_block_surface() {
        let source = r#"
puzzle board {
rules {
fix once {
[ Player ] -> [ Player ]
}
}
}
"#;
        let context = scan_surface_source(source);
        let fix_line = context
            .lines
            .iter()
            .find(|line| line.content.trim() == "fix once {")
            .unwrap();

        assert!(fix_line.structural_events.iter().any(|event| {
            matches!(
                event,
                super::SourceStructureEvent::Open {
                    header,
                    role: super::SourceBlockRole::Statement,
                    ..
                } if header == "fix once"
            )
        }));
    }

    #[test]
    fn surface_source_scan_keeps_rule_program_and_routine_in_source_tree() {
        let source = r#"
puzzle board {
routine push once {
[ Player | Crate ] -> [ Player | Crate ]
}
rules {
push
}
on_level_start {
push
}
}
"#;
        let context = scan_surface_source(source);

        for header in ["routine push once", "rules", "on_level_start"] {
            let line = context
                .lines
                .iter()
                .find(|line| line.content.trim() == format!("{header} {{"))
                .unwrap();
            assert!(line.structural_events.iter().any(|event| {
                matches!(
                    event,
                    super::SourceStructureEvent::Open {
                        header: opened_header,
                        role: super::SourceBlockRole::SourceTree,
                        ..
                    } if opened_header == header
                )
            }));
        }
    }

    #[test]
    fn surface_source_scan_keeps_scene_scope_after_choice_block() {
        let source = r#"
puzzle board {
scene title {
choice "New Game" -> {
start playing
}
}
scene playing {
layout {
}
}
}
"#;
        let context = scan_surface_source(source);
        let layout_line = context
            .lines
            .iter()
            .find(|line| line.content.trim() == "layout {")
            .unwrap();

        assert!(layout_line.structural_events.iter().any(|event| {
            matches!(
                event,
                super::SourceStructureEvent::Open {
                    header,
                    scope: super::SourceScope::SceneLayout,
                    ..
                } if header == "layout"
            )
        }));
    }

    #[test]
    fn surface_source_scan_closes_unbraced_levels_before_scene() {
        let source = r#"
levels board of board {
legend {
. = empty
P = Player
}
level one
P.
}

scene playing {
layout {
}
}
"#;
        let context = scan_surface_source(source);
        let layout_line = context
            .lines
            .iter()
            .find(|line| line.content.trim() == "layout {")
            .unwrap();

        assert!(layout_line.structural_events.iter().any(|event| {
            matches!(
                event,
                super::SourceStructureEvent::Open {
                    header,
                    scope: super::SourceScope::SceneLayout,
                    ..
                } if header == "layout"
            )
        }));
    }

    #[test]
    fn surface_source_scan_recognizes_puzzle_default_scene_layout() {
        let source = r#"
puzzle board {
layout {
title
}
}
"#;
        let context = scan_surface_source(source);
        let layout_line = context
            .lines
            .iter()
            .find(|line| line.content.trim() == "layout {")
            .unwrap();

        assert!(layout_line.structural_events.iter().any(|event| {
            matches!(
                event,
                super::SourceStructureEvent::Open {
                    header,
                    scope: super::SourceScope::SceneLayout,
                    ..
                } if header == "layout"
            )
        }));
    }

    #[test]
    fn surface_source_scan_keeps_unbraced_visual_shape_rows_raw() {
        let source = r#"
visuals {
shapes {
Box
aaa
111

Pull
0
}
}
"#;
        let context = scan_surface_source(source);
        let box_header = source.find("Box").unwrap();
        let box_row = source.find("aaa").unwrap();
        let pull_header = source.find("Pull").unwrap();
        let pull_row = source.rfind("\n0").unwrap() + 1;

        assert!(
            !context
                .raw_ranges()
                .iter()
                .any(|(start, _)| *start == box_header)
        );
        assert!(
            context
                .raw_ranges()
                .iter()
                .any(|(start, _)| *start == box_row)
        );
        assert!(
            !context
                .raw_ranges()
                .iter()
                .any(|(start, _)| *start == pull_header)
        );
        assert!(
            context
                .raw_ranges()
                .iter()
                .any(|(start, _)| *start == pull_row)
        );
    }
}
