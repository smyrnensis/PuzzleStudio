use std::collections::{HashMap, HashSet};

use crate::semantic::{SemanticKind, SemanticToken, semantic_tokens};
use crate::source::{
    SourceScope, SourceSectionPart, scan_source_context, split_tokens, strip_line_comment,
};
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
    Query,
    Scene,
    Asset,
    Color,
    Number,
    String,
    Comment,
    Operator,
    Section,
    SectionRule,
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
            HighlightKind::Query => "syntax-query",
            HighlightKind::Scene => "syntax-scene",
            HighlightKind::Asset => "syntax-asset",
            HighlightKind::Color => "syntax-color",
            HighlightKind::Number => "syntax-number",
            HighlightKind::String => "syntax-string",
            HighlightKind::Comment => "syntax-comment",
            HighlightKind::Operator => "syntax-operator",
            HighlightKind::Section => "syntax-keyword",
            HighlightKind::SectionRule => "syntax-section-rule",
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
    for name in game.input_labels.values() {
        symbols.insert(name.clone(), HighlightKind::Input);
    }
    for name in game.global_labels.values() {
        symbols.insert(name.clone(), HighlightKind::State);
    }
    for name in game.query_labels.values() {
        symbols.insert(name.clone(), HighlightKind::Query);
    }
    for name in game.conditions.keys() {
        symbols.insert(name.clone(), HighlightKind::Query);
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
            if let puzzle3d_model::VariantValueSet3::Named(values) = &axis.values {
                for value in values {
                    symbols.insert(value.clone(), HighlightKind::Variant);
                }
            }
        }
    }
    for group in &puzzle.catalog.groups {
        symbols.insert(group.name.clone(), HighlightKind::Group);
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
    let visual_color_aliases = scan_visual_color_aliases(&context);
    let visual_named_color_ranges = scan_visual_named_color_ranges(&context, &visual_color_aliases);
    let visual_ascii_color_ranges = scan_visual_ascii_color_ranges(source, &visual_color_aliases);
    let mut chars = source.char_indices().peekable();

    while let Some((index, ch)) = chars.next() {
        if let Some((end, part)) = context.section_starting_at(index) {
            let kind = match part {
                SourceSectionPart::Title => HighlightKind::Section,
                SourceSectionPart::Rule => HighlightKind::SectionRule,
            };
            push_span(&mut out, kind, &source[index..end]);
            skip_until(&mut chars, end);
            continue;
        }

        if let Some(range) = visual_ascii_color_range_starting_at(&visual_ascii_color_ranges, index)
        {
            push_colored_text_span(&mut out, &range.color, &source[range.start..range.end]);
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

        if is_word_start(ch) {
            let end = consume_while(source, index, is_word_continue);
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
                );
            }
            skip_until(&mut chars, end);
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
            push_operator_run(&mut out, source, index, end);
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
            scope @ (SourceScope::Objects
            | SourceScope::Sounds
            | SourceScope::Tags
            | SourceScope::Group
            | SourceScope::Layers
            | SourceScope::Scratch
            | SourceScope::Visuals
            | SourceScope::Keys),
        ) => Some(scope),
        Some(SourceScope::SceneKeys) => Some(SourceScope::Keys),
        Some(_) => Some(SourceScope::Other),
    }
}

fn collect_line_symbols(
    tokens: &[&str],
    scope: Option<SourceScope>,
    symbols: &mut HashMap<String, HighlightKind>,
    family_bases: &mut HashSet<String>,
    family_axes: &mut HashMap<String, usize>,
    family_axis_names: &mut HashSet<String>,
) {
    match tokens {
        ["routine", "display", name, ..] | ["rule", "display", name, ..] => {
            insert_source_symbol(symbols, name, HighlightKind::Effect);
        }
        ["routine", name, ..] | ["rule", name, ..] => {
            insert_source_symbol(symbols, name, HighlightKind::Effect);
        }
        ["command" | "input", name, ..] | ["direction", name, ..] => {
            insert_source_symbol(symbols, name, HighlightKind::Input);
        }
        ["query", name, ..] => {
            insert_source_symbol(symbols, name, HighlightKind::Query);
        }
        ["scene", name, ..] => {
            insert_source_symbol(symbols, name, HighlightKind::Scene);
        }
        ["puzzle" | "puzzle3", name, ..] => {
            insert_source_symbol(symbols, name, HighlightKind::Scene);
        }
        ["levels3", name, "of", model, ..] | ["sprites3", name, "of", model, ..] => {
            insert_source_symbol(symbols, name, HighlightKind::Scene);
            insert_source_symbol(symbols, model, HighlightKind::Scene);
        }
        ["levels3" | "sprites3", name, ..] => {
            insert_source_symbol(symbols, name, HighlightKind::Scene);
        }
        ["map", name, axis] => {
            insert_source_symbol(symbols, name, HighlightKind::Effect);
            insert_source_symbol(symbols, axis, HighlightKind::Group);
            family_axis_names.insert((*axis).to_string());
        }
        ["sfx", name, ..] | ["music", name, ..] if scope == Some(SourceScope::Sounds) => {
            insert_source_symbol(symbols, name, HighlightKind::Asset);
        }
        ["shape", table, ..] | ["colors", table, ..] => {
            if let Some((name, axis)) = table.split_once(':') {
                insert_source_symbol(symbols, name, HighlightKind::Asset);
                insert_source_symbol(symbols, axis, HighlightKind::Variant);
            }
        }
        [name] if scope == Some(SourceScope::Visuals) => {
            insert_source_symbol(symbols, name, HighlightKind::Asset);
        }
        ["object", spec, ..] if *spec != "=" => collect_object_declaration_spec(
            spec,
            symbols,
            family_bases,
            family_axes,
            family_axis_names,
        ),
        ["var" | "const" | "global", name, "=", ..]
        | ["persistent", "var" | "const", name, "=", ..]
        | ["persistent", name, "=", ..] => {
            insert_source_symbol(symbols, name, HighlightKind::State);
        }
        ["group", name, "=", selectors @ ..] => {
            insert_source_symbol(symbols, name, HighlightKind::Group);
            collect_selector_specs(selectors, symbols);
        }
        [name, "=", values @ ..] if scope == Some(SourceScope::Group) => {
            insert_source_symbol(symbols, name, HighlightKind::Group);
            collect_selector_specs(values, symbols);
        }
        [
            "layer" | "layers" | "collision_layers",
            _name,
            "=",
            selectors @ ..,
        ] if scope == Some(SourceScope::Objects) => {
            collect_selector_specs(selectors, symbols);
        }
        [name, "=", selectors @ ..] if scope == Some(SourceScope::Layers) => {
            insert_source_symbol(symbols, name, HighlightKind::Group);
            collect_selector_specs(selectors, symbols);
        }
        ["each", spec, ..] if scope == Some(SourceScope::Layers) => {
            collect_object_spec(spec, symbols);
        }
        ["each", spec, ..] if scope == Some(SourceScope::Objects) => {
            collect_object_declaration_spec(
                spec,
                symbols,
                family_bases,
                family_axes,
                family_axis_names,
            );
        }
        ["display", spec, ..] if scope == Some(SourceScope::Objects) => {
            collect_object_declaration_spec(
                spec,
                symbols,
                family_bases,
                family_axes,
                family_axis_names,
            );
        }
        [specs @ ..] if scope == Some(SourceScope::Objects) => {
            collect_object_declaration_specs(
                specs,
                symbols,
                family_bases,
                family_axes,
                family_axis_names,
            );
        }
        [spec] if scope == Some(SourceScope::Scratch) => collect_scratch_spec(spec, symbols),
        [..] if scope == Some(SourceScope::Keys) => collect_key_binding_symbols(tokens, symbols),
        [name, "=", values @ ..]
            if scope == Some(SourceScope::Tags) && tag_set_tokens(name, values) =>
        {
            insert_source_symbol(symbols, name, HighlightKind::Group);
            family_axis_names.insert((*name).to_string());
            for value in values {
                insert_source_symbol(symbols, value, HighlightKind::Variant);
            }
        }
        _ => {}
    }
}

fn collect_key_binding_symbols(tokens: &[&str], symbols: &mut HashMap<String, HighlightKind>) {
    let Some(separator) = tokens.iter().position(|token| matches!(*token, "=" | "->")) else {
        if let Some(separator) = tokens.iter().position(|token| *token == "<-") {
            for input in &tokens[..separator] {
                insert_source_symbol(symbols, input, HighlightKind::Input);
            }
            for key in &tokens[separator + 1..] {
                insert_source_symbol(symbols, key, HighlightKind::Input);
            }
        }
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
        collect_object_spec(spec, symbols);
    }
}

fn collect_object_declaration_specs(
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
        collect_object_declaration_spec(
            spec,
            symbols,
            family_bases,
            family_axes,
            family_axis_names,
        );
    }
}

fn collect_object_declaration_spec(
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
        insert_source_symbol(symbols, part, HighlightKind::Variant);
    }
}

fn clean_object_spec(spec: &str) -> &str {
    let spec = spec.trim_matches(|ch: char| matches!(ch, '[' | ']' | '(' | ')' | '|'));
    spec.split_once('{').map_or(spec, |(head, _)| head)
}

fn collect_scratch_spec(spec: &str, symbols: &mut HashMap<String, HighlightKind>) {
    let Some((name, ty)) = spec.split_once(':') else {
        insert_source_symbol(symbols, spec, HighlightKind::Scratch);
        return;
    };
    insert_source_symbol(symbols, name, HighlightKind::Scratch);
    if ty != "int" {
        insert_source_symbol(symbols, ty, HighlightKind::Variant);
    }
}

fn tag_set_tokens(name: &str, values: &[&str]) -> bool {
    is_source_identifier(name)
        && !parser_keyword(name)
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

fn symbol_priority(kind: HighlightKind) -> u8 {
    match kind {
        HighlightKind::Object => 6,
        HighlightKind::Group => 5,
        HighlightKind::State
        | HighlightKind::Scratch
        | HighlightKind::Input
        | HighlightKind::Effect
        | HighlightKind::Emission
        | HighlightKind::Query
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

fn classify_bare_word(
    token: &str,
    symbols: &HashMap<String, HighlightKind>,
    _family_bases: &HashSet<String>,
) -> Option<HighlightKind> {
    classify_word(token, symbols)
}

fn classify_word(token: &str, symbols: &HashMap<String, HighlightKind>) -> Option<HighlightKind> {
    if let Some(kind) = symbols.get(token).copied() {
        return Some(kind);
    }
    if let Some((head, _)) = token.split_once(':') {
        if let Some(kind @ HighlightKind::Object) = symbols.get(head).copied() {
            return Some(kind);
        }
    }
    if parser_keyword(token) {
        return Some(HighlightKind::Keyword);
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
) {
    let parts = split_highlight_word(token);
    let supplied_axes = token.matches(':').count();
    let use_schema_selector_coloring = token.contains(':') && !token.contains('.');

    for (index, part) in parts.iter().enumerate() {
        if let Some(separator) = part.separator_before {
            push_span(out, HighlightKind::Operator, separator);
        }
        let absolute_start = token_start + part.start;
        let absolute_end = token_start + part.end;
        let text = &token[part.start..part.end];
        let kind = if let Some(kind) =
            semantic_kind_at(semantic_ranges, absolute_start, absolute_end)
        {
            Some(kind)
        } else if local_binding_at(binding_ranges, absolute_start, absolute_end, text) {
            Some(HighlightKind::Binding)
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
        } else if use_schema_selector_coloring && index > 0 && family_axis_names.contains(text) {
            Some(HighlightKind::Group)
        } else if use_schema_selector_coloring && index > 0 && text == "*" {
            Some(HighlightKind::Group)
        } else if token == text {
            classify_bare_word(text, symbols, family_bases)
        } else {
            classify_word(text, symbols)
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
        SemanticKind::Query => HighlightKind::Query,
        SemanticKind::Scene => HighlightKind::Scene,
        SemanticKind::Asset => HighlightKind::Asset,
        SemanticKind::Number => HighlightKind::Number,
        SemanticKind::String => HighlightKind::String,
    }
}

#[derive(Clone, Debug)]
struct VisualAsciiColorRange {
    start: usize,
    end: usize,
    color: String,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VisualHighlightScope {
    Sprites,
    SpriteEntry,
    Colors,
    ColorTable,
    Palettes,
    PaletteTable,
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
            Some(
                VisualHighlightScope::Sprites
                    | VisualHighlightScope::SpriteEntry
                    | VisualHighlightScope::Palettes
                    | VisualHighlightScope::PaletteTable
            )
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
            "palettes" => Some(VisualHighlightScope::Palettes),
            "shapes" | "shape" => Some(VisualHighlightScope::Other),
            _ if line.content.trim_end().ends_with('{') => Some(VisualHighlightScope::SpriteEntry),
            _ => None,
        },
        Some(VisualHighlightScope::Colors)
            if !has_assignment && first.contains(':') && line.content.trim_end().ends_with('{') =>
        {
            Some(VisualHighlightScope::ColorTable)
        }
        Some(VisualHighlightScope::Palettes)
            if !has_assignment && first.contains(':') && line.content.trim_end().ends_with('{') =>
        {
            Some(VisualHighlightScope::PaletteTable)
        }
        Some(
            VisualHighlightScope::SpriteEntry
            | VisualHighlightScope::ColorTable
            | VisualHighlightScope::PaletteTable,
        ) if line.content.trim_end().ends_with('{') => Some(VisualHighlightScope::Other),
        _ => None,
    }
}

fn is_visual_closing_line(line: &crate::source::SourceContextLine) -> bool {
    let trimmed = strip_line_comment(&line.content).trim();
    matches!(trimmed, "}" | "end")
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
    source: &str,
    aliases: &HashMap<String, String>,
) -> Vec<VisualAsciiColorRange> {
    let mut ranges = Vec::new();
    let mut in_sprites = false;
    let mut sprites_depth = 0i32;
    let mut pending_color_row = false;
    let mut palette = HashMap::<char, String>::new();
    let mut offset = 0usize;

    for line in source.split_inclusive('\n') {
        let line_end = offset + line.len();
        let content_end = line_end - usize::from(line.ends_with('\n'));
        let content = &source[offset..content_end];
        let trimmed = content.trim();
        let tokens = trimmed.split_whitespace().collect::<Vec<_>>();

        if !in_sprites {
            if matches!(tokens.as_slice(), ["sprites" | "sprites3", ..]) {
                in_sprites = true;
                sprites_depth = brace_delta(content).max(1);
            }
            offset = line_end;
            continue;
        }

        if trimmed.is_empty() {
            pending_color_row = false;
            palette.clear();
            offset = line_end;
            continue;
        }

        let starts_entry = visual_sprite_entry_header(&tokens, trimmed);
        if pending_color_row && let Some(next_palette) = visual_ascii_palette(&tokens, aliases) {
            palette = next_palette;
            pending_color_row = false;
            offset = line_end;
            continue;
        }

        if !palette.is_empty() && visual_ascii_row(trimmed, &palette) {
            add_visual_ascii_row_ranges(&mut ranges, offset, content, trimmed, &palette);
        } else if starts_entry {
            pending_color_row = true;
            palette.clear();
        } else if !matches!(tokens.as_slice(), ["{"] | ["}"]) {
            pending_color_row = false;
            palette.clear();
        }

        sprites_depth += brace_delta(content);
        if sprites_depth <= 0 {
            in_sprites = false;
            pending_color_row = false;
            palette.clear();
        }
        offset = line_end;
    }

    ranges
}

fn brace_delta(line: &str) -> i32 {
    line.chars().fold(0, |depth, ch| match ch {
        '{' => depth + 1,
        '}' => depth - 1,
        _ => depth,
    })
}

fn visual_sprite_entry_header(tokens: &[&str], trimmed: &str) -> bool {
    let Some(first) = tokens.first().copied() else {
        return false;
    };
    if matches!(
        first,
        "shape" | "colors" | "ascii" | "sprites" | "sprites3" | "{" | "}" | "end"
    ) || is_visual_color_token(first)
    {
        return false;
    }
    tokens.len() == 1 || trimmed.ends_with('{')
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
        if let Some(color) = palette.get(&ch).cloned() {
            ranges.push(VisualAsciiColorRange { start, end, color });
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
        let tokens = split_tokens(trimmed);

        if matches!(trimmed, "}" | "end") {
            stack.pop();
        }

        if !trimmed.is_empty() && !matches!(trimmed, "}" | "end") {
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
    matches!(
        token,
        "assets"
            | "align"
            | "author"
            | "sounds"
            | "button"
            | "camera"
            | "column"
            | "command"
            | "component_effect"
            | "const"
            | "colors"
            | "collision_layers"
            | "css"
            | "direction"
            | "default_wait_time"
            | "display"
            | "effect"
            | "each"
            | "else"
            | "end"
            | "flickscreen"
            | "for"
            | "gap"
            | "screen_focus"
            | "from"
            | "global"
            | "grid"
            | "homepage"
            | "puzzle"
            | "group"
            | "groups"
            | "if"
            | "in"
            | "import"
            | "input"
            | "inputs"
            | "interactive_look"
            | "interactive_zoom"
            | "keys"
            | "layer"
            | "layers"
            | "legend"
            | "level"
            | "level_menu"
            | "levels"
            | "levels3"
            | "lose_conditions"
            | "map"
            | "menu"
            | "music"
            | "name"
            | "objects"
            | "occupied_cells"
            | "on"
            | "on_display"
            | "on_level_clear"
            | "on_level_start"
            | "on_scene_start"
            | "of"
            | "once"
            | "once_all"
            | "once_per_level"
            | "box"
            | "persistent"
            | "pitch"
            | "puzzle3"
            | "query"
            | "region"
            | "repeat"
            | "resources"
            | "render"
            | "row"
            | "routine"
            | "rule"
            | "rules"
            | "scene"
            | "script"
            | "scratch"
            | "sfx"
            | "shape"
            | "show_index"
            | "show_solved"
            | "size"
            | "sprite"
            | "sprites"
            | "sprites3"
            | "state"
            | "tags"
            | "subtitle"
            | "text"
            | "theme"
            | "title"
            | "var"
            | "view"
            | "win_conditions"
            | "with"
            | "yaw"
            | "zoom"
            | "zoomscreen"
    )
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

fn is_word_continue(ch: char) -> bool {
    ch == '@'
        || ch == '_'
        || ch == ':'
        || ch == '.'
        || ch == '-'
        || ch == '*'
        || ch.is_ascii_alphanumeric()
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

fn push_operator_run(out: &mut String, source: &str, start: usize, end: usize) {
    let mut plain_start = start;
    for (offset, ch) in source[start..end].char_indices() {
        let index = start + offset;
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

fn push_colored_text_span(out: &mut String, color: &str, text: &str) {
    out.push_str("<span class=\"syntax-sprite-pixel\" style=\"--syntax-sprite-pixel-color: ");
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
    }

    #[test]
    fn highlights_section_header_sugar() {
        let highlighted = highlight_source(
            r#"
title section_header

puzzle board {
======
LEGEND
======
P = Player
}
"#,
        );

        assert!(highlighted.html.contains("syntax-section-rule\">======"));
        assert!(highlighted.html.contains("syntax-keyword\">LEGEND"));
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
objects {
Player Box:color
}
group {
pushable = Player Box:red
}
display_objects {
@Cursor @Aura:color
}
group active = Player Box:blue
var moves = 0
persistent var best = 0
scratch {
mark
shade:color
steps:int
}
legend P = Player
legend B = Box:red
main {
once [ Player{mark} | Box:red ] -> [ @Cursor | Box:blue{shade} ]
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
        assert!(highlighted.html.contains("syntax-variant\">red"));
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
query blocked = no Player
objects {
Player Box:kind
}
group {
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
button "Play" -> start levels in playing
button "Continue" -> continue levels in playing
transitions {
start -> goto playing
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
        assert!(highlighted.html.contains("syntax-variant\">A"));
        assert!(highlighted.html.contains("syntax-variant\">B"));
        assert!(highlighted.html.contains("syntax-input\">jump"));
        assert!(highlighted.html.contains("syntax-query\">blocked"));
        assert!(highlighted.html.contains("syntax-asset\">bump"));
        assert!(highlighted.html.contains("syntax-effect\">start"));
        assert!(highlighted.html.contains("syntax-effect\">continue"));
        assert!(highlighted.html.contains("syntax-scene\">playing"));
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
objects {
Box:kind Player
}
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
objects {
Target:kind
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
            "syntax-object\">Target</span><span class=\"syntax-operator\">:</span><span class=\"syntax-variant\">A"
        ));
        assert!(highlighted.html.contains(
            "syntax-object\">Target</span><span class=\"syntax-operator\">:</span><span class=\"syntax-variant\">B"
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
            "syntax-object\">Box</span><span class=\"syntax-operator\">:</span><span class=\"syntax-variant\">A"
        ));
    }

    #[test]
    fn highlights_builtin_direction_axes_as_group_like_names() {
        let highlighted = highlight_source(
            r#"
title direction_axis_highlight

puzzle board {
objects {
Facing:directions
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
objects {
Player
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
            highlighted.html.contains("syntax-object\">Player</span><span class=\"syntax-operator\">{</span><span class=\"syntax-literal\">&gt;</span><span class=\"syntax-operator\">}")
        );
        assert!(
            highlighted.html.contains("syntax-object\">Player</span><span class=\"syntax-operator\">{</span><span class=\"syntax-literal\">&lt;</span><span class=\"syntax-operator\">}")
        );
        assert!(
            !highlighted
                .html
                .contains("-<span class=\"syntax-literal\">&gt;")
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
group {
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
                .contains("syntax-sprite-pixel\" style=\"--syntax-sprite-pixel-color: #111\">0</span><span class=\"syntax-sprite-pixel\" style=\"--syntax-sprite-pixel-color: #222\">1</span>.")
        );
        assert!(
            !highlighted
                .html
                .contains("style=\"--syntax-color-token: #.#")
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
palettes {
floor = light_green dark_green
piece:kind {
A = light_green
B = dark_green
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
                .contains("syntax-sprite-pixel\" style=\"--syntax-sprite-pixel-color: #90ee90\">0</span><span class=\"syntax-sprite-pixel\" style=\"--syntax-sprite-pixel-color: #008000\">1</span>.")
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
    fn highlights_scene_key_bindings() {
        let highlighted = highlight_source(
            r#"
title key_highlight

scene pause {
keys {
Escape Enter Space = resume
q = quit
}
transitions {
quit -> goto title
resume -> back
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
objects {
Player
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
objects {
Player
}
rules {
[ Player ] -> [ Player ]
}
level start {
.
}
}

scene title {
view {
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
button "Menu" -> enter menu
button "Toggle" -> toggle menu
main {
enter -> emit choose_level(cursor.value)
}
transitions {
choose_level:level -> playing.goto level
}
}

scene playing {
}

scene menu {
}
"#,
        );
        assert!(highlighted.html.contains("syntax-scene\">title"));
        assert!(highlighted.html.contains("syntax-keyword\">view"));
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
                .contains("syntax-effect\">enter</span> <span class=\"syntax-scene\">menu")
        );
        assert!(
            highlighted
                .html
                .contains("syntax-effect\">toggle</span> <span class=\"syntax-scene\">menu")
        );
        assert!(highlighted.html.contains("choose_level"));
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
view size 4 3 {
column gap 1 align center top {
puzzle3 board
row gap 1 {
button "Restart" -> board.restart
button "Levels" -> start levels basic in level_select
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
        assert!(highlighted.html.contains("syntax-effect\">start"));
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
. = Floor
}
level start {
.
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
view {
puzzle3 board {
inputs {
forward <- w ArrowUp
backward <- s ArrowDown
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
objects {
Player
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
                .contains("syntax-emission\">sfx</span> <span class=\"syntax-asset\">clear")
        );
        assert!(
            highlighted.html.contains(
                "syntax-effect\">play_music</span> <span class=\"syntax-asset\">music_name"
            )
        );
    }

    #[test]
    fn highlights_parser_typed_declaration_parts() {
        let highlighted = highlight_source(
            r#"
title Fixban

sounds {
sfx clear seed=clear01 type=jump
music music_name seed=bgm01 tone=0 bpm=100 volume=0.5
}

puzzle fixban {
objects {
Player
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
                .contains("syntax-keyword\">tone</span><span class=\"syntax-operator\">=</span><span class=\"syntax-number\">0")
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
            "syntax-state\">board</span><span class=\"syntax-operator\">.</span><span class=\"syntax-keyword\">level</span><span class=\"syntax-operator\">.</span><span class=\"syntax-string\">label"
        ));
        assert!(highlighted.html.contains(
            "syntax-state\">board</span><span class=\"syntax-operator\">.</span><span class=\"syntax-keyword\">level</span><span class=\"syntax-operator\">.</span><span class=\"syntax-query\">last"
        ));
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
objects {
Box:kind
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
        assert!(highlighted.html.contains("syntax-variant\">A</span>"));
        assert!(
            highlighted
                .html
                .contains("syntax-color\" style=\"--syntax-color-token: #4a4\">#4a4")
        );
    }

    #[test]
    fn keeps_section_header_scoped_rows_plain() {
        let highlighted = highlight_source(
            r#"
title section_scoped_highlight

puzzle board {
tags {
kind = A B
}
objects {
Box:kind
}
======
LEGEND
======
1 = Box:A
A = Box:B
======
LEVELS
======
level start {
1A
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
    fn top_level_levels_scope_keeps_map_rows_plain() {
        let highlighted = highlight_source(
            r#"
title top_level_levels_highlight

puzzle board {
layers 2
objects {
Player
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
        assert!(highlighted.html.contains("\nP1\n"));
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
_ = empty
. = Floor
G = Goal
}

level microban_01 {
    ####__
    #__#__

    ......
    ..G...
    .G....
}
}
"#,
        );

        assert!(
            highlighted
                .html
                .contains("\n    ......\n    ..G...\n    .G....\n")
        );
        assert!(!highlighted.html.contains("..G<span"));
        assert!(
            !highlighted
                .html
                .contains(".<span class=\"syntax-effect\">G</span>....")
        );
    }

    #[test]
    fn spec_3d_microban_01_second_slice_rows_stay_plain() {
        let source = include_str!("../../../games/spec_3d.puzzle");
        let highlighted = highlight_source(source);

        assert!(highlighted.html.contains("\n    ..G...\n"));
        assert!(highlighted.html.contains("\n    .G....\n"));
        assert!(
            !highlighted
                .html
                .contains(".<span class=\"syntax-effect\">G</span>....")
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
    fn top_level_unbraced_levels_scope_keeps_map_rows_plain() {
        let highlighted = highlight_source(
            r#"
title top_level_unbraced_levels_highlight

puzzle board {
layers 2
objects {
Player
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
        assert!(highlighted.html.contains("\nP1\n"));
        assert!(
            !highlighted
                .html
                .contains("P<span class=\"syntax-number\">1</span>")
        );
        assert!(!highlighted.html.contains("syntax-object\">P</span>1"));
    }

    #[test]
    fn highlights_routine_definitions_and_calls() {
        let highlighted = highlight_source(
            r#"
title routine_highlight

puzzle board {
layers 2
legend . = empty
objects {
Player
}
routine move_player once {
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
        assert!(highlighted.html.contains("syntax-effect\">move_player"));
        assert!(highlighted.html.contains("syntax-effect\">paint"));
    }

    #[test]
    fn object_can_be_used_as_a_group_name_without_keyword_color() {
        let highlighted = highlight_source(
            r#"
title keyword_group_highlight

puzzle board {
layers 2
legend . = empty
objects {
Player Box
group {
object = Box
}
}
rules {
object Player 1
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
    fn flag_can_be_used_as_an_object_name_without_literal_color() {
        let highlighted = highlight_source(
            r#"
title flag_object_highlight

puzzle board {
layers 2
legend . = empty
objects {
flag
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
objects {
Player
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
    fn star_schema_family_selectors_keep_object_color() {
        let highlighted = highlight_source(
            r#"
title schema_family_highlight

puzzle board {
tags {
kind = A B
}
objects {
Target:kind
Box
group {
target = Target:*
}
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
                .contains("syntax-object\">Target</span><span class=\"syntax-operator\">:</span><span class=\"syntax-variant\">A")
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
objects {
Target:kind:phase
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
                .contains("syntax-group\">Target</span><span class=\"syntax-operator\">:</span><span class=\"syntax-variant\">A")
        );
        assert!(
            highlighted
                .html
                .contains("syntax-object\">Target</span><span class=\"syntax-operator\">:</span><span class=\"syntax-variant\">A</span><span class=\"syntax-operator\">:</span><span class=\"syntax-variant\">hot")
        );
    }
}
