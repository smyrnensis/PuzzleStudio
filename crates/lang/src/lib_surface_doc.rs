pub fn parse_game(source: &str) -> Result<LoadedDocument, DiagnosticReport> {
    parse_game_document(source)
}

pub fn parse_game_for_path(
    source: &str,
    path: impl AsRef<Path>,
) -> Result<LoadedDocument, DiagnosticReport> {
    validate_source_profile_for_path(source, path)?;
    parse_game_document(source)
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocumentRuntimeSources {
    pub model_2d: String,
    pub model_3d: String,
}

pub fn split_document_runtime_sources(
    source: &str,
) -> Result<DocumentRuntimeSources, DiagnosticReport> {
    let mixed_sources = split_mixed_game_document_source(source)?;
    Ok(DocumentRuntimeSources {
        model_2d: strip_document_shell_source(&mixed_sources.puzzle2d)?,
        model_3d: strip_document_shell_source(&mixed_sources.puzzle3d)?,
    })
}

pub fn parse_game2d(source: &str) -> Result<LoadedGame, DiagnosticReport> {
    parse_game2d_document(source)
}

#[derive(Clone, Debug, Default)]
struct SurfaceScan {
    raw_ranges: Vec<SourceSpan>,
    plain_ranges: Vec<SourceSpan>,
    lines: Vec<SurfaceScanLine>,
}

#[derive(Clone, Debug)]
struct SurfaceScanLine {
    tokens: Vec<String>,
    token_spans: Vec<SourceToken>,
    structural_token_spans: Vec<SourceToken>,
    structural_lines: Vec<String>,
    structural_events: Vec<source::SourceStructureEvent>,
    visual_scope: Option<SurfaceVisualScope>,
    scope: Option<SourceScope>,
    start: usize,
    line: usize,
    content: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SurfaceVisualScope {
    Sprites,
    SpriteEntry,
    Colors,
    ColorTable,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SurfaceDocumentProducts {
    semantic_tokens: bool,
    completion_symbols: bool,
    highlight_ranges: bool,
    visual_sprite_refs: bool,
}

impl SurfaceDocumentProducts {
    const FULL: Self = Self {
        semantic_tokens: true,
        completion_symbols: true,
        highlight_ranges: true,
        visual_sprite_refs: true,
    };

    const STRUCTURE_ONLY: Self = Self {
        semantic_tokens: false,
        completion_symbols: false,
        highlight_ranges: false,
        visual_sprite_refs: false,
    };

    const SOURCE_TARGET: Self = Self {
        semantic_tokens: false,
        completion_symbols: false,
        highlight_ranges: false,
        visual_sprite_refs: true,
    };

    const COMPLETION_SYMBOLS: Self = Self {
        semantic_tokens: false,
        completion_symbols: true,
        highlight_ranges: false,
        visual_sprite_refs: false,
    };

    fn needs_parser_catalog(self) -> bool {
        self.semantic_tokens || self.completion_symbols
    }
}

pub(crate) fn parse_surface_document(source: &str) -> SurfaceDocument {
    build_surface_document(source, SurfaceDocumentProducts::FULL)
}

fn parse_surface_compile_document(source: &str) -> Result<SurfaceDocument, DiagnosticReport> {
    let mut document = parse_surface_document(source);
    document.logical_lines = logical_lines_with_locations(source)?;
    Ok(document)
}

pub fn validate_surface_document_projection(source: &str) -> Result<(), DiagnosticReport> {
    try_build_surface_document(source, SurfaceDocumentProducts::FULL, true).map(|_| ())
}

pub(crate) fn parse_surface_structure_document(source: &str) -> SurfaceDocument {
    build_surface_document(source, SurfaceDocumentProducts::STRUCTURE_ONLY)
}

pub(crate) fn parse_surface_completion_context_document(source: &str) -> SurfaceDocument {
    build_surface_document(source, SurfaceDocumentProducts::STRUCTURE_ONLY)
}

pub(crate) fn parse_surface_completion_symbols_document(source: &str) -> SurfaceDocument {
    build_surface_document(source, SurfaceDocumentProducts::COMPLETION_SYMBOLS)
}

fn parse_surface_source_target_document(source: &str) -> SurfaceDocument {
    build_surface_document(source, SurfaceDocumentProducts::SOURCE_TARGET)
}

fn build_surface_document(source: &str, products: SurfaceDocumentProducts) -> SurfaceDocument {
    try_build_surface_document(source, products, false).expect("surface document scan failed")
}

fn try_build_surface_document(
    source: &str,
    products: SurfaceDocumentProducts,
    strict_projection: bool,
) -> Result<SurfaceDocument, DiagnosticReport> {
    let scan = scan_surface_document_source(source);
    let mut sink = SurfaceSink::default();
    let structural_blocks = surface_structural_blocks(&scan);
    sink.set_structural_blocks(structural_blocks.clone());
    if products.completion_symbols {
        record_surface_builtin_completion_symbols(&mut sink);
        record_surface_completion_value_sets(&scan, &mut sink);
    }
    let parser_catalog = products
        .needs_parser_catalog()
        .then(|| parser_surface_catalog(source))
        .flatten();
    if products.semantic_tokens {
        record_structural_block_surface_tokens(&structural_blocks, strict_projection, &mut sink)?;
    }
    let mut option_stack = Vec::<SurfaceOptionBlock>::new();
    for line in &scan.lines {
        let option_block = active_surface_option_block(&option_stack);
        sink.line(
            line.tokens.clone(),
            line.token_spans.clone(),
            line.scope,
            line.start,
            line.line,
            line.content.clone(),
            option_block,
        );
        if products.semantic_tokens {
            record_surface_document_line(
                option_block,
                line.scope,
                line.start,
                &line.content,
                &line.structural_token_spans,
                &line.structural_lines,
                &line.structural_events,
                &mut sink,
            );
        }
        if products.completion_symbols {
            record_surface_completion_line(
                option_block,
                line.scope,
                &line.structural_token_spans,
                &mut sink,
            );
        }
        update_surface_option_block_stack(line, &mut option_stack);
    }
    if let Some(catalog) = parser_catalog.as_ref() {
        if products.semantic_tokens {
            record_parser_resolved_surface_tokens(&scan, catalog, &mut sink);
        }
        if products.completion_symbols {
            record_parser_catalog_completion_symbols(catalog, &mut sink);
        }
    }
    if products.highlight_ranges {
        sink.set_highlight_ranges(surface_highlight_ranges(&scan));
    }
    if products.visual_sprite_refs {
        sink.visual_sprite_refs_mut()
            .merge(surface_visual_sprite_refs(source, &scan));
    }
    if products.completion_symbols {
        normalize_surface_completion_symbols(&mut sink);
    }
    Ok(sink.into_document())
}

fn scan_surface_document_source(source: &str) -> SurfaceScan {
    let context = source::scan_surface_source(source);
    let mut scan = SurfaceScan {
        raw_ranges: context
            .raw_ranges()
            .iter()
            .map(|(start, end)| SourceSpan {
                start: *start,
                end: *end,
            })
            .collect(),
        plain_ranges: context
            .plain_ranges()
            .iter()
            .map(|(start, end)| SourceSpan {
                start: *start,
                end: *end,
            })
            .collect(),
        lines: context
            .lines
            .into_iter()
            .map(|line| SurfaceScanLine {
                tokens: line.tokens,
                token_spans: line.token_spans,
                structural_token_spans: line.structural_token_spans,
                structural_lines: line.structural_lines,
                structural_events: line.structural_events,
                visual_scope: None,
                scope: line.scope,
                start: line.start,
                line: line.line,
                content: line.content,
            })
            .collect(),
    };
    recognize_surface_scan_lines(&mut scan);
    scan
}

fn recognize_surface_scan_lines(scan: &mut SurfaceScan) {
    recognize_surface_visual_scopes(scan);
}

fn recognize_surface_visual_scopes(scan: &mut SurfaceScan) {
    let mut stack = Vec::<SurfaceVisualScope>::new();
    for line in &mut scan.lines {
        let current = stack.last().copied();
        line.visual_scope = current;
        if is_visual_closing_line(line) {
            stack.pop();
            continue;
        }
        if let Some(opened) = surface_visual_opening_scope(current, line) {
            stack.push(opened);
        }
    }
}

fn surface_structural_blocks(scan: &SurfaceScan) -> Vec<SurfaceStructuralBlock> {
    let mut blocks = Vec::<SurfaceStructuralBlock>::new();
    let mut stack = Vec::<usize>::new();
    let mut option_stack = Vec::<SurfaceOptionBlock>::new();
    for line in &scan.lines {
        for event in &line.structural_events {
            match event {
                source::SourceStructureEvent::Open {
                    header,
                    scope,
                    role,
                    virtual_braces,
                } => {
                    let tokens = split_header_tokens(header)
                        .into_iter()
                        .map(str::to_string)
                        .collect::<Vec<_>>();
                    let option_block = surface_option_block_for_opening(&tokens, &option_stack);
                    let authoring_content = tokens.first().and_then(|surface| {
                        authoring_grammar::authoring_source_block(surface)
                            .and_then(|spec| spec.content)
                    });
                    let parent = stack.iter().rev().find_map(|index| {
                        matches!(
                            blocks[*index].role,
                            SurfaceStructuralBlockRole::SourceTree
                        )
                        .then_some(*index)
                    });
                    let block = SurfaceStructuralBlock {
                        header: header.clone(),
                        scope: *scope,
                        role: surface_structural_block_role(*role),
                        authoring_kind: match option_block {
                            SurfaceOptionBlock::Authoring(kind) => Some(kind),
                            _ => None,
                        },
                        authoring_content,
                        virtual_braces: *virtual_braces,
                        start: line_start_offset(&line.content, line.start),
                        end: line.start + line.content.len(),
                        depth: stack
                            .iter()
                            .filter(|index| {
                                matches!(
                                    blocks[**index].role,
                                    SurfaceStructuralBlockRole::SourceTree
                                )
                            })
                            .count(),
                        parent,
                    };
                    let index = blocks.len();
                    blocks.push(block);
                    stack.push(index);
                    option_stack.push(option_block);
                }
                source::SourceStructureEvent::Close => {
                    stack.pop();
                    option_stack.pop();
                }
            }
        }
    }
    blocks
}

fn surface_structural_block_role(role: source::SourceBlockRole) -> SurfaceStructuralBlockRole {
    match role {
        source::SourceBlockRole::SourceTree => SurfaceStructuralBlockRole::SourceTree,
        source::SourceBlockRole::Statement => SurfaceStructuralBlockRole::Statement,
    }
}

fn line_start_offset(content: &str, line_start: usize) -> usize {
    line_start + content.len() - content.trim_start().len()
}

fn surface_highlight_ranges(scan: &SurfaceScan) -> SurfaceHighlightRanges {
    let mut ranges = SurfaceHighlightRanges {
        raw_ranges: scan.raw_ranges.clone(),
        plain_ranges: scan.plain_ranges.clone(),
        level_ascii_ranges: scan_level_ascii_surface_ranges(scan),
        visual_ascii_color_ranges: Vec::new(),
        visual_named_color_ranges: Vec::new(),
        visual_separator_ranges: Vec::new(),
    };
    ranges
        .level_ascii_ranges
        .extend(scan_visual_shape_ascii_surface_ranges(scan));
    let visual_color_aliases = scan_visual_color_aliases(scan);
    ranges.visual_named_color_ranges =
        scan_visual_named_color_surface_ranges(scan, &visual_color_aliases);
    ranges.visual_ascii_color_ranges =
        scan_visual_ascii_color_surface_ranges(scan, &visual_color_aliases);
    ranges.visual_separator_ranges = scan_visual_separator_surface_ranges(scan, &visual_color_aliases);
    ranges
}

#[derive(Clone, Debug)]
struct LevelAsciiScanLevel {
    variable_chars: HashSet<char>,
    local_chars: HashSet<char>,
    braced: bool,
    is_2d: bool,
}

#[derive(Clone, Copy, Debug)]
enum LevelLegendTarget {
    Variable { enabled: bool },
    Local(usize),
}

fn scan_level_ascii_surface_ranges(scan: &SurfaceScan) -> Vec<SurfaceAsciiRange> {
    let mut ranges = Vec::new();
    let mut variable_chars = HashSet::<char>::new();
    let mut levels = Vec::<LevelAsciiScanLevel>::new();
    let mut line_levels = vec![None::<usize>; scan.lines.len()];
    let mut current_level = None::<usize>;
    let mut level_legend_stack = Vec::<LevelLegendTarget>::new();
    let mut levels_2d_stack = Vec::<bool>::new();

    for (line_index, line) in scan.lines.iter().enumerate() {
        let raw = strip_line_comment(&line.content);
        let trimmed = raw.trim();
        let tokens = line.tokens.iter().map(String::as_str).collect::<Vec<_>>();

        if let Some(level_index) = current_level
            && !levels[level_index].braced
            && !matches!(
                line.scope,
                Some(SourceScope::UnbracedLevel | SourceScope::Legend | SourceScope::Other)
            )
        {
            current_level = None;
        }

        let levels_is_2d = levels_2d_stack.last().copied().unwrap_or(true);
        let implicit_level_row = current_level.is_none()
            && line.scope == Some(SourceScope::Levels)
            && levels_is_2d
            && starts_implicit_unbraced_level_row(trimmed, &tokens);
        if starts_level_header(line.scope, trimmed, &tokens, levels_is_2d) || implicit_level_row {
            let braced = trimmed.ends_with('{') || matches!(tokens.as_slice(), ["{"]);
            let level_index = levels.len();
            levels.push(LevelAsciiScanLevel {
                variable_chars: variable_chars.clone(),
                local_chars: HashSet::new(),
                braced,
                is_2d: levels_is_2d,
            });
            current_level = Some(level_index);
        }

        if let Some(ch) = inline_legend_directive_char(&tokens) {
            if let Some(level_index) = current_level
                && matches!(
                    line.scope,
                    Some(SourceScope::Level | SourceScope::UnbracedLevel)
                )
            {
                levels[level_index].local_chars.insert(ch);
            } else if levels_is_2d {
                variable_chars.insert(ch);
            }
        } else if let Some(target) = level_legend_stack.last().copied()
            && let Some(ch) = legend_row_char(&tokens)
        {
            match target {
                LevelLegendTarget::Variable { enabled } if enabled => {
                    variable_chars.insert(ch);
                }
                LevelLegendTarget::Local(level_index) => {
                    levels[level_index].local_chars.insert(ch);
                }
                _ => {}
            }
        }

        if let Some(level_index) = current_level
            && levels[level_index].is_2d
            && is_level_ascii_map_row(line.scope, trimmed, &tokens, implicit_level_row)
        {
            line_levels[line_index] = Some(level_index);
        }

        if opens_level_legend_block(trimmed, &tokens) {
            let target = if let Some(level_index) = current_level {
                if matches!(
                    line.scope,
                    Some(SourceScope::Level | SourceScope::UnbracedLevel)
                ) {
                    LevelLegendTarget::Local(level_index)
                } else {
                    LevelLegendTarget::Variable {
                        enabled: levels_is_2d,
                    }
                }
            } else {
                LevelLegendTarget::Variable {
                    enabled: levels_is_2d,
                }
            };
            level_legend_stack.push(target);
        }

        if starts_levels_block(&tokens) {
            levels_2d_stack.push(tokens.first().copied() == Some("levels"));
        }

        if line.scope == Some(SourceScope::Legend) && trimmed == "}" {
            level_legend_stack.pop();
        }
        if line.scope == Some(SourceScope::Level) && trimmed == "}" {
            current_level = None;
        }
        if line.scope == Some(SourceScope::Levels) && trimmed == "}" {
            levels_2d_stack.pop();
        }
    }

    for (line, level_index) in scan.lines.iter().zip(line_levels) {
        let Some(level_index) = level_index else {
            continue;
        };
        let mut known_chars = levels[level_index].variable_chars.clone();
        known_chars.extend(levels[level_index].local_chars.iter().copied());
        add_level_ascii_line_surface_ranges(&mut ranges, line, &known_chars);
    }

    ranges
}

fn starts_levels_block(tokens: &[&str]) -> bool {
    matches!(tokens.first().copied(), Some("levels" | "levels3"))
}

fn starts_level_header(
    scope: Option<SourceScope>,
    trimmed: &str,
    tokens: &[&str],
    levels_is_2d: bool,
) -> bool {
    if !levels_is_2d || trimmed.is_empty() {
        return false;
    }
    matches!(tokens, ["level", ..]) || (scope == Some(SourceScope::Levels) && matches!(tokens, ["{"]))
}

fn starts_implicit_unbraced_level_row(trimmed: &str, tokens: &[&str]) -> bool {
    !trimmed.is_empty()
        && trimmed != "}"
        && !trimmed.ends_with('{')
        && !matches!(tokens, ["legend", ..] | ["level", ..])
}

fn opens_level_legend_block(trimmed: &str, tokens: &[&str]) -> bool {
    matches!(tokens, ["legend"]) && (trimmed == "legend" || trimmed.ends_with('{'))
}

fn inline_legend_directive_char(tokens: &[&str]) -> Option<char> {
    match tokens {
        ["legend", ch, "=", ..] => single_char_token(ch),
        _ => None,
    }
}

fn legend_row_char(tokens: &[&str]) -> Option<char> {
    match tokens {
        [ch, "=", ..] => single_char_token(ch),
        _ => None,
    }
}

fn single_char_token(token: &str) -> Option<char> {
    let mut chars = token.chars();
    let ch = chars.next()?;
    chars.next().is_none().then_some(ch)
}

fn is_level_ascii_map_row(
    scope: Option<SourceScope>,
    trimmed: &str,
    tokens: &[&str],
    implicit_level_row: bool,
) -> bool {
    if trimmed.is_empty() || trimmed == "}" {
        return false;
    }
    if !implicit_level_row && !matches!(scope, Some(SourceScope::Level | SourceScope::UnbracedLevel))
    {
        return false;
    }
    if matches!(
        tokens,
        ["legend", ..] | ["on_level_start", ..] | ["on_level_clear", ..]
    ) {
        return false;
    }
    !crate::is_level_event_sugar(trimmed, tokens)
}

fn add_level_ascii_line_surface_ranges(
    ranges: &mut Vec<SurfaceAsciiRange>,
    line: &SurfaceScanLine,
    known_chars: &HashSet<char>,
) {
    let raw = strip_line_comment(&line.content);
    let leading = raw.len() - raw.trim_start().len();
    let body = raw.trim();
    let mut column = 0usize;
    for ch in body.chars() {
        let start = line.start + leading + column;
        let end = start + ch.len_utf8();
        if !ch.is_whitespace() {
            ranges.push(SurfaceAsciiRange {
                span: SourceSpan { start, end },
                known: known_chars.contains(&ch),
            });
        }
        column += ch.len_utf8();
    }
}

fn scan_visual_shape_ascii_surface_ranges(scan: &SurfaceScan) -> Vec<SurfaceAsciiRange> {
    let mut ranges = Vec::new();
    let mut in_shapes_block = false;

    for line in &scan.lines {
        let raw = strip_line_comment(&line.content);
        let trimmed = raw.trim();
        let tokens = line.tokens.iter().map(String::as_str).collect::<Vec<_>>();

        if in_shapes_block
            && matches!(
                line.scope,
                Some(SourceScope::VisualShapeTable | SourceScope::VisualShapeEntry)
            )
            && scan.raw_ranges.iter().any(|range| range.start == line.start)
        {
            add_known_ascii_line_surface_ranges(&mut ranges, line);
        }

        if in_shapes_block && line.scope == Some(SourceScope::VisualShapeTable) && trimmed == "}" {
            in_shapes_block = false;
            continue;
        }

        if line.scope == Some(SourceScope::Visuals) && matches!(tokens.as_slice(), ["shapes"]) {
            in_shapes_block = true;
        }
    }

    ranges
}

fn add_known_ascii_line_surface_ranges(ranges: &mut Vec<SurfaceAsciiRange>, line: &SurfaceScanLine) {
    let known_chars = line
        .content
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<HashSet<_>>();
    add_level_ascii_line_surface_ranges(ranges, line, &known_chars);
}

fn scan_visual_named_color_surface_ranges(
    scan: &SurfaceScan,
    aliases: &HashMap<String, String>,
) -> Vec<SurfaceVisualNamedColorRange> {
    let mut ranges = Vec::<SurfaceVisualNamedColorRange>::new();

    if aliases.is_empty() {
        return ranges;
    }

    for line in &scan.lines {
        if !matches!(
            line.visual_scope,
            Some(SurfaceVisualScope::Sprites | SurfaceVisualScope::SpriteEntry)
        ) || is_visual_closing_line(line)
        {
            continue;
        }
        add_visual_named_color_references(
            line.visual_scope,
            &mut ranges,
            &line.token_spans,
            aliases,
        );
    }

    ranges
}

fn scan_visual_color_aliases(scan: &SurfaceScan) -> HashMap<String, String> {
    let mut aliases = HashMap::<String, String>::new();

    for line in &scan.lines {
        if line.visual_scope != Some(SurfaceVisualScope::Colors) || is_visual_closing_line(line) {
            continue;
        }
        let tokens = &line.token_spans;
        let [name, equals, color] = tokens.as_slice() else {
            continue;
        };
        if equals.text != "="
            || name.text.contains(':')
            || !surface_identifier_token(&name.text)
            || !highlightable_visual_color_token(&color.text)
        {
            continue;
        }
        aliases.insert(name.text.clone(), color.text.clone());
    }

    aliases
}

fn surface_visual_opening_scope(
    current: Option<SurfaceVisualScope>,
    line: &SurfaceScanLine,
) -> Option<SurfaceVisualScope> {
    let opens_block = line.content.trim_end().ends_with('{');
    let first = line.tokens.first().map(String::as_str);
    let has_assignment = line.tokens.iter().any(|token| token == "=");
    match (current, first) {
        (None, Some("sprites" | "sprites3")) => Some(SurfaceVisualScope::Sprites),
        (Some(SurfaceVisualScope::Sprites), Some(first)) => match first {
            "palette" => Some(SurfaceVisualScope::Colors),
            "shapes" => Some(SurfaceVisualScope::Other),
            _ if opens_block => Some(SurfaceVisualScope::SpriteEntry),
            _ => None,
        },
        (Some(SurfaceVisualScope::Colors), Some(first))
            if !has_assignment && first.contains(':') && opens_block =>
        {
            Some(SurfaceVisualScope::ColorTable)
        }
        (Some(scope), _) if opens_block => Some(scope),
        _ => None,
    }
}

fn is_visual_closing_line(line: &SurfaceScanLine) -> bool {
    surface_code_trim(&line.content) == "}"
}

fn add_visual_named_color_references(
    scope: Option<SurfaceVisualScope>,
    ranges: &mut Vec<SurfaceVisualNamedColorRange>,
    tokens: &[SourceToken],
    aliases: &HashMap<String, String>,
) {
    let first_color_index = match scope {
        Some(SurfaceVisualScope::Sprites | SurfaceVisualScope::SpriteEntry) => {
            if tokens.is_empty()
                || !tokens
                    .iter()
                    .all(|token| visual_color_value_for_token(&token.text, aliases).is_some())
            {
                return;
            }
            0
        }
        _ => {
            let Some(equals) = tokens.iter().position(|token| token.text == "=") else {
                return;
            };
            equals + 1
        }
    };
    for token in &tokens[first_color_index..] {
        if let Some(color) = aliases.get(&token.text) {
            ranges.push(SurfaceVisualNamedColorRange {
                span: SourceSpan {
                    start: token.start,
                    end: token.end,
                },
                color: color.clone(),
            });
        }
    }
}

fn highlightable_visual_color_token(value: &str) -> bool {
    if value.starts_with('#') {
        return surface_hex_color_end(value, 0, '#') == Some(value.len());
    }
    is_visual_color_token(value)
}

fn scan_visual_ascii_color_surface_ranges(
    scan: &SurfaceScan,
    aliases: &HashMap<String, String>,
) -> Vec<SurfaceVisualAsciiColorRange> {
    let mut ranges = Vec::new();

    let mut line_index = 0usize;
    while line_index < scan.lines.len() {
        let line = &scan.lines[line_index];
        if !visual_sprite_entry_header_line(line, aliases) {
            line_index += 1;
            continue;
        }

        if visual_inline_sprite_entry_line(line, aliases) {
            line_index += 1;
        } else if line.content.trim_end().ends_with('{') {
            line_index = scan_braced_visual_sprite_entry(scan, line_index, aliases, &mut ranges);
        } else {
            line_index = scan_unbraced_visual_sprite_entry(scan, line_index, aliases, &mut ranges);
        }
    }

    ranges
}

fn scan_visual_separator_surface_ranges(
    scan: &SurfaceScan,
    aliases: &HashMap<String, String>,
) -> Vec<SourceSpan> {
    let mut ranges = Vec::new();

    let mut line_index = 0usize;
    while line_index < scan.lines.len() {
        let line = &scan.lines[line_index];
        if !visual_sprite_entry_header_line(line, aliases) {
            line_index += 1;
            continue;
        }

        if visual_inline_sprite_entry_line(line, aliases) {
            line_index += 1;
        } else if line.content.trim_end().ends_with('{') {
            line_index =
                scan_braced_visual_sprite_separators(scan, line_index, aliases, &mut ranges);
        } else {
            line_index =
                scan_unbraced_visual_sprite_separators(scan, line_index, aliases, &mut ranges);
        }
    }

    ranges
}

#[derive(Default)]
struct VisualSpritePixelScan {
    palette: HashMap<char, String>,
}

impl VisualSpritePixelScan {
    fn has_palette(&self) -> bool {
        !self.palette.is_empty()
    }
}

fn scan_braced_visual_sprite_entry(
    scan: &SurfaceScan,
    start: usize,
    aliases: &HashMap<String, String>,
    ranges: &mut Vec<SurfaceVisualAsciiColorRange>,
) -> usize {
    let mut scan_state = VisualSpritePixelScan::default();
    let mut index = start + 1;
    while index < scan.lines.len() {
        let line = &scan.lines[index];
        if line.scope == Some(SourceScope::Visuals) {
            break;
        }
        scan_visual_sprite_body_line(&mut scan_state, ranges, line, aliases);
        index += 1;
    }
    index.max(start + 1)
}

fn scan_braced_visual_sprite_separators(
    scan: &SurfaceScan,
    start: usize,
    aliases: &HashMap<String, String>,
    ranges: &mut Vec<SourceSpan>,
) -> usize {
    let mut index = start + 1;
    while index < scan.lines.len() {
        let line = &scan.lines[index];
        if line.scope == Some(SourceScope::Visuals) {
            break;
        }
        scan_visual_sprite_separator_line(ranges, line, aliases);
        index += 1;
    }
    index.max(start + 1)
}

fn scan_unbraced_visual_sprite_entry(
    scan: &SurfaceScan,
    start: usize,
    aliases: &HashMap<String, String>,
    ranges: &mut Vec<SurfaceVisualAsciiColorRange>,
) -> usize {
    let mut scan_state = VisualSpritePixelScan::default();
    let mut index = start + 1;
    while index < scan.lines.len() {
        let line = &scan.lines[index];
        if line.scope != Some(SourceScope::VisualShapeEntry) || is_visual_closing_line(line) {
            break;
        }
        scan_visual_sprite_body_line(&mut scan_state, ranges, line, aliases);
        index += 1;
    }
    index.max(start + 1)
}

fn scan_unbraced_visual_sprite_separators(
    scan: &SurfaceScan,
    start: usize,
    aliases: &HashMap<String, String>,
    ranges: &mut Vec<SourceSpan>,
) -> usize {
    let mut index = start + 1;
    while index < scan.lines.len() {
        let line = &scan.lines[index];
        if line.scope != Some(SourceScope::VisualShapeEntry) || is_visual_closing_line(line) {
            break;
        }
        scan_visual_sprite_separator_line(ranges, line, aliases);
        index += 1;
    }
    index.max(start + 1)
}

fn scan_visual_sprite_body_line(
    scan_state: &mut VisualSpritePixelScan,
    ranges: &mut Vec<SurfaceVisualAsciiColorRange>,
    line: &SurfaceScanLine,
    aliases: &HashMap<String, String>,
) {
    let raw = strip_line_comment(&line.content);
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "}" {
        return;
    }
    let tokens = line.tokens.iter().map(String::as_str).collect::<Vec<_>>();
    if let Some(palette) = visual_ascii_palette_for_line(&tokens, aliases) {
        scan_state.palette = palette;
        return;
    }
    if scan_state.has_palette() && visual_ascii_row(trimmed, &scan_state.palette) {
        add_visual_ascii_row_surface_ranges(
            ranges,
            line.start,
            raw,
            trimmed,
            &scan_state.palette,
        );
    }
}

fn scan_visual_sprite_separator_line(
    ranges: &mut Vec<SourceSpan>,
    line: &SurfaceScanLine,
    aliases: &HashMap<String, String>,
) {
    let raw = strip_line_comment(&line.content);
    let trimmed = raw.trim();
    if trimmed != ">" {
        return;
    }
    let tokens = line.tokens.iter().map(String::as_str).collect::<Vec<_>>();
    if visual_ascii_palette_for_line(&tokens, aliases).is_some() {
        return;
    }
    let leading = raw.len() - raw.trim_start().len();
    ranges.push(SourceSpan {
        start: line.start + leading,
        end: line.start + leading + ">".len(),
    });
}

fn visual_sprite_entry_header_line(
    line: &SurfaceScanLine,
    aliases: &HashMap<String, String>,
) -> bool {
    if line.scope != Some(SourceScope::Visuals) || is_visual_closing_line(line) {
        return false;
    }
    let tokens = line.tokens.iter().map(String::as_str).collect::<Vec<_>>();
    match tokens.as_slice() {
        [selector] => visual_sprite_selector_token(selector),
        [selector, "rotate", "from", _]
        | [selector, "rotate", "using", _, "from", _]
        | [selector, "rotate", _, "from", _] => visual_sprite_selector_token(selector),
        [selector, source] => {
            visual_sprite_selector_token(selector)
                && (surface_visual_image_source(source)
                    || visual_sprite_entry_start_color_token(source, aliases))
        }
        _ => false,
    }
}

fn visual_inline_sprite_entry_line(
    line: &SurfaceScanLine,
    aliases: &HashMap<String, String>,
) -> bool {
    let tokens = line.tokens.iter().map(String::as_str).collect::<Vec<_>>();
    matches!(
        tokens.as_slice(),
        [selector, source]
            if visual_sprite_selector_token(selector)
                && (surface_visual_image_source(source)
                    || visual_sprite_entry_start_color_token(source, aliases))
    )
}

fn visual_sprite_entry_start_color_token(token: &str, aliases: &HashMap<String, String>) -> bool {
    highlightable_visual_color_token(token) || aliases.contains_key(token) || token.contains(':')
}

fn visual_sprite_selector_token(value: &str) -> bool {
    if matches!(
        value,
        "shape" | "shapes" | "palette" | "colors" | "ascii" | "sprites" | "sprites3"
    ) {
        return false;
    }
    let cleaned = value.trim_start_matches('@');
    let mut parts = cleaned.split(':');
    let Some(first) = parts.next() else {
        return false;
    };
    surface_identifier_token(first) && parts.all(visual_sprite_selector_part_token)
}

fn visual_sprite_selector_part_token(value: &str) -> bool {
    value == "*" || surface_highlight_value_atom(value) || visual_sprite_selector_map_call(value)
}

fn visual_sprite_selector_map_call(value: &str) -> bool {
    let Some((name, rest)) = value.split_once('(') else {
        return false;
    };
    let Some(arg) = rest.strip_suffix(')') else {
        return false;
    };
    surface_identifier_token(name) && surface_identifier_token(arg)
}

fn surface_visual_image_source(value: &str) -> bool {
    let lower = value
        .trim_matches(|ch| matches!(ch, '"' | '\''))
        .to_ascii_lowercase();
    lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".gif")
        || lower.ends_with(".webp")
        || lower.ends_with(".svg")
        || lower.ends_with(".avif")
}

fn visual_ascii_palette(
    tokens: &[&str],
    aliases: &HashMap<String, String>,
) -> Option<HashMap<char, String>> {
    if tokens.is_empty()
        || !tokens
            .iter()
            .all(|token| visual_color_value_for_token(token, aliases).is_some())
    {
        return None;
    }
    let mut palette = HashMap::new();
    for (index, color) in tokens.iter().enumerate() {
        let token = visual_color_token_for_index(index)?;
        if let Some(color) = visual_color_value_for_token(color, aliases) {
            palette.insert(token, color);
        }
    }
    (!palette.is_empty()).then_some(palette)
}

fn visual_ascii_palette_for_line(
    tokens: &[&str],
    aliases: &HashMap<String, String>,
) -> Option<HashMap<char, String>> {
    match tokens {
        ["colors", "=", colors @ ..] if !colors.is_empty() => visual_ascii_palette(colors, aliases),
        ["colors", colors @ ..] if !colors.is_empty() => visual_ascii_palette(colors, aliases),
        colors => visual_ascii_palette(colors, aliases),
    }
}

fn visual_color_value_for_token(token: &str, aliases: &HashMap<String, String>) -> Option<String> {
    if let Some(color) = aliases.get(token) {
        return Some(color.clone());
    }
    highlightable_visual_color_token(token).then(|| token.to_string())
}

fn visual_ascii_row(row: &str, palette: &HashMap<char, String>) -> bool {
    !row.is_empty()
        && !row.contains(char::is_whitespace)
        && row.chars().all(|ch| ch == '.' || palette.contains_key(&ch))
}

fn add_visual_ascii_row_surface_ranges(
    ranges: &mut Vec<SurfaceVisualAsciiColorRange>,
    line_start: usize,
    content: &str,
    trimmed: &str,
    palette: &HashMap<char, String>,
) {
    let leading = content.len() - content.trim_start().len();
    let mut column = 0usize;
    for ch in trimmed.chars() {
        let start = line_start + leading + column;
        let end = start + ch.len_utf8();
        if ch == '.' {
            ranges.push(SurfaceVisualAsciiColorRange {
                span: SourceSpan { start, end },
                color: "transparent".to_string(),
                transparent: true,
            });
        } else if let Some(color) = palette.get(&ch).cloned() {
            ranges.push(SurfaceVisualAsciiColorRange {
                span: SourceSpan { start, end },
                color,
                transparent: false,
            });
        }
        column += ch.len_utf8();
    }
}

fn surface_hex_color_end(source: &str, index: usize, ch: char) -> Option<usize> {
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

fn surface_highlight_value_atom(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn surface_visual_sprite_refs(source: &str, scan: &SurfaceScan) -> SurfaceVisualSpriteRefs {
    let mut refs = SurfaceVisualSpriteRefs::default();
    for line in &scan.lines {
        let Some(kind) = line.tokens.first().map(String::as_str) else {
            continue;
        };
        if !matches!(
            (kind, line.scope),
            (
                "palette",
                Some(SourceScope::Visuals | SourceScope::VisualColorTable)
            ) | (
                "shapes",
                Some(SourceScope::Visuals | SourceScope::VisualShapeTable)
            )
        ) {
            continue;
        }
        let line_end = surface_scan_line_end(line);
        let Some(open_index) = source[line.start..line_end]
            .find('{')
            .map(|offset| line.start + offset)
        else {
            continue;
        };
        let Some(close_index) = surface_find_matching_brace(source, open_index) else {
            continue;
        };
        match kind {
            "palette" => {
                collect_surface_visual_flat_asset_names(scan, open_index, close_index, &mut refs)
            }
            "shapes" => {
                collect_surface_visual_shape_names(scan, open_index, close_index, &mut refs)
            }
            _ => {}
        }
    }
    refs
}

fn collect_surface_visual_flat_asset_names(
    scan: &SurfaceScan,
    open_index: usize,
    close_index: usize,
    refs: &mut SurfaceVisualSpriteRefs,
) {
    for line in &scan.lines {
        if line.start <= open_index || line.start >= close_index {
            continue;
        }
        if surface_visual_asset_depth_at_line(scan, open_index, line.start) != 0 {
            continue;
        }
        match line.tokens.as_slice() {
            [name, equals, ..] if equals == "=" && surface_identifier_token(name) => {
                refs.color_names.insert(name.clone());
                if let Some(color) = surface_visual_color_assignment_value(&line.content) {
                    refs.color_assets.insert(name.clone(), color);
                }
            }
            [name]
                if line.scope == Some(SourceScope::VisualColorTable)
                    && surface_identifier_token(name) =>
            {
                refs.color_names.insert(name.clone());
                if let Some(color) = surface_visual_color_assignment_value(&line.content) {
                    refs.color_assets.insert(name.clone(), color);
                }
            }
            _ => {}
        }
    }
}

fn collect_surface_visual_shape_names(
    scan: &SurfaceScan,
    open_index: usize,
    close_index: usize,
    refs: &mut SurfaceVisualSpriteRefs,
) {
    for line in &scan.lines {
        if line.start <= open_index || line.start >= close_index {
            continue;
        }
        if surface_visual_asset_depth_at_line(scan, open_index, line.start) != 0 {
            continue;
        }
        let Some(first) = line.tokens.first() else {
            continue;
        };
        if first == "shape" {
            if let Some(name) = line.tokens.get(1) {
                refs.shape_names.insert(name.clone());
                if let Some(rows) = surface_visual_plain_shape_rows(scan, line, close_index) {
                    refs.shape_assets.insert(name.clone(), rows);
                }
            }
        } else if surface_visual_shape_ref_token(first) {
            refs.shape_names.insert(first.clone());
            if let Some(rows) = surface_visual_plain_shape_rows(scan, line, close_index) {
                refs.shape_assets.insert(first.clone(), rows);
            }
        }
    }
}

fn surface_visual_color_assignment_value(line: &str) -> Option<String> {
    let (_, value) = strip_line_comment(line).split_once('=')?;
    let value = value.trim().split_whitespace().next()?.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn surface_visual_plain_shape_rows(
    scan: &SurfaceScan,
    header: &SurfaceScanLine,
    close_index: usize,
) -> Option<Vec<String>> {
    let mut rows = Vec::new();
    for line in scan
        .lines
        .iter()
        .filter(|line| line.start > header.start && line.start < close_index)
    {
        let row = surface_code_trim(&line.content);
        if row.is_empty() {
            if rows.is_empty() {
                continue;
            }
            break;
        }
        if row == "}" {
            break;
        }
        if !rows.is_empty() && surface_visual_shape_ref_token(row) {
            break;
        }
        rows.push(row.to_string());
    }
    (!rows.is_empty()).then_some(rows)
}

fn surface_visual_asset_depth_at_line(
    scan: &SurfaceScan,
    open_index: usize,
    line_start: usize,
) -> usize {
    let mut depth = 0usize;
    for line in &scan.lines {
        if line.start <= open_index || line.start >= line_start {
            continue;
        }
        let trimmed = surface_code_trim(&line.content);
        if trimmed == "}" {
            depth = depth.saturating_sub(1);
        }
        if trimmed.ends_with('{') {
            depth += 1;
        }
    }
    depth
}

fn surface_visual_shape_ref_token(value: &str) -> bool {
    if surface_visual_shape_name_token(value) {
        return true;
    }
    let Some((table, value)) = value.split_once(':') else {
        return false;
    };
    surface_identifier_token(table)
        && !value.is_empty()
        && value.chars().all(|ch| {
            ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '+' | '*' | '(' | ')')
        })
}

fn surface_visual_shape_name_token(value: &str) -> bool {
    if matches!(
        value,
        "shape" | "shapes" | "palette" | "colors" | "ascii" | "sprites" | "sprites3"
    ) {
        return false;
    }
    let Some(first) = value.chars().next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && value.chars().all(|ch| {
            ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '+' | '*' | '(' | ')')
        })
}

fn surface_identifier_token(value: &str) -> bool {
    let Some(first) = value.chars().next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn surface_scan_line_end(line: &SurfaceScanLine) -> usize {
    line.start + line.content.len()
}

fn surface_code_trim(line: &str) -> &str {
    strip_line_comment(line).trim()
}

fn surface_find_matching_brace(source: &str, open_index: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut quote = None::<char>;
    let mut escaped = false;
    let mut in_comment = false;
    let mut iter = source[open_index..].char_indices().peekable();
    while let Some((relative, ch)) = iter.next() {
        let index = open_index + relative;
        if in_comment {
            if ch == '\n' {
                in_comment = false;
            }
            continue;
        }
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == active_quote {
                quote = None;
            }
            continue;
        }
        if ch == '/' && iter.peek().is_some_and(|(_, next)| *next == '/') {
            in_comment = true;
            continue;
        }
        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            continue;
        }
        match ch {
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

pub(crate) struct SurfaceDocumentSemantics {
    pub(crate) tokens: Vec<semantic::SemanticToken>,
}

pub(crate) fn surface_document_semantics(source: &str) -> SurfaceDocumentSemantics {
    let document = parse_surface_document(source);
    SurfaceDocumentSemantics {
        tokens: surface_document_semantic_tokens(&document),
    }
}

pub(crate) fn surface_document_semantic_tokens(
    document: &SurfaceDocument,
) -> Vec<semantic::SemanticToken> {
    project_surface_semantic_tokens(&document.semantic_tokens)
}

fn record_surface_document_line(
    option_block: Option<SurfaceOptionBlock>,
    scope: Option<SourceScope>,
    line_start: usize,
    line: &str,
    tokens: &[SourceToken],
    structural_lines: &[String],
    _structural_events: &[source::SourceStructureEvent],
    sink: &mut SurfaceSink,
) {
    let in_authoring_option_block = option_block
        .and_then(|block| block.authoring_parent_kind())
        .is_some();
    if !in_authoring_option_block {
        record_authoring_declaration_surface_tokens(scope, tokens, sink);
        record_general_surface_tokens(scope, tokens, sink);
    }
    record_legend_empty_literal_surface_tokens(scope, tokens, sink);
    record_visual_surface_line(scope, tokens, sink);
    if record_inline_authoring_surface_line(option_block, line_start, line, structural_lines, sink) {
        return;
    }
    if record_document_prelude_surface_line(scope, tokens, sink) {
        return;
    }
    if record_option_surface_line(option_block, scope, tokens, sink) {
        return;
    }
    if record_sounds_operation_surface_line(option_block, tokens, sink) {
        return;
    }
    if tokens
        .first()
        .is_some_and(|token| token.text.as_str() == "scene")
    {
        record_scene_surface_line(scope, line_start, line, tokens, sink);
        return;
    }
    if is_scene_surface_scope(scope) {
        record_scene_surface_line(scope, line_start, line, tokens, sink);
        return;
    }
    record_fix_default_surface_tokens(scope, tokens, line, sink);
    record_standard_move_surface_tokens(scope, tokens, sink);
    record_rule_routine_header_surface_line(scope, line_start, line, sink);
    record_rule_statement_surface_tokens(scope, tokens, sink);
    record_rule_line_surface_tokens(scope, line_start, line, sink);
    record_oriented_pattern_arg_surface_line(scope, line_start, line, sink);
    record_rewrite_surface_line(scope, line, tokens, sink);
}

fn record_structural_block_surface_tokens(
    blocks: &[SurfaceStructuralBlock],
    strict_projection: bool,
    sink: &mut SurfaceSink,
) -> Result<(), DiagnosticReport> {
    for block in blocks {
        let tokens = source_line_tokens(&block.header, block.start);
        if tokens.is_empty() {
            continue;
        }
        if record_authoring_block_header_surface_tokens(block, &tokens, sink)
            || record_rule_routine_block_header_surface_tokens(block, sink)
            || record_visual_shape_entry_block_header_surface_tokens(block, &tokens, sink)
            || record_condition_block_header_surface_tokens(&tokens, sink)
            || record_unbraced_level_block_header_surface_tokens(block, &tokens, sink)
            || record_known_structural_block_header_surface_tokens(block, &tokens, sink)
        {
            continue;
        }
        if strict_projection {
            return Err(DiagnosticReport::error_at_line(
                format!(
                    "unowned structural block header `{}` in scope {:?} role {:?}; add an owner-specific surface projector or a universal structural header rule",
                    block.header, block.scope, block.role
                ),
                block.header.clone(),
            ));
        }
    }
    Ok(())
}

fn record_authoring_block_header_surface_tokens(
    block: &SurfaceStructuralBlock,
    tokens: &[SourceToken],
    sink: &mut SurfaceSink,
) -> bool {
    let Some(kind) = block.authoring_kind else {
        return false;
    };
    mark_authoring_surface_spans(
        authoring_grammar::project_authoring_header_surface(kind, tokens),
        sink,
    )
}

fn record_rule_routine_block_header_surface_tokens(
    block: &SurfaceStructuralBlock,
    sink: &mut SurfaceSink,
) -> bool {
    if !matches!(block.scope, SourceScope::Puzzle | SourceScope::Other) {
        return false;
    }
    let Some(spans) = puzzle_authoring::rule_routine_block_header_surface_spans(&block.header)
    else {
        return false;
    };
    record_rule_routine_header_surface_spans(block.start, spans, sink);
    true
}

fn record_rule_routine_header_surface_line(
    scope: Option<SourceScope>,
    line_start: usize,
    line: &str,
    sink: &mut SurfaceSink,
) {
    if !matches!(scope, Some(SourceScope::Puzzle | SourceScope::Other)) {
        return;
    }
    if !matches!(
        puzzle_authoring::rule_statement_block_surface(line, scope == Some(SourceScope::Other)),
        Some(puzzle_authoring::RuleStatementBlockSurface::Routine)
    ) {
        return;
    }
    let Some(spans) = puzzle_authoring::rule_routine_block_header_surface_spans(line) else {
        return;
    };
    record_rule_routine_header_surface_spans(line_start, spans, sink);
}

fn record_rule_routine_header_surface_spans(
    line_start: usize,
    spans: puzzle_authoring::RuleRoutineBlockHeaderSurfaceSpans,
    sink: &mut SurfaceSink,
) {
    mark_surface_span(line_start, spans.keyword, SurfaceSemanticKind::Keyword, sink);
    if let Some(name) = spans.name {
        mark_surface_span(line_start, name, SurfaceSemanticKind::Effect, sink);
    }
    for modifier in spans.modifiers {
        mark_surface_span(line_start, modifier, SurfaceSemanticKind::Keyword, sink);
    }
}

fn record_visual_shape_entry_block_header_surface_tokens(
    block: &SurfaceStructuralBlock,
    tokens: &[SourceToken],
    sink: &mut SurfaceSink,
) -> bool {
    if block.scope != SourceScope::VisualShapeEntry {
        return false;
    }
    let Some(first) = tokens.first() else {
        return false;
    };
    if !visual_sprite_selector_token(&first.text) {
        return false;
    }
    record_visual_table_ref_surface_token(first, sink);
    record_visual_shape_entry_header_surface_keywords(&tokens[1..], sink);
    true
}

fn record_visual_shape_entry_header_surface_keywords(tokens: &[SourceToken], sink: &mut SurfaceSink) {
    for token in tokens {
        if matches!(token.text.as_str(), "rotate" | "from" | "using") {
            add_scene_effect_token_range(sink, token, SurfaceSemanticKind::Keyword);
        }
    }
}

fn record_condition_block_header_surface_tokens(
    tokens: &[SourceToken],
    sink: &mut SurfaceSink,
) -> bool {
    let Some(first) = tokens.first() else {
        return false;
    };
    if !matches!(first.text.as_str(), "win_conditions" | "lose_conditions") {
        return false;
    }
    add_scene_effect_token_range(sink, first, SurfaceSemanticKind::Condition);
    for token in &tokens[1..] {
        add_scene_effect_token_range(sink, token, SurfaceSemanticKind::Keyword);
    }
    true
}

fn record_unbraced_level_block_header_surface_tokens(
    block: &SurfaceStructuralBlock,
    tokens: &[SourceToken],
    sink: &mut SurfaceSink,
) -> bool {
    if block.scope != SourceScope::UnbracedLevel {
        return false;
    }
    match tokens {
        [keyword, rest @ ..] if keyword.text == "level" => {
            add_scene_effect_token_range(sink, keyword, SurfaceSemanticKind::Keyword);
            for token in rest {
                add_scene_effect_token_range(sink, token, SurfaceSemanticKind::Binding);
            }
        }
        _ => {
            for token in tokens {
                add_scene_effect_token_range(sink, token, SurfaceSemanticKind::Literal);
            }
        }
    }
    true
}

fn record_known_structural_block_header_surface_tokens(
    block: &SurfaceStructuralBlock,
    tokens: &[SourceToken],
    sink: &mut SurfaceSink,
) -> bool {
    if !(structural_container_header_scope(block.scope)
        || source_tree_rule_program_header(&block.header)
        || source_tree_lifecycle_header(&block.header))
    {
        return false;
    }
    record_universal_structural_block_header_surface_tokens(tokens, sink)
}

fn record_universal_structural_block_header_surface_tokens(
    tokens: &[SourceToken],
    sink: &mut SurfaceSink,
) -> bool {
    if tokens.is_empty() {
        return false;
    }
    for (index, token) in tokens.iter().enumerate() {
        let kind = if index == 0 {
            SurfaceSemanticKind::Keyword
        } else {
            SurfaceSemanticKind::Binding
        };
        add_scene_effect_token_range(sink, token, kind);
    }
    true
}

fn structural_container_header_scope(scope: SourceScope) -> bool {
    !matches!(
        scope,
        SourceScope::Other | SourceScope::UnbracedLevel | SourceScope::VisualShapeEntry
    )
}

fn source_tree_lifecycle_header(header: &str) -> bool {
    split_header_tokens(header)
        .first()
        .is_some_and(|first| puzzle_lifecycle_event(first).is_some())
}

fn source_tree_rule_program_header(header: &str) -> bool {
    let mut block_header = String::with_capacity(header.len() + 2);
    block_header.push_str(header);
    block_header.push_str(" {");
    puzzle_authoring::rule_program_block_surface(&block_header).is_some()
}

fn record_general_surface_tokens(
    scope: Option<SourceScope>,
    tokens: &[SourceToken],
    sink: &mut SurfaceSink,
) {
    let token_texts = tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    if let Some(index) = metadata_directive_value_token_index(&token_texts)
        && let Some(value) = tokens.get(index)
    {
        add_scene_effect_token_range(sink, value, SurfaceSemanticKind::String);
    }
    for token in tokens {
        record_level_path_surface_token(token, sink);
        record_condition_reference_surface_token(token, sink);
    }
    for index in 0..tokens.len() {
        match map_header_token_syntax(&token_texts, index) {
            Some(MapHeaderTokenSyntax::Keyword) => {
                add_scene_effect_token_range(sink, &tokens[index], SurfaceSemanticKind::Keyword);
            }
            Some(MapHeaderTokenSyntax::Name) => {
                add_scene_effect_token_range(sink, &tokens[index], SurfaceSemanticKind::Effect);
            }
            Some(MapHeaderTokenSyntax::Axis) => {
                add_scene_effect_token_range(sink, &tokens[index], SurfaceSemanticKind::Group);
            }
            None => {}
        }
    }
    record_state_declaration_surface_tokens(tokens, sink);
    if is_scene_surface_scope(scope)
        && let Some((index, syntax)) = scene_state_lhs_syntax(&token_texts)
        && let Some(name) = tokens.get(index)
    {
        let kind = match syntax {
            SceneStateLhsSyntax::PuzzleSlot | SceneStateLhsSyntax::Variable => {
                SurfaceSemanticKind::State
            }
        };
        add_scene_effect_token_range(sink, name, kind);
    }
}

fn record_state_declaration_surface_tokens(tokens: &[SourceToken], sink: &mut SurfaceSink) {
    let name = match tokens {
        [keyword, name, ..] if matches!(keyword.text.as_str(), "var" | "const") => Some(name),
        [persistent, keyword, name, ..]
            if persistent.text == "persistent"
                && matches!(keyword.text.as_str(), "var" | "const") =>
        {
            Some(name)
        }
        [persistent, name, ..] if persistent.text == "persistent" => Some(name),
        _ => None,
    };
    if let Some(name) = name {
        add_scene_effect_token_range(sink, name, SurfaceSemanticKind::State);
    }
}

fn record_condition_reference_surface_token(token: &SourceToken, sink: &mut SurfaceSink) {
    let mut offset = 0usize;
    for part in token.text.split('.') {
        if matches!(part, "win_conditions" | "lose_conditions") {
            add_scene_effect_token_subrange(
                sink,
                token,
                offset,
                offset + part.len(),
                SurfaceSemanticKind::Condition,
            );
        }
        offset += part.len() + 1;
    }
}

fn record_level_path_surface_token(token: &SourceToken, sink: &mut SurfaceSink) {
    if !token.text.contains('.') {
        return;
    }
    let parts = token.text.split('.').collect::<Vec<_>>();
    let mut offset = 0usize;
    for (index, part) in parts.iter().enumerate() {
        if let Some(syntax) = level_path_part_syntax(&parts, index) {
            let kind = match syntax {
                LevelPathPartSyntax::Owner => SurfaceSemanticKind::State,
                LevelPathPartSyntax::TextProperty => SurfaceSemanticKind::String,
                LevelPathPartSyntax::NumberProperty => SurfaceSemanticKind::Number,
                LevelPathPartSyntax::ConditionProperty => SurfaceSemanticKind::Condition,
            };
            add_scene_effect_token_subrange(sink, token, offset, offset + part.len(), kind);
        }
        offset += part.len() + 1;
    }
}

fn record_legend_empty_literal_surface_tokens(
    scope: Option<SourceScope>,
    tokens: &[SourceToken],
    sink: &mut SurfaceSink,
) {
    let token_texts = tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    let syntax = match scope {
        Some(SourceScope::Legend) => crate::syntax::legend_block_row_syntax(&token_texts, true),
        Some(SourceScope::Level | SourceScope::UnbracedLevel) => {
            crate::syntax::legend_directive_syntax(&token_texts, true)
        }
        _ => None,
    };
    let Some(syntax) = syntax else {
        return;
    };
    if token_texts[syntax.rhs_start..] != ["empty"] {
        return;
    }
    if let Some(empty) = tokens.get(syntax.rhs_start) {
        add_scene_effect_token_range(sink, empty, SurfaceSemanticKind::Literal);
    }
}

fn record_sounds_operation_surface_line(
    option_block: Option<SurfaceOptionBlock>,
    tokens: &[SourceToken],
    sink: &mut SurfaceSink,
) -> bool {
    if option_block != Some(SurfaceOptionBlock::Authoring(
        authoring_grammar::AuthoringKind::SoundsConfig,
    )) {
        return false;
    }
    match tokens {
        [operation, arrow, sfx, name]
            if matches!(operation.text.as_str(), "undo" | "restart")
                && arrow.text == "->"
                && sfx.text == "sfx" =>
        {
            add_scene_effect_token_range(sink, operation, SurfaceSemanticKind::Keyword);
            add_scene_effect_token_range(sink, sfx, SurfaceSemanticKind::Keyword);
            add_scene_effect_token_range(sink, name, SurfaceSemanticKind::Asset);
            true
        }
        _ => false,
    }
}

fn record_fix_default_surface_tokens(
    scope: Option<SourceScope>,
    structural_tokens: &[SourceToken],
    line: &str,
    sink: &mut SurfaceSink,
) {
    if scope != Some(SourceScope::Other) {
        return;
    }
    if !structural_tokens
        .first()
        .is_some_and(|token| token.text == "fix")
    {
        return;
    }
    let token_texts = structural_tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    if parse_fix_defaults(&token_texts, line, &[]).is_err() {
        return;
    }
    for token in structural_tokens {
        add_scene_effect_token_range(sink, token, SurfaceSemanticKind::Keyword);
    }
}

fn record_rule_statement_surface_tokens(
    scope: Option<SourceScope>,
    tokens: &[SourceToken],
    sink: &mut SurfaceSink,
) {
    if scope != Some(SourceScope::Other) {
        return;
    }
    let line = tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let Ok(surface) = puzzle_authoring::rule_statement_surface(&line) else {
        return;
    };
    match surface {
        puzzle_authoring::RuleStatementSurface::ApplicationBlock { .. } => {
            if let Some(first) = tokens.first() {
                sink.mark(
                    SourceSpan {
                        start: first.start,
                        end: first.end,
                    },
                    SurfaceSemanticKind::Keyword,
                );
            }
        }
        puzzle_authoring::RuleStatementSurface::Call { name } => {
            if let [call] = tokens
                && name == call.text
            {
                record_rule_call_surface_token(call, sink);
            }
        }
        puzzle_authoring::RuleStatementSurface::RuleLine(_) => {}
    }
}

fn record_rule_call_surface_token(token: &SourceToken, sink: &mut SurfaceSink) -> bool {
    if !matches!(
        puzzle_authoring::rule_statement_surface(&token.text),
        Ok(puzzle_authoring::RuleStatementSurface::Call { name }) if name == token.text
    ) {
        return false;
    }
    add_scene_effect_token_range(sink, token, SurfaceSemanticKind::Effect);
    true
}

fn record_rule_line_surface_tokens(
    scope: Option<SourceScope>,
    line_start: usize,
    line: &str,
    sink: &mut SurfaceSink,
) {
    if scope != Some(SourceScope::Other) {
        return;
    }
    if let Ok(spans) = puzzle_authoring::rule_line_semantic_surface_spans(line) {
        for span in spans {
            mark_surface_span(
                line_start,
                span.span,
                rule_semantic_surface_kind(span.kind),
                sink,
            );
        }
    }
    let Ok(surface) = puzzle_authoring::rule_line_surface_spans(line) else {
        return;
    };
    match surface {
        puzzle_authoring::RuleLineSurfaceSpans::StandardStep { .. } => {}
        puzzle_authoring::RuleLineSurfaceSpans::InputRewrite {
            application,
            surface,
        } => {
            mark_rule_application_surface_span(line_start, application, sink);
            mark_surface_span(
                line_start,
                surface.input,
                SurfaceSemanticKind::Keyword,
                sink,
            );
            if let Some(orientation) = surface.orientation {
                mark_surface_span(line_start, orientation, SurfaceSemanticKind::Keyword, sink);
            }
        }
        puzzle_authoring::RuleLineSurfaceSpans::NeutralRewrite {
            application,
            rewrite: _,
        } => {
            mark_rule_application_surface_span(line_start, application, sink);
        }
        puzzle_authoring::RuleLineSurfaceSpans::OrientedRewrite {
            application,
            orientation,
            rewrite: _,
        } => {
            mark_rule_application_surface_span(line_start, application, sink);
            mark_surface_span(line_start, orientation, SurfaceSemanticKind::Keyword, sink);
        }
    }
}

fn rule_semantic_surface_kind(
    kind: puzzle_authoring::RuleSemanticSurfaceKind,
) -> SurfaceSemanticKind {
    match kind {
        puzzle_authoring::RuleSemanticSurfaceKind::Direction
        | puzzle_authoring::RuleSemanticSurfaceKind::Keyword => SurfaceSemanticKind::Keyword,
        puzzle_authoring::RuleSemanticSurfaceKind::Object => SurfaceSemanticKind::Object,
        puzzle_authoring::RuleSemanticSurfaceKind::Mark => SurfaceSemanticKind::Mark,
    }
}

fn mark_rule_application_surface_span(
    line_start: usize,
    application: Option<puzzle_authoring::RuleApplicationSurfaceSpan>,
    sink: &mut SurfaceSink,
) {
    if let Some(application) = application {
        mark_surface_span(
            line_start,
            application.span,
            SurfaceSemanticKind::Keyword,
            sink,
        );
    }
}

fn record_oriented_pattern_arg_surface_line(
    scope: Option<SourceScope>,
    line_start: usize,
    line: &str,
    sink: &mut SurfaceSink,
) {
    if scope != Some(SourceScope::Other) {
        return;
    }
    for arg in parenthesized_arg_spans(line) {
        let Ok(Some(surface)) = oriented_pattern_arg_surface(&line[arg.clone()], line) else {
            continue;
        };
        match surface.orientation {
            OrientedPatternArgOrientationSurface::Neutral => {}
            OrientedPatternArgOrientationSurface::Input { input, axis } => {
                mark_surface_span(
                    line_start + arg.start,
                    input,
                    SurfaceSemanticKind::Keyword,
                    sink,
                );
                if let Some(axis) = axis {
                    mark_surface_span(
                        line_start + arg.start,
                        axis,
                        SurfaceSemanticKind::Keyword,
                        sink,
                    );
                }
            }
            OrientedPatternArgOrientationSurface::Orientation { orientation } => {
                mark_surface_span(
                    line_start + arg.start,
                    orientation,
                    SurfaceSemanticKind::Keyword,
                    sink,
                );
            }
        }
    }
}

fn parenthesized_arg_spans(line: &str) -> Vec<std::ops::Range<usize>> {
    let mut spans = Vec::new();
    let mut stack = Vec::new();
    for (index, ch) in line.char_indices() {
        match ch {
            '(' => stack.push(index),
            ')' => {
                if let Some(open) = stack.pop() {
                    if let Some(span) = trimmed_range(line, open + 1, index) {
                        spans.push(span);
                    }
                }
            }
            _ => {}
        }
    }
    spans
}

fn mark_surface_span(
    line_start: usize,
    range: std::ops::Range<usize>,
    kind: SurfaceSemanticKind,
    sink: &mut SurfaceSink,
) {
    sink.mark(
        SourceSpan {
            start: line_start + range.start,
            end: line_start + range.end,
        },
        kind,
    );
}

fn record_document_prelude_surface_line(
    scope: Option<SourceScope>,
    tokens: &[SourceToken],
    sink: &mut SurfaceSink,
) -> bool {
    if scope.is_some() {
        return false;
    }
    let Some(first) = tokens.first() else {
        return false;
    };
    if record_authoring_root_surface_line(tokens, sink) {
        return true;
    }
    if let Some(action) = model_top_level_surface_directive(&first.text) {
        add_scene_effect_token_range(sink, first, SurfaceSemanticKind::Keyword);
        match action {
            ModelTopLevelDirective::Puzzle | ModelTopLevelDirective::Scene => {
                if let Some(name) = tokens.get(1) {
                    add_scene_effect_token_range(sink, name, SurfaceSemanticKind::Scene);
                }
            }
            _ => {}
        }
        return true;
    }
    if classify_known_mixed_document_section(
        &tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>(),
    )
    .is_some()
    {
        add_scene_effect_token_range(sink, first, SurfaceSemanticKind::Keyword);
        if matches!(first.text.as_str(), "puzzle3")
            && let Some(name) = tokens.get(1)
        {
            add_scene_effect_token_range(sink, name, SurfaceSemanticKind::Scene);
        }
        return true;
    }
    false
}

fn record_authoring_root_surface_line(tokens: &[SourceToken], sink: &mut SurfaceSink) -> bool {
    record_authoring_surface_line(authoring_grammar::AuthoringKind::Root, tokens, sink)
}

fn record_inline_authoring_surface_line(
    option_block: Option<SurfaceOptionBlock>,
    line_start: usize,
    line: &str,
    structural_lines: &[String],
    sink: &mut SurfaceSink,
) -> bool {
    if structural_lines.len() <= 1 {
        return false;
    }
    let root_kind = match option_block.and_then(|block| block.authoring_parent_kind()) {
        Some(kind) => kind,
        None if structural_lines.first().is_some_and(|line| {
            surface_piece_tokens(line)
                .first()
                .is_some_and(|surface| {
                    authoring_grammar::placed_authoring_kind(
                        authoring_grammar::AuthoringKind::Root,
                        surface,
                    )
                    .is_some()
                })
        }) =>
        {
            authoring_grammar::AuthoringKind::Root
        }
        None => return false,
    };
    let mut kind_stack = vec![root_kind];
    let mut cursor = 0usize;
    let mut marked_any = false;

    for structural_line in structural_lines {
        let trimmed = structural_line.trim();
        if trimmed == "}" {
            if kind_stack.len() > 1 {
                kind_stack.pop();
            }
            continue;
        }

        let Some(tokens) = surface_tokens_for_structural_piece(line, line_start, trimmed, &mut cursor)
        else {
            continue;
        };
        let Some(current_kind) = kind_stack.last().copied() else {
            continue;
        };
        marked_any |= record_authoring_surface_line(current_kind, &tokens, sink);

        if trimmed.ends_with('{')
            && let Some(first) = tokens.first()
            && let Some(child_kind) =
                authoring_grammar::placed_authoring_kind(current_kind, first.text.as_str())
        {
            kind_stack.push(child_kind);
        }
    }

    marked_any
}

fn surface_tokens_for_structural_piece(
    line: &str,
    line_start: usize,
    piece: &str,
    cursor: &mut usize,
) -> Option<Vec<SourceToken>> {
    let mut tokens = Vec::new();
    for text in surface_piece_tokens(piece) {
        let search = &line.get(*cursor..)?;
        let relative = search.find(text)?;
        let start = *cursor + relative;
        let end = start + text.len();
        tokens.push(SourceToken {
            text: text.to_string(),
            start: line_start + start,
            end: line_start + end,
        });
        *cursor = end;
    }
    Some(tokens)
}

fn surface_piece_tokens(piece: &str) -> Vec<&str> {
    piece
        .split(|ch: char| ch.is_whitespace() || matches!(ch, '{' | '}' | ',' | ';'))
        .filter(|token| !token.is_empty())
        .collect()
}

fn active_surface_option_block(stack: &[SurfaceOptionBlock]) -> Option<SurfaceOptionBlock> {
    stack.iter().rev().copied().find(|block| {
        matches!(
            block,
            SurfaceOptionBlock::Puzzle3
                | SurfaceOptionBlock::Authoring(_)
                | SurfaceOptionBlock::LevelMenu
        )
    })
}

fn update_surface_option_block_stack(
    line: &SurfaceScanLine,
    stack: &mut Vec<SurfaceOptionBlock>,
) {
    for event in &line.structural_events {
        match event {
            source::SourceStructureEvent::Open { header, .. } => {
                let tokens = split_header_tokens(header)
                    .into_iter()
                    .map(str::to_string)
                    .collect::<Vec<_>>();
                stack.push(surface_option_block_for_opening(&tokens, stack));
            }
            source::SourceStructureEvent::Close => {
                stack.pop();
            }
        }
    }
}

fn surface_option_block_for_opening(
    tokens: &[String],
    stack: &[SurfaceOptionBlock],
) -> SurfaceOptionBlock {
    let Some(first) = tokens.first().map(String::as_str) else {
        return SurfaceOptionBlock::Other;
    };
    match first {
        "puzzle3" => SurfaceOptionBlock::Puzzle3,
        "puzzle" => SurfaceOptionBlock::Puzzle2,
        "render" => surface_authoring_option_block_for_opening(first, stack)
            .unwrap_or(SurfaceOptionBlock::Other),
        "level_menu" => SurfaceOptionBlock::LevelMenu,
        _ => surface_authoring_option_block_for_opening(first, stack)
            .unwrap_or(SurfaceOptionBlock::Other),
    }
}

fn surface_authoring_option_block_for_opening(
    surface: &str,
    stack: &[SurfaceOptionBlock],
) -> Option<SurfaceOptionBlock> {
    let parent = stack
        .iter()
        .rev()
        .find_map(|block| block.authoring_parent_kind())
        .unwrap_or(authoring_grammar::AuthoringKind::Root);
    authoring_grammar::placed_authoring_kind(parent, surface).map(SurfaceOptionBlock::Authoring)
}

fn record_option_surface_line(
    block: Option<SurfaceOptionBlock>,
    scope: Option<SourceScope>,
    tokens: &[SourceToken],
    sink: &mut SurfaceSink,
) -> bool {
    if tokens.is_empty() {
        return false;
    }
    match block {
        Some(SurfaceOptionBlock::LevelMenu) => {
            mark_option_surface_tokens(tokens, LEVEL_MENU_OPTIONS, sink)
        }
        Some(block) if block.authoring_parent_kind().is_some() => {
            let kind = block
                .authoring_parent_kind()
                .expect("checked authoring parent kind");
            record_authoring_surface_line(kind, tokens, sink)
        }
        _ if scope == Some(SourceScope::Puzzle) => {
            record_authoring_surface_line(authoring_grammar::AuthoringKind::Root, tokens, sink)
        }
        _ => false,
    }
}

fn record_authoring_surface_line(
    kind: authoring_grammar::AuthoringKind,
    tokens: &[SourceToken],
    sink: &mut SurfaceSink,
) -> bool {
    record_authoring_child_surface_line(kind, tokens, sink)
        || mark_authoring_surface_spans(
            authoring_grammar::project_authoring_row_surface(kind, tokens),
            sink,
        )
        || mark_authoring_surface_spans(
            authoring_grammar::project_authoring_definition_surface(kind, tokens),
            sink,
        )
        || record_authoring_content_surface_line(kind, tokens, sink)
}

fn record_authoring_content_surface_line(
    kind: authoring_grammar::AuthoringKind,
    tokens: &[SourceToken],
    sink: &mut SurfaceSink,
) -> bool {
    let authoring_grammar::AuthoringBody::Content(content) =
        authoring_grammar::authoring_kind_spec(kind).body
    else {
        return false;
    };
    mark_authoring_surface_spans(
        authoring_grammar::project_authoring_content_surface(content, tokens),
        sink,
    )
}

fn record_authoring_child_surface_line(
    parent: authoring_grammar::AuthoringKind,
    tokens: &[SourceToken],
    sink: &mut SurfaceSink,
) -> bool {
    if tokens
        .iter()
        .any(|token| token.text == "=" || token.text.contains('='))
    {
        return false;
    }
    let Some(first) = tokens.first() else {
        return false;
    };
    let Some(child) = authoring_grammar::placed_authoring_kind(parent, first.text.as_str()) else {
        return false;
    };
    mark_authoring_surface_spans(
        authoring_grammar::project_authoring_header_surface(child, tokens),
        sink,
    )
}

fn mark_authoring_surface_spans(
    spans: Vec<authoring_grammar::AuthoringSurfaceSpan>,
    sink: &mut SurfaceSink,
) -> bool {
    let mut marked_any = false;
    for span in spans {
        sink.mark(
            span.span,
            authoring_grammar::authoring_surface_role_semantic_kind(span.role),
        );
        marked_any = true;
    }
    marked_any
}

fn mark_option_surface_tokens(
    tokens: &[SourceToken],
    option_names: &[&str],
    sink: &mut SurfaceSink,
) -> bool {
    let mut marked_any = false;
    let mut index = 0usize;
    while index < tokens.len() {
        if let Some((name, value)) = tokens[index].text.split_once('=') {
            if option_names.contains(&name)
                && mark_token_part(sink, &tokens[index], name, SurfaceSemanticKind::Setting)
            {
                marked_any = true;
            }
            if !value.is_empty() {
                marked_any |= mark_token_part(
                    sink,
                    &tokens[index],
                    value,
                    option_value_surface_kind(value),
                );
            }
            index += 1;
            continue;
        }
        let token = &tokens[index];
        if option_names.contains(&token.text.as_str()) {
            marked_any |= mark_token_part(sink, token, &token.text, SurfaceSemanticKind::Setting);
            if tokens.get(index + 1).is_some_and(|next| next.text == "=")
                && let Some(value) = tokens.get(index + 2)
            {
                sink.mark(
                    SourceSpan {
                        start: value.start,
                        end: value.end,
                    },
                    option_value_surface_kind(&value.text),
                );
                marked_any = true;
                index += 3;
                continue;
            }
        }
        index += 1;
    }
    marked_any
}

fn option_value_surface_kind(value: &str) -> SurfaceSemanticKind {
    if surface_number_literal(value) {
        SurfaceSemanticKind::Number
    } else if matches!(value, "true" | "false") {
        SurfaceSemanticKind::Literal
    } else if value.starts_with('"') && value.ends_with('"') {
        SurfaceSemanticKind::String
    } else {
        SurfaceSemanticKind::Literal
    }
}

fn surface_number_literal(value: &str) -> bool {
    value.parse::<u16>().is_ok()
        || value
            .strip_suffix("ms")
            .or_else(|| value.strip_suffix('s'))
            .is_some_and(|number| number.parse::<u16>().is_ok())
}

fn mark_token_part(
    sink: &mut SurfaceSink,
    token: &SourceToken,
    part: &str,
    kind: SurfaceSemanticKind,
) -> bool {
    let Some(relative_start) = token.text.find(part) else {
        return false;
    };
    if part.is_empty() {
        return false;
    }
    sink.mark(
        SourceSpan {
            start: token.start + relative_start,
            end: token.start + relative_start + part.len(),
        },
        kind,
    );
    true
}

fn is_scene_surface_scope(scope: Option<SourceScope>) -> bool {
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

fn record_scene_surface_line(
    scope: Option<SourceScope>,
    line_start: usize,
    line: &str,
    tokens: &[SourceToken],
    sink: &mut SurfaceSink,
) {
    let Some(first) = tokens.first() else {
        return;
    };
    if first.text == "scene" {
        add_scene_effect_token_range(sink, first, SurfaceSemanticKind::Keyword);
        if let Some(name) = tokens.get(1) {
            add_scene_effect_token_range(sink, name, SurfaceSemanticKind::Scene);
        }
        return;
    }
    if scope == Some(SourceScope::SceneTransitions) && first.text == "step" {
        add_scene_effect_token_range(sink, first, SurfaceSemanticKind::Keyword);
        if let Some(target) = tokens.get(1) {
            add_scene_effect_token_range(sink, target, SurfaceSemanticKind::State);
        }
        return;
    }
    if scope == Some(SourceScope::SceneState) {
        return;
    }
    if scene_block_keyword(&first.text)
        || puzzle_scene::SceneComponentKind::from_keyword(&first.text).is_some()
    {
        add_scene_effect_token_range(sink, first, SurfaceSemanticKind::Keyword);
    }
    record_scene_layout_attr_surface_tokens(tokens, sink);
    if let Some(arrow) = tokens.iter().position(|token| token.text == "->") {
        if tokens
            .iter()
            .any(|token| token.text.contains('[') || token.text.contains(']'))
        {
            return;
        }
        if matches!(first.text.as_str(), "button" | "choice") {
            if let Some((label, label_start)) =
                scene_line_button_label_source(line_start, line, first)
            {
                sink.extend(scene_expr_surface_document_from_source(label, label_start));
            }
        }
        record_scene_condition_surface_tokens(&tokens[..arrow], sink);
        sink.extend(scene_effect_surface_document(&tokens[arrow + 1..]));
        return;
    }
    match first.text.as_str() {
        "title" | "subtitle" | "text" if tokens.len() > 1 => {
            let expr_start = first.end.saturating_sub(line_start);
            if expr_start <= line.len() {
                let expr = line[expr_start..].trim_start();
                let trim_offset = line[expr_start..]
                    .find(|ch: char| !ch.is_whitespace())
                    .unwrap_or(0);
                sink.extend(scene_expr_surface_document_from_source(
                    expr,
                    first.end + trim_offset,
                ));
            }
            return;
        }
        _ => {}
    }
    if first.text == "button" || first.text == "choice" || scope == Some(SourceScope::LevelMenu) {
        return;
    }
    sink.extend(scene_effect_surface_document(tokens));
}

fn scene_line_button_label_source<'a>(
    line_start: usize,
    line: &'a str,
    first: &SourceToken,
) -> Option<(&'a str, usize)> {
    let label_start = first.end.checked_sub(line_start)?;
    let arrow = line[label_start..]
        .find("->")
        .map(|offset| label_start + offset)?;
    let raw = &line[label_start..arrow];
    let trim_start = raw
        .find(|ch: char| !ch.is_whitespace())
        .unwrap_or(raw.len());
    let trim_end = raw
        .rfind(|ch: char| !ch.is_whitespace())
        .map(|index| index + raw[index..].chars().next().map(char::len_utf8).unwrap_or(0))
        .unwrap_or(trim_start);
    (trim_start < trim_end).then_some((
        &raw[trim_start..trim_end],
        line_start + label_start + trim_start,
    ))
}

fn record_scene_layout_attr_surface_tokens(tokens: &[SourceToken], sink: &mut SurfaceSink) {
    let Some(first) = tokens.first() else {
        return;
    };
    let attr_start = match first.text.as_str() {
        "layout" | "row" | "column" | "box" => 1,
        "puzzle" | "puzzle3" if tokens.get(2).is_some_and(|token| token.text == "=") => {
            if tokens.get(4).is_some_and(|token| token.text == "level") {
                6
            } else {
                4
            }
        }
        "puzzle" | "puzzle3" | "frame" => 2,
        _ => return,
    };
    let attrs = tokens
        .iter()
        .skip(attr_start)
        .take_while(|token| token.text != "{")
        .cloned()
        .collect::<Vec<_>>();
    if attrs.is_empty() {
        return;
    }
    let attr_texts = attrs
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    if puzzle_scene::parse_scene_layout_attrs(&attr_texts).is_err() {
        return;
    }
    for token in &attrs {
        mark_scene_layout_attr_token(token, sink);
    }
}

fn mark_scene_layout_attr_token(token: &SourceToken, sink: &mut SurfaceSink) {
    let (name, value) = token
        .text
        .split_once('=')
        .map_or((token.text.as_str(), None), |(name, value)| {
            (name, Some(value))
        });
    if !name.is_empty()
        && name
            .chars()
            .next()
            .is_some_and(|ch| ch == '_' || ch.is_ascii_alphabetic())
    {
        let name_start = token.text.find(name).unwrap_or(0);
        sink.mark(
            SourceSpan {
                start: token.start + name_start,
                end: token.start + name_start + name.len(),
            },
            SurfaceSemanticKind::Keyword,
        );
    }
    if let Some(value) = value
        && !value.is_empty()
    {
        let value_start = token.text.find(value).unwrap_or(token.text.len());
        let kind = if value.parse::<u16>().is_ok() {
            SurfaceSemanticKind::Number
        } else {
            SurfaceSemanticKind::Literal
        };
        sink.mark(
            SourceSpan {
                start: token.start + value_start,
                end: token.start + value_start + value.len(),
            },
            kind,
        );
    } else if token.text.parse::<u16>().is_ok() {
        sink.mark(
            SourceSpan {
                start: token.start,
                end: token.end,
            },
            SurfaceSemanticKind::Number,
        );
    } else if matches!(
        token.text.as_str(),
        "left" | "right" | "center" | "top" | "bottom" | "true" | "false"
    ) {
        sink.mark(
            SourceSpan {
                start: token.start,
                end: token.end,
            },
            SurfaceSemanticKind::Literal,
        );
    }
}

fn scene_expr_surface_document_from_source(source: &str, base_start: usize) -> SurfaceDocument {
    let mut sink = SurfaceSink::default();
    let trimmed_start = source
        .find(|ch: char| !ch.is_whitespace())
        .unwrap_or(source.len());
    let trimmed_end = source
        .rfind(|ch: char| !ch.is_whitespace())
        .map(|index| {
            index
                + source[index..]
                    .chars()
                    .next()
                    .map(char::len_utf8)
                    .unwrap_or(0)
        })
        .unwrap_or(trimmed_start);
    if trimmed_start >= trimmed_end {
        return sink.into_document();
    }
    let expr = &source[trimmed_start..trimmed_end];
    let expr_start = base_start + trimmed_start;
    if let Ok(parsed) = parse_scene_expr(expr, expr) {
        add_scene_expr_surface_tokens(&parsed, expr, expr_start, &mut sink);
    }
    sink.into_document()
}

fn add_scene_expr_surface_tokens(
    expr: &SceneExpr,
    source: &str,
    absolute_start: usize,
    sink: &mut SurfaceSink,
) {
    match expr {
        SceneExpr::Bool(_) => {
            mark_scene_expr_trimmed(sink, source, absolute_start, SurfaceSemanticKind::Literal);
        }
        SceneExpr::Int(_) => {
            mark_scene_expr_trimmed(sink, source, absolute_start, SurfaceSemanticKind::Number);
        }
        SceneExpr::Text(_) => {
            mark_scene_expr_trimmed(sink, source, absolute_start, SurfaceSemanticKind::String);
        }
        SceneExpr::Path(parts) => {
            add_scene_path_surface_tokens(parts, absolute_start, sink);
        }
        SceneExpr::LevelSelector { collection, .. } => {
            if let Some(collection_start) = source.find(collection) {
                sink.mark(
                    SourceSpan {
                        start: absolute_start + collection_start,
                        end: absolute_start + collection_start + collection.len(),
                    },
                    SurfaceSemanticKind::Binding,
                );
            }
        }
        SceneExpr::Call { name, args: _ } => {
            if let Some(name_start) = source.find(name) {
                sink.mark(
                    SourceSpan {
                        start: absolute_start + name_start,
                        end: absolute_start + name_start + name.len(),
                    },
                    SurfaceSemanticKind::Effect,
                );
            }
            for arg in scene_expr_call_arg_spans(source) {
                if let Ok(parsed) = parse_scene_expr(&source[arg.clone()], &source[arg.clone()]) {
                    add_scene_expr_surface_tokens(
                        &parsed,
                        &source[arg.clone()],
                        absolute_start + arg.start,
                        sink,
                    );
                } else {
                    let arg_source = &source[arg.clone()];
                    if let Some(parts) = parse_view_path(arg_source) {
                        add_scene_path_surface_tokens(&parts, absolute_start + arg.start, sink);
                    }
                }
            }
        }
        SceneExpr::Binary { op, left, right } => {
            let operator = match op {
                SceneBinaryOp::And => "and",
                SceneBinaryOp::Eq => "==",
                SceneBinaryOp::In => "in",
                SceneBinaryOp::NotEq => "!=",
            };
            if let Some(operator_start) = source.find(operator) {
                add_scene_expr_surface_tokens(
                    left,
                    &source[..operator_start],
                    absolute_start,
                    sink,
                );
                sink.mark(
                    SourceSpan {
                        start: absolute_start + operator_start,
                        end: absolute_start + operator_start + operator.len(),
                    },
                    SurfaceSemanticKind::Keyword,
                );
                add_scene_expr_surface_tokens(
                    right,
                    &source[operator_start + operator.len()..],
                    absolute_start + operator_start + operator.len(),
                    sink,
                );
            }
        }
        SceneExpr::If {
            condition,
            then_branch,
            else_branch,
        } => {
            if let Some(if_start) = source.find("if") {
                sink.mark(
                    SourceSpan {
                        start: absolute_start + if_start,
                        end: absolute_start + if_start + 2,
                    },
                    SurfaceSemanticKind::Keyword,
                );
            }
            if let Some(else_start) = source.rfind("else") {
                sink.mark(
                    SourceSpan {
                        start: absolute_start + else_start,
                        end: absolute_start + else_start + 4,
                    },
                    SurfaceSemanticKind::Keyword,
                );
            }
            for (branch, branch_source) in scene_if_expr_surface_parts(source).into_iter().zip([
                condition.as_ref(),
                then_branch.as_ref(),
                else_branch.as_ref(),
            ]) {
                add_scene_expr_surface_tokens(
                    branch_source,
                    &source[branch.clone()],
                    absolute_start + branch.start,
                    sink,
                );
            }
        }
    }
}

fn scene_if_expr_surface_parts(source: &str) -> Vec<std::ops::Range<usize>> {
    let Some(if_end) = source.find("if").map(|index| index + 2) else {
        return Vec::new();
    };
    let Some(then_open) = source[if_end..].find('{').map(|offset| if_end + offset) else {
        return Vec::new();
    };
    let Some(then_close) = source[then_open + 1..]
        .find('}')
        .map(|offset| then_open + 1 + offset)
    else {
        return Vec::new();
    };
    let Some(else_start) = source[then_close + 1..]
        .find("else")
        .map(|offset| then_close + 1 + offset)
    else {
        return Vec::new();
    };
    let Some(else_open) = source[else_start + 4..]
        .find('{')
        .map(|offset| else_start + 4 + offset)
    else {
        return Vec::new();
    };
    let Some(else_close) = source[else_open + 1..]
        .rfind('}')
        .map(|offset| else_open + 1 + offset)
    else {
        return Vec::new();
    };
    [
        if_end..then_open,
        then_open + 1..then_close,
        else_open + 1..else_close,
    ]
    .into_iter()
    .filter_map(|range| trimmed_range(source, range.start, range.end))
    .collect()
}

fn mark_scene_expr_trimmed(
    sink: &mut SurfaceSink,
    source: &str,
    absolute_start: usize,
    kind: SurfaceSemanticKind,
) {
    let Some(start) = source.find(|ch: char| !ch.is_whitespace()) else {
        return;
    };
    let Some(end_start) = source.rfind(|ch: char| !ch.is_whitespace()) else {
        return;
    };
    let end = end_start
        + source[end_start..]
            .chars()
            .next()
            .map(char::len_utf8)
            .unwrap_or(0);
    if start < end {
        sink.mark(
            SourceSpan {
                start: absolute_start + start,
                end: absolute_start + end,
            },
            kind,
        );
    }
}

fn add_scene_path_surface_tokens(parts: &[String], absolute_start: usize, sink: &mut SurfaceSink) {
    let part_refs = parts.iter().map(String::as_str).collect::<Vec<_>>();
    let mut offset = 0usize;
    for (index, part) in part_refs.iter().enumerate() {
        if let Some(syntax) = level_path_part_syntax(&part_refs, index) {
            let kind = match syntax {
                LevelPathPartSyntax::Owner => SurfaceSemanticKind::State,
                LevelPathPartSyntax::TextProperty => SurfaceSemanticKind::String,
                LevelPathPartSyntax::NumberProperty => SurfaceSemanticKind::Number,
                LevelPathPartSyntax::ConditionProperty => SurfaceSemanticKind::Condition,
            };
            sink.mark(
                SourceSpan {
                    start: absolute_start + offset,
                    end: absolute_start + offset + part.len(),
                },
                kind,
            );
        }
        offset += part.len() + 1;
    }
}

fn scene_expr_call_arg_spans(source: &str) -> Vec<std::ops::Range<usize>> {
    let Some(open) = source.find('(') else {
        return Vec::new();
    };
    let Some(close) = source.rfind(')') else {
        return Vec::new();
    };
    if open >= close {
        return Vec::new();
    }
    let mut args = Vec::new();
    let mut start = open + 1;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (relative, ch) in source[open + 1..close].char_indices() {
        let index = open + 1 + relative;
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
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                if let Some(range) = trimmed_range(source, start, index) {
                    args.push(range);
                }
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    if let Some(range) = trimmed_range(source, start, close) {
        args.push(range);
    }
    args
}

fn trimmed_range(source: &str, start: usize, end: usize) -> Option<std::ops::Range<usize>> {
    if start >= end {
        return None;
    }
    let slice = &source[start..end];
    let left = slice.find(|ch: char| !ch.is_whitespace())?;
    let right_start = slice.rfind(|ch: char| !ch.is_whitespace())?;
    let right = right_start
        + slice[right_start..]
            .chars()
            .next()
            .map(char::len_utf8)
            .unwrap_or(0);
    Some(start + left..start + right)
}

fn record_scene_condition_surface_tokens(tokens: &[SourceToken], sink: &mut SurfaceSink) {
    let Some(first) = tokens.first() else {
        return;
    };
    if first.text == "if" {
        add_scene_effect_token_range(sink, first, SurfaceSemanticKind::Keyword);
    }
}

fn scene_block_keyword(value: &str) -> bool {
    matches!(
        value,
        "keys" | "on_scene_start" | "resources" | "rules" | "state" | "layout"
    )
}

fn record_standard_move_surface_tokens(
    scope: Option<SourceScope>,
    tokens: &[SourceToken],
    sink: &mut SurfaceSink,
) {
    if scope != Some(SourceScope::Other) {
        return;
    }
    if let [call] = tokens
        && call.text == "move"
    {
        add_scene_effect_token_range(sink, call, SurfaceSemanticKind::Effect);
    }
}

fn record_rewrite_surface_line(
    scope: Option<SourceScope>,
    line: &str,
    tokens: &[SourceToken],
    sink: &mut SurfaceSink,
) {
    if let Some(arrow) = tokens.iter().position(|token| token.text == "->") {
        let rhs = &tokens[arrow + 1..];
        let effect_start = rhs
            .iter()
            .rposition(|token| token.text.contains(']'))
            .map_or(0, |index| index + 1);
        if effect_start < rhs.len() {
            let effect_tokens = &rhs[effect_start..];
            sink.extend(rewrite_effect_surface_document(effect_tokens));
            if let [call] = effect_tokens {
                record_rule_call_surface_token(call, sink);
            }
        }
        return;
    }

    let token_texts = tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    if matches!(
        scope,
        Some(SourceScope::Levels | SourceScope::Level | SourceScope::UnbracedLevel)
    ) && crate::is_level_event_sugar(strip_line_comment(line).trim(), &token_texts)
    {
        sink.extend(rewrite_effect_surface_document(tokens));
        return;
    }

    if scope == Some(SourceScope::Other) {
        sink.extend(rewrite_effect_surface_document(tokens));
    }
}

fn record_visual_surface_line(
    scope: Option<SourceScope>,
    tokens: &[SourceToken],
    sink: &mut SurfaceSink,
) {
    let Some(first) = tokens.first() else {
        return;
    };
    match scope {
        Some(SourceScope::Visuals) => match tokens {
            [keyword, shape_ref, ..] if keyword.text == "shape" => {
                add_scene_effect_token_range(sink, keyword, SurfaceSemanticKind::Keyword);
                record_visual_table_expr_surface_token(shape_ref, sink);
            }
            [keyword, rest @ ..]
                if matches!(
                    keyword.text.as_str(),
                    "colors" | "palette"
                        | "image"
                        | "contain"
                        | "cover"
                        | "stretch"
                        | "selector"
                        | "offset"
                        | "sampling"
                        | "duration"
                        | "frame_duration"
                        | "pixels_per_cell"
                        | "rotate"
                ) =>
            {
                add_scene_effect_token_range(sink, keyword, SurfaceSemanticKind::Keyword);
                record_visual_rotation_surface_keywords(rest, sink);
            }
            _ if visual_sprite_selector_token(&first.text) => {
                record_visual_table_ref_surface_token(first, sink);
            }
            _ => {}
        },
        Some(SourceScope::VisualShapeTable) => match tokens {
            [keyword, rest @ ..] if keyword.text == "rotate" => {
                add_scene_effect_token_range(sink, keyword, SurfaceSemanticKind::Keyword);
                record_visual_rotation_surface_keywords(rest, sink);
            }
            [keyword, ..]
                if matches!(keyword.text.as_str(), "shape" | "shapes" | "palette" | "colors") =>
            {
                add_scene_effect_token_range(sink, keyword, SurfaceSemanticKind::Keyword);
            }
            _ => record_visual_table_ref_surface_token(first, sink),
        },
        Some(SourceScope::VisualColorTable) => {
            if tokens.get(1).is_some_and(|token| token.text == "=") {
                return;
            }
            if matches!(
                first.text.as_str(),
                "shape" | "shapes" | "palette" | "colors" | "rotate"
            ) {
                add_scene_effect_token_range(sink, first, SurfaceSemanticKind::Keyword);
            } else {
                record_visual_table_ref_surface_token(first, sink);
            }
        }
        Some(SourceScope::VisualShapeEntry) => match tokens {
            [keyword, shape_ref, ..] if keyword.text == "shape" => {
                add_scene_effect_token_range(sink, keyword, SurfaceSemanticKind::Keyword);
                record_visual_table_expr_surface_token(shape_ref, sink);
            }
            [keyword, rest @ ..] if matches!(keyword.text.as_str(), "rotate" | "colors") => {
                add_scene_effect_token_range(sink, keyword, SurfaceSemanticKind::Keyword);
                record_visual_rotation_surface_keywords(rest, sink);
            }
            _ => {}
        },
        _ => {}
    }
}

fn record_visual_table_ref_surface_token(token: &SourceToken, sink: &mut SurfaceSink) {
    let Some((start, end)) = surface_identifier_bounds(&token.text) else {
        return;
    };
    add_scene_effect_token_subrange(sink, token, start, end, SurfaceSemanticKind::Asset);
    record_visual_selector_suffix_surface_tokens(token, end, SurfaceSemanticKind::Group, sink);
}

fn record_visual_table_expr_surface_token(token: &SourceToken, sink: &mut SurfaceSink) {
    let Some((start, end)) = surface_identifier_bounds(&token.text) else {
        return;
    };
    add_scene_effect_token_subrange(sink, token, start, end, SurfaceSemanticKind::Asset);
    record_visual_selector_suffix_surface_tokens(token, end, SurfaceSemanticKind::Variant, sink);
}

fn record_visual_selector_suffix_surface_tokens(
    token: &SourceToken,
    base_end: usize,
    kind: SurfaceSemanticKind,
    sink: &mut SurfaceSink,
) {
    let mut cursor = base_end;
    while let Some(separator) = token.text[cursor..].find(':') {
        let start = cursor + separator + 1;
        let end = token.text[start..]
            .find(':')
            .map_or(token.text.len(), |offset| start + offset);
        let value = &token.text[start..end];
        if visual_sprite_selector_part_token(value) {
            add_scene_effect_token_subrange(sink, token, start, end, kind);
        }
        cursor = end;
    }
}

fn record_visual_rotation_surface_keywords(tokens: &[SourceToken], sink: &mut SurfaceSink) {
    for token in tokens {
        if matches!(token.text.as_str(), "from" | "using") {
            add_scene_effect_token_range(sink, token, SurfaceSemanticKind::Keyword);
        }
    }
}

fn surface_identifier_bounds(value: &str) -> Option<(usize, usize)> {
    let start = value
        .char_indices()
        .find_map(|(index, ch)| surface_word_start(ch).then_some(index))?;
    surface_identifier_end(value, start).map(|end| (start, end))
}

fn surface_identifier_end(value: &str, start: usize) -> Option<usize> {
    if start >= value.len() {
        return None;
    }
    let mut end = start;
    for (offset, ch) in value[start..].char_indices() {
        if offset == 0 {
            if !surface_word_start(ch) {
                return None;
            }
        } else if !surface_word_continue(ch) || matches!(ch, ':' | '.') {
            break;
        }
        end = start + offset + ch.len_utf8();
    }
    (end > start).then_some(end)
}

fn surface_word_start(ch: char) -> bool {
    ch == '@' || ch == '_' || ch.is_ascii_alphabetic()
}

fn surface_word_continue(ch: char) -> bool {
    ch == '@' || ch == '_' || ch == '-' || ch.is_ascii_alphanumeric()
}

#[cfg(test)]
mod surface_document_flow_tests {
    use super::{
        parse_surface_completion_context_document, parse_surface_completion_symbols_document,
        parse_surface_document, parse_surface_structure_document, source_line_tokens,
        validate_surface_document_projection,
    };
    use crate::surface::SurfaceSemanticKind;

    #[test]
    fn structure_only_surface_document_shares_full_structural_product() {
        let source = r#"
puzzle board {
  rules {
    routine push {
      if some([ Player ]) {
        [ Player ] -> [ Player ]
      }
    }
  }
  levels {
    level "one"
    P
  }
}
"#;
        let full = parse_surface_document(source);
        let structure = parse_surface_structure_document(source);
        assert_eq!(structure.lines, full.lines);
        assert_eq!(structure.structural_blocks, full.structural_blocks);
        assert!(structure.semantic_tokens.is_empty());
        assert!(structure.highlight_ranges.raw_ranges.is_empty());
        assert!(structure.completion_symbols.objects.is_empty());
    }

    #[test]
    fn completion_context_surface_document_skips_derived_products() {
        let source = r#"
puzzle board {
  sounds {
    sfx click = "click.wav"
  }
  rules {
  }
}
"#;
        let full = parse_surface_document(source);
        let context = parse_surface_completion_context_document(source);
        assert_eq!(context.lines, full.lines);
        assert_eq!(context.structural_blocks, full.structural_blocks);
        assert!(context.semantic_tokens.is_empty());
        assert!(context.highlight_ranges.raw_ranges.is_empty());
        assert!(context.completion_symbols.sfx.is_empty());
        assert!(context.visual_sprite_refs.color_names.is_empty());
    }

    #[test]
    fn completion_symbols_surface_document_skips_non_completion_products() {
        let source = r#"
puzzle board {
  sounds {
    sfx click = "click.wav"
  }
}
"#;
        let symbols = parse_surface_completion_symbols_document(source);
        assert!(symbols.completion_symbols.sfx.contains("click"));
        assert!(symbols.semantic_tokens.is_empty());
        assert!(symbols.highlight_ranges.raw_ranges.is_empty());
        assert!(symbols.visual_sprite_refs.color_names.is_empty());
    }

    #[test]
    fn surface_document_entrypoints_share_single_builder() {
        let source = include_str!("lib_surface_doc.rs");
        let required = [
            "build_surface_document(source, SurfaceDocumentProducts::FULL)",
            "build_surface_document(source, SurfaceDocumentProducts::STRUCTURE_ONLY)",
            "build_surface_document(source, SurfaceDocumentProducts::COMPLETION_SYMBOLS)",
            "build_surface_document(source, SurfaceDocumentProducts::SOURCE_TARGET)",
        ];
        for required in required {
            assert!(
                source.contains(required),
                "surface document entrypoints must delegate to one builder via {required}"
            );
        }
    }

    #[test]
    fn visual_scope_is_recognized_before_surface_products() {
        let source = include_str!("lib_surface_doc.rs");
        assert!(
            source.contains("visual_scope: Option<SurfaceVisualScope>"),
            "surface scan lines must carry recognized visual scope"
        );
        let forbidden = ["fn visual_", "highlight_scopes"].concat();
        assert!(
            !source.contains(&forbidden),
            "surface products must read recognized visual scope instead of rebuilding it via {forbidden}"
        );
    }

    #[test]
    fn semantic_tokens_follow_structural_blocks_not_header_whitelists() {
        let surface_doc_source = include_str!("lib_surface_doc.rs");
        assert!(
            surface_doc_source.contains("record_structural_block_surface_tokens(&structural_blocks"),
            "semantic tokens must start from the universal structural block product"
        );
        let source_scanner_source = include_str!("source.rs");
        assert!(
            !source_scanner_source.contains("source_tree_header_keyword"),
            "source scanner must not own highlight header whitelist decisions"
        );
    }

    #[test]
    fn every_structural_header_token_receives_a_surface_token() {
        let source = r#"
puzzle board {
rules {
routine Push once {
[ Player ] -> [ > Player ]
}
}
}
"#;
        let document = parse_surface_document(source);

        for block in document
            .structural_blocks
            .iter()
        {
            for header_token in source_line_tokens(&block.header, block.start) {
                assert!(
                    document.semantic_tokens.iter().any(|token| {
                        token.span.start == header_token.start && token.span.end == header_token.end
                    }),
                    "structural block `{}` left header token `{}` without a surface token",
                    block.header,
                    header_token.text
                );
            }
        }
    }

    #[test]
    fn puzzle_statement_headers_receive_surface_tokens() {
        let source = r#"
puzzle board {
rules {
}
on_level_start {
}
on_level_clear {
}
}
"#;
        let document = parse_surface_document(source);

        for header in ["rules", "on_level_start", "on_level_clear"] {
            let start = source.find(&format!("{header} {{")).unwrap();
            assert!(
                document.semantic_tokens.iter().any(|token| {
                    token.span.start == start
                        && token.span.end == start + header.len()
                        && token.kind == SurfaceSemanticKind::Keyword
                }),
                "statement block header `{header}` did not receive a keyword surface token"
            );
        }
    }

    #[test]
    fn scene_keys_and_routine_headers_receive_surface_tokens() {
        let source = r#"
scene title {
keys {
Enter -> start playing
}
routine continue_game {
goto playing
}
}
"#;
        let document = parse_surface_document(source);

        for (needle, text, kind) in [
            ("keys {", "keys", SurfaceSemanticKind::Keyword),
            ("routine continue_game", "routine", SurfaceSemanticKind::Keyword),
            (
                "routine continue_game",
                "continue_game",
                SurfaceSemanticKind::Binding,
            ),
        ] {
            let needle_start = source.find(needle).unwrap();
            let start = source[needle_start..].find(text).unwrap() + needle_start;
            assert!(
                document.semantic_tokens.iter().any(|token| {
                    token.span.start == start
                        && token.span.end == start + text.len()
                        && token.kind == kind
                }),
                "scene token `{text}` did not receive {kind:?}"
            );
        }
    }

    #[test]
    fn reserved_literals_and_scene_options_receive_surface_tokens() {
        let source = r#"
puzzle board {
levels {
legend {
. = empty
}
level "start" {
.
}
}
}

scene level_select {
layout {
level_menu {
show_index = true
show_solved=false
columns = 3
}
}
}
"#;
        let document = parse_surface_document(source);

        for (needle, text, kind) in [
            (". = empty", "empty", SurfaceSemanticKind::Literal),
            ("show_index = true", "show_index", SurfaceSemanticKind::Setting),
            ("show_index = true", "true", SurfaceSemanticKind::Literal),
            ("show_solved=false", "show_solved", SurfaceSemanticKind::Setting),
            ("show_solved=false", "false", SurfaceSemanticKind::Literal),
            ("columns = 3", "columns", SurfaceSemanticKind::Setting),
            ("columns = 3", "3", SurfaceSemanticKind::Number),
        ] {
            let needle_start = source.find(needle).unwrap();
            let start = source[needle_start..].find(text).unwrap() + needle_start;
            assert!(
                document.semantic_tokens.iter().any(|token| {
                    token.span.start == start
                        && token.span.end == start + text.len()
                        && token.kind == kind
                }),
                "token `{text}` in `{needle}` did not receive {kind:?}"
            );
        }
    }

    #[test]
    fn condition_source_tree_headers_are_owned_surface_tokens() {
        let source = r#"
puzzle board {
win_conditions {
some([ Goal ])
}
lose_conditions any {
some([ Trap ])
}
}
"#;
        let document = parse_surface_document(source);

        for header in ["win_conditions", "lose_conditions any"] {
            let block = document
                .structural_blocks
                .iter()
                .find(|block| block.header == header)
                .expect("condition structural block");
            for token in source_line_tokens(&block.header, block.start) {
                assert!(
                    document.semantic_tokens.iter().any(|semantic| {
                        semantic.span.start == token.start && semantic.span.end == token.end
                    }),
                    "condition block `{}` left header token `{}` without a surface token",
                    block.header,
                    token.text
                );
            }
        }
    }

    #[test]
    fn unbraced_level_source_tree_headers_are_owned_surface_tokens() {
        let source = r#"
puzzle board {
levels {
legend {
P = Player
. = empty
x = Wall
}
level one
P.x

P.x
}
}
"#;
        let document = parse_surface_document(source);

        assert!(
            document
                .structural_blocks
                .iter()
                .any(|block| block.header == "level one"),
            "named unbraced level block should be present"
        );
        for header in ["P.x"] {
            let block = document
                .structural_blocks
                .iter()
                .find(|block| block.header == header)
                .expect("unbraced level structural block");
            for token in source_line_tokens(&block.header, block.start) {
                assert!(
                    document.semantic_tokens.iter().any(|semantic| {
                        semantic.span.start == token.start && semantic.span.end == token.end
                    }),
                    "unbraced level block `{}` left header token `{}` without a surface token",
                    block.header,
                    token.text
                );
            }
        }
    }

    #[test]
    fn scene_layout_source_tree_headers_are_owned_surface_tokens() {
        let source = r#"
scene playing {
layout {
row {
text "Ready"
}
}
}
"#;
        let document = parse_surface_document(source);
        let block = document
            .structural_blocks
            .iter()
            .find(|block| block.header == "layout")
            .expect("layout structural block");

        assert!(
            source_line_tokens(&block.header, block.start)
                .into_iter()
                .all(|header_token| document.semantic_tokens.iter().any(|semantic| {
                    semantic.span.start == header_token.start
                        && semantic.span.end == header_token.end
                })),
            "layout header should be owned by scene surface projection"
        );
    }

    #[test]
    fn unowned_source_tree_header_reports_surface_projection_error() {
        let source = r#"
puzzle board {
__invalid_unowned_surface_node__ {
}
}
"#;
        let error =
            validate_surface_document_projection(source).expect_err("unowned header should fail");

        assert!(
            error
                .to_string()
                .contains("unowned structural block header `__invalid_unowned_surface_node__`"),
            "{error}"
        );
    }
}
