use std::collections::{BTreeMap, BTreeSet};

use crate::semantic::{
    SemanticCompletionSlot, SemanticKind, SettingCompletionSet, is_completion_keyword,
    semantic_builtin_effect_commands, semantic_completion_context,
};
use crate::source::{SourceScope, scan_source_context};
use crate::{THEME_PRESET_NAMES, THEME_SETTING_SPECS};

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
    Literal,
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
    Condition,
    Puzzle,
    Scene,
    Level,
    Sfx,
    Music,
    Sprite,
    Asset,
    Setting,
    Theme,
}

impl CompletionKind {
    fn as_str(self) -> &'static str {
        match self {
            CompletionKind::Keyword => "keyword",
            CompletionKind::Literal => "literal",
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
            CompletionKind::Condition => "condition",
            CompletionKind::Puzzle => "puzzle",
            CompletionKind::Scene => "scene",
            CompletionKind::Level => "level",
            CompletionKind::Sfx => "sfx",
            CompletionKind::Music => "music",
            CompletionKind::Sprite => "sprite",
            CompletionKind::Asset => "asset",
            CompletionKind::Setting => "setting",
            CompletionKind::Theme => "theme",
        }
    }
}

pub fn suggest_source_completions(source: &str, cursor_offset: usize) -> CompletionList {
    let context = semantic_completion_context(source, cursor_offset);
    let mut symbols = collect_completion_symbols(source);
    remove_current_token_symbols(&mut symbols, &context.token_text);

    let mut items = Vec::<CompletionItem>::new();

    if let Some(axis_items) = selector_axis_items(&symbols, &context.token_text) {
        items.extend(axis_items);
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
    direction_sets: BTreeSet<String>,
    inputs: BTreeSet<String>,
    commands: BTreeSet<String>,
    effects: BTreeSet<String>,
    emissions: BTreeSet<String>,
    routines: BTreeSet<String>,
    condition_defs: BTreeSet<String>,
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
    symbols.direction_sets.extend(
        ["directions", "horizontal", "vertical"]
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
        symbols
            .condition_defs
            .extend(game.condition_labels.values().cloned());
        symbols
            .condition_defs
            .extend(game.conditions.keys().cloned());
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
        ["puzzle", name, ..] => {
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
        ["input", name, ..] | ["direction", name, ..] => {
            insert_identifier(&mut symbols.inputs, name);
        }
        ["condition", name, ..] => {
            insert_identifier(&mut symbols.condition_defs, name);
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
        ["var" | "const", name, ..]
        | ["persistent", "var" | "const", name, ..]
        | ["persistent", name, ..] => {
            insert_identifier(&mut symbols.states, name);
        }
        [name, "=", ..] if scope == Some(SourceScope::SceneState) => {
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
            if values.iter().all(|value| is_direction_value(value)) {
                symbols.direction_sets.insert((*name).to_string());
            }
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
        [name, "=", ty] if scope == Some(SourceScope::Scratch) => {
            collect_scratch_spec(name, Some(*ty), symbols)
        }
        [spec] if scope == Some(SourceScope::Scratch) => {
            let cleaned = clean_spec(spec);
            let (name, ty) = cleaned
                .split_once('=')
                .map_or((cleaned, None), |(name, ty)| (name, Some(ty)));
            collect_scratch_spec(name, ty, symbols);
        }
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

fn collect_scratch_spec(name: &str, ty: Option<&str>, symbols: &mut CompletionSymbols) {
    insert_identifier(&mut symbols.scratches, name);
    if let Some(ty) = ty.filter(|ty| !matches!(*ty, "bool" | "int")) {
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

fn selector_axis_items(symbols: &CompletionSymbols, token: &str) -> Option<Vec<CompletionItem>> {
    let (base, partial) = token.split_once(':')?;
    let axes = symbols.object_axes.get(base)?;
    let supplied = partial.split(':').count();
    let axis = axes.get(supplied.saturating_sub(1))?;
    let axis_values = symbols.value_sets.get(axis)?;
    let mut items = Vec::new();

    if !axis_values.iter().any(|value| value == "_") {
        items.push(CompletionItem {
            label: "_".to_string(),
            kind: CompletionKind::Variant,
            insert_text: "_".to_string(),
            detail: "tag".to_string(),
        });
    }
    for value in axis_values {
        items.push(CompletionItem {
            label: value.clone(),
            kind: CompletionKind::Variant,
            insert_text: value.clone(),
            detail: "tag".to_string(),
        });
    }
    for (name, values) in &symbols.value_sets {
        if name == axis {
            continue;
        }
        if axis_values.iter().any(|value| value == name) {
            continue;
        }
        if values.iter().all(|value| axis_values.contains(value)) {
            items.push(CompletionItem {
                label: name.clone(),
                kind: CompletionKind::ValueSet,
                insert_text: name.clone(),
                detail: "tags".to_string(),
            });
        }
    }
    Some(items)
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
        SemanticCompletionSlot::Literals(literals) => {
            for literal in literals {
                items.push(CompletionItem {
                    label: (*literal).to_string(),
                    kind: CompletionKind::Literal,
                    insert_text: (*literal).to_string(),
                    detail: "literal".to_string(),
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
        SemanticCompletionSlot::DirectionSets => add_named_items(
            items,
            symbols.direction_sets.iter(),
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
        SemanticCompletionSlot::Conditions => add_named_items(
            items,
            symbols.condition_defs.iter(),
            CompletionKind::Condition,
            "condition",
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
        SemanticCompletionSlot::Themes => {
            for theme in THEME_PRESET_NAMES {
                items.push(CompletionItem {
                    label: (*theme).to_string(),
                    kind: CompletionKind::Theme,
                    insert_text: (*theme).to_string(),
                    detail: "theme".to_string(),
                });
            }
        }
        SemanticCompletionSlot::Settings(settings) => {
            let setting_names: Vec<&'static str> = match settings {
                SettingCompletionSet::Static(settings) => settings.iter().copied().collect(),
                SettingCompletionSet::Theme => THEME_SETTING_SPECS
                    .iter()
                    .map(|spec| spec.canonical)
                    .collect(),
            };
            for setting in setting_names {
                items.push(CompletionItem {
                    label: setting.to_string(),
                    kind: CompletionKind::Setting,
                    insert_text: setting.to_string(),
                    detail: "setting".to_string(),
                });
            }
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

fn remove_current_token_symbols(symbols: &mut CompletionSymbols, token: &str) {
    let name = clean_spec(token);
    if !is_identifier(name) {
        return;
    }
    symbols.objects.remove(name);
    symbols.groups.remove(name);
    symbols.states.remove(name);
    symbols.scratches.remove(name);
    symbols.value_set_names.remove(name);
    symbols.variants.remove(name);
    symbols.directions.remove(name);
    symbols.direction_sets.remove(name);
    symbols.inputs.remove(name);
    symbols.commands.remove(name);
    symbols.effects.remove(name);
    symbols.emissions.remove(name);
    symbols.routines.remove(name);
    symbols.condition_defs.remove(name);
    symbols.puzzles.remove(name);
    symbols.scenes.remove(name);
    symbols.levels.remove(name);
    symbols.sfx.remove(name);
    symbols.music.remove(name);
    symbols.sprites.remove(name);
    symbols.assets.remove(name);
    symbols.value_sets.remove(name);
    symbols.object_axes.remove(name);
}

fn keyword_insert_text(keyword: &str) -> &str {
    match keyword {
        "objects"
        | "layers"
        | "groups"
        | "scratch"
        | "legend"
        | "rules"
        | "levels"
        | "resources"
        | "keys"
        | "tags"
        | "on_level_start"
        | "on_level_clear"
        | "on_last_level_clear"
        | "on_display" => keyword,
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

fn is_direction_value(value: &str) -> bool {
    matches!(value, "up" | "down" | "left" | "right")
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
    use crate::syntax::PUZZLE_LIFECYCLE_BLOCKS;

    #[test]
    fn suggests_objects_by_prefix() {
        let source = r#"
title complete_objects
puzzle board {
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
    fn does_not_suggest_current_group_selector_token_as_object() {
        let source = r#"
title complete_group_objects
puzzle board {
objects {
Player
}
groups {
Actors = Pl
}
}
"#;
        let cursor = source.find("Actors = Pl").unwrap() + "Actors = Pl".len();
        let list = suggest_source_completions(source, cursor);

        assert!(list.items.iter().any(|item| item.label == "Player"));
        assert!(
            !list
                .items
                .iter()
                .any(|item| { item.kind == CompletionKind::Object && item.label == "Pl" })
        );
    }

    #[test]
    fn does_not_suggest_current_object_definition_token() {
        let source = r#"
title complete_object_definitions
puzzle board {
objects {
Player
Pl
}
}
"#;
        let cursor = source.rfind("Pl").unwrap() + "Pl".len();
        let list = suggest_source_completions(source, cursor);

        assert!(list.items.iter().any(|item| item.label == "Player"));
        assert!(
            !list
                .items
                .iter()
                .any(|item| { item.kind == CompletionKind::Object && item.label == "Pl" })
        );
    }

    #[test]
    fn does_not_suggest_current_layer_definition_token() {
        let source = r#"
title complete_layer_definitions
puzzle board {
objects {
Player
}
layers {
Pl
}
}
"#;
        let cursor = source.rfind("Pl").unwrap() + "Pl".len();
        let list = suggest_source_completions(source, cursor);

        assert!(list.items.iter().any(|item| item.label == "Player"));
        assert!(
            !list
                .items
                .iter()
                .any(|item| { item.kind == CompletionKind::Object && item.label == "Pl" })
        );
    }

    #[test]
    fn suggests_selector_axis_values_after_colon() {
        let source = r#"
title complete_variants
puzzle board {
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
puzzle board {
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
    fn labels_builtin_axes_by_completion_context() {
        let source = r#"
title complete_contextual_axes
puzzle board {
objects {
Player
Box:directions
}
rules {
for h in hori
once hori [
[ Player{hori
[ Box:hori
}
}
"#;
        let for_cursor = source.find("in hori").unwrap() + "in hori".len();
        let for_list = suggest_source_completions(source, for_cursor);
        assert!(
            for_list.items.iter().any(|item| {
                item.label == "horizontal" && item.kind == CompletionKind::ValueSet
            })
        );
        assert!(
            !for_list.items.iter().any(|item| {
                item.label == "horizontal" && item.kind == CompletionKind::Direction
            })
        );

        let rewrite_cursor = source.find("once hori").unwrap() + "once hori".len();
        let rewrite_list = suggest_source_completions(source, rewrite_cursor);
        assert!(
            rewrite_list.items.iter().any(|item| {
                item.label == "horizontal" && item.kind == CompletionKind::Direction
            })
        );
        assert!(
            !rewrite_list.items.iter().any(|item| {
                item.label == "horizontal" && item.kind == CompletionKind::ValueSet
            })
        );

        let scratch_cursor = source.find("Player{hori").unwrap() + "Player{hori".len();
        let scratch_list = suggest_source_completions(source, scratch_cursor);
        assert!(
            scratch_list.items.iter().any(|item| {
                item.label == "horizontal" && item.kind == CompletionKind::Direction
            })
        );
        assert!(
            !scratch_list.items.iter().any(|item| {
                item.label == "horizontal" && item.kind == CompletionKind::ValueSet
            })
        );

        let selector_cursor = source.find("Box:hori").unwrap() + "Box:hori".len();
        let selector_list = suggest_source_completions(source, selector_cursor);
        assert!(selector_list.items.iter().any(|item| {
            item.label == "horizontal"
                && item.kind == CompletionKind::ValueSet
                && item.detail == "tags"
        }));
        assert!(
            !selector_list.items.iter().any(|item| {
                item.label == "horizontal" && item.kind == CompletionKind::Direction
            })
        );
    }

    #[test]
    fn suggests_scene_names_after_goto() {
        let source = r#"
title complete_goto
scene title {
rules {
input start -> goto 
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
    fn suggests_level_flow_effect_commands() {
        let source = r#"
title complete_level_flow_effects
scene title {
layout {
button "Play" -> st
button "New Game" -> go
}
}
"#;
        let start_cursor = source.find("-> st").unwrap() + "-> st".len();
        let start_list = suggest_source_completions(source, start_cursor);
        assert!(
            start_list
                .items
                .iter()
                .any(|item| item.label == "start" && item.kind == CompletionKind::Effect)
        );

        let goto_cursor = source.find("-> go").unwrap() + "-> go".len();
        let goto_list = suggest_source_completions(source, goto_cursor);
        assert!(
            goto_list
                .items
                .iter()
                .any(|item| item.label == "goto" && item.kind == CompletionKind::Effect)
        );
    }

    #[test]
    fn line_head_suggests_scope_words_not_every_symbol() {
        let source = r#"
title complete_line_head
puzzle board {
objects {
Player
}
rules {

}
}
"#;
        let cursor = source.find("\n\n}").unwrap() + 1;
        let list = suggest_source_completions(source, cursor);

        assert!(
            list.items
                .iter()
                .any(|item| item.label == "if" && item.kind == CompletionKind::Keyword)
        );
        assert!(
            list.items
                .iter()
                .any(|item| item.label == "input" && item.kind == CompletionKind::Keyword)
        );
        assert!(
            !list
                .items
                .iter()
                .any(|item| item.label == "Player" && item.kind == CompletionKind::Object)
        );

        let prefix_source = source.replacen("\n\n}", "\nin\n}", 1);
        let prefix_cursor = prefix_source.find("\nin\n").unwrap() + "\nin".len();
        let prefix_list = suggest_source_completions(&prefix_source, prefix_cursor);
        assert!(
            prefix_list
                .items
                .iter()
                .any(|item| item.label == "input" && item.kind == CompletionKind::Keyword)
        );
    }

    #[test]
    fn arrow_position_suggests_effect_words_only() {
        let source = r#"
title complete_arrow_position
puzzle board {
objects {
Player
}
rules {
[ Player ] -> 
}
}
"#;
        let cursor = source.find("-> ").unwrap() + "-> ".len();
        let list = suggest_source_completions(source, cursor);

        assert!(
            list.items
                .iter()
                .any(|item| item.label == "next_level" && item.kind == CompletionKind::Effect)
        );
        assert!(
            !list
                .items
                .iter()
                .any(|item| item.label == "Player" && item.kind == CompletionKind::Object)
        );
    }

    #[test]
    fn scene_for_source_suggestions_are_scene_owned() {
        let source = r#"
title complete_scene_for_source
scene menu {
state {
items = 0
}
layout {
for item in 
}
}
"#;
        let cursor = source.find("in ").unwrap() + "in ".len();
        let list = suggest_source_completions(source, cursor);

        assert!(
            list.items
                .iter()
                .any(|item| item.label == "levels" && item.kind == CompletionKind::Keyword)
        );
        assert!(
            list.items
                .iter()
                .any(|item| item.label == "items" && item.kind == CompletionKind::State)
        );
        assert!(
            !list
                .items
                .iter()
                .any(|item| item.label == "directions" && item.kind == CompletionKind::ValueSet)
        );
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
    fn suggests_3d_render_options_from_parser_names() {
        let source = r#"
title complete_3d_render_options
puzzle3 board {
layers {
actor
}
objects {
Player actor
}
render {
camera {
ya
}
}
rules {
}
}
"#;
        let camera_cursor = source.find("ya").unwrap() + "ya".len();
        let camera_list = suggest_source_completions(source, camera_cursor);
        assert!(camera_list.items.iter().any(|item| {
            item.label == "yaw" && item.kind == CompletionKind::Setting && item.detail == "setting"
        }));

        let render_cursor = source.find("camera {").unwrap();
        let render_list = suggest_source_completions(source, render_cursor);
        assert!(
            render_list
                .items
                .iter()
                .any(|item| item.label == "camera" && item.kind == CompletionKind::Setting)
        );
    }

    #[test]
    fn suggests_contextual_option_names_only_in_owned_blocks() {
        let source = r#"
title complete_contextual_options
sounds {
sfx click se
music bgm he
}
animation {
tween du
}
scene menu {
layout {
level_menu {
show_
}
}
}
"#;
        let sfx_cursor = source.find("sfx click se").unwrap() + "sfx click se".len();
        let sfx_list = suggest_source_completions(source, sfx_cursor);
        assert!(
            sfx_list
                .items
                .iter()
                .any(|item| item.label == "seed" && item.kind == CompletionKind::Setting)
        );
        assert!(
            !sfx_list
                .items
                .iter()
                .any(|item| item.label == "tone" && item.kind == CompletionKind::Setting)
        );

        let music_cursor = source.find("music bgm he").unwrap() + "music bgm he".len();
        let music_list = suggest_source_completions(source, music_cursor);
        assert!(
            music_list
                .items
                .iter()
                .any(|item| item.label == "height" && item.kind == CompletionKind::Setting)
        );

        let tween_cursor = source.find("tween du").unwrap() + "tween du".len();
        let tween_list = suggest_source_completions(source, tween_cursor);
        assert!(
            tween_list
                .items
                .iter()
                .any(|item| item.label == "duration" && item.kind == CompletionKind::Setting)
        );

        let menu_cursor = source.find("show_").unwrap() + "show_".len();
        let menu_list = suggest_source_completions(source, menu_cursor);
        assert!(
            menu_list
                .items
                .iter()
                .any(|item| item.label == "show_index" && item.kind == CompletionKind::Setting)
        );

        let scene_cursor = source.find("scene menu").unwrap() + "scene menu".len();
        let scene_list = suggest_source_completions(source, scene_cursor);
        assert!(
            !scene_list
                .items
                .iter()
                .any(|item| item.label == "duration" && item.kind == CompletionKind::Setting)
        );
    }

    #[test]
    fn suggests_theme_names_after_theme_keyword() {
        let source = r#"
title complete_theme_names
theme p
"#;
        let cursor = source.find("theme p").unwrap() + "theme p".len();
        let list = suggest_source_completions(source, cursor);

        assert!(
            list.items
                .iter()
                .any(|item| item.label == "pixel" && item.kind == CompletionKind::Theme)
        );
        assert!(
            list.items
                .iter()
                .any(|item| item.label == "paper" && item.kind == CompletionKind::Theme)
        );
    }

    #[test]
    fn suggests_theme_settings_inside_theme_block() {
        let source = r#"
title complete_theme_settings
theme clean {

}
"#;
        let cursor = source.find("\n\n").unwrap() + 1;
        let list = suggest_source_completions(source, cursor);

        assert!(list.items.iter().any(|item| {
            item.label == "background_color" && item.kind == CompletionKind::Setting
        }));
        assert!(
            list.items
                .iter()
                .any(|item| { item.label == "text_color" && item.kind == CompletionKind::Setting })
        );
        assert!(
            list.items.iter().any(|item| {
                item.label == "accent_color" && item.kind == CompletionKind::Setting
            })
        );
        assert!(list.items.iter().all(|item| item.label != "ui_font"));
        assert!(list.items.iter().all(|item| item.label != "board_color"));
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
    fn builtin_presentation_commands_are_effects_not_commands() {
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
            item.label == "sfx" && item.kind == CompletionKind::Effect && item.detail == "effect"
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
puzzle board {
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
    fn suggests_boolean_literals() {
        let source = r#"
title complete_boolean_literals
puzzle board {
var enabled = tr
rules {
if enabled == fa
}
}
"#;
        let true_cursor = source.find("= tr").unwrap() + "= tr".len();
        let true_list = suggest_source_completions(source, true_cursor);
        assert!(true_list.items.iter().any(|item| {
            item.label == "true" && item.kind == CompletionKind::Literal && item.detail == "literal"
        }));

        let false_cursor = source.find("== fa").unwrap() + "== fa".len();
        let false_list = suggest_source_completions(source, false_cursor);
        assert!(false_list.items.iter().any(|item| {
            item.label == "false"
                && item.kind == CompletionKind::Literal
                && item.detail == "literal"
        }));
    }

    #[test]
    fn does_not_suggest_removed_command_keyword() {
        let source = r#"
title complete_removed_command
puzzle board {
co
}
"#;
        let cursor = source.find("co").unwrap() + "co".len();
        let list = suggest_source_completions(source, cursor);

        assert!(
            !list
                .items
                .iter()
                .any(|item| item.label == "command" && item.kind == CompletionKind::Keyword)
        );
    }

    #[test]
    fn does_not_suggest_removed_global_keyword() {
        let top_level = "g";
        let top_level_list = suggest_source_completions(top_level, top_level.len());
        assert!(
            !top_level_list
                .items
                .iter()
                .any(|item| item.label == "global" && item.kind == CompletionKind::Keyword)
        );

        let puzzle_source = r#"
title complete_removed_global
puzzle board {
g
}
"#;
        let puzzle_cursor = puzzle_source.find("\ng").unwrap() + "\ng".len();
        let puzzle_list = suggest_source_completions(puzzle_source, puzzle_cursor);
        assert!(
            !puzzle_list
                .items
                .iter()
                .any(|item| item.label == "global" && item.kind == CompletionKind::Keyword)
        );
    }

    #[test]
    fn suggests_all_puzzle_lifecycle_blocks() {
        let source = r#"
title complete_lifecycle
puzzle board {
on_
}
"#;
        let cursor = source.find("on_").unwrap() + "on_".len();
        let list = suggest_source_completions(source, cursor);

        for keyword in PUZZLE_LIFECYCLE_BLOCKS {
            assert!(
                list.items
                    .iter()
                    .any(|item| item.label == *keyword && item.kind == CompletionKind::Keyword),
                "missing lifecycle completion {keyword}"
            );
        }
    }

    #[test]
    fn distinguishes_value_sets_and_puzzles() {
        let source = r#"
title complete_kinds
puzzle sokoban {
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
layout {
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
puzzle board {
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
