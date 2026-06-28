use std::collections::{HashMap, HashSet};

use crate::semantic::{SemanticKind, SemanticToken, first_identifier_bounds, semantic_tokens};
use crate::source::{SourceScope, scan_source_context, split_header_tokens, strip_line_comment};
use crate::syntax::{is_parser_keyword, is_puzzle_line_head_keyword};
use crate::{
    LoadedDocumentModel, RewriteEffectCommandSyntax, is_visual_color_token,
    rewrite_effect_command_syntax, scene_effect_command_syntax, visual_color_token_for_index,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HighlightedSource {
    pub html: String,
    pub parsed: bool,
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
    Scratch,
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
            HighlightKind::Scratch => "syntax-scratch",
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
    let parsed = crate::parse_game(source).ok();
    let mut symbols = HashMap::<String, HighlightKind>::new();
    let mut family_bases = HashSet::<String>::new();
    let mut family_axes = HashMap::<String, usize>::new();
    let mut family_axis_names = HashSet::<String>::new();
    for builtin_axis in ["directions", "horizontal", "vertical"] {
        symbols.insert(builtin_axis.to_string(), HighlightKind::Group);
        family_axis_names.insert(builtin_axis.to_string());
    }
    if let Some(document) = &parsed {
        for model in &document.models {
            match model {
                LoadedDocumentModel::Puzzle2d { name, game } => {
                    symbols.insert(name.clone(), HighlightKind::Scene);
                    collect_loaded_game_symbols(game, &mut symbols);
                }
                LoadedDocumentModel::Puzzle3d { name, puzzle } => {
                    symbols.insert(name.clone(), HighlightKind::Scene);
                    collect_puzzle3_symbols(
                        puzzle,
                        &mut symbols,
                        &mut family_bases,
                        &mut family_axes,
                        &mut family_axis_names,
                    );
                }
            }
        }
        for scene in &document.scenes {
            symbols.insert(scene.name.clone(), HighlightKind::Scene);
            for puzzle in &scene.state.puzzles {
                symbols.insert(puzzle.name.clone(), HighlightKind::State);
                symbols.insert(puzzle.model.clone(), HighlightKind::Scene);
            }
        }
        for sfx in &document.sounds.sfx {
            symbols.insert(sfx.name.clone(), HighlightKind::Asset);
        }
        for music in &document.sounds.music {
            symbols.insert(music.name.clone(), HighlightKind::Asset);
        }
    }

    collect_source_symbols(
        source,
        &mut symbols,
        &mut family_bases,
        &mut family_axes,
        &mut family_axis_names,
    );

    HighlightedSource {
        html: highlight_html(
            source,
            &symbols,
            &family_bases,
            &family_axes,
            &family_axis_names,
        ),
        parsed: parsed.is_some(),
    }
}

fn collect_loaded_game_symbols(
    game: &crate::LoadedGame,
    symbols: &mut HashMap<String, HighlightKind>,
) {
    for name in game.object_labels.values() {
        symbols.insert(name.clone(), HighlightKind::Object);
    }
    for name in game.object_groups.keys() {
        insert_source_symbol(symbols, name, HighlightKind::Group);
    }
    for name in game.input_labels.values() {
        symbols.insert(name.clone(), HighlightKind::Input);
    }
    for name in game.global_labels.values() {
        symbols.insert(name.clone(), HighlightKind::State);
    }
    for name in game.condition_labels.values() {
        symbols.insert(name.clone(), HighlightKind::Condition);
    }
    for name in game.conditions.keys() {
        symbols.insert(name.clone(), HighlightKind::Condition);
    }
    for level in &game.levels {
        symbols.insert(level.name.clone(), HighlightKind::Scene);
    }
    for scene in &game.scenes {
        symbols.insert(scene.name.clone(), HighlightKind::Scene);
    }
    for sfx in &game.sounds.sfx {
        symbols.insert(sfx.name.clone(), HighlightKind::Asset);
    }
    for music in &game.sounds.music {
        symbols.insert(music.name.clone(), HighlightKind::Asset);
    }
    for sprite in &game.visuals.sprites {
        symbols.insert(sprite.name.clone(), HighlightKind::Asset);
    }
}

fn collect_puzzle3_symbols(
    puzzle: &crate::ParsedPuzzle3,
    symbols: &mut HashMap<String, HighlightKind>,
    family_bases: &mut HashSet<String>,
    family_axes: &mut HashMap<String, usize>,
    family_axis_names: &mut HashSet<String>,
) {
    for object in &puzzle.catalog.objects {
        symbols.insert(object.name.clone(), HighlightKind::Object);
    }
    for family in &puzzle.catalog.families {
        family_bases.insert(family.name.clone());
        family_axes.insert(family.name.clone(), family.axes.len());
        symbols.insert(family.name.clone(), HighlightKind::Object);
        for axis in &family.axes {
            family_axis_names.insert(axis.name.clone());
            symbols.insert(axis.name.clone(), HighlightKind::Group);
            if let puzzle_3d::VariantValueSet3::Named(values) = &axis.values {
                for value in values {
                    symbols.insert(value.clone(), HighlightKind::Object);
                }
            }
        }
    }
    for group in &puzzle.catalog.groups {
        insert_source_symbol(symbols, &group.name, HighlightKind::Group);
    }
    for input in &puzzle.game.inputs {
        symbols.insert(input.name.clone(), HighlightKind::Input);
    }
    if let Some(level_bundle) = &puzzle.level_bundle {
        for level in &level_bundle.levels {
            symbols.insert(level.name.clone(), HighlightKind::Scene);
        }
    }
    if let Some(sprite_set) = &puzzle.sprite_set {
        symbols.insert(sprite_set.name.clone(), HighlightKind::Asset);
        for sprite in &sprite_set.sprites {
            symbols.insert(sprite.name.clone(), HighlightKind::Asset);
        }
    }
}

fn highlight_html(
    source: &str,
    symbols: &HashMap<String, HighlightKind>,
    family_bases: &HashSet<String>,
    family_axes: &HashMap<String, usize>,
    family_axis_names: &HashSet<String>,
) -> String {
    let mut out = String::with_capacity(source.len().saturating_add(source.len() / 8));
    let context = scan_source_context(source);
    let binding_ranges = scan_for_binding_ranges(source);
    let semantic_ranges = semantic_tokens(source);
    let keyword_ranges = scan_contextual_keyword_ranges(&context);
    let brace_ranges = scan_brace_ranges(source);
    let level_ascii_ranges = scan_level_ascii_ranges(&context);
    let visual_color_aliases = scan_visual_color_aliases(&context);
    let visual_named_color_ranges = scan_visual_named_color_ranges(&context, &visual_color_aliases);
    let visual_ascii_color_ranges = scan_visual_ascii_color_ranges(&context, &visual_color_aliases);
    let mut chars = source.char_indices().peekable();

    while let Some((index, ch)) = chars.next() {
        if let Some(range) = level_ascii_range_starting_at(&level_ascii_ranges, index) {
            let kind = if range.known {
                HighlightKind::LevelCell
            } else {
                HighlightKind::InvalidLevelCell
            };
            push_span(&mut out, kind, &source[range.start..range.end]);
            skip_until(&mut chars, range.end);
            continue;
        }

        if let Some(range) = visual_ascii_color_range_starting_at(&visual_ascii_color_ranges, index)
        {
            push_colored_text_span(
                &mut out,
                &range.color,
                &source[range.start..range.end],
                range.transparent,
            );
            skip_until(&mut chars, range.end);
            continue;
        }

        if let Some(range) = visual_named_color_range_starting_at(&visual_named_color_ranges, index)
        {
            push_color_text_span(&mut out, &range.color, &source[range.start..range.end]);
            skip_until(&mut chars, range.end);
            continue;
        }

        if let Some(end) = context.raw_range_starting_at(index) {
            if let Some(next_start) = next_raw_embedded_highlight_start(
                index,
                end,
                &level_ascii_ranges,
                &visual_ascii_color_ranges,
                &visual_named_color_ranges,
            ) && next_start > index
            {
                escape_html_into(&mut out, &source[index..next_start]);
                skip_until(&mut chars, next_start);
                continue;
            }
            escape_html_into(&mut out, &source[index..end]);
            skip_until(&mut chars, end);
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
            push_span(&mut out, HighlightKind::String, &source[index..end]);
            continue;
        }

        if let Some(end) = hex_color_end(source, index, ch) {
            push_color_span(&mut out, &source[index..end]);
            skip_until(&mut chars, end);
            continue;
        }

        if is_number_start(source, index, ch) {
            let end = consume_while(source, index, |value| {
                value.is_ascii_digit() || matches!(value, '.' | '_' | '-')
            });
            if context.is_plain_range(index, end) {
                escape_html_into(&mut out, &source[index..end]);
            } else {
                push_span(&mut out, HighlightKind::Number, &source[index..end]);
            }
            skip_until(&mut chars, end);
            continue;
        }

        if is_word_start_at(source, index, ch) {
            let end = consume_word(source, index);
            let token = &source[index..end];
            if context.is_plain_range(index, end) {
                escape_html_into(&mut out, token);
            } else {
                push_word(
                    &mut out,
                    token,
                    index,
                    symbols,
                    family_bases,
                    family_axes,
                    family_axis_names,
                    &binding_ranges,
                    &semantic_ranges,
                    &keyword_ranges,
                );
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

fn collect_source_symbols(
    source: &str,
    symbols: &mut HashMap<String, HighlightKind>,
    family_bases: &mut HashSet<String>,
    family_axes: &mut HashMap<String, usize>,
    family_axis_names: &mut HashSet<String>,
) {
    let context = scan_source_context(source);
    for line in context.lines {
        if line.tokens.is_empty() {
            continue;
        }
        let tokens = line.tokens.iter().map(String::as_str).collect::<Vec<_>>();
        let scope = symbol_collection_scope(line.scope);
        collect_line_symbols(
            &tokens,
            scope,
            line.content.trim_end().ends_with('{'),
            symbols,
            family_bases,
            family_axes,
            family_axis_names,
        );
    }
}

fn symbol_collection_scope(scope: Option<SourceScope>) -> Option<SourceScope> {
    match scope {
        None | Some(SourceScope::Puzzle) => None,
        Some(
            scope @ (SourceScope::Sounds
            | SourceScope::Tags
            | SourceScope::Group
            | SourceScope::Layers
            | SourceScope::Scratch
            | SourceScope::Visuals
            | SourceScope::VisualShapeTable
            | SourceScope::Keys),
        ) => Some(scope),
        Some(SourceScope::SceneKeys) => Some(SourceScope::Keys),
        Some(SourceScope::SceneState) => Some(SourceScope::SceneState),
        Some(_) => Some(SourceScope::Other),
    }
}

fn collect_line_symbols(
    tokens: &[&str],
    scope: Option<SourceScope>,
    line_opens_block: bool,
    symbols: &mut HashMap<String, HighlightKind>,
    family_bases: &mut HashSet<String>,
    family_axes: &mut HashMap<String, usize>,
    family_axis_names: &mut HashSet<String>,
) {
    match tokens {
        ["routine", "display", name, ..] | ["rule", "display", name, ..] => {
            insert_declared_source_symbol(symbols, name, HighlightKind::Effect);
        }
        ["routine", name, ..] | ["rule", name, ..] => {
            insert_declared_source_symbol(symbols, name, HighlightKind::Effect);
        }
        ["input", name, ..] | ["direction", name, ..] => {
            insert_declared_source_symbol(symbols, name, HighlightKind::Input);
        }
        ["condition", name, ..] => {
            insert_declared_source_symbol(symbols, name, HighlightKind::Condition);
        }
        ["scene", name, ..] if scope.is_none() => {
            insert_declared_source_symbol(symbols, name, HighlightKind::Scene);
        }
        ["puzzle" | "puzzle3", name, ..] if scope.is_none() => {
            insert_declared_source_symbol(symbols, name, HighlightKind::Scene);
        }
        ["levels3", name, "of", model, ..] | ["sprites3", name, "of", model, ..]
            if scope.is_none() =>
        {
            insert_declared_source_symbol(symbols, name, HighlightKind::Scene);
            insert_source_symbol(symbols, model, HighlightKind::Scene);
        }
        ["levels3" | "sprites3", name, ..] if scope.is_none() => {
            insert_declared_source_symbol(symbols, name, HighlightKind::Scene);
        }
        ["map", name, axis] => {
            insert_declared_source_symbol(symbols, name, HighlightKind::Effect);
            insert_declared_source_symbol(symbols, axis, HighlightKind::Group);
            family_axis_names.insert((*axis).to_string());
        }
        ["sfx", name, ..] | ["music", name, ..] if scope == Some(SourceScope::Sounds) => {
            insert_declared_source_symbol(symbols, name, HighlightKind::Asset);
        }
        ["shape", table, ..] | ["colors", table, ..] => {
            collect_visual_table_symbol(table, symbols);
        }
        [name] if scope == Some(SourceScope::Visuals) => {
            insert_source_symbol(symbols, name, HighlightKind::Asset);
        }
        [table, ..] if scope == Some(SourceScope::VisualShapeTable) && line_opens_block => {
            collect_visual_table_symbol(table, symbols);
        }
        ["var" | "const", name, "=", ..] if scope == Some(SourceScope::SceneState) => {
            insert_declared_source_symbol(symbols, name, HighlightKind::State);
        }
        ["persistent", "var" | "const", name, "=", ..]
            if scope == Some(SourceScope::SceneState) =>
        {
            insert_declared_source_symbol(symbols, name, HighlightKind::State);
        }
        ["persistent", name, "=", ..] if scope == Some(SourceScope::SceneState) => {
            insert_declared_source_symbol(symbols, name, HighlightKind::State);
        }
        [name, "=", ..] if scope == Some(SourceScope::SceneState) => {
            insert_declared_source_symbol(symbols, name, HighlightKind::State);
        }
        ["var" | "const", name, "=", ..]
        | ["persistent", "var" | "const", name, "=", ..]
        | ["persistent", name, "=", ..] => {
            insert_declared_source_symbol(symbols, name, HighlightKind::State);
        }
        ["group", name, "=", selectors @ ..] => {
            insert_declared_source_symbol(symbols, name, HighlightKind::Group);
            collect_selector_specs(selectors, symbols);
        }
        [name, "=", values @ ..] if scope == Some(SourceScope::Group) => {
            insert_declared_source_symbol(symbols, name, HighlightKind::Group);
            collect_selector_specs(values, symbols);
        }
        [name, "=", selectors @ ..] if scope == Some(SourceScope::Layers) => {
            insert_declared_source_symbol(symbols, name, HighlightKind::Group);
            collect_layer_selector_specs(
                name,
                selectors,
                symbols,
                family_bases,
                family_axes,
                family_axis_names,
            );
        }
        ["each", selectors @ ..] if scope == Some(SourceScope::Layers) => {
            collect_schema_object_specs(
                selectors,
                symbols,
                family_bases,
                family_axes,
                family_axis_names,
            );
        }
        [first, selectors @ ..] if scope == Some(SourceScope::Layers) && *first != "for" => {
            collect_selector_spec(first, symbols);
            collect_selector_specs(selectors, symbols);
        }
        [name, "=", ty] if scope == Some(SourceScope::Scratch) => {
            collect_scratch_spec(name, Some(*ty), symbols)
        }
        [spec] if scope == Some(SourceScope::Scratch) => {
            let (name, ty) = spec
                .split_once('=')
                .map_or((*spec, None), |(name, ty)| (name, Some(ty)));
            collect_scratch_spec(name, ty, symbols);
        }
        [..] if matches!(scope, Some(SourceScope::Keys | SourceScope::SceneKeys)) => {
            collect_key_binding_symbols(tokens, symbols)
        }
        [name, "=", values @ ..]
            if scope == Some(SourceScope::Tags) && tag_set_tokens(name, values) =>
        {
            insert_declared_source_symbol(symbols, name, HighlightKind::Group);
            family_axis_names.insert((*name).to_string());
            for value in values {
                insert_object_name_atom_symbol(symbols, value);
            }
        }
        _ => {}
    }
}

fn collect_visual_table_symbol(table: &str, symbols: &mut HashMap<String, HighlightKind>) {
    if let Some((name, axis)) = table.split_once(':') {
        insert_source_symbol(symbols, name, HighlightKind::Asset);
        insert_source_symbol(symbols, axis, HighlightKind::Group);
    } else {
        insert_source_symbol(symbols, table, HighlightKind::Asset);
    }
}

fn collect_key_binding_symbols(tokens: &[&str], symbols: &mut HashMap<String, HighlightKind>) {
    let Some(separator) = tokens.iter().position(|token| matches!(*token, "=" | "->")) else {
        return;
    };
    if separator == 0 || separator + 1 >= tokens.len() {
        return;
    }
    for key in &tokens[..separator] {
        insert_source_symbol(symbols, key, HighlightKind::Input);
    }
    if let Some(action) = tokens.get(separator + 1).copied()
        && is_source_identifier(action)
        && !parser_effect(action)
    {
        insert_source_symbol(symbols, action, HighlightKind::Input);
    }
}

fn collect_selector_specs(specs: &[&str], symbols: &mut HashMap<String, HighlightKind>) {
    for spec in specs {
        if matches!(*spec, "=" | "display" | "each") || parser_keyword(spec) {
            continue;
        }
        collect_selector_spec(spec, symbols);
    }
}

fn collect_layer_selector_specs(
    layer_name: &str,
    specs: &[&str],
    symbols: &mut HashMap<String, HighlightKind>,
    family_bases: &mut HashSet<String>,
    family_axes: &mut HashMap<String, usize>,
    family_axis_names: &mut HashSet<String>,
) {
    for spec in specs {
        if matches!(*spec, "=" | "display" | "each") || parser_keyword(spec) {
            continue;
        }
        collect_layer_selector_spec(
            layer_name,
            spec,
            symbols,
            family_bases,
            family_axes,
            family_axis_names,
        );
    }
}

fn collect_selector_spec(spec: &str, symbols: &mut HashMap<String, HighlightKind>) {
    let cleaned = clean_object_spec(spec);
    let mut parts = cleaned.split(':');
    let Some(base) = parts.next() else {
        return;
    };
    if matches!(symbols.get(base), Some(HighlightKind::Group)) {
        return;
    }
    collect_object_spec(spec, symbols);
}

fn collect_layer_selector_spec(
    layer_name: &str,
    spec: &str,
    symbols: &mut HashMap<String, HighlightKind>,
    family_bases: &mut HashSet<String>,
    family_axes: &mut HashMap<String, usize>,
    family_axis_names: &mut HashSet<String>,
) {
    let cleaned = clean_object_spec(spec);
    let mut parts = cleaned.split(':');
    let Some(base) = parts.next() else {
        return;
    };
    if base != layer_name && matches!(symbols.get(base), Some(HighlightKind::Group)) {
        return;
    }
    collect_schema_object_spec(spec, symbols, family_bases, family_axes, family_axis_names);
}

fn collect_schema_object_specs(
    specs: &[&str],
    symbols: &mut HashMap<String, HighlightKind>,
    family_bases: &mut HashSet<String>,
    family_axes: &mut HashMap<String, usize>,
    family_axis_names: &mut HashSet<String>,
) {
    for spec in specs {
        if matches!(*spec, "=" | "display" | "each") || parser_keyword(spec) {
            continue;
        }
        collect_schema_object_spec(spec, symbols, family_bases, family_axes, family_axis_names);
    }
}

fn collect_schema_object_spec(
    spec: &str,
    symbols: &mut HashMap<String, HighlightKind>,
    family_bases: &mut HashSet<String>,
    family_axes: &mut HashMap<String, usize>,
    family_axis_names: &mut HashSet<String>,
) {
    let cleaned = clean_object_spec(spec);
    let parts = cleaned.split(':').collect::<Vec<_>>();
    if parts.len() > 1 {
        let base = parts[0];
        if is_source_symbol_name(base) {
            family_bases.insert(base.to_string());
            family_axes.insert(base.to_string(), parts.len() - 1);
        }
        for axis in parts.iter().skip(1) {
            if is_source_symbol_name(axis) {
                family_axis_names.insert((*axis).to_string());
            }
        }
    }
    collect_object_spec(spec, symbols);
}

fn collect_object_spec(spec: &str, symbols: &mut HashMap<String, HighlightKind>) {
    let spec = clean_object_spec(spec);
    let mut parts = spec.split(':');
    let Some(base) = parts.next() else {
        return;
    };
    insert_source_symbol(symbols, base, HighlightKind::Object);
    for part in parts {
        insert_object_name_atom_symbol(symbols, part);
    }
}

fn clean_object_spec(spec: &str) -> &str {
    let spec = spec.trim_matches(|ch: char| matches!(ch, '[' | ']' | '(' | ')' | '|'));
    spec.split_once('{').map_or(spec, |(head, _)| head)
}

fn collect_scratch_spec(
    name: &str,
    ty: Option<&str>,
    symbols: &mut HashMap<String, HighlightKind>,
) {
    insert_scratch_symbol(symbols, name);
    if let Some(ty) = ty.filter(|ty| !matches!(*ty, "bool" | "int")) {
        insert_source_symbol(symbols, ty, HighlightKind::Variant);
    }
}

fn insert_scratch_symbol(symbols: &mut HashMap<String, HighlightKind>, name: &str) {
    if is_source_symbol_name(name) {
        insert_source_symbol(symbols, name, HighlightKind::Scratch);
    } else if is_source_scratch_qualified_identifier(name) && !parser_literal(name) {
        symbols.insert(name.to_string(), HighlightKind::Scratch);
    }
}

fn insert_object_name_atom_symbol(symbols: &mut HashMap<String, HighlightKind>, name: &str) {
    if matches!(symbols.get(name), Some(HighlightKind::Group)) {
        return;
    }
    insert_declared_source_symbol(symbols, name, HighlightKind::Object);
}

fn tag_set_tokens(name: &str, values: &[&str]) -> bool {
    is_source_identifier(name)
        && !values.is_empty()
        && values.iter().all(|value| is_source_identifier(value))
}

fn insert_source_symbol(
    symbols: &mut HashMap<String, HighlightKind>,
    name: &str,
    kind: HighlightKind,
) {
    if !is_source_symbol_name(name) || parser_keyword(name) || parser_literal(name) {
        return;
    }
    match symbols.get(name).copied() {
        Some(existing) if symbol_priority(existing) > symbol_priority(kind) => {}
        _ => {
            symbols.insert(name.to_string(), kind);
        }
    }
}

fn insert_declared_source_symbol(
    symbols: &mut HashMap<String, HighlightKind>,
    name: &str,
    kind: HighlightKind,
) {
    if !is_source_symbol_name(name) || parser_literal(name) {
        return;
    }
    match symbols.get(name).copied() {
        Some(existing) if symbol_priority(existing) > symbol_priority(kind) => {}
        _ => {
            symbols.insert(name.to_string(), kind);
        }
    }
}

fn symbol_priority(kind: HighlightKind) -> u8 {
    match kind {
        HighlightKind::Object => 6,
        HighlightKind::Group => 5,
        HighlightKind::State
        | HighlightKind::Scratch
        | HighlightKind::Input
        | HighlightKind::Effect
        | HighlightKind::Emission
        | HighlightKind::Condition
        | HighlightKind::Scene
        | HighlightKind::Asset => 4,
        HighlightKind::Variant => 2,
        _ => 1,
    }
}

fn is_source_symbol_name(name: &str) -> bool {
    if let Some(rest) = name.strip_prefix('@') {
        return is_source_identifier(rest);
    }
    is_source_identifier(name)
}

fn is_source_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn is_source_scratch_qualified_identifier(name: &str) -> bool {
    let mut parts = name.split(':');
    let Some(first) = parts.next() else {
        return false;
    };
    is_source_identifier(first) && parts.all(is_source_scratch_value_atom)
}

fn is_source_value_atom(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn is_source_scratch_value_atom(name: &str) -> bool {
    is_source_value_atom(name) || matches!(name, ">" | "<" | "^" | "v")
}

fn scan_contextual_keyword_ranges(
    context: &crate::source::SourceContext,
) -> HashSet<(usize, usize)> {
    let mut ranges = HashSet::<(usize, usize)>::new();
    for line in &context.lines {
        let tokens = line
            .token_spans
            .iter()
            .filter_map(highlight_token_identifier)
            .collect::<Vec<_>>();
        for index in 0..tokens.len() {
            let (text, start, end) = tokens[index];
            if parser_keyword(text) && contextual_keyword_token(line.scope, &tokens, index) {
                ranges.insert((start, end));
            }
        }
    }
    ranges
}

fn highlight_token_identifier(token: &crate::source::SourceToken) -> Option<(&str, usize, usize)> {
    let (relative_start, relative_end) = first_identifier_bounds(&token.text)?;
    Some((
        &token.text[relative_start..relative_end],
        token.start + relative_start,
        token.start + relative_end,
    ))
}

fn contextual_keyword_token(
    scope: Option<SourceScope>,
    tokens: &[(&str, usize, usize)],
    index: usize,
) -> bool {
    let Some((keyword, _, _)) = tokens.get(index).copied() else {
        return false;
    };
    if index == 0 && line_head_keyword(scope, keyword, tokens) {
        return true;
    }
    match keyword {
        "in" => tokens
            .get(index.checked_sub(2).unwrap_or(usize::MAX))
            .is_some_and(|(for_keyword, _, _)| *for_keyword == "for"),
        "of" => tokens
            .first()
            .is_some_and(|(first, _, _)| matches!(*first, "levels" | "levels3" | "sprites3")),
        "display" => {
            index == 1
                && tokens
                    .first()
                    .is_some_and(|(first, _, _)| matches!(*first, "routine" | "rule"))
        }
        "input" => is_rule_like_scope(scope) && index > 0 && before_pattern_token(tokens, index),
        "puzzle" | "puzzle3" => index > 0,
        _ => false,
    }
}

fn line_head_keyword(
    scope: Option<SourceScope>,
    keyword: &str,
    tokens: &[(&str, usize, usize)],
) -> bool {
    if tokens
        .get(1)
        .is_some_and(|(separator, _, _)| *separator == "=")
    {
        return matches!(scope, Some(SourceScope::SceneState)) && matches!(keyword, "persistent");
    }

    match scope {
        None => false,
        Some(SourceScope::Puzzle) => is_puzzle_line_head_keyword(keyword),
        Some(SourceScope::Sounds) => matches!(keyword, "sfx" | "music"),
        Some(SourceScope::Assets) => matches!(keyword, "css"),
        Some(SourceScope::Tags) => matches!(keyword, "for"),
        Some(SourceScope::Group) => matches!(keyword, "for"),
        Some(SourceScope::Layers) => matches!(keyword, "each" | "for"),
        Some(SourceScope::Scratch) => matches!(keyword, "var" | "const"),
        Some(SourceScope::Keys | SourceScope::SceneKeys) => {
            matches!(keyword, "input" | "direction")
        }
        Some(SourceScope::Legend) => matches!(keyword, "legend"),
        Some(SourceScope::Levels | SourceScope::Level | SourceScope::UnbracedLevel) => {
            matches!(keyword, "legend" | "level")
        }
        Some(SourceScope::Scene) => matches!(
            keyword,
            "button" | "for" | "if" | "keys" | "layout" | "level_menu" | "rules" | "state"
        ),
        Some(SourceScope::SceneLayout) => matches!(
            keyword,
            "box"
                | "button"
                | "column"
                | "for"
                | "if"
                | "level_menu"
                | "puzzle"
                | "puzzle3"
                | "row"
                | "text"
                | "title"
                | "subtitle"
        ),
        Some(SourceScope::SceneState) => matches!(keyword, "var" | "const" | "persistent"),
        Some(SourceScope::SceneTransitions) => matches!(
            keyword,
            "button" | "else" | "for" | "if" | "input" | "routine" | "step"
        ),
        Some(SourceScope::LevelMenu) => matches!(keyword, "button" | "for" | "if"),
        Some(
            SourceScope::Visuals
            | SourceScope::VisualShapeTable
            | SourceScope::VisualShapeEntry
            | SourceScope::VisualColorTable,
        ) => matches!(keyword, "colors" | "shape" | "shapes"),
        Some(SourceScope::Other) => {
            matches!(keyword, "else" | "for" | "if" | "layers" | "repeat")
                || rewrite_application_keyword(keyword)
        }
    }
}

fn is_rule_like_scope(scope: Option<SourceScope>) -> bool {
    matches!(
        scope,
        Some(SourceScope::Other | SourceScope::SceneTransitions)
    )
}

fn before_pattern_token(tokens: &[(&str, usize, usize)], index: usize) -> bool {
    tokens
        .get(index + 1)
        .is_some_and(|(next, _, _)| matches!(*next, "[" | "{"))
}

fn rewrite_application_keyword(value: &str) -> bool {
    matches!(
        value,
        "fix" | "once" | "once_all" | "once_per_level" | "repeat"
    )
}

fn classify_bare_word(
    token: &str,
    symbols: &HashMap<String, HighlightKind>,
    contextual_keyword: bool,
) -> Option<HighlightKind> {
    classify_word(token, symbols, contextual_keyword)
}

fn classify_word(
    token: &str,
    symbols: &HashMap<String, HighlightKind>,
    contextual_keyword: bool,
) -> Option<HighlightKind> {
    if contextual_keyword && parser_keyword(token) {
        return Some(HighlightKind::Keyword);
    }
    if let Some(kind) = symbols.get(token).copied() {
        return Some(kind);
    }
    if let Some((head, _)) = token.split_once(':') {
        if let Some(kind @ HighlightKind::Object) = symbols.get(head).copied() {
            return Some(kind);
        }
    }
    if parser_literal(token) {
        return Some(HighlightKind::Literal);
    }
    if parser_emission(token) {
        return Some(HighlightKind::Emission);
    }
    if parser_effect(token) {
        return Some(HighlightKind::Effect);
    }
    None
}

fn push_word(
    out: &mut String,
    token: &str,
    token_start: usize,
    symbols: &HashMap<String, HighlightKind>,
    family_bases: &HashSet<String>,
    family_axes: &HashMap<String, usize>,
    family_axis_names: &HashSet<String>,
    binding_ranges: &[BindingRange],
    semantic_ranges: &[SemanticToken],
    keyword_ranges: &HashSet<(usize, usize)>,
) {
    let qualified_scratch = matches!(symbols.get(token), Some(HighlightKind::Scratch));
    if qualified_scratch && !token.contains(':') {
        push_span(out, HighlightKind::Scratch, token);
        return;
    }

    let parts = split_highlight_word(token);
    let supplied_axes = token.matches(':').count();
    let use_schema_selector_coloring =
        token.contains(':') && !token.contains('.') && !qualified_scratch;
    let schema_selector_head = parts.first().map(|part| &token[part.start..part.end]);

    for (index, part) in parts.iter().enumerate() {
        if let Some(separator) = part.separator_before {
            let separator_kind = if separator == "#" {
                HighlightKind::Binding
            } else {
                HighlightKind::Operator
            };
            push_span(out, separator_kind, separator);
        }
        let absolute_start = token_start + part.start;
        let absolute_end = token_start + part.end;
        let text = &token[part.start..part.end];
        let semantic_kind = semantic_kind_at(semantic_ranges, absolute_start, absolute_end);
        let symbol_kind = symbols.get(text).copied();
        let contextual_keyword = keyword_ranges.contains(&(absolute_start, absolute_end));
        let kind = if part.separator_before == Some("#") {
            Some(HighlightKind::Binding)
        } else if qualified_scratch {
            qualified_scratch_part_kind(index, text, symbol_kind, family_axis_names)
        } else if use_schema_selector_coloring
            && index > 0
            && local_binding_at(binding_ranges, absolute_start, absolute_end, text)
        {
            Some(HighlightKind::Binding)
        } else if use_schema_selector_coloring
            && index > 0
            && symbol_kind == Some(HighlightKind::State)
        {
            Some(HighlightKind::State)
        } else if use_schema_selector_coloring && index > 0 && family_axis_names.contains(text) {
            Some(HighlightKind::Group)
        } else if use_schema_selector_coloring && index > 0 && text == "*" {
            Some(HighlightKind::Group)
        } else if use_schema_selector_coloring
            && index > 0
            && schema_selector_head_is_known(
                schema_selector_head,
                symbols,
                family_bases,
                family_axes,
            )
        {
            Some(HighlightKind::Object)
        } else if semantic_kind == Some(HighlightKind::Scene)
            && symbol_kind == Some(HighlightKind::State)
        {
            symbol_kind
        } else if let Some(kind) = semantic_kind {
            Some(kind)
        } else if local_binding_at(binding_ranges, absolute_start, absolute_end, text) {
            Some(HighlightKind::Binding)
        } else if use_schema_selector_coloring && index == 0 && text == "*" {
            Some(HighlightKind::Group)
        } else if use_schema_selector_coloring
            && index == 0
            && matches!(symbols.get(text), Some(HighlightKind::Object))
        {
            let family_axis_count = family_axes.get(text).copied().unwrap_or(supplied_axes);
            if supplied_axes < family_axis_count {
                Some(HighlightKind::Group)
            } else {
                Some(HighlightKind::Object)
            }
        } else if token == text {
            classify_bare_word(text, symbols, contextual_keyword)
        } else {
            classify_word(text, symbols, contextual_keyword)
        };
        if let Some(kind) = kind {
            push_span(out, kind, text);
        } else {
            escape_html_into(out, text);
        }
    }
    if let Some(last) = parts.last()
        && last.end < token.len()
    {
        escape_html_into(out, &token[last.end..]);
    }
}

fn qualified_scratch_part_kind(
    index: usize,
    text: &str,
    symbol_kind: Option<HighlightKind>,
    family_axis_names: &HashSet<String>,
) -> Option<HighlightKind> {
    if index == 0 {
        return Some(HighlightKind::Scratch);
    }
    if family_axis_names.contains(text) || symbol_kind == Some(HighlightKind::Group) {
        return Some(HighlightKind::Group);
    }
    if symbol_kind == Some(HighlightKind::Variant) || is_source_scratch_value_atom(text) {
        return Some(HighlightKind::Variant);
    }
    symbol_kind
}

fn schema_selector_head_is_known(
    head: Option<&str>,
    symbols: &HashMap<String, HighlightKind>,
    family_bases: &HashSet<String>,
    family_axes: &HashMap<String, usize>,
) -> bool {
    let Some(head) = head else {
        return false;
    };
    family_bases.contains(head)
        || family_axes.contains_key(head)
        || (head == "*" && (!family_bases.is_empty() || !family_axes.is_empty()))
        || matches!(
            symbols.get(head),
            Some(HighlightKind::Object | HighlightKind::Group)
        )
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

fn semantic_kind_at(ranges: &[SemanticToken], start: usize, end: usize) -> Option<HighlightKind> {
    ranges
        .iter()
        .rev()
        .find(|range| range.start == start && range.end == end)
        .map(|range| highlight_kind_for_semantic(range.kind))
}

fn highlight_kind_for_semantic(kind: SemanticKind) -> HighlightKind {
    match kind {
        SemanticKind::Keyword => HighlightKind::Keyword,
        SemanticKind::Literal => HighlightKind::Literal,
        SemanticKind::Binding => HighlightKind::Binding,
        SemanticKind::Effect => HighlightKind::Effect,
        SemanticKind::Emission => HighlightKind::Emission,
        SemanticKind::Object => HighlightKind::Object,
        SemanticKind::Input => HighlightKind::Input,
        SemanticKind::State => HighlightKind::State,
        SemanticKind::Group => HighlightKind::Group,
        SemanticKind::Variant => HighlightKind::Variant,
        SemanticKind::Condition => HighlightKind::Condition,
        SemanticKind::Scene => HighlightKind::Scene,
        SemanticKind::Theme => HighlightKind::Theme,
        SemanticKind::Asset => HighlightKind::Asset,
        SemanticKind::Setting => HighlightKind::Keyword,
        SemanticKind::Number => HighlightKind::Number,
        SemanticKind::String => HighlightKind::String,
    }
}

#[derive(Clone, Debug)]
struct VisualAsciiColorRange {
    start: usize,
    end: usize,
    color: String,
    transparent: bool,
}

fn visual_ascii_color_range_starting_at(
    ranges: &[VisualAsciiColorRange],
    start: usize,
) -> Option<&VisualAsciiColorRange> {
    ranges.iter().find(|range| range.start == start)
}

#[derive(Clone, Debug)]
struct VisualNamedColorRange {
    start: usize,
    end: usize,
    color: String,
}

fn visual_named_color_range_starting_at(
    ranges: &[VisualNamedColorRange],
    start: usize,
) -> Option<&VisualNamedColorRange> {
    ranges.iter().find(|range| range.start == start)
}

#[derive(Clone, Debug)]
struct LevelAsciiRange {
    start: usize,
    end: usize,
    known: bool,
}

fn level_ascii_range_starting_at(
    ranges: &[LevelAsciiRange],
    start: usize,
) -> Option<&LevelAsciiRange> {
    ranges.iter().find(|range| range.start == start)
}

fn next_raw_embedded_highlight_start(
    raw_start: usize,
    raw_end: usize,
    level_ascii_ranges: &[LevelAsciiRange],
    visual_ascii_color_ranges: &[VisualAsciiColorRange],
    visual_named_color_ranges: &[VisualNamedColorRange],
) -> Option<usize> {
    level_ascii_ranges
        .iter()
        .map(|range| range.start)
        .chain(visual_ascii_color_ranges.iter().map(|range| range.start))
        .chain(visual_named_color_ranges.iter().map(|range| range.start))
        .filter(|start| *start >= raw_start && *start < raw_end)
        .min()
}

#[derive(Clone, Debug)]
struct LevelAsciiScanLevel {
    global_chars: HashSet<char>,
    local_chars: HashSet<char>,
    braced: bool,
    is_2d: bool,
}

#[derive(Clone, Copy, Debug)]
enum LevelLegendTarget {
    Global { enabled: bool },
    Local(usize),
}

fn scan_level_ascii_ranges(context: &crate::source::SourceContext) -> Vec<LevelAsciiRange> {
    let mut ranges = Vec::new();
    let mut global_chars = HashSet::<char>::new();
    let mut levels = Vec::<LevelAsciiScanLevel>::new();
    let mut line_levels = vec![None::<usize>; context.lines.len()];
    let mut current_level = None::<usize>;
    let mut level_legend_stack = Vec::<LevelLegendTarget>::new();
    let mut levels_2d_stack = Vec::<bool>::new();

    for (line_index, line) in context.lines.iter().enumerate() {
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
                global_chars: global_chars.clone(),
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
                global_chars.insert(ch);
            }
        } else if let Some(target) = level_legend_stack.last().copied()
            && let Some(ch) = legend_row_char(&tokens)
        {
            match target {
                LevelLegendTarget::Global { enabled } if enabled => {
                    global_chars.insert(ch);
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
                    LevelLegendTarget::Global {
                        enabled: levels_is_2d,
                    }
                }
            } else {
                LevelLegendTarget::Global {
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

    for (line, level_index) in context.lines.iter().zip(line_levels) {
        let Some(level_index) = level_index else {
            continue;
        };
        let mut known_chars = levels[level_index].global_chars.clone();
        known_chars.extend(levels[level_index].local_chars.iter().copied());
        add_level_ascii_line_ranges(&mut ranges, line, &known_chars);
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
    matches!(tokens, ["level", ..])
        || (scope == Some(SourceScope::Levels) && matches!(tokens, ["{"]))
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
    if !implicit_level_row
        && !matches!(scope, Some(SourceScope::Level | SourceScope::UnbracedLevel))
    {
        return false;
    }
    if matches!(
        tokens,
        ["legend", ..] | ["on_level_start", ..] | ["on_level_clear", ..]
    ) {
        return false;
    }
    !is_level_event_sugar(trimmed, tokens)
}

fn is_level_event_sugar(trimmed: &str, tokens: &[&str]) -> bool {
    matches!(tokens, ["sfx", _] | ["wait"] | ["wait", _])
        || trimmed.strip_prefix("message ").is_some()
}

fn add_level_ascii_line_ranges(
    ranges: &mut Vec<LevelAsciiRange>,
    line: &crate::source::SourceContextLine,
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
            ranges.push(LevelAsciiRange {
                start,
                end,
                known: known_chars.contains(&ch),
            });
        }
        column += ch.len_utf8();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VisualHighlightScope {
    Sprites,
    SpriteEntry,
    Colors,
    ColorTable,
    Other,
}

fn scan_visual_named_color_ranges(
    context: &crate::source::SourceContext,
    aliases: &HashMap<String, String>,
) -> Vec<VisualNamedColorRange> {
    let scopes = visual_highlight_scopes(context);
    let mut ranges = Vec::<VisualNamedColorRange>::new();

    if aliases.is_empty() {
        return ranges;
    }

    for (line, scope) in context.lines.iter().zip(scopes.iter().copied()) {
        if !matches!(
            scope,
            Some(VisualHighlightScope::Sprites | VisualHighlightScope::SpriteEntry)
        ) || is_visual_closing_line(line)
        {
            continue;
        }
        add_visual_named_color_references(scope, &mut ranges, &line.token_spans, aliases);
    }

    ranges
}

fn scan_visual_color_aliases(context: &crate::source::SourceContext) -> HashMap<String, String> {
    let scopes = visual_highlight_scopes(context);
    let mut aliases = HashMap::<String, String>::new();

    for (line, scope) in context.lines.iter().zip(scopes.iter().copied()) {
        if scope != Some(VisualHighlightScope::Colors) || is_visual_closing_line(line) {
            continue;
        }
        let tokens = &line.token_spans;
        let [name, equals, color] = tokens.as_slice() else {
            continue;
        };
        if equals.text != "="
            || name.text.contains(':')
            || !is_source_identifier(&name.text)
            || !highlightable_visual_color_token(&color.text)
        {
            continue;
        }
        aliases.insert(name.text.clone(), color.text.clone());
    }

    aliases
}

fn visual_highlight_scopes(
    context: &crate::source::SourceContext,
) -> Vec<Option<VisualHighlightScope>> {
    let mut scopes = Vec::with_capacity(context.lines.len());
    let mut stack = Vec::<VisualHighlightScope>::new();
    for line in &context.lines {
        let current = stack.last().copied();
        scopes.push(current);
        if is_visual_closing_line(line) {
            stack.pop();
            continue;
        }
        if let Some(opened) = visual_highlight_opening_scope(current, line) {
            stack.push(opened);
        }
    }
    scopes
}

fn visual_highlight_opening_scope(
    current: Option<VisualHighlightScope>,
    line: &crate::source::SourceContextLine,
) -> Option<VisualHighlightScope> {
    let first = line.tokens.first().map(String::as_str)?;
    let has_assignment = line.tokens.iter().any(|token| token == "=");
    match current {
        None if matches!(first, "sprites" | "sprites3") => Some(VisualHighlightScope::Sprites),
        Some(VisualHighlightScope::Sprites) => match first {
            "colors" => Some(VisualHighlightScope::Colors),
            "shapes" => Some(VisualHighlightScope::Other),
            _ if line.content.trim_end().ends_with('{') => Some(VisualHighlightScope::SpriteEntry),
            _ => None,
        },
        Some(VisualHighlightScope::Colors)
            if !has_assignment && first.contains(':') && line.content.trim_end().ends_with('{') =>
        {
            Some(VisualHighlightScope::ColorTable)
        }
        Some(VisualHighlightScope::SpriteEntry | VisualHighlightScope::ColorTable)
            if line.content.trim_end().ends_with('{') =>
        {
            Some(VisualHighlightScope::Other)
        }
        _ => None,
    }
}

fn is_visual_closing_line(line: &crate::source::SourceContextLine) -> bool {
    let trimmed = strip_line_comment(&line.content).trim();
    trimmed == "}"
}

fn add_visual_named_color_references(
    scope: Option<VisualHighlightScope>,
    ranges: &mut Vec<VisualNamedColorRange>,
    tokens: &[crate::source::SourceToken],
    aliases: &HashMap<String, String>,
) {
    let first_color_index = match scope {
        Some(VisualHighlightScope::Sprites | VisualHighlightScope::SpriteEntry) => {
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
            ranges.push(VisualNamedColorRange {
                start: token.start,
                end: token.end,
                color: color.clone(),
            });
        }
    }
}

fn highlightable_visual_color_token(value: &str) -> bool {
    if value.starts_with('#') {
        return hex_color_end(value, 0, '#') == Some(value.len());
    }
    is_visual_color_token(value)
}

fn scan_visual_ascii_color_ranges(
    context: &crate::source::SourceContext,
    aliases: &HashMap<String, String>,
) -> Vec<VisualAsciiColorRange> {
    let mut ranges = Vec::new();

    let mut line_index = 0usize;
    while line_index < context.lines.len() {
        let line = &context.lines[line_index];
        if !visual_sprite_entry_header_line(line, aliases) {
            line_index += 1;
            continue;
        }

        if visual_inline_sprite_entry_line(line, aliases) {
            line_index += 1;
        } else if line.content.trim_end().ends_with('{') {
            line_index = scan_braced_visual_sprite_entry(context, line_index, aliases, &mut ranges);
        } else {
            line_index =
                scan_unbraced_visual_sprite_entry(context, line_index, aliases, &mut ranges);
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
    context: &crate::source::SourceContext,
    start: usize,
    aliases: &HashMap<String, String>,
    ranges: &mut Vec<VisualAsciiColorRange>,
) -> usize {
    let mut scan = VisualSpritePixelScan::default();
    let mut index = start + 1;
    while index < context.lines.len() {
        let line = &context.lines[index];
        if line.scope == Some(SourceScope::Visuals) {
            break;
        }
        scan_visual_sprite_body_line(&mut scan, ranges, line, aliases);
        index += 1;
    }
    index.max(start + 1)
}

fn scan_unbraced_visual_sprite_entry(
    context: &crate::source::SourceContext,
    start: usize,
    aliases: &HashMap<String, String>,
    ranges: &mut Vec<VisualAsciiColorRange>,
) -> usize {
    let mut scan = VisualSpritePixelScan::default();
    let mut index = start + 1;
    while index < context.lines.len() {
        let line = &context.lines[index];
        if line.scope != Some(SourceScope::Visuals) || is_visual_closing_line(line) {
            break;
        }
        if !code_trim(&line.content).is_empty()
            && visual_sprite_entry_boundary(context, index, scan.has_palette(), aliases)
        {
            break;
        }
        scan_visual_sprite_body_line(&mut scan, ranges, line, aliases);
        index += 1;
    }
    index.max(start + 1)
}

fn scan_visual_sprite_body_line(
    scan: &mut VisualSpritePixelScan,
    ranges: &mut Vec<VisualAsciiColorRange>,
    line: &crate::source::SourceContextLine,
    aliases: &HashMap<String, String>,
) {
    let raw = strip_line_comment(&line.content);
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "}" {
        return;
    }
    let tokens = line.tokens.iter().map(String::as_str).collect::<Vec<_>>();
    if let Some(palette) = visual_ascii_palette_for_line(&tokens, aliases) {
        scan.palette = palette;
        return;
    }
    if scan.has_palette() && visual_ascii_row(trimmed, &scan.palette) {
        add_visual_ascii_row_ranges(ranges, line.start, raw, trimmed, &scan.palette);
    }
}

fn visual_sprite_entry_header_line(
    line: &crate::source::SourceContextLine,
    aliases: &HashMap<String, String>,
) -> bool {
    if line.scope != Some(SourceScope::Visuals) || is_visual_closing_line(line) {
        return false;
    }
    let tokens = line.tokens.iter().map(String::as_str).collect::<Vec<_>>();
    match tokens.as_slice() {
        [selector] => visual_sprite_selector_token(selector),
        [selector, source] => {
            visual_sprite_selector_token(selector)
                && (is_visual_image_source(source)
                    || visual_sprite_entry_start_color_token(source, aliases))
        }
        _ => false,
    }
}

fn visual_inline_sprite_entry_line(
    line: &crate::source::SourceContextLine,
    aliases: &HashMap<String, String>,
) -> bool {
    let tokens = line.tokens.iter().map(String::as_str).collect::<Vec<_>>();
    matches!(
        tokens.as_slice(),
        [selector, source]
            if visual_sprite_selector_token(selector)
                && (is_visual_image_source(source)
                    || visual_sprite_entry_start_color_token(source, aliases))
    )
}

fn visual_sprite_entry_boundary(
    context: &crate::source::SourceContext,
    line_index: usize,
    current_has_palette: bool,
    aliases: &HashMap<String, String>,
) -> bool {
    let Some(line) = context.lines.get(line_index) else {
        return false;
    };
    if !current_has_palette {
        return false;
    }
    let tokens = line.tokens.iter().map(String::as_str).collect::<Vec<_>>();
    match tokens.as_slice() {
        ["colors" | "shape" | "shapes", ..] => true,
        [selector, source]
            if visual_sprite_selector_token(selector)
                && (is_visual_image_source(source)
                    || visual_sprite_entry_start_color_token(source, aliases)) =>
        {
            true
        }
        [selector]
            if visual_sprite_selector_token(selector) && line.content.trim_end().ends_with('{') =>
        {
            true
        }
        [selector] if current_has_palette && visual_sprite_selector_token(selector) => context
            .lines
            .iter()
            .skip(line_index + 1)
            .find(|next| {
                next.scope == Some(SourceScope::Visuals) && !code_trim(&next.content).is_empty()
            })
            .is_some_and(|next| visual_line_starts_sprite_source(next, aliases)),
        _ => false,
    }
}

fn visual_line_starts_sprite_source(
    line: &crate::source::SourceContextLine,
    aliases: &HashMap<String, String>,
) -> bool {
    let tokens = line.tokens.iter().map(String::as_str).collect::<Vec<_>>();
    match tokens.as_slice() {
        ["pixels_per_cell" | "offset" | "rotate", ..] => true,
        ["colors", colors @ ..] => {
            !colors.is_empty()
                && colors
                    .iter()
                    .all(|token| visual_color_value_for_token(token, aliases).is_some())
        }
        [source] if is_visual_image_source(source) => true,
        colors if visual_ascii_palette(colors, aliases).is_some() => true,
        _ => false,
    }
}

fn visual_sprite_entry_start_color_token(token: &str, aliases: &HashMap<String, String>) -> bool {
    highlightable_visual_color_token(token) || aliases.contains_key(token) || token.contains(':')
}

fn visual_sprite_selector_token(value: &str) -> bool {
    if matches!(
        value,
        "shape" | "shapes" | "colors" | "ascii" | "sprites" | "sprites3"
    ) {
        return false;
    }
    let cleaned = value.trim_start_matches('@');
    let Some(first) = cleaned.chars().next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && cleaned
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | ':'))
}

fn is_visual_image_source(value: &str) -> bool {
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
        ["colors", colors @ ..] if !colors.is_empty() => visual_ascii_palette(colors, aliases),
        colors => visual_ascii_palette(colors, aliases),
    }
}

fn code_trim(line: &str) -> &str {
    strip_line_comment(line).trim()
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

fn add_visual_ascii_row_ranges(
    ranges: &mut Vec<VisualAsciiColorRange>,
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
            ranges.push(VisualAsciiColorRange {
                start,
                end,
                color: "transparent".to_string(),
                transparent: true,
            });
        } else if let Some(color) = palette.get(&ch).cloned() {
            ranges.push(VisualAsciiColorRange {
                start,
                end,
                color,
                transparent: false,
            });
        }
        column += ch.len_utf8();
    }
}

#[derive(Clone, Debug)]
struct BindingRange {
    start: usize,
    end: usize,
    names: HashSet<String>,
}

fn local_binding_at(ranges: &[BindingRange], start: usize, end: usize, token: &str) -> bool {
    ranges
        .iter()
        .any(|range| start >= range.start && end <= range.end && range.names.contains(token))
}

fn scan_for_binding_ranges(source: &str) -> Vec<BindingRange> {
    let mut ranges = Vec::<BindingRange>::new();
    let mut stack = Vec::<Option<String>>::new();
    let mut offset = 0usize;

    for line in source.split_inclusive('\n') {
        let line_end = offset + line.len();
        let content_end = line_end - usize::from(line.ends_with('\n'));
        let content = &source[offset..content_end];
        let raw = strip_line_comment(content);
        let trimmed = raw.trim();
        let tokens = split_header_tokens(trimmed);

        if trimmed == "}" {
            stack.pop();
        }

        if !trimmed.is_empty() && trimmed != "}" {
            match tokens.as_slice() {
                ["for", binding, "in", _source, ..] => {
                    stack.push(Some((*binding).to_string()));
                }
                [first, ..] if opens_local_scope(first, trimmed) => {
                    stack.push(None);
                }
                _ => {}
            }
        }

        let names = stack
            .iter()
            .filter_map(|binding| binding.as_ref())
            .cloned()
            .collect::<HashSet<_>>();
        if !names.is_empty() {
            ranges.push(BindingRange {
                start: offset,
                end: content_end,
                names,
            });
        }

        offset = line_end;
    }

    ranges
}

fn opens_local_scope(first: &str, trimmed: &str) -> bool {
    trimmed.ends_with('{') || matches!(first, "if" | "once" | "repeat" | "fix")
}

// Parser-owned surface vocabulary. The browser editor consumes highlighted HTML
// from this crate instead of carrying an independent .puzzle grammar table.
fn parser_keyword(token: &str) -> bool {
    token != "level" && is_parser_keyword(token)
}

fn parser_literal(token: &str) -> bool {
    matches!(
        token,
        "all"
            | "and"
            | "any"
            | "ascii"
            | "backward"
            | "bottom"
            | "canonical"
            | "center"
            | "count"
            | "down"
            | "empty"
            | "exists"
            | "false"
            | "forward"
            | "frames"
            | "full"
            | "int"
            | "left"
            | "mirrored"
            | "no"
            | "none"
            | "or"
            | "right"
            | "some"
            | "top"
            | "true"
            | "up"
            | "v"
    )
}

fn parser_effect(token: &str) -> bool {
    scene_effect_command_syntax(token).is_some() || rewrite_effect_command_syntax(token).is_some()
}

fn parser_emission(token: &str) -> bool {
    matches!(
        rewrite_effect_command_syntax(token),
        Some(RewriteEffectCommandSyntax::Emission)
    )
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
                && is_inline_selector_scratch_brace(source, index, ch)
            {
                HighlightKind::Scratch
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

fn is_inline_selector_scratch_brace(source: &str, index: usize, brace: char) -> bool {
    match brace {
        '{' => {
            is_inline_selector_scratch_open(source, index)
                && inline_selector_scratch_close(source, index).is_some()
        }
        '}' => matching_inline_selector_scratch_open(source, index).is_some(),
        _ => false,
    }
}

fn is_inline_selector_scratch_open(source: &str, index: usize) -> bool {
    let before = source[..index].chars().next_back();
    let after = source[index + 1..].chars().next();
    before.is_some_and(is_selector_token_char) && after.is_some_and(|ch| !ch.is_whitespace())
}

fn inline_selector_scratch_close(source: &str, open: usize) -> Option<usize> {
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

fn matching_inline_selector_scratch_open(source: &str, close: usize) -> Option<usize> {
    let line_start = source[..close]
        .rfind('\n')
        .map(|index| index + 1)
        .unwrap_or(0);
    for (offset, ch) in source[line_start..close].char_indices().rev() {
        let index = line_start + offset;
        match ch {
            '{' if is_inline_selector_scratch_open(source, index) => return Some(index),
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
    use super::highlight_source;
    use crate::syntax::{PUZZLE_LIFECYCLE_BLOCKS, PUZZLE_LINE_HEAD_KEYWORDS};

    #[test]
    fn highlights_parser_symbols_from_a_valid_game() {
        let source = r#"
title highlight_symbols

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
level start
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
    fn highlights_braces_by_depth_and_marks_unmatched_braces() {
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
    fn highlights_braces_by_source_order() {
        let highlighted = highlight_source("puzzle board {\nrules { } }\n");

        assert_eq!(highlighted.html.matches("syntax-brace-invalid").count(), 0);
        assert!(highlighted.html.contains(
            "<span class=\"syntax-keyword\">rules</span> <span class=\"syntax-brace-depth-1\">{</span> <span class=\"syntax-brace-depth-1\">}</span> <span class=\"syntax-brace-depth-0\">}</span>"
        ));
    }

    #[test]
    fn highlights_all_puzzle_lifecycle_blocks_from_shared_syntax() {
        let source = r#"
title lifecycle_highlight

puzzle board {
layers {
actor = Player
}
rules {
once [ Player ] -> [ Player ]
}
on_level_start {
}
on_level_clear {
}
on_last_level_clear {
}
}

levels {
legend {
. = empty
P = Player
}
level start
P
}
"#;
        let highlighted = highlight_source(source);

        assert!(highlighted.parsed);
        for keyword in PUZZLE_LIFECYCLE_BLOCKS {
            assert!(
                highlighted
                    .html
                    .contains(&format!("syntax-keyword\">{keyword}")),
                "missing lifecycle highlight {keyword}"
            );
        }
    }

    #[test]
    fn highlights_all_puzzle_line_head_keywords_from_shared_syntax() {
        for keyword in PUZZLE_LINE_HEAD_KEYWORDS {
            let source = format!("title highlight_{keyword}\npuzzle board {{\n{keyword}\n}}\n");
            let highlighted = highlight_source(&source);

            assert!(
                highlighted
                    .html
                    .contains(&format!("syntax-keyword\">{keyword}")),
                "missing puzzle line-head highlight {keyword}"
            );
        }
    }

    #[test]
    fn highlights_model_top_level_keywords_from_parser_surface() {
        for keyword in crate::model_top_level_completion_keywords() {
            let source = format!("{keyword}\n");
            let highlighted = highlight_source(&source);

            assert!(
                highlighted
                    .html
                    .contains(&format!("syntax-keyword\">{keyword}")),
                "missing parser-surface top-level highlight {keyword}"
            );
        }
    }

    #[test]
    fn highlights_valid_animation_block_header() {
        let source = r#"
title animation_highlight

animation {
tween {
duration 90ms
}
}

puzzle board {
layers {
actor = Player
}
rules {
}
levels {
legend {
. = empty
P = Player
}
level first
P
}
}
"#;
        let highlighted = highlight_source(source);

        assert!(highlighted.parsed);
        assert!(highlighted.html.contains("syntax-keyword\">animation"));
        assert!(highlighted.html.contains("syntax-keyword\">tween"));
        assert!(highlighted.html.contains("syntax-keyword\">duration"));
    }

    #[test]
    fn highlights_parser_surface_keywords_after_parse_error() {
        let source = r#"
title animation_highlight_parse_error

animation {
tween {
duration 90ms
}
}

puzzle board {
layers {
actor = Player
}
rules {
}
thing Player 1
levels {
legend {
. = empty
P = Player
}
level first
P
}
}
"#;
        let highlighted = highlight_source(source);

        assert!(!highlighted.parsed);
        assert!(highlighted.html.contains("syntax-keyword\">animation"));
        assert!(highlighted.html.contains("syntax-keyword\">tween"));
        assert!(highlighted.html.contains("syntax-keyword\">duration"));
    }

    #[test]
    fn highlights_puzzle_scoped_animation_from_surface_stack_after_parse_error() {
        let source = r#"
title puzzle_animation_highlight_parse_error

puzzle board {
layers {
actor = Player
}
animation {
tween {
duration 90ms
}
}
thing Player 1
rules {
}
levels {
legend {
. = empty
P = Player
}
level first
P
}
}
"#;
        let highlighted = highlight_source(source);

        assert!(!highlighted.parsed);
        assert!(highlighted.html.contains("syntax-keyword\">animation"));
        assert!(highlighted.html.contains("syntax-keyword\">tween"));
        assert!(highlighted.html.contains("syntax-keyword\">duration"));
    }

    #[test]
    fn highlights_lifecycle_after_inline_else_brace_continuation() {
        let source = r#"
title lifecycle_after_else

puzzle board {
tags {
gate_no = 1...5
}
scratch {
checked
}
var locked_room_count = 3
layers {
gate = Gate:gate_no
}
empty .
legend 1 = Gate:1
rules {
for n in 1...5 {
if some([ Gate:n{checked} ]) {
if locked_room_count > n {
locked_room_count -= n
[ Gate:n{checked} ] -> [  ]
} else {
[ Gate:n{checked} ] -> [ Gate:n ]
}
}
}
}
on_level_start {
}
level start {
1
}
}
"#;
        let highlighted = highlight_source(source);

        assert!(highlighted.html.contains(
            "<span class=\"syntax-keyword\">on_level_start</span> <span class=\"syntax-brace-depth-1\">{</span>"
        ));
        assert!(highlighted.html.contains(
            "<span class=\"syntax-keyword\">level</span> <span class=\"syntax-scene\">start</span>"
        ));
        assert_eq!(highlighted.html.matches("syntax-brace-invalid").count(), 0);
    }

    #[test]
    fn highlights_3d_render_option_names() {
        let highlighted = highlight_source(
            r#"
title highlight_3d_render_options
puzzle3 board {
layers {
actor
}
layers {
__legacy_layer_0 = Player actor
}
render {
camera {
yaw = 15
interactive_look
}
}
rules {
}
}
"#,
        );

        assert!(highlighted.html.contains("syntax-keyword\">camera"));
        assert!(highlighted.html.contains("syntax-keyword\">yaw"));
        assert!(
            highlighted
                .html
                .contains("syntax-keyword\">interactive_look")
        );
    }

    #[test]
    fn highlights_contextual_option_names() {
        let highlighted = highlight_source(
            r#"
title highlight_contextual_options
sounds {
sfx click seed=click01
music bgm height=0.7
}
animation {
tween {
duration 90ms
}
}
scene menu {
layout {
level_menu {
show_index = true
}
}
}
theme clean {
background_color = #123456
accent_color #abcdef
}
"#,
        );

        assert!(highlighted.html.contains("syntax-keyword\">seed"));
        assert!(highlighted.html.contains("syntax-keyword\">height"));
        assert!(highlighted.html.contains("syntax-keyword\">duration"));
        assert!(highlighted.html.contains("syntax-keyword\">show_index"));
        assert!(
            highlighted
                .html
                .contains("syntax-keyword\">background_color")
        );
        assert!(highlighted.html.contains("syntax-keyword\">accent_color"));
    }

    #[test]
    fn removed_global_syntax_is_not_highlighted_as_keyword_or_state() {
        let highlighted = highlight_source(
            r#"
title no_global_highlight
puzzle board {
global moved = false
}
"#,
        );

        assert!(!highlighted.html.contains("syntax-keyword\">global"));
        assert!(!highlighted.html.contains("syntax-state\">moved"));
    }

    #[test]
    fn highlights_scene_step_rule_directive() {
        let highlighted = highlight_source(
            r#"
title scene_step_highlight

scene playing {
rules {
step board
}
}
"#,
        );

        assert!(highlighted.html.contains("syntax-keyword\">step"));
        assert!(highlighted.html.contains("syntax-state\">board"));
    }

    #[test]
    fn highlights_arrow_tokens() {
        let highlighted = highlight_source(
            r#"
title arrow_highlight

puzzle board {
layers {
__legacy_layer_0 = Player Box
}
rules {
[ Player ]->[ Box ]
}
level start {
.
}
}
"#,
        );

        assert!(highlighted.html.contains(
            "</span><span class=\"syntax-arrow\">-&gt;</span><span class=\"syntax-operator\">["
        ));
    }

    #[test]
    fn highlights_declared_authoring_elements() {
        let highlighted = highlight_source(
            r#"
title colored_elements

puzzle board {
tags {
color = red blue
}
layers {
__legacy_layer_0 = Player Box:color
}
groups {
pushable = Player Box:red
}
layers {
@__legacy_layer_0 = @Cursor @Aura:color
}
group active = Player Box:blue
var moves = 0
persistent var best = 0
scratch {
mark
shade = color
steps = int
}
legend P = Player
legend B = Box:red
main {
once [ Player{mark} | Box:red ] -> [ @Cursor | Box:blue{shade:blue} ]
}
level start {
PB
}
}
"#,
        );

        assert!(highlighted.html.contains("syntax-object\">Player"));
        assert!(highlighted.html.contains("syntax-object\">@Cursor"));
        assert!(highlighted.html.contains("syntax-group\">pushable"));
        assert!(highlighted.html.contains("syntax-group\">active"));
        assert!(highlighted.html.contains("syntax-state\">moves"));
        assert!(highlighted.html.contains("syntax-state\">best"));
        assert!(highlighted.html.contains("syntax-scratch\">mark"));
        assert!(highlighted.html.contains("syntax-scratch\">shade"));
        assert!(highlighted.html.contains("syntax-group\">color"));
        assert!(highlighted.html.contains("syntax-object\">red"));
    }

    #[test]
    fn highlights_tag_definition_values_as_object_name_atoms() {
        let highlighted = highlight_source(
            r#"
title tag_definition_highlight

puzzle board {
tags {
facing = left right
}
}
"#,
        );

        assert!(highlighted.html.contains("syntax-group\">facing"));
        assert!(highlighted.html.contains("syntax-object\">left"));
        assert!(highlighted.html.contains("syntax-object\">right"));
    }

    #[test]
    fn highlights_keyword_named_tag_axis_in_schema_selectors() {
        let highlighted = highlight_source(
            r#"
title keyword_axis_highlight

puzzle board {
tags {
state = stack movable
}
layers {
actor = Box:state
}
rules {
[ Box:state | Box:stack | Box:movable ] -> [ Box:movable ]
}
level start {
.
}
}
"#,
        );
        assert!(highlighted.html.contains(
            "syntax-object\">Box</span><span class=\"syntax-operator\">:</span><span class=\"syntax-group\">state"
        ));
        assert!(highlighted.html.contains(
            "syntax-object\">Box</span><span class=\"syntax-operator\">:</span><span class=\"syntax-object\">stack"
        ));
        assert!(highlighted.html.contains(
            "syntax-object\">Box</span><span class=\"syntax-operator\">:</span><span class=\"syntax-object\">movable"
        ));
    }

    #[test]
    fn family_wildcard_schema_selectors_highlight_by_selector_grammar() {
        let highlighted = highlight_source(
            r#"
title family_wildcard_highlight

puzzle board {
tags {
state = stack movable
}
layers {
actor = Crate:state
}
groups {
movable = Crate:movable
}
rules {
[ *:stack ] -> [ *:movable ]
}
level start {
.
}
}
"#,
        );

        assert!(highlighted.html.contains(
            "syntax-group\">*</span><span class=\"syntax-operator\">:</span><span class=\"syntax-object\">stack"
        ));
        assert!(highlighted.html.contains(
            "syntax-group\">*</span><span class=\"syntax-operator\">:</span><span class=\"syntax-object\">movable"
        ));
    }

    #[test]
    fn highlights_state_backed_schema_selector_slots_as_state_references() {
        let highlighted = highlight_source(
            r#"
title dynamic_selector_highlight

puzzle board {
var count = 2
tags {
num = 1 2 3
}
layers {
actor = Box:num
}
empty .
rules {
[ Box:count ] -> [ Box:count ]
}
level start {
.
}
}
"#,
        );

        assert!(highlighted.html.contains(
            "syntax-object\">Box</span><span class=\"syntax-operator\">:</span><span class=\"syntax-group\">num"
        ));
        assert!(highlighted.html.contains(
            "syntax-object\">Box</span><span class=\"syntax-operator\">:</span><span class=\"syntax-state\">count"
        ));
    }

    #[test]
    fn highlights_declared_elements_even_when_source_does_not_parse() {
        let highlighted = highlight_source(
            r#"
title fallback_elements

puzzle board {
tags {
kind = A B
}
input jump
condition blocked = no Player
layers {
__legacy_layer_0 = Player Box:kind
}
groups {
pushable = Box:A
}
keys {
Space = jump
}
rules {
[ Box:A | blocked ] -> [ Box:B | Player ]
}
}

sounds {
sfx bump seed=hit type=jump
}

scene title {
button "Play" -> goto playing
button "Continue" -> goto playing
rules {
input start -> goto playing
}
}
scene playing {
}
"#,
        );

        assert!(!highlighted.parsed);
        assert!(highlighted.html.contains("syntax-object\">Player"));
        assert!(highlighted.html.contains("syntax-object\">Box"));
        assert!(highlighted.html.contains("syntax-group\">pushable"));
        assert!(highlighted.html.contains("syntax-object\">A"));
        assert!(highlighted.html.contains("syntax-object\">B"));
        assert!(highlighted.html.contains("syntax-input\">jump"));
        assert!(highlighted.html.contains("syntax-condition\">blocked"));
        assert!(highlighted.html.contains("syntax-asset\">bump"));
        assert!(highlighted.html.contains("syntax-input\">start"));
        assert!(highlighted.html.contains("syntax-effect\">goto"));
        assert!(highlighted.html.contains("syntax-scene\">playing"));
    }

    #[test]
    fn highlights_selector_occurrence_labels() {
        let highlighted = highlight_source(
            r#"
title occurrence_label_highlight

puzzle copy {
layers {
actor = Box Crate
}
groups {
solid = Box Crate
}
rules {
once [ solid#1 | solid#2 ] -> [ solid#2 | solid#1 ]
}
}
"#,
        );

        assert!(highlighted.html.contains(
            "syntax-group\">solid</span><span class=\"syntax-binding\">#</span><span class=\"syntax-binding\">1"
        ));
        assert!(highlighted.html.contains(
            "syntax-group\">solid</span><span class=\"syntax-binding\">#</span><span class=\"syntax-binding\">2"
        ));
    }

    #[test]
    fn highlights_for_bindings_as_local_bindings() {
        let highlighted = highlight_source(
            r#"
title binding_highlight

puzzle board {
tags {
kind = A B
}
layers {
__legacy_layer_0 = Box:kind Box:P Box:L Box:A Box:Y Box:E Box:R
}
legend P = Box:P
legend l = Box:L
legend a = Box:A
legend y = Box:Y
legend e = Box:E
legend r = Box:R
rules {
for k in kind {
[ Box:k | Player ] -> [ Player | Box:k ]
}
}
level start {
.
}
}
"#,
        );

        assert!(highlighted.html.contains("syntax-binding\">k"));
        assert!(
            highlighted
                .html
                .contains("syntax-object\">Box</span><span class=\"syntax-operator\">:</span><span class=\"syntax-binding\">k")
        );
        assert!(highlighted.html.contains("syntax-group\">kind"));
    }

    #[test]
    fn highlights_object_family_axis_names_separately_from_values() {
        let highlighted = highlight_source(
            r#"
title family_axis_highlight

puzzle board {
tags {
kind = A B
}
layers {
__legacy_layer_0 = Target:kind
}
rules {
[ Target:kind | Target:A | Target ] -> [ Target:B | Target ]
}
level start {
.
}
}
"#,
        );

        assert!(highlighted.html.contains(
            "syntax-object\">Target</span><span class=\"syntax-operator\">:</span><span class=\"syntax-group\">kind"
        ));
        assert!(highlighted.html.contains(
            "syntax-object\">Target</span><span class=\"syntax-operator\">:</span><span class=\"syntax-object\">A"
        ));
        assert!(highlighted.html.contains(
            "syntax-object\">Target</span><span class=\"syntax-operator\">:</span><span class=\"syntax-object\">B"
        ));
        assert_eq!(
            highlighted
                .html
                .matches("<span class=\"syntax-object\">Target</span>")
                .count(),
            6
        );
        assert_eq!(
            highlighted
                .html
                .matches("<span class=\"syntax-group\">Target</span>")
                .count(),
            0
        );
    }

    #[test]
    fn highlights_value_set_axes_in_layer_selectors_as_group_like_names() {
        let highlighted = highlight_source(
            r#"
title layer_axis_highlight

puzzle board {
tags {
kind = A B
}
layers {
actor = Box:kind Wall
}
rules {
[ Box:kind | Box:A ] -> [ Box:B | Box:kind ]
}
level start {
.
}
}
"#,
        );

        assert!(highlighted.html.contains(
            "syntax-object\">Box</span><span class=\"syntax-operator\">:</span><span class=\"syntax-group\">kind"
        ));
        assert!(highlighted.html.contains(
            "syntax-object\">Box</span><span class=\"syntax-operator\">:</span><span class=\"syntax-object\">A"
        ));
    }

    #[test]
    fn highlights_builtin_direction_axes_as_group_like_names() {
        let highlighted = highlight_source(
            r#"
title direction_axis_highlight

puzzle board {
layers {
__legacy_layer_0 = Facing:directions
}
rules {
for dir in directions {
[ Facing:directions | Facing:up ] -> [ Facing:dir | Facing:down ]
}
}
level start {
.
}
}
"#,
        );

        assert!(highlighted.html.contains(
            "syntax-object\">Facing</span><span class=\"syntax-operator\">:</span><span class=\"syntax-group\">directions"
        ));
        assert!(highlighted.html.contains(
            "syntax-binding\">dir</span> <span class=\"syntax-keyword\">in</span> <span class=\"syntax-group\">directions"
        ));
    }

    #[test]
    fn highlights_direction_glyphs_as_literals() {
        let highlighted = highlight_source(
            r#"
title glyph_highlight

puzzle board {
layers {
actor = Player
}
rules {
[ > Player ] -> [ Player ]
[ < Player ] -> [ Player ]
[ ^ Player ] -> [ Player ]
[ v Player ] -> [ Player ]
[ Player{>} ] -> [ Player{<} ]
[ Player{^} ] -> [ Player{v} ]
}
level start {
.
}
}
"#,
        );

        assert!(highlighted.html.contains("syntax-literal\">&gt;"));
        assert!(highlighted.html.contains("syntax-literal\">&lt;"));
        assert!(highlighted.html.contains("syntax-literal\">^"));
        assert!(highlighted.html.contains("syntax-literal\">v"));
        assert!(
            highlighted.html.contains("syntax-object\">Player</span><span class=\"syntax-scratch\">{</span><span class=\"syntax-literal\">&gt;</span><span class=\"syntax-scratch\">}")
        );
        assert!(
            highlighted.html.contains("syntax-object\">Player</span><span class=\"syntax-scratch\">{</span><span class=\"syntax-literal\">&lt;</span><span class=\"syntax-scratch\">}")
        );
        assert!(
            !highlighted
                .html
                .contains("-<span class=\"syntax-literal\">&gt;")
        );
        assert!(
            !highlighted
                .html
                .contains("syntax-arrow\">-<span class=\"syntax-literal\">&gt;")
        );
    }

    #[test]
    fn highlights_layer_tags_and_layer_selectors() {
        let highlighted = highlight_source(
            r#"
title layer_highlight

puzzle board {
layers {
floor = Goal
solid = Player Box Wall
}
groups {
blocked = solid
}
legend P = Player
main {
once input [ Player | no blocked ] -> [ | Player ]
}
level start {
P
}
}
"#,
        );

        assert!(highlighted.html.contains("syntax-group\">floor"));
        assert!(highlighted.html.contains("syntax-group\">solid"));
        assert!(highlighted.html.contains("syntax-group\">blocked"));
        assert!(highlighted.html.contains("syntax-object\">Goal"));
        assert!(highlighted.html.contains("syntax-object\">Player"));
    }

    #[test]
    fn highlights_anonymous_layer_entries_as_objects() {
        let highlighted = highlight_source(
            r#"
title anonymous_layer_highlight

puzzle board {
layers {
Floor
Goal
solid = Player Box Wall
}
}
"#,
        );

        assert!(highlighted.html.contains("syntax-object\">Floor"));
        assert!(highlighted.html.contains("syntax-object\">Goal"));
        assert!(
            highlighted
                .html
                .contains("syntax-group\">solid</span> <span class=\"syntax-operator\">=</span>")
        );
        assert!(highlighted.html.contains("syntax-object\">Player"));
    }

    #[test]
    fn highlights_solid_layer_name_as_group_not_object() {
        let highlighted = highlight_source(
            r#"
title solid_layer_highlight

puzzle board {
layers {
solid = Player Box Wall
}
groups {
blocked = solid
}
rules {
once [ Player | no solid ] -> [ | Player ]
}
level start {
.
}
}
"#,
        );
        assert!(
            highlighted
                .html
                .contains("syntax-group\">solid</span> <span class=\"syntax-operator\">=</span>")
        );
        assert!(highlighted.html.contains(
            "syntax-group\">blocked</span> <span class=\"syntax-operator\">=</span> <span class=\"syntax-group\">solid"
        ));
        assert!(
            highlighted
                .html
                .contains("syntax-literal\">no</span> <span class=\"syntax-group\">solid")
        );
        assert!(!highlighted.html.contains("syntax-object\">solid"));
    }

    #[test]
    fn highlights_hex_colors_with_their_own_color() {
        let highlighted = highlight_source(
            r#"
sprites {
colors {
piece_color:kind {
A = #4a4
B = #ff004d80
}
}
Floor
#111 #222
01.
Wall
#444
}
level accidental {
#.#BBb#
}
"#,
        );
        assert!(
            highlighted
                .html
                .contains("syntax-color\" style=\"--syntax-color-token: #4a4\">#4a4")
        );
        assert!(
            highlighted
                .html
                .contains("syntax-color\" style=\"--syntax-color-token: #ff004d80\">#ff004d80")
        );
        assert!(
            highlighted
                .html
                .contains("syntax-color\" style=\"--syntax-color-token: #444\">#444")
        );
        assert!(highlighted.html.contains("syntax-keyword\">sprites</span>"));
        assert!(
            highlighted
                .html
                .contains("syntax-sprite-pixel\" style=\"--syntax-sprite-pixel-color: #111\">0</span><span class=\"syntax-sprite-pixel\" style=\"--syntax-sprite-pixel-color: #222\">1</span><span class=\"syntax-sprite-pixel is-transparent\" style=\"--syntax-sprite-pixel-color: transparent\">.</span>")
        );
        assert!(
            !highlighted
                .html
                .contains("style=\"--syntax-color-token: #.#")
        );
    }

    #[test]
    fn highlights_ascii_pixels_for_sprite_named_like_color() {
        let highlighted = highlight_source(
            r#"
sprites {
red
#f00
0
}
"#,
        );

        assert!(highlighted.html.contains(
            "syntax-sprite-pixel\" style=\"--syntax-sprite-pixel-color: #f00\">0</span>"
        ));
    }

    #[test]
    fn highlights_color_name_rows_as_sprite_colors() {
        let highlighted = highlight_source(
            r#"
sprites {
Player
red blue
01
}
"#,
        );

        assert!(
            highlighted
                .html
                .contains("syntax-sprite-pixel\" style=\"--syntax-sprite-pixel-color: red\">0</span><span class=\"syntax-sprite-pixel\" style=\"--syntax-sprite-pixel-color: blue\">1</span>")
        );
    }

    #[test]
    fn highlights_sprite_pixels_with_attached_braces_and_tabs() {
        let source = "title attached_sprite_braces\n\npuzzle default {\nlayers {\n__legacy_layer_0 = Player\n}\nsprites{\n\tPlayer{\n\t\t#ff0000 #0000ff\n\t\t01.\n\t}\n}\nrules {\n}\n}\nlevels {\nlegend {\n. = empty\nP = Player\n}\nlevel start\nP\n}\n";
        let highlighted = highlight_source(source);

        assert!(highlighted.parsed);
        assert!(
            highlighted
                .html
                .contains("syntax-sprite-pixel\" style=\"--syntax-sprite-pixel-color: #ff0000\">0</span><span class=\"syntax-sprite-pixel\" style=\"--syntax-sprite-pixel-color: #0000ff\">1</span><span class=\"syntax-sprite-pixel is-transparent\" style=\"--syntax-sprite-pixel-color: transparent\">.</span>")
        );
    }

    #[test]
    fn highlights_canonical_sprite_pixels_after_metadata_rows() {
        let highlighted = highlight_source(
            r##"
title canonical_sprite_highlight

puzzle default {
layers {
__legacy_layer_0 = Player Box
}
sprites {
Player {
pixels_per_cell 5 5
offset 2 -1
#e94f64 #2f80ed
01
}
Box {
colors #111111 #222222
01
}
}
rules {
}
}
levels {
legend {
. = empty
P = Player
B = Box
}
level start
P
}
"##,
        );

        assert!(highlighted.parsed);
        assert!(
            highlighted
                .html
                .contains("syntax-sprite-pixel\" style=\"--syntax-sprite-pixel-color: #e94f64\">0</span><span class=\"syntax-sprite-pixel\" style=\"--syntax-sprite-pixel-color: #2f80ed\">1</span>")
        );
        assert!(
            highlighted
                .html
                .contains("syntax-sprite-pixel\" style=\"--syntax-sprite-pixel-color: #111111\">0</span><span class=\"syntax-sprite-pixel\" style=\"--syntax-sprite-pixel-color: #222222\">1</span>")
        );
    }

    #[test]
    fn highlights_sprites3_pixels_across_blank_slice_separators() {
        let highlighted = highlight_source(
            r#"
sprites3 {
Player
#000000 #ffa500 #ffffff #0000ff
.....
.....
.000.
.....
.....

.....
.....
.111.
.....
.....

.....
.....
22222
.....
.....

.....
.....
.333.
.....
.....

.....
.....
.3.3.
.....
.....
}
"#,
        );
        assert!(
            highlighted
                .html
                .contains("syntax-sprite-pixel\" style=\"--syntax-sprite-pixel-color: #000000\">0</span><span class=\"syntax-sprite-pixel\" style=\"--syntax-sprite-pixel-color: #000000\">0</span><span class=\"syntax-sprite-pixel\" style=\"--syntax-sprite-pixel-color: #000000\">0</span>")
        );
        assert!(
            highlighted
                .html
                .contains("syntax-sprite-pixel\" style=\"--syntax-sprite-pixel-color: #ffa500\">1</span><span class=\"syntax-sprite-pixel\" style=\"--syntax-sprite-pixel-color: #ffa500\">1</span><span class=\"syntax-sprite-pixel\" style=\"--syntax-sprite-pixel-color: #ffa500\">1</span>")
        );
        assert!(
            highlighted
                .html
                .contains("syntax-sprite-pixel\" style=\"--syntax-sprite-pixel-color: #ffffff\">2</span><span class=\"syntax-sprite-pixel\" style=\"--syntax-sprite-pixel-color: #ffffff\">2</span><span class=\"syntax-sprite-pixel\" style=\"--syntax-sprite-pixel-color: #ffffff\">2</span><span class=\"syntax-sprite-pixel\" style=\"--syntax-sprite-pixel-color: #ffffff\">2</span><span class=\"syntax-sprite-pixel\" style=\"--syntax-sprite-pixel-color: #ffffff\">2</span>")
        );
        assert!(
            highlighted
                .html
                .contains("syntax-sprite-pixel\" style=\"--syntax-sprite-pixel-color: #0000ff\">3</span><span class=\"syntax-sprite-pixel is-transparent\" style=\"--syntax-sprite-pixel-color: transparent\">.</span><span class=\"syntax-sprite-pixel\" style=\"--syntax-sprite-pixel-color: #0000ff\">3</span>")
        );
    }

    #[test]
    fn highlights_registered_sprite_color_names_with_swatches() {
        let highlighted = highlight_source(
            r#"
sprites {
colors {
light_green = #90ee90
dark_green = #008000
piece_color:kind {
A = #ff004d
B = #29adff
}
}
Floor {
light_green dark_green
01.
}
}
"#,
        );

        assert!(
            highlighted
                .html
                .contains("syntax-color\" style=\"--syntax-color-token: #90ee90\">#90ee90")
        );
        assert!(
            highlighted
                .html
                .contains("syntax-color\" style=\"--syntax-color-token: #008000\">#008000")
        );
        assert!(
            highlighted
                .html
                .contains("style=\"--syntax-color-token: #90ee90\">light_green</span> <span class=\"syntax-color\" style=\"--syntax-color-token: #008000\">dark_green")
        );
        assert!(
            highlighted
                .html
                .contains("style=\"--syntax-color-token: #ff004d\">#ff004d")
        );
        assert!(
            highlighted
                .html
                .contains("syntax-sprite-pixel\" style=\"--syntax-sprite-pixel-color: #90ee90\">0</span><span class=\"syntax-sprite-pixel\" style=\"--syntax-sprite-pixel-color: #008000\">1</span><span class=\"syntax-sprite-pixel is-transparent\" style=\"--syntax-sprite-pixel-color: transparent\">.</span>")
        );
        assert!(
            !highlighted
                .html
                .contains("style=\"--syntax-color-token: #90ee90\">light_green</span> <span class=\"syntax-operator\">=")
        );
        assert!(
            !highlighted
                .html
                .contains("style=\"--syntax-color-token: #ff004d\">A")
        );
    }

    #[test]
    fn highlights_color_alias_rows_after_shape_tables() {
        let highlighted = highlight_source(
            r##"
title locked_style_color_alias_highlight

puzzle default {
tags {
num = 1 2
}
layers {
__legacy_layer_0 = Gate:num GoalCount:num
}
legend 1 = Gate:1
legend {
. = empty
}
sprites {
shapes {
gate_shape
010
111
010
}

colors {
Gate_color_1 = #921e87
Gate_color_2 = #c2c3c7
GoalCount = #acacac
}

Gate:num
Gate_color_1 Gate_color_2
01
shape gate_shape

GoalCount:1
GoalCount
0
}
rules {
}
levels {
level start
1
}
}
"##,
        );

        assert!(highlighted.html.contains(
            "style=\"--syntax-color-token: #921e87\">Gate_color_1</span> <span class=\"syntax-color\" style=\"--syntax-color-token: #c2c3c7\">Gate_color_2"
        ));
        assert!(highlighted.html.contains(
            "syntax-sprite-pixel\" style=\"--syntax-sprite-pixel-color: #921e87\">0</span><span class=\"syntax-sprite-pixel\" style=\"--syntax-sprite-pixel-color: #c2c3c7\">1</span>"
        ));
        assert!(highlighted.html.contains(
            "style=\"--syntax-color-token: #acacac\">GoalCount</span>\n<span class=\"syntax-sprite-pixel\" style=\"--syntax-sprite-pixel-color: #acacac\">0</span>"
        ));
        assert!(highlighted.html.contains(
            "syntax-object\">GoalCount</span><span class=\"syntax-operator\">:</span><span class=\"syntax-object\">1</span>"
        ));
    }

    #[test]
    fn highlights_scene_key_bindings() {
        let highlighted = highlight_source(
            r#"
title key_highlight

scene pause {
keys {
Escape Enter Space -> resume
q -> quit
}
rules {
input quit -> goto title
input resume -> goto playing
}
}
"#,
        );

        assert!(highlighted.html.contains("syntax-input\">Escape"));
        assert!(highlighted.html.contains("syntax-input\">Enter"));
        assert!(highlighted.html.contains("syntax-input\">Space"));
        assert!(highlighted.html.contains("syntax-input\">resume"));
        assert!(highlighted.html.contains("syntax-input\">quit"));
        assert!(!highlighted.html.contains("syntax-effect\">quit"));
    }

    #[test]
    fn highlights_rewrite_direction_prefixes_as_keywords() {
        let highlighted = highlight_source(
            r#"
title rewrite_direction_highlight

puzzle board {
layers {
actor = Player
}
rules {
right [ Player | ] -> [ | Player ]
once left [ Player | ] -> [ | Player ]
fix once up {
[ Player ] -> [ Player ]
}
}
level start {
.
}
}
"#,
        );

        assert!(highlighted.html.contains("syntax-keyword\">right"));
        assert!(highlighted.html.contains("syntax-keyword\">left"));
        assert!(highlighted.html.contains("syntax-keyword\">up"));
        assert!(!highlighted.html.contains("syntax-input\">right"));
        assert!(!highlighted.html.contains("syntax-input\">left"));
    }

    #[test]
    fn highlights_scene_headers_and_effect_targets() {
        let highlighted = highlight_source(
            r#"
title scene_highlight

puzzle board {
layers {
__legacy_layer_0 = Player
}
legend {
. = empty
}
rules {
[ Player ] -> [ Player ]
}
level start {
.
}
}

scene title {
layout {
title
subtitle
row {
column {
box {
text "Start"
}
}
}
}
button "Start" -> playing.goto first
button "Menu" -> goto menu
button "Restart" -> start playing
rules {
input start -> playing.goto first
}
}

scene playing {
}

scene menu {
}
"#,
        );
        assert!(highlighted.html.contains("syntax-scene\">title"));
        assert!(highlighted.html.contains("syntax-keyword\">layout"));
        assert!(
            highlighted
                .html
                .contains("syntax-keyword\">title</span>\n<span class=\"syntax-keyword\">subtitle")
        );
        assert!(highlighted.html.contains("syntax-keyword\">row"));
        assert!(highlighted.html.contains("syntax-keyword\">column"));
        assert!(highlighted.html.contains("syntax-keyword\">box"));
        assert!(
            highlighted
                .html
                .contains("syntax-scene\">playing</span><span class=\"syntax-operator\">.</span><span class=\"syntax-effect\">goto")
        );
        assert!(
            highlighted
                .html
                .contains("syntax-effect\">goto</span> <span class=\"syntax-scene\">menu")
        );
        assert!(
            highlighted
                .html
                .contains("syntax-effect\">start</span> <span class=\"syntax-scene\">playing")
        );
        assert!(highlighted.html.contains("syntax-input\">start"));
    }

    #[test]
    fn highlights_keyword_named_scene_state_slot_as_state_reference() {
        let highlighted = highlight_source(
            r#"
title state_slot_highlight

puzzle board {
layers {
__legacy_layer_0 = Player
}
rules {
[ Player ] -> [ Player ]
}
levels {
legend {
. = empty
}
level start {
.
}
}
}

scene playing {
state {
state = puzzle board
}
layout {
puzzle state
text state.level.label
}
button "Restart" -> state.restart
}
"#,
        );

        assert!(
            highlighted.parsed,
            "test fixture should parse before checking contextual highlight"
        );
        assert!(highlighted.html.contains(
            "<span class=\"syntax-keyword\">state</span> <span class=\"syntax-brace-depth-1\">{</span>"
        ));
        assert!(highlighted.html.contains(
            "<span class=\"syntax-state\">state</span> <span class=\"syntax-operator\">=</span> <span class=\"syntax-keyword\">puzzle"
        ));
        assert!(highlighted.html.contains(
            "<span class=\"syntax-keyword\">puzzle</span> <span class=\"syntax-state\">state"
        ));
        assert!(highlighted.html.contains(
            "syntax-state\">state</span><span class=\"syntax-operator\">.</span><span class=\"syntax-effect\">restart"
        ));
    }

    #[test]
    fn highlights_canonical_3d_model_and_shared_scene_layout() {
        let source = r#"
title highlight_3d

puzzle3 push3d {
layers {
floor = Goal
solid = Player Box Wall
}
render {
camera {
yaw 34
pitch 38
zoom 1.1
interactive_look true
interactive_zoom true
}
grid {
occupied_cells true
}
shade false
}
rules {
forward [ Player | Box | no solid ] -> [ | Player | Box ]
forward [ Player | no solid ] -> [ | Player ]
}
}

levels3 basic of push3d {
legend {
. = empty
P = Player
B = Box
# = Wall
G = Goal
}

level push3d_01 {
.....
.P.B.
.....

.....
.....
..G..
}
}

scene playing3d {
state {
board = puzzle3 push3d
}
layout size 4 3 {
column gap 1 align center top {
puzzle3 board
row gap 1 {
button "Restart" -> board.restart
button "Levels" -> goto level_select
}
}
}
}
"#;
        let highlighted = highlight_source(source);

        assert!(highlighted.html.contains("syntax-keyword\">puzzle3"));
        assert!(highlighted.html.contains("syntax-keyword\">render"));
        assert!(highlighted.html.contains("syntax-keyword\">camera"));
        assert!(highlighted.html.contains("syntax-keyword\">yaw"));
        assert!(
            highlighted
                .html
                .contains("syntax-keyword\">interactive_look")
        );
        assert!(highlighted.html.contains("syntax-keyword\">grid"));
        assert!(highlighted.html.contains("syntax-keyword\">occupied_cells"));
        assert!(highlighted.html.contains("syntax-keyword\">shade"));
        assert!(highlighted.html.contains("syntax-keyword\">levels3"));
        assert!(highlighted.html.contains("syntax-scene\">basic"));
        assert!(highlighted.html.contains("syntax-scene\">push3d"));
        assert!(highlighted.html.contains("syntax-keyword\">size"));
        assert!(highlighted.html.contains("syntax-keyword\">gap"));
        assert!(highlighted.html.contains("syntax-keyword\">align"));
        assert!(highlighted.html.contains("syntax-literal\">center"));
        assert!(highlighted.html.contains("syntax-literal\">top"));
        assert!(highlighted.html.contains("syntax-effect\">goto"));
        assert!(highlighted.html.contains("syntax-scene\">level_select"));
        assert!(highlighted.html.contains("\n.P.B.\n"));
        assert!(
            !highlighted
                .html
                .contains("P<span class=\"syntax-operator\">.")
        );
    }

    #[test]
    fn highlights_3d_objects_when_layer_and_object_share_a_name() {
        let source = r#"
title same_name_3d

puzzle3 same_name {
layers {
Floor = Floor
}
rules {
[ Floor ] -> [ Floor ]
}
}

levels3 basic of same_name {
legend {
, = Floor
}
level start {
,
}
}
"#;
        let highlighted = highlight_source(source);

        assert!(highlighted.html.contains(
            "syntax-group\">Floor</span> <span class=\"syntax-operator\">=</span> <span class=\"syntax-object\">Floor"
        ));
        assert_eq!(
            highlighted
                .html
                .matches("<span class=\"syntax-object\">Floor</span>")
                .count(),
            4
        );
    }

    #[test]
    fn highlights_canonical_3d_inputs_and_sprites3_without_parser_success() {
        let highlighted = highlight_source(
            r#"
puzzle3 push3d {
layers {
solid = Player Box
}
rules {
forward [ Player | Box ] -> [ | Player ]
right:backward [ Player | Box ;; Goal | ] -> [ | Player ;; Goal | Box ]
}
}

sprites3 basic of push3d {
Floor
#90ee90 #008000
01
10
}

scene playing {
layout {
puzzle3 board {
keys {
w ArrowUp -> forward
s ArrowDown -> backward
}
}
}
}
"#,
        );

        assert!(highlighted.html.contains("syntax-keyword\">sprites3"));
        assert!(highlighted.html.contains("syntax-asset\">Floor"));
        assert!(highlighted.html.contains("syntax-input\">forward"));
        assert!(highlighted.html.contains("syntax-input\">ArrowUp"));
        assert!(highlighted.html.contains("syntax-keyword\">right"));
        assert!(highlighted.html.contains("syntax-keyword\">backward"));
    }

    #[test]
    fn highlights_same_word_by_grammar_context() {
        let highlighted = highlight_source(
            r#"
title contextual_highlight

sounds {
sfx clear seed=clear01 type=jump
music music_name seed=bgm01
}

puzzle board {
layers {
__legacy_layer_0 = Player
}
rules {
[ Player ] -> [ Player ]
}
level start {
.
}
}

scene title {
on_scene_start {
sfx clear
play_music music_name
}
}
"#,
        );

        assert!(
            highlighted
                .html
                .contains("syntax-keyword\">sfx</span> <span class=\"syntax-asset\">clear")
        );
        assert!(
            highlighted
                .html
                .contains("syntax-effect\">sfx</span> <span class=\"syntax-asset\">clear")
        );
        assert!(
            highlighted.html.contains(
                "syntax-effect\">play_music</span> <span class=\"syntax-asset\">music_name"
            )
        );
    }

    #[test]
    fn highlights_theme_state_and_condition_contexts() {
        let highlighted = highlight_source(
            r#"
title theme_state_condition_highlight
theme clean
var count = 1

scene playing {
if win_conditions -> goto title
}
"#,
        );

        assert!(highlighted.html.contains(
            "<span class=\"syntax-keyword\">theme</span> <span class=\"syntax-theme\">clean"
        ));
        assert!(highlighted.html.contains(
            "<span class=\"syntax-keyword\">var</span> <span class=\"syntax-state\">count"
        ));
        assert!(highlighted.html.contains(
            "<span class=\"syntax-keyword\">if</span> <span class=\"syntax-condition\">win_conditions"
        ));
    }

    #[test]
    fn highlights_keyword_named_music_asset_when_parse_fails() {
        let highlighted = highlight_source(
            r#"
title keyword_named_music

sounds {
music music seed=bgm01
}

scene title {
layout {
button "New Game" -> goto playing play_music music
}
}
"#,
        );

        assert!(!highlighted.parsed);
        assert!(
            highlighted
                .html
                .contains("syntax-effect\">play_music</span> <span class=\"syntax-asset\">music")
        );
    }

    #[test]
    fn highlights_parser_typed_declaration_parts() {
        let highlighted = highlight_source(
            r#"
title Fixban

sounds {
sfx clear seed=clear01 type=jump
music music_name seed=bgm01 bars=8 height=0 bpm=100 volume=0.5
}

puzzle fixban {
layers {
__legacy_layer_0 = Player
}
map rotate directions {
up -> right
}
rules {
[ Player ] -> [ Player ]
}
level start {
.
}
}

scene playing {
board = puzzle fixban
subtitle board.level.label
if board.level.last {
goto title
}
}
"#,
        );

        assert!(highlighted.html.contains("syntax-string\">Fixban"));
        assert!(
            highlighted
                .html
                .contains("syntax-keyword\">seed</span><span class=\"syntax-operator\">=</span><span class=\"syntax-string\">clear01")
        );
        assert!(
            highlighted
                .html
                .contains("syntax-keyword\">type</span><span class=\"syntax-operator\">=</span><span class=\"syntax-string\">jump")
        );
        assert!(
            highlighted
                .html
                .contains("syntax-keyword\">height</span><span class=\"syntax-operator\">=</span><span class=\"syntax-number\">0")
        );
        assert!(
            highlighted
                .html
                .contains("syntax-effect\">rotate</span> <span class=\"syntax-group\">directions")
        );
        assert!(
            highlighted
                .html
                .contains("syntax-state\">board</span> <span class=\"syntax-operator\">=</span> <span class=\"syntax-keyword\">puzzle</span> <span class=\"syntax-scene\">fixban")
        );
        assert!(highlighted.html.contains(
            "syntax-state\">board</span><span class=\"syntax-operator\">.</span>level<span class=\"syntax-operator\">.</span><span class=\"syntax-string\">label"
        ));
        assert!(highlighted.html.contains(
            "syntax-state\">board</span><span class=\"syntax-operator\">.</span>level<span class=\"syntax-operator\">.</span><span class=\"syntax-condition\">last"
        ));
    }

    #[test]
    fn highlights_scene_projection_and_layout_words() {
        let highlighted = highlight_source(
            r#"
title projection_highlight
theme name

scene menu {
layout {
column scroll=true {
for l in levels {
button join(l.num, ". ", l.title, " ", l.solved) -> goto playing(l)
}
}
}
}
"#,
        );

        assert!(highlighted.html.contains(
            "<span class=\"syntax-keyword\">theme</span> <span class=\"syntax-theme\">name"
        ));
        assert!(
            highlighted
                .html
                .contains("syntax-keyword\">scroll</span><span class=\"syntax-operator\">=")
        );
        assert!(
            highlighted
                .html
                .contains("syntax-effect\">join</span><span class=\"syntax-operator\">(")
        );
        assert!(
            highlighted
                .html
                .contains("syntax-binding\">l</span><span class=\"syntax-operator\">.</span>num")
        );
        assert!(
            highlighted
                .html
                .contains("syntax-binding\">l</span><span class=\"syntax-operator\">.</span>title")
        );
        assert!(
            highlighted.html.contains(
                "syntax-binding\">l</span><span class=\"syntax-operator\">.</span>solved"
            )
        );
        assert!(!highlighted.html.contains("syntax-keyword\">level</span>"));
    }

    #[test]
    fn keeps_scoped_table_keys_and_raw_rows_plain() {
        let highlighted = highlight_source(
            r#"
title scoped_highlight

puzzle board {
tags {
kind = A B
}
layers {
__legacy_layer_0 = Box:kind
}
legend {
1 = Box:A
A = Box:B
}
sprites {
colors {
piece_color:kind {
A = #4a4
}
}
shapes {
mark:kind {
A {
010
111
}
}
}
}
level start {
1A
}
}
"#,
        );

        assert!(!highlighted.html.contains("syntax-number\">1</span> ="));
        assert!(!highlighted.html.contains("syntax-variant\">A</span> ="));
        assert!(!highlighted.html.contains("syntax-variant\">A</span> {"));
        assert!(!highlighted.html.contains("syntax-number\">010</span>"));
        assert!(!highlighted.html.contains("syntax-number\">1</span>A"));
        assert!(highlighted.html.contains("syntax-object\">Box"));
        assert!(highlighted.html.contains("syntax-object\">A</span>"));
        assert!(
            highlighted
                .html
                .contains("syntax-color\" style=\"--syntax-color-token: #4a4\">#4a4")
        );
    }

    #[test]
    fn highlights_visual_shape_table_declarations_as_assets() {
        let highlighted = highlight_source(
            r#"
title visual_shape_table_highlight

puzzle board {
tags {
kind = A B
}
layers {
actor = Box:kind
}
legend {
. = empty
}
rules {
}
sprites {
shapes {
mark_shape:kind {
A {
010
111
010
}
B {
111
010
111
}
}
}
}
level start {
.
}
}
"#,
        );

        assert!(highlighted.html.contains(
            "syntax-asset\">mark_shape</span><span class=\"syntax-operator\">:</span><span class=\"syntax-group\">kind"
        ));
        assert!(
            highlighted
                .html
                .contains("A <span class=\"syntax-brace-depth")
        );
        assert!(
            !highlighted
                .html
                .contains("syntax-asset\">A</span> <span class=\"syntax-brace-depth")
        );
        assert!(!highlighted.html.contains("syntax-number\">010</span>"));
    }

    #[test]
    fn highlights_visual_shape_variant_braces_without_coloring_ascii_rows() {
        let highlighted = highlight_source(
            r#"
title shape_variant_braces

puzzle board {
tags {
kind = A B
}
layers {
actor = Box:kind
}
rules {
}
sprites {
shapes {
mark:kind {
A {
00000
01010
00100
01010
00000
}
B {
00000
00100
01010
00100
00000
}
}
}
}
}
"#,
        );

        assert_eq!(highlighted.html.matches("syntax-brace-invalid").count(), 0);
        assert!(
            highlighted
                .html
                .contains("A <span class=\"syntax-brace-depth-4\">{</span>")
        );
        assert!(
            highlighted
                .html
                .contains("B <span class=\"syntax-brace-depth-4\">{</span>")
        );
        assert!(!highlighted.html.contains("syntax-number\">01010</span>"));
    }

    #[test]
    fn keeps_block_scoped_rows_plain() {
        let highlighted = highlight_source(
            r#"
title section_scoped_highlight

puzzle board {
tags {
kind = A B
}
layers {
__legacy_layer_0 = Box:kind
}
legend {
1 = Box:A
A = Box:B
}
levels {
level start {
1A
}
}
}
"#,
        );

        assert!(!highlighted.html.contains("syntax-number\">1</span> ="));
        assert!(!highlighted.html.contains("syntax-variant\">A</span> ="));
        assert!(!highlighted.html.contains("syntax-number\">1</span>A"));
        assert!(highlighted.html.contains("syntax-object\">Box"));
    }

    #[test]
    fn top_level_levels_scope_highlights_known_and_unknown_map_cells() {
        let highlighted = highlight_source(
            r#"
title top_level_levels_highlight

puzzle board {
layers 2
layers {
__legacy_layer_0 = Player
}
legend P = Player
rules {
[ Player ] -> [ Player ]
}
}

levels {
level start {
P1
}
level second {
P
}
}
"#,
        );

        assert!(highlighted.html.contains("syntax-keyword\">levels"));
        assert!(highlighted.html.contains("syntax-keyword\">level"));
        assert!(highlighted.html.contains(
            "\n<span class=\"syntax-level-cell\">P</span><span class=\"syntax-level-cell-invalid\">1</span>\n"
        ));
        assert!(
            !highlighted
                .html
                .contains("P<span class=\"syntax-number\">1</span>")
        );
        assert!(!highlighted.html.contains("syntax-object\">P</span>1"));
    }

    #[test]
    fn braced_levels3_keeps_blank_separated_slices_plain() {
        let highlighted = highlight_source(
            r#"
title levels3_highlight

puzzle3 board {
layers {
floor = Floor
}
rules {
}
}

levels3 microban of board {
legend {
. = empty
, = Floor
G = Goal
}

level microban_01 {
    ####..
    #..#..

    ,,,,,,
    ,,G,,,
    ,G,,,,
}
}
"#,
        );

        assert!(
            highlighted
                .html
                .contains("\n    ,,,,,,\n    ,,G,,,\n    ,G,,,,\n")
        );
        assert!(!highlighted.html.contains(",,G<span"));
        assert!(
            !highlighted
                .html
                .contains(",<span class=\"syntax-effect\">G</span>,,,,")
        );
    }

    #[test]
    fn spec_3d_microban_01_second_slice_rows_stay_plain() {
        let source = include_str!("../../../games/spec_3d.puzzle3");
        let highlighted = highlight_source(source);

        assert!(highlighted.html.contains("\n,,G,,,\n"));
        assert!(highlighted.html.contains("\n,G,,,,\n"));
        assert!(
            !highlighted
                .html
                .contains(",<span class=\"syntax-effect\">G</span>,,,,")
        );
    }

    #[test]
    fn highlights_named_levels_header_legend_rhs_and_level_names() {
        let highlighted = highlight_source(
            r#"
title named_levels_highlight

puzzle sokoban {
layers {
floor = Goal
solid = Box Wall Player
}
rules {
[ Player ] -> [ Player ]
}
}

levels microban of sokoban {
legend {
. = empty
G = Goal
* = Goal Box
}

level microban_01
G*
}
"#,
        );

        assert!(highlighted.parsed);
        assert!(
            highlighted
                .html
                .contains("syntax-scene\">microban</span> <span class=\"syntax-keyword\">of</span> <span class=\"syntax-scene\">sokoban")
        );
        assert!(highlighted.html.contains("syntax-object\">Goal"));
        assert!(highlighted.html.contains("syntax-object\">Box"));
        assert!(
            highlighted
                .html
                .contains("syntax-keyword\">level</span> <span class=\"syntax-scene\">microban_01")
        );
        assert!(!highlighted.html.contains("syntax-object\">G</span>*"));
    }

    #[test]
    fn top_level_unbraced_levels_scope_highlights_known_and_unknown_map_cells() {
        let highlighted = highlight_source(
            r#"
title top_level_unbraced_levels_highlight

puzzle board {
layers 2
layers {
__legacy_layer_0 = Player
}
legend P = Player
rules {
[ Player ] -> [ Player ]
}
}

levels {
level start
P1

level second
P
}
"#,
        );

        assert!(highlighted.html.contains("syntax-keyword\">levels"));
        assert!(highlighted.html.contains("syntax-keyword\">level"));
        assert!(highlighted.html.contains(
            "\n<span class=\"syntax-level-cell\">P</span><span class=\"syntax-level-cell-invalid\">1</span>\n"
        ));
        assert!(
            !highlighted
                .html
                .contains("P<span class=\"syntax-number\">1</span>")
        );
        assert!(!highlighted.html.contains("syntax-object\">P</span>1"));
    }

    #[test]
    fn level_local_legend_highlights_cells_even_when_declared_after_rows() {
        let highlighted = highlight_source(
            r#"
title level_local_legend_highlight

puzzle board {
layers 2
layers {
__legacy_layer_0 = Player Box
}
legend P = Player
rules {
[ Player ] -> [ Player ]
}
}

levels {
level start {
PX
legend X = Box
}
level second {
X
}
}
"#,
        );

        assert!(highlighted.html.contains(
            "\n<span class=\"syntax-level-cell\">P</span><span class=\"syntax-level-cell\">X</span>\n"
        ));
        assert!(
            highlighted
                .html
                .contains("\n<span class=\"syntax-level-cell-invalid\">X</span>\n")
        );
    }

    #[test]
    fn highlights_routine_definitions_and_calls() {
        let highlighted = highlight_source(
            r#"
title routine_highlight

puzzle board {
layers 2
legend . = empty
layers {
__legacy_layer_0 = Player
}
routine move_player once {
[ Player ] -> [ Player ]
}
routine slide repeat {
[ Player ] -> [ Player ]
}
rule display paint once {
display [ Player ] -> [ Player ]
}
on_level_start {
move_player
display paint
}
rules {
move_player
}
level start {
.
}
}
"#,
        );

        assert!(highlighted.html.contains("syntax-keyword\">routine"));
        assert!(highlighted.html.contains("syntax-keyword\">rule"));
        assert!(highlighted.html.contains("syntax-keyword\">once"));
        assert!(highlighted.html.contains("syntax-keyword\">repeat"));
        assert!(highlighted.html.contains("syntax-effect\">move_player"));
        assert!(highlighted.html.contains("syntax-effect\">slide"));
        assert!(highlighted.html.contains("syntax-effect\">paint"));
    }

    #[test]
    fn highlights_standard_move_routine_call() {
        let highlighted = highlight_source(
            r#"
title standard_move_highlight

puzzle board {
layers {
actor = Player
}
rules {
move
}
}
"#,
        );

        assert!(highlighted.html.contains("syntax-effect\">move</span>"));
    }

    #[test]
    fn object_can_be_used_as_a_group_name_without_keyword_color() {
        let highlighted = highlight_source(
            r#"
title keyword_group_highlight

puzzle board {
layers 2
legend . = empty
layers {
__legacy_layer_0 = Player Box
}
groups {
object = Box
}
rules {
layers {
__legacy_layer_1 = Player
}
}
level start {
.
}
}
"#,
        );

        assert!(highlighted.html.contains("syntax-group\">object</span>"));
        assert!(!highlighted.html.contains("syntax-keyword\">object</span>"));
    }

    #[test]
    fn layer_can_be_used_as_an_object_name_without_keyword_highlight() {
        let highlighted = highlight_source(
            r#"
title layer_object_highlight

puzzle board {
layers {
floor = layer
}
rules {
[ layer ]
}
}
"#,
        );

        assert!(highlighted.html.contains("syntax-object\">layer</span>"));
        assert!(!highlighted.html.contains("syntax-keyword\">layer</span>"));
    }

    #[test]
    fn keyword_named_group_does_not_override_block_keyword_highlight() {
        let highlighted = highlight_source(
            r#"
title keyword_group_scope_highlight

puzzle board {
layers {
actor = Player
}
groups {
rules = Player
}
rules {
[ rules | Player ] -> [ Player ]
}
}
"#,
        );

        assert!(highlighted.html.contains(
            "<span class=\"syntax-keyword\">rules</span> <span class=\"syntax-brace-depth-1\">{</span>"
        ));
        assert!(highlighted.html.contains(
            "<span class=\"syntax-group\">rules</span> <span class=\"syntax-operator\">=</span>"
        ));
        assert!(highlighted.html.contains(
            "<span class=\"syntax-group\">rules</span> <span class=\"syntax-operator\">|</span>"
        ));
    }

    #[test]
    fn flag_can_be_used_as_an_object_name_without_literal_color() {
        let highlighted = highlight_source(
            r#"
title flag_object_highlight

puzzle board {
layers 2
legend . = empty
layers {
__legacy_layer_0 = flag
}
rules {
[ flag ] -> [ flag ]
}
level start {
.
}
}
"#,
        );

        assert!(highlighted.html.contains("syntax-object\">flag</span>"));
        assert!(!highlighted.html.contains("syntax-literal\">flag</span>"));
        assert!(!highlighted.html.contains("syntax-keyword\">flag</span>"));
    }

    #[test]
    fn flag_can_be_used_as_a_scratch_name_without_literal_color() {
        let highlighted = highlight_source(
            r#"
title flag_scratch_highlight

puzzle board {
layers 2
legend . = empty
layers {
__legacy_layer_0 = Player
}
scratch {
flag
}
rules {
[ Player ] -> [ Player{flag} ]
}
level start {
.
}
}
"#,
        );

        assert!(highlighted.html.contains("syntax-scratch\">flag</span>"));
        assert!(!highlighted.html.contains("syntax-literal\">flag</span>"));
        assert!(!highlighted.html.contains("syntax-keyword\">flag</span>"));
    }

    #[test]
    fn highlights_selector_scratch_braces_apart_from_block_braces() {
        let highlighted = highlight_source(
            r#"
title scratch_brace_highlight

puzzle board {
layers {
actor = Player
}
scratch {
mark
}
rules {
[ Player{} | Player{mark} ] -> [ Player{mark} | Player ]
}
level start {
.
}
}
"#,
        );

        assert!(highlighted.html.contains(
            "syntax-object\">Player</span><span class=\"syntax-scratch\">{</span><span class=\"syntax-scratch\">}</span>"
        ));
        assert!(highlighted.html.contains(
            "syntax-object\">Player</span><span class=\"syntax-scratch\">{</span><span class=\"syntax-scratch\">mark</span><span class=\"syntax-scratch\">}</span>"
        ));
        assert!(highlighted.html.contains("syntax-brace-depth-0\">{</span>"));
        assert!(highlighted.html.contains("syntax-brace-depth-1\">{</span>"));
    }

    #[test]
    fn qualified_scratch_names_highlight_tag_parts_with_tag_colors() {
        let highlighted = highlight_source(
            r#"
title qualified_scratch_highlight

puzzle board {
tags {
color = red blue
}
layers 2
legend . = empty
layers {
__legacy_layer_0 = Player
}
scratch {
enter:directions = bool
paint:red
push:>
count:3
}
rules {
[ Player ] -> [ Player{enter:directions paint:red push:> count:3} ]
}
level start {
.
}
}
"#,
        );

        assert!(highlighted.html.contains(
            "syntax-scratch\">enter</span><span class=\"syntax-operator\">:</span><span class=\"syntax-group\">directions</span>"
        ));
        assert!(highlighted.html.contains(
            "syntax-scratch\">paint</span><span class=\"syntax-operator\">:</span><span class=\"syntax-variant\">red</span>"
        ));
        assert!(highlighted.html.contains(
            "syntax-scratch\">push</span><span class=\"syntax-operator\">:</span><span class=\"syntax-variant\">&gt;</span>"
        ));
        assert!(highlighted.html.contains(
            "syntax-scratch\">count</span><span class=\"syntax-operator\">:</span><span class=\"syntax-variant\">3</span>"
        ));
        assert!(
            !highlighted
                .html
                .contains("syntax-scratch\">enter:directions</span>")
        );
    }

    #[test]
    fn star_schema_family_selectors_keep_object_color() {
        let highlighted = highlight_source(
            r#"
title schema_family_highlight

puzzle board {
tags {
kind = A B
}
layers {
__legacy_layer_0 = Target:kind
__legacy_layer_1 = Box
}
groups {
target = Target:*
}
rules {
[ Target:A | Target:* ] -> [ Target:B | Target:* ]
}
level start {
.
}
}
"#,
        );

        assert!(
            highlighted
                .html
                .contains("syntax-object\">Target</span><span class=\"syntax-operator\">:</span><span class=\"syntax-group\">kind")
        );
        assert!(
            highlighted
                .html
                .contains("syntax-object\">Target</span><span class=\"syntax-operator\">:</span><span class=\"syntax-object\">A")
        );
        assert!(
            highlighted
                .html
                .contains("syntax-object\">Target</span><span class=\"syntax-operator\">:</span><span class=\"syntax-group\">*")
        );
        assert_eq!(
            highlighted
                .html
                .matches("<span class=\"syntax-object\">Target</span>")
                .count(),
            6
        );
        assert_eq!(
            highlighted
                .html
                .matches("<span class=\"syntax-group\">Target</span>")
                .count(),
            0
        );
    }

    #[test]
    fn partial_schema_selectors_use_selector_alias_color() {
        let highlighted = highlight_source(
            r#"
title partial_schema_selector_highlight

puzzle board {
tags {
kind = A B
}
tags {
phase = hot cold
}
layers {
actor = Target:kind:phase
}
rules {
[ Target:A | Target:A:hot ] -> [ Target:B | Target:B:cold ]
}
level start {
.
}
}
"#,
        );
        assert!(
            highlighted
                .html
                .contains("syntax-object\">Target</span><span class=\"syntax-operator\">:</span><span class=\"syntax-group\">kind</span><span class=\"syntax-operator\">:</span><span class=\"syntax-group\">phase")
        );
        assert!(
            highlighted
                .html
                .contains("syntax-object\">Target</span><span class=\"syntax-operator\">:</span><span class=\"syntax-object\">A")
        );
        assert!(
            highlighted
                .html
                .contains("syntax-object\">Target</span><span class=\"syntax-operator\">:</span><span class=\"syntax-object\">A</span><span class=\"syntax-operator\">:</span><span class=\"syntax-object\">hot")
        );
    }
}
