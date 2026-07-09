use crate::DiagnosticReport;
use crate::syntax::puzzle_lifecycle_event;

#[cfg(test)]
pub(crate) fn logical_lines(source: &str) -> Result<Vec<String>, DiagnosticReport> {
    logical_lines_with_locations(source)
        .map(|lines| lines.into_iter().map(|line| line.text).collect())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LogicalLine {
    pub(crate) text: String,
    pub(crate) line: usize,
}

impl LogicalLine {
    fn new(text: impl Into<String>, line: usize) -> Self {
        Self {
            text: text.into(),
            line,
        }
    }

    fn with_text(&self, text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            line: self.line,
        }
    }
}

pub(crate) fn logical_lines_with_locations(
    source: &str,
) -> Result<Vec<LogicalLine>, DiagnosticReport> {
    let mut lines = Vec::new();
    let mut preserve_level_blanks = false;
    let mut preserve_sprite_blanks = false;
    let mut level_brace_depth = 0i32;
    let mut sprite_brace_depth = 0i32;
    let mut level_end_depth = None::<usize>;
    let raw_lines = source
        .lines()
        .enumerate()
        .map(|(index, raw_line)| {
            LogicalLine::new(strip_line_comment(raw_line).trim().to_string(), index + 1)
        })
        .collect::<Vec<_>>();
    let raw_lines = expand_structural_sugar(&raw_lines)?;

    for index in 0..raw_lines.len() {
        let logical_line = &raw_lines[index];
        let line = logical_line.text.as_str();
        if line.is_empty() {
            if preserve_level_blanks || preserve_sprite_blanks {
                lines.push(logical_line.clone());
            }
            continue;
        }

        let tokens = split_header_tokens(line);
        if matches!(
            tokens.as_slice(),
            ["sprites"] | ["sprites", ..] | ["sprites3"] | ["sprites3", ..]
        ) && line.ends_with('{')
            && !preserve_level_blanks
        {
            preserve_sprite_blanks = true;
            sprite_brace_depth = 0;
        }
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
        if preserve_sprite_blanks {
            sprite_brace_depth += line.chars().filter(|ch| *ch == '{').count() as i32;
            sprite_brace_depth -= line.chars().filter(|ch| *ch == '}').count() as i32;
        }
        lines.push(logical_line.clone());
        if preserve_level_blanks && level_brace_depth <= 0 && level_end_depth.is_none() {
            preserve_level_blanks = false;
        }
        if level_end_depth == Some(0) {
            preserve_level_blanks = false;
            level_end_depth = None;
        }
        if preserve_sprite_blanks && sprite_brace_depth <= 0 {
            preserve_sprite_blanks = false;
        }
    }
    normalize_brace_blocks(&lines)
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

fn expand_structural_sugar(lines: &[LogicalLine]) -> Result<Vec<LogicalLine>, DiagnosticReport> {
    let mut expanded = Vec::new();
    let mut block_stack = Vec::<String>::new();

    for logical_line in lines {
        if logical_line.text.is_empty() {
            expanded.push(logical_line.clone());
            continue;
        }

        expanded.extend(
            expand_structural_source_line(&logical_line.text, &mut block_stack)?
                .into_iter()
                .map(|text| logical_line.with_text(text)),
        );
    }

    Ok(expanded)
}

fn expand_structural_source_line(
    line: &str,
    block_stack: &mut Vec<String>,
) -> Result<Vec<String>, DiagnosticReport> {
    let split_semicolons = !block_stack.iter().any(|block| ascii_sensitive_block(block));
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
    matches!(
        block,
        "levels" | "levels3" | "sprites" | "sprite" | "sprites3" | "map"
    )
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

fn normalize_brace_blocks(lines: &[LogicalLine]) -> Result<Vec<LogicalLine>, DiagnosticReport> {
    let mut normalized = Vec::new();
    let mut levels_brace_depth = 0i32;

    for logical_line in lines {
        normalize_brace_block_line(logical_line, &mut levels_brace_depth, &mut normalized)?;
    }

    Ok(normalized)
}

fn normalize_brace_block_line(
    logical_line: &LogicalLine,
    levels_brace_depth: &mut i32,
    normalized: &mut Vec<LogicalLine>,
) -> Result<(), DiagnosticReport> {
    let line = logical_line.text.as_str();
    if line == "}" {
        normalized.push(logical_line.with_text("}"));
        if *levels_brace_depth > 0 {
            *levels_brace_depth -= 1;
        }
        return Ok(());
    }

    if let Some(rest) = line.strip_prefix('}') {
        let rest = rest.trim_start();
        match rest {
            "else" => {
                normalized.push(logical_line.with_text("}"));
                normalized.push(logical_line.with_text("else"));
            }
            "else {" | "else{" => {
                normalized.push(logical_line.with_text("}"));
                normalized.push(logical_line.with_text("else {"));
            }
            rest if rest.starts_with("->") => {
                normalized.push(logical_line.with_text("}"));
                normalized.push(logical_line.with_text(rest));
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
        normalized.push(logical_line.with_text("else {"));
        return Ok(());
    }

    if line == "{" {
        normalized.push(logical_line.with_text("{"));
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
            normalized.push(logical_line.with_text(format!("{header} {{")));
            return Ok(());
        }
        if preserve_level_header {
            normalized.push(logical_line.with_text(format!("{header} {{")));
            if *levels_brace_depth > 0 || is_levels_header {
                *levels_brace_depth += 1;
            }
            return Ok(());
        }
        normalized.push(logical_line.with_text(format!("{header} {{")));
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
        ["levels"]
            | ["levels", "of", _]
            | ["levels", _, "of", _]
            | ["levels3"]
            | ["levels3", "of", _]
            | ["levels3", _, "of", _]
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
            | ["groups"]
            | ["layers"]
            | ["collision_layers"]
            | ["legend"]
            | ["win_conditions", ..]
            | ["lose_conditions", ..]
            | ["puzzle3", ..]
            | ["colors"]
            | ["shapes"]
            | ["objects"]
            | ["display_objects"]
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
            | ["rule", ..]
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
            | ["level_menu"]
            | ["fix", ..]
            | ["once"]
            | ["once_all"]
            | ["once_per_level"]
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
    LevelMenu,
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

#[derive(Clone, Debug)]
pub(crate) struct SurfaceSourceLine {
    pub(crate) tokens: Vec<String>,
    pub(crate) token_spans: Vec<SourceToken>,
    pub(crate) structural_token_spans: Vec<SourceToken>,
    pub(crate) structural_lines: Vec<String>,
    pub(crate) structural_events: Vec<SourceStructureEvent>,
    pub(crate) scope: Option<SourceScope>,
    pub(crate) start: usize,
    pub(crate) line: usize,
    pub(crate) content: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SourceStructureEvent {
    Open {
        header: String,
        scope: SourceScope,
        role: SourceBlockRole,
        virtual_braces: bool,
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
}

#[derive(Clone, Debug, Default)]
pub(crate) struct SurfaceSourceScan {
    raw: Vec<(usize, usize)>,
    plain: Vec<(usize, usize)>,
    pub(crate) lines: Vec<SurfaceSourceLine>,
}

impl SurfaceSourceScan {
    pub(crate) fn raw_ranges(&self) -> &[(usize, usize)] {
        &self.raw
    }

    pub(crate) fn plain_ranges(&self) -> &[(usize, usize)] {
        &self.plain
    }
}

pub(crate) fn scan_surface_source(source: &str) -> SurfaceSourceScan {
    let mut context = SurfaceSourceScan::default();
    let mut block_stack = Vec::<SourceBlockStackEntry>::new();
    let mut structural_block_stack = Vec::<String>::new();
    let mut normalize_levels_brace_depth = 0i32;
    let mut unbraced_visual_shape_body = false;
    let mut offset = 0usize;

    for (line_index, line) in source.split_inclusive('\n').enumerate() {
        let line_end = offset + line.len();
        let content_end = line_end - usize::from(line.ends_with('\n'));
        let content = &source[offset..content_end];
        let raw = strip_line_comment(content);
        let trimmed = raw.trim();
        let tokens = surface_source_tokens(trimmed);
        let mut structural_events = Vec::<SourceStructureEvent>::new();

        if block_stack.last().is_some_and(|entry| {
            entry.virtual_braces && entry.scope == SourceScope::VisualShapeEntry
        }) && starts_unbraced_visual_entry(trimmed, &tokens)
        {
            push_close_events(
                close_virtual_block(&mut block_stack),
                &mut structural_events,
            );
        }

        let current = block_stack.last().map(|entry| entry.scope);
        let in_unbraced_visual_shape_body = unbraced_visual_shape_body
            && current == Some(SourceScope::VisualShapeTable)
            && !trimmed.is_empty()
            && trimmed != "}";

        if trimmed.is_empty() {
            push_close_events(close_blank_line(&mut block_stack), &mut structural_events);
        }

        if !trimmed.is_empty() && trimmed != "}" {
            match source_line_role(current, trimmed, &tokens, in_unbraced_visual_shape_body) {
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

        unbraced_visual_shape_body = next_unbraced_visual_shape_body(
            current,
            trimmed,
            &tokens,
            in_unbraced_visual_shape_body,
        );

        let structural_lines = surface_source_stack_lines(
            trimmed,
            &mut structural_block_stack,
            &mut normalize_levels_brace_depth,
        );
        for stack_line in &structural_lines {
            let tokens = surface_source_tokens(&stack_line);
            let current = block_stack.last().map(|entry| entry.scope);
            if stack_line == "}" {
                push_close_events(close_block_line(&mut block_stack), &mut structural_events);
            } else if source_opens_block(&stack_line, &tokens, current)
                && let Some(opened) = opening_scope(&stack_line, &tokens, current)
            {
                let role = source_block_role(&stack_line, &tokens, current, opened);
                let virtual_braces = source_block_uses_virtual_braces(&stack_line, current, opened);
                structural_events.push(SourceStructureEvent::Open {
                    header: structural_header(stack_line),
                    scope: opened,
                    role,
                    virtual_braces,
                });
                block_stack.push(SourceBlockStackEntry {
                    scope: opened,
                    virtual_braces,
                });
            }
        }

        context.lines.push(SurfaceSourceLine {
            tokens: tokens.iter().map(|token| (*token).to_string()).collect(),
            token_spans: source_line_tokens(raw, offset),
            structural_token_spans: surface_source_token_spans(raw, offset),
            structural_lines,
            structural_events,
            scope: current,
            start: offset,
            line: line_index + 1,
            content: content.to_string(),
        });

        offset = line_end;
    }

    context
}

fn surface_source_stack_lines(
    trimmed: &str,
    structural_block_stack: &mut Vec<String>,
    normalize_levels_brace_depth: &mut i32,
) -> Vec<String> {
    if trimmed.is_empty() {
        return Vec::new();
    }

    let mut normalized = Vec::new();
    let expanded = match expand_structural_source_line(trimmed, structural_block_stack) {
        Ok(expanded) => expanded,
        Err(_) => return Vec::new(),
    };
    for line in expanded {
        let logical_line = LogicalLine::new(line, 0);
        if normalize_brace_block_line(&logical_line, normalize_levels_brace_depth, &mut normalized)
            .is_err()
        {
            return Vec::new();
        }
    }
    normalized
        .into_iter()
        .map(|logical_line| logical_line.text)
        .collect()
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
            if is_visual_sprite_directive_row(tokens)
                || is_visual_sprite_palette_row(tokens)
                || is_visual_sprite_duration_row(tokens) =>
        {
            SourceLineRole::Normal
        }
        Some(SourceScope::VisualShapeEntry) => SourceLineRole::Raw,
        Some(SourceScope::VisualColorTable) => SourceLineRole::PlainAssignmentLeft,
        _ => SourceLineRole::Normal,
    }
}

fn is_visual_sprite_directive_row(tokens: &[&str]) -> bool {
    matches!(
        tokens,
        ["colors", ..]
            | ["image", ..]
            | ["contain", ..]
            | ["cover", ..]
            | ["stretch", ..]
            | ["offset", ..]
            | ["sampling", ..]
            | ["pixels_per_cell", ..]
            | ["rotate", ..]
            | ["shape", ..]
    )
}

fn is_visual_sprite_palette_row(tokens: &[&str]) -> bool {
    !tokens.is_empty()
        && tokens
            .iter()
            .all(|token| *token == "transparent" || token.starts_with('#'))
}

fn is_visual_sprite_duration_row(tokens: &[&str]) -> bool {
    let [value] = tokens else {
        return false;
    };
    let Some(number) = value
        .strip_suffix("ms")
        .or_else(|| value.strip_suffix('s'))
    else {
        return false;
    };
    !number.is_empty() && number.chars().any(|ch| ch.is_ascii_digit())
}

fn starts_unbraced_visual_entry(trimmed: &str, tokens: &[&str]) -> bool {
    !trimmed.ends_with('{') && is_unbraced_visual_entry_header(tokens)
}

fn is_unbraced_visual_entry_header(tokens: &[&str]) -> bool {
    let Some(name) = tokens.first() else {
        return false;
    };
    if !is_visual_sprite_selector_header_token(name) || is_visual_sprite_directive_row(tokens) {
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

fn is_visual_sprite_selector_header_token(value: &str) -> bool {
    if matches!(
        value,
        "shape" | "shapes" | "colors" | "ascii" | "sprites" | "sprites3"
    ) {
        return false;
    }
    let cleaned = value.trim_start_matches('@');
    let mut parts = cleaned.split(':');
    let Some(first) = parts.next() else {
        return false;
    };
    is_surface_source_identifier(first) && parts.all(is_visual_sprite_selector_part_token)
}

fn is_visual_sprite_selector_part_token(value: &str) -> bool {
    value == "*"
        || (!value.is_empty()
            && value
                .chars()
                .all(|ch| ch == '_' || ch.is_ascii_alphanumeric()))
        || is_visual_sprite_selector_map_call(value)
}

fn is_visual_sprite_selector_map_call(value: &str) -> bool {
    let Some((name, rest)) = value.split_once('(') else {
        return false;
    };
    let Some(arg) = rest.strip_suffix(')') else {
        return false;
    };
    is_surface_source_identifier(name) && is_surface_source_identifier(arg)
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
    if current == Some(SourceScope::Visuals) && line.ends_with('{') {
        match tokens {
            ["colors"] => return Some(SourceScope::VisualColorTable),
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
        ["puzzle", ..] | ["puzzle3", ..] => Some(SourceScope::Puzzle),
        ["level", ..] => Some(SourceScope::Level),
        ["shapes"] => Some(SourceScope::VisualShapeTable),
        ["colors"] => Some(SourceScope::VisualColorTable),
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
    if let Some(scope) = source_scope_for_authoring_source_block(name) {
        return Some(scope);
    }
    match name {
        "puzzle" => Some(SourceScope::Puzzle),
        "tags" => Some(SourceScope::Tags),
        "layers" | "collision_layers" => Some(SourceScope::Layers),
        "groups" => Some(SourceScope::Group),
        "marks" => Some(SourceScope::Mark),
        "map" => Some(SourceScope::Map),
        "keys" | "inputs" => Some(SourceScope::Keys),
        "resources" => Some(SourceScope::Other),
        "legend" => Some(SourceScope::Legend),
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
    use super::{logical_lines, scan_surface_source, split_header_tokens, split_tokens};

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
    fn surface_source_scan_keeps_unbraced_visual_shape_rows_raw() {
        let source = r#"
sprites {
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
