use std::collections::{BTreeMap, BTreeSet};

use crate::semantic::{
    SemanticCompletionSlot, SemanticKind, is_completion_keyword, semantic_builtin_effect_commands,
    semantic_completion_context,
};
use crate::source::{SourceScope, scan_source_context};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletionList {
    pub replace_start: usize,
    pub replace_end: usize,
    pub items: Vec<CompletionItem>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionKind,
    pub insert_text: String,
    pub detail: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompletionKind {
    Keyword,
    Object,
    Group,
    State,
    Scratch,
    ValueSet,
    Variant,
    Direction,
    Input,
    Command,
    Effect,
    Emission,
    Routine,
    Query,
    Puzzle,
    Scene,
    Level,
    Sfx,
    Music,
    Sprite,
    Asset,
}

impl CompletionKind {
    fn as_str(self) -> &'static str {
        match self {
            CompletionKind::Keyword => "keyword",
            CompletionKind::Object => "object",
            CompletionKind::Group => "group",
            CompletionKind::State => "state",
            CompletionKind::Scratch => "scratch",
            CompletionKind::ValueSet => "tags",
            CompletionKind::Variant => "tag",
            CompletionKind::Direction => "direction",
            CompletionKind::Input => "input",
            CompletionKind::Command => "command",
            CompletionKind::Effect => "effect",
            CompletionKind::Emission => "emission",
            CompletionKind::Routine => "routine",
            CompletionKind::Query => "query",
            CompletionKind::Puzzle => "puzzle",
            CompletionKind::Scene => "scene",
            CompletionKind::Level => "level",
            CompletionKind::Sfx => "sfx",
            CompletionKind::Music => "music",
            CompletionKind::Sprite => "sprite",
            CompletionKind::Asset => "asset",
        }
    }
}

pub fn suggest_source_completions(source: &str, cursor_offset: usize) -> CompletionList {
    let context = semantic_completion_context(source, cursor_offset);
    let symbols = collect_completion_symbols(source);

    let mut items = Vec::<CompletionItem>::new();

    if let Some(axis_values) = selector_axis_values(&symbols, &context.token_text) {
        add_named_items(
            &mut items,
            axis_values.iter(),
            CompletionKind::Variant,
            "tag",
        );
    } else {
        for slot in &context.slots {
            add_slot_items(&mut items, &symbols, *slot);
        }
    }

    let prefix = completion_prefix(&context.token_text);
    let mut seen = BTreeSet::<(String, CompletionKind)>::new();
    items.retain(|item| {
        if !prefix.is_empty() && !item.label.starts_with(prefix) {
            return false;
        }
        seen.insert((item.label.clone(), item.kind))
    });
    items.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| left.label.cmp(&right.label))
    });
    items.truncate(80);

    CompletionList {
        replace_start: context.replace_start,
        replace_end: context.replace_end,
        items,
    }
}

pub fn completion_list_json(list: &CompletionList) -> String {
    let mut out = String::new();
    out.push('{');
    push_json_number(&mut out, "replaceStart", list.replace_start);
    out.push(',');
    push_json_number(&mut out, "replaceEnd", list.replace_end);
    out.push_str(",\"items\":[");
    for (index, item) in list.items.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_string(&mut out, "label", &item.label);
        out.push(',');
        push_json_string(&mut out, "kind", item.kind.as_str());
        out.push(',');
        push_json_string(&mut out, "insertText", &item.insert_text);
        out.push(',');
        push_json_string(&mut out, "detail", &item.detail);
        out.push('}');
    }
    out.push_str("]}");
    out
}

#[derive(Default)]
struct CompletionSymbols {
    objects: BTreeSet<String>,
    groups: BTreeSet<String>,
    states: BTreeSet<String>,
    scratches: BTreeSet<String>,
    value_set_names: BTreeSet<String>,
    variants: BTreeSet<String>,
    directions: BTreeSet<String>,
    inputs: BTreeSet<String>,
    commands: BTreeSet<String>,
    effects: BTreeSet<String>,
    emissions: BTreeSet<String>,
    routines: BTreeSet<String>,
    queries: BTreeSet<String>,
    puzzles: BTreeSet<String>,
    scenes: BTreeSet<String>,
    levels: BTreeSet<String>,
    sfx: BTreeSet<String>,
    music: BTreeSet<String>,
    sprites: BTreeSet<String>,
    assets: BTreeSet<String>,
    value_sets: BTreeMap<String, Vec<String>>,
    object_axes: BTreeMap<String, Vec<String>>,
}

fn collect_completion_symbols(source: &str) -> CompletionSymbols {
    let mut symbols = CompletionSymbols::default();
    symbols.value_sets.insert(
        "directions".to_string(),
        ["up", "down", "left", "right"]
            .into_iter()
            .map(str::to_string)
            .collect(),
    );
    symbols.value_sets.insert(
        "horizontal".to_string(),
        ["left", "right"].into_iter().map(str::to_string).collect(),
    );
    symbols.value_sets.insert(
        "vertical".to_string(),
        ["up", "down"].into_iter().map(str::to_string).collect(),
    );
    symbols.directions.extend(
        ["up", "down", "left", "right"]
            .into_iter()
            .map(str::to_string),
    );
    symbols.value_set_names.extend(
        ["directions", "horizontal", "vertical"]
            .into_iter()
            .map(str::to_string),
    );
    add_builtin_effect_commands(&mut symbols.effects, &mut symbols.emissions);

    if let Ok(game) = crate::parse_game2d(source) {
        symbols.objects.extend(game.object_labels.values().cloned());
        symbols.inputs.extend(game.input_labels.values().cloned());
        symbols.states.extend(game.global_labels.values().cloned());
        symbols.queries.extend(game.query_labels.values().cloned());
        symbols.queries.extend(game.conditions.keys().cloned());
        symbols
            .scenes
            .extend(game.scenes.iter().map(|scene| scene.name.clone()));
        symbols
            .levels
            .extend(game.levels.iter().map(|level| level.name.clone()));
        symbols
            .sfx
            .extend(game.sounds.sfx.iter().map(|sfx| sfx.name.clone()));
        symbols
            .music
            .extend(game.sounds.music.iter().map(|music| music.name.clone()));
        symbols.sprites.extend(
            game.visuals
                .sprites
                .iter()
                .map(|sprite| sprite.name.clone()),
        );
    }

    let context = scan_source_context(source);
    for line in context.lines {
        let tokens = line.tokens.iter().map(String::as_str).collect::<Vec<_>>();
        if tokens.is_empty() {
            continue;
        }
        collect_line_symbols(&tokens, line.scope, &mut symbols);
    }
    for name in symbols.value_set_names.clone() {
        symbols.variants.remove(&name);
    }

    symbols
}

fn collect_line_symbols(
    tokens: &[&str],
    scope: Option<SourceScope>,
    symbols: &mut CompletionSymbols,
) {
    match tokens {
        ["puzzle", name, ..] | ["model", "puzzle", name, ..] => {
            insert_identifier(&mut symbols.puzzles, name);
        }
        ["scene", name, ..] => {
            insert_identifier(&mut symbols.scenes, name);
        }
        ["level", name, ..] => {
            insert_identifier(&mut symbols.levels, name);
        }
        ["routine", "display", name, ..] | ["rule", "display", name, ..] => {
            insert_identifier(&mut symbols.routines, name);
        }
        ["routine", name, ..] | ["rule", name, ..] => {
            insert_identifier(&mut symbols.routines, name);
        }
        ["command" | "input", name, ..] | ["direction", name, ..] => {
            insert_identifier(&mut symbols.inputs, name);
        }
        ["query", name, ..] => {
            insert_identifier(&mut symbols.queries, name);
        }
        ["sfx", name, ..] if scope == Some(SourceScope::Sounds) => {
            insert_identifier(&mut symbols.sfx, name);
        }
        ["music", name, ..] if scope == Some(SourceScope::Sounds) => {
            insert_identifier(&mut symbols.music, name);
        }
        ["shape", table, ..] | ["colors", table, ..] => {
            if let Some((name, axis)) = table.split_once(':') {
                insert_identifier(&mut symbols.sprites, name);
                insert_identifier(&mut symbols.variants, axis);
            }
        }
        ["object", spec, ..] if *spec != "=" => collect_object_spec(spec, symbols),
        ["var" | "const" | "global", name, ..]
        | ["persistent", "var" | "const", name, ..]
        | ["persistent", name, ..] => {
            insert_identifier(&mut symbols.states, name);
        }
        [name, "=", values @ ..]
            if scope == Some(SourceScope::Tags) && tag_set_tokens(name, values) =>
        {
            insert_identifier(&mut symbols.value_set_names, name);
            let values = values
                .iter()
                .filter(|value| is_identifier(value))
                .map(|value| (*value).to_string())
                .collect::<Vec<_>>();
            symbols
                .value_sets
                .insert((*name).to_string(), values.clone());
            symbols.variants.extend(values);
        }
        [name, "=", selectors @ ..] if scope == Some(SourceScope::Group) => {
            insert_identifier(&mut symbols.groups, name);
            collect_selector_specs(selectors, symbols);
        }
        [name, "=", selectors @ ..] if scope == Some(SourceScope::Layers) => {
            insert_identifier(&mut symbols.groups, name);
            collect_selector_specs(selectors, symbols);
        }
        ["each", spec, ..] if scope == Some(SourceScope::Layers) => {
            collect_object_spec(spec, symbols);
        }
        ["each", spec, ..] | ["display", spec, ..] if scope == Some(SourceScope::Objects) => {
            collect_object_spec(spec, symbols);
        }
        [specs @ ..] if scope == Some(SourceScope::Objects) => {
            for spec in specs {
                collect_object_spec(spec, symbols);
            }
        }
        [spec] if scope == Some(SourceScope::Scratch) => collect_scratch_spec(spec, symbols),
        [..] if scope == Some(SourceScope::Keys) => collect_keys(tokens, symbols),
        _ => {}
    }
}

fn collect_selector_specs(specs: &[&str], symbols: &mut CompletionSymbols) {
    for spec in specs {
        if matches!(*spec, "=" | "display" | "each") || is_completion_keyword(spec) {
            continue;
        }
        collect_object_spec(spec, symbols);
    }
}

fn collect_object_spec(spec: &str, symbols: &mut CompletionSymbols) {
    let cleaned = clean_spec(spec);
    let parts = cleaned.split(':').collect::<Vec<_>>();
    let Some(base) = parts.first().copied() else {
        return;
    };
    insert_identifier(&mut symbols.objects, base);
    if parts.len() > 1 {
        symbols.object_axes.insert(
            base.to_string(),
            parts[1..].iter().map(|part| (*part).to_string()).collect(),
        );
    }
    for part in &parts[1..] {
        insert_identifier(&mut symbols.variants, part);
    }
}

fn collect_scratch_spec(spec: &str, symbols: &mut CompletionSymbols) {
    let cleaned = clean_spec(spec);
    let (name, ty) = cleaned.split_once(':').unwrap_or((cleaned, ""));
    insert_identifier(&mut symbols.scratches, name);
    if !ty.is_empty() && ty != "int" {
        insert_identifier(&mut symbols.variants, ty);
    }
}

fn collect_keys(tokens: &[&str], symbols: &mut CompletionSymbols) {
    let Some(separator) = tokens.iter().position(|token| matches!(*token, "=" | "->")) else {
        return;
    };
    for token in &tokens[..separator] {
        insert_identifier(&mut symbols.inputs, token);
    }
    if let Some(command) = tokens.get(separator + 1) {
        insert_identifier(&mut symbols.commands, command);
    }
}

fn selector_axis_values(symbols: &CompletionSymbols, token: &str) -> Option<Vec<String>> {
    let (base, partial) = token.split_once(':')?;
    let axes = symbols.object_axes.get(base)?;
    let supplied = partial.split(':').count();
    let axis = axes.get(supplied.saturating_sub(1))?;
    let mut values = symbols.value_sets.get(axis).cloned()?;
    if !values.iter().any(|value| value == "_") {
        values.insert(0, "_".to_string());
    }
    Some(values)
}

fn completion_prefix(token: &str) -> &str {
    token.rsplit_once(':').map_or(token, |(_, tail)| tail)
}

fn add_slot_items(
    items: &mut Vec<CompletionItem>,
    symbols: &CompletionSymbols,
    slot: SemanticCompletionSlot,
) {
    match slot {
        SemanticCompletionSlot::Keywords(keywords) => {
            for keyword in keywords {
                items.push(CompletionItem {
                    label: (*keyword).to_string(),
                    kind: CompletionKind::Keyword,
                    insert_text: keyword_insert_text(keyword).to_string(),
                    detail: "keyword".to_string(),
                });
            }
        }
        SemanticCompletionSlot::Objects => add_named_items(
            items,
            symbols.objects.iter(),
            CompletionKind::Object,
            "object",
        ),
        SemanticCompletionSlot::Groups => add_named_items(
            items,
            symbols.groups.iter(),
            CompletionKind::Group,
            "selector",
        ),
        SemanticCompletionSlot::States => {
            add_named_items(items, symbols.states.iter(), CompletionKind::State, "state")
        }
        SemanticCompletionSlot::Scratches => add_named_items(
            items,
            symbols.scratches.iter(),
            CompletionKind::Scratch,
            "scratch",
        ),
        SemanticCompletionSlot::Variants => add_named_items(
            items,
            symbols.variants.iter(),
            CompletionKind::Variant,
            "tag",
        ),
        SemanticCompletionSlot::ValueSets => add_named_items(
            items,
            symbols.value_set_names.iter(),
            CompletionKind::ValueSet,
            "tags",
        ),
        SemanticCompletionSlot::Directions => add_named_items(
            items,
            symbols.directions.iter(),
            CompletionKind::Direction,
            "direction",
        ),
        SemanticCompletionSlot::Inputs => {
            add_named_items(items, symbols.inputs.iter(), CompletionKind::Input, "input")
        }
        SemanticCompletionSlot::Commands => add_named_items(
            items,
            symbols.commands.iter(),
            CompletionKind::Command,
            "command",
        ),
        SemanticCompletionSlot::Effects => add_named_items(
            items,
            symbols.effects.iter(),
            CompletionKind::Effect,
            "effect",
        ),
        SemanticCompletionSlot::Emissions => add_named_items(
            items,
            symbols.emissions.iter(),
            CompletionKind::Emission,
            "emission",
        ),
        SemanticCompletionSlot::Routines => add_named_items(
            items,
            symbols.routines.iter(),
            CompletionKind::Routine,
            "routine",
        ),
        SemanticCompletionSlot::Queries => add_named_items(
            items,
            symbols.queries.iter(),
            CompletionKind::Query,
            "query",
        ),
        SemanticCompletionSlot::Scenes => {
            add_named_items(items, symbols.scenes.iter(), CompletionKind::Scene, "scene")
        }
        SemanticCompletionSlot::Puzzles => add_named_items(
            items,
            symbols.puzzles.iter(),
            CompletionKind::Puzzle,
            "puzzle",
        ),
        SemanticCompletionSlot::Levels => {
            add_named_items(items, symbols.levels.iter(), CompletionKind::Level, "level")
        }
        SemanticCompletionSlot::SfxAssets => {
            add_named_items(items, symbols.sfx.iter(), CompletionKind::Sfx, "sfx")
        }
        SemanticCompletionSlot::MusicAssets => {
            add_named_items(items, symbols.music.iter(), CompletionKind::Music, "music")
        }
        SemanticCompletionSlot::Sprites => add_named_items(
            items,
            symbols.sprites.iter(),
            CompletionKind::Sprite,
            "sprite",
        ),
        SemanticCompletionSlot::Assets => {
            add_named_items(items, symbols.assets.iter(), CompletionKind::Asset, "asset")
        }
    }
}

fn add_named_items<'a>(
    items: &mut Vec<CompletionItem>,
    names: impl Iterator<Item = &'a String>,
    kind: CompletionKind,
    detail: &str,
) {
    for name in names {
        items.push(CompletionItem {
            label: name.clone(),
            kind,
            insert_text: name.clone(),
            detail: detail.to_string(),
        });
    }
}

fn keyword_insert_text(keyword: &str) -> &str {
    match keyword {
        "objects" | "layers" | "group" | "scratch" | "legend" | "rules" | "levels"
        | "resources" | "keys" | "tags" | "on_level_start" | "on_level_clear" | "on_display" => {
            keyword
        }
        _ => keyword,
    }
}

fn add_builtin_effect_commands(effects: &mut BTreeSet<String>, emissions: &mut BTreeSet<String>) {
    for (command, kind) in semantic_builtin_effect_commands() {
        match kind {
            SemanticKind::Emission => {
                emissions.insert(command.to_string());
            }
            SemanticKind::Effect => {
                effects.insert(command.to_string());
            }
            _ => {}
        }
    }
}

fn tag_set_tokens(name: &str, values: &[&str]) -> bool {
    is_identifier(name)
        && !is_completion_keyword(name)
        && !values.is_empty()
        && values.iter().all(|value| is_identifier(value))
}

fn clean_spec(spec: &str) -> &str {
    let spec = spec.trim_matches(|ch: char| matches!(ch, '[' | ']' | '(' | ')' | '|'));
    spec.split_once('{').map_or(spec, |(head, _)| head)
}

fn insert_identifier(target: &mut BTreeSet<String>, value: &str) {
    if is_identifier(value) && !is_completion_keyword(value) {
        target.insert(value.to_string());
    }
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '@' || first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| {
            ch == '_' || ch == ':' || ch == '.' || ch == '-' || ch.is_ascii_alphanumeric()
        })
}

fn push_json_number(out: &mut String, key: &str, value: usize) {
    out.push('"');
    out.push_str(key);
    out.push_str("\":");
    out.push_str(&value.to_string());
}

fn push_json_string(out: &mut String, key: &str, value: &str) {
    out.push('"');
    out.push_str(key);
    out.push_str("\":\"");
    escape_json_string(out, value);
    out.push('"');
}

fn escape_json_string(out: &mut String, value: &str) {
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CompletionKind, completion_list_json, suggest_source_completions};

    #[test]
    fn suggests_objects_by_prefix() {
        let source = r#"
title complete_objects
model puzzle board {
tags {
kind = A B
}
objects {
Player
Box:kind
}
rules {
[ Pl
}
}
"#;
        let cursor = source.find("[ Pl").unwrap() + "[ Pl".len();
        let list = suggest_source_completions(source, cursor);

        assert!(list.items.iter().any(|item| item.label == "Player"));
    }

    #[test]
    fn suggests_selector_axis_values_after_colon() {
        let source = r#"
title complete_variants
model puzzle board {
tags {
kind = A B
}
objects {
Box:kind
}
rules {
[ Box:
}
}
"#;
        let cursor = source.rfind("Box:").unwrap() + "Box:".len();
        let list = suggest_source_completions(source, cursor);

        assert!(list.items.iter().any(|item| item.label == "_"));
        assert!(list.items.iter().any(|item| item.label == "A"));
        assert!(list.items.iter().any(|item| item.label == "B"));
    }

    #[test]
    fn labels_tag_axes_and_values_without_duplicate_axis_values() {
        let source = r#"
title complete_tags
model puzzle board {
tags {
color = red blue
}
objects {
Box:color
}
rules {
for c in co
[ Box:r
}
}
"#;
        let axis_cursor = source.find("in co").unwrap() + "in co".len();
        let axis_list = suggest_source_completions(source, axis_cursor);
        assert!(axis_list.items.iter().any(|item| {
            item.label == "color" && item.kind == CompletionKind::ValueSet && item.detail == "tags"
        }));
        assert!(
            !axis_list
                .items
                .iter()
                .any(|item| { item.label == "color" && item.kind == CompletionKind::Variant })
        );

        let tag_cursor = source.find("[ Box:r").unwrap() + "[ Box:r".len();
        let tag_list = suggest_source_completions(source, tag_cursor);
        assert!(tag_list.items.iter().any(|item| {
            item.label == "red" && item.kind == CompletionKind::Variant && item.detail == "tag"
        }));

        let axis_json = completion_list_json(&axis_list);
        assert!(axis_json.contains(r#""label":"color","kind":"tags""#));
        assert!(!axis_json.contains(r#""label":"color","kind":"tag""#));
        let tag_json = completion_list_json(&tag_list);
        assert!(tag_json.contains(r#""label":"red","kind":"tag""#));
    }

    #[test]
    fn suggests_scene_names_after_goto() {
        let source = r#"
title complete_goto
scene title {
transitions {
start -> goto 
}
}
scene playing {
}
"#;
        let cursor = source.find("goto ").unwrap() + "goto ".len();
        let list = suggest_source_completions(source, cursor);

        assert!(list.items.iter().any(|item| item.label == "playing"));
    }

    #[test]
    fn suggests_sfx_with_specific_kind_after_sfx() {
        let source = r#"
title complete_sfx
sounds {
sfx clear seed=clear01 type=jump
music music_name seed=bgm01
}
scene playing {
rules {
win -> sfx c
}
}
"#;
        let cursor = source.rfind("sfx c").unwrap() + "sfx c".len();
        let list = suggest_source_completions(source, cursor);

        assert!(list.items.iter().any(|item| {
            item.label == "clear" && item.kind == CompletionKind::Sfx && item.detail == "sfx"
        }));
        assert!(!list.items.iter().any(|item| item.label == "music_name"));
    }

    #[test]
    fn sfx_definition_does_not_suggest_sfx_assets_after_sfx_keyword() {
        let source = r#"
title complete_sounds_sfx
sounds {
sfx clear seed=clear01 type=jump
sfx c
}
"#;
        let cursor = source.rfind("sfx c").unwrap() + "sfx c".len();
        let list = suggest_source_completions(source, cursor);

        assert!(
            !list
                .items
                .iter()
                .any(|item| item.label == "clear" && item.kind == CompletionKind::Sfx)
        );
    }

    #[test]
    fn sounds_scope_suggests_sfx_as_sounds_keyword() {
        let source = r#"
title complete_sounds_keyword
sounds {
s
}
"#;
        let cursor = source.rfind("\ns\n").unwrap() + "\ns".len();
        let list = suggest_source_completions(source, cursor);

        assert!(
            list.items
                .iter()
                .any(|item| item.label == "sfx" && item.kind == CompletionKind::Keyword)
        );
        assert!(
            !list
                .items
                .iter()
                .any(|item| item.label == "sfx" && item.kind == CompletionKind::Emission)
        );
    }

    #[test]
    fn builtin_presentation_commands_are_emissions_not_commands() {
        let source = r#"
title complete_emissions
sounds {
sfx clear seed=clear01 type=jump
}
scene playing {
rules {
win -> s
}
}
"#;
        let cursor = source.find("win -> s").unwrap() + "win -> s".len();
        let list = suggest_source_completions(source, cursor);

        assert!(list.items.iter().any(|item| {
            item.label == "sfx"
                && item.kind == CompletionKind::Emission
                && item.detail == "emission"
        }));
        assert!(
            !list
                .items
                .iter()
                .any(|item| item.label == "sfx" && item.kind == CompletionKind::Command)
        );
        assert!(
            !list
                .items
                .iter()
                .any(|item| item.label == "sfx" && item.kind == CompletionKind::Keyword)
        );
    }

    #[test]
    fn builtin_model_commands_are_effects_not_commands() {
        let source = r#"
title complete_effects
model puzzle board {
objects {
Player
}
rules {
[ Player ] -> n
}
}
"#;
        let cursor = source.find("-> n").unwrap() + "-> n".len();
        let list = suggest_source_completions(source, cursor);

        assert!(list.items.iter().any(|item| {
            item.label == "next_level"
                && item.kind == CompletionKind::Effect
                && item.detail == "effect"
        }));
        assert!(
            !list
                .items
                .iter()
                .any(|item| item.label == "next_level" && item.kind == CompletionKind::Command)
        );
    }

    #[test]
    fn distinguishes_value_sets_and_puzzles() {
        let source = r#"
title complete_kinds
model puzzle sokoban {
tags {
kind = A B
}
objects {
Box:kind
}
rules {
for k in ki
}
}
scene playing {
view {
board = puzzle so
}
}
"#;
        let value_cursor = source.find("in ki").unwrap() + "in ki".len();
        let value_list = suggest_source_completions(source, value_cursor);
        assert!(
            value_list
                .items
                .iter()
                .any(|item| { item.label == "kind" && item.kind == CompletionKind::ValueSet })
        );

        let puzzle_cursor = source.rfind("puzzle so").unwrap() + "puzzle so".len();
        let puzzle_list = suggest_source_completions(source, puzzle_cursor);
        assert!(
            puzzle_list
                .items
                .iter()
                .any(|item| { item.label == "sokoban" && item.kind == CompletionKind::Puzzle })
        );
    }

    #[test]
    fn layer_names_are_single_selector_completions() {
        let source = r#"
title complete_layer_selectors
model puzzle board {
objects {
Player
Goal
}
layers {
floor = Goal
actor = Player
}
rules {
[ Player | no flo
}
}
"#;
        let cursor = source.find("no flo").unwrap() + "no flo".len();
        let list = suggest_source_completions(source, cursor);
        let floor_items = list
            .items
            .iter()
            .filter(|item| item.label == "floor")
            .collect::<Vec<_>>();

        assert_eq!(floor_items.len(), 1);
        assert_eq!(floor_items[0].kind, CompletionKind::Group);
        assert_eq!(floor_items[0].detail, "selector");
    }
}
