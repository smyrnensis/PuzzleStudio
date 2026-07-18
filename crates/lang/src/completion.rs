use std::collections::BTreeSet;

use crate::THEME_PRESET_NAMES;
use crate::semantic::{SemanticCompletionSlot, SettingCompletionSet};
use crate::surface::SurfaceCompletionSymbols;
use crate::surface_completion::surface_completion_context_for_document;
use crate::syntax::VISUAL_NAMED_COLORS;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletionList {
    pub replace_start: usize,
    pub replace_end: usize,
    pub items: Vec<CompletionItem>,
    pub warnings: Vec<String>,
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
    Mark,
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
    Shape,
    Setting,
    Theme,
    Color,
}

impl CompletionKind {
    fn as_str(self) -> &'static str {
        match self {
            CompletionKind::Keyword => "keyword",
            CompletionKind::Literal => "literal",
            CompletionKind::Object => "object",
            CompletionKind::Group => "group",
            CompletionKind::State => "state",
            CompletionKind::Mark => "mark",
            CompletionKind::ValueSet => "tags",
            CompletionKind::Variant => "tag",
            CompletionKind::Direction => "direction",
            CompletionKind::Input => "input",
            CompletionKind::Command => "command",
            CompletionKind::Effect => "effect",
            CompletionKind::Emission => "emission",
            CompletionKind::Routine => "routine",
            CompletionKind::Condition => "query",
            CompletionKind::Puzzle => "puzzle",
            CompletionKind::Scene => "scene",
            CompletionKind::Level => "level",
            CompletionKind::Sfx => "sfx",
            CompletionKind::Music => "music",
            CompletionKind::Sprite => "sprite",
            CompletionKind::Asset => "asset",
            CompletionKind::Shape => "shape",
            CompletionKind::Setting => "setting",
            CompletionKind::Theme => "theme",
            CompletionKind::Color => "color",
        }
    }
}

pub fn suggest_source_completions(source: &str, cursor_offset: usize) -> CompletionList {
    let context_document = crate::parse_surface_completion_context_document(source);
    let context = surface_completion_context_for_document(source, cursor_offset, &context_document);
    if completion_context_requires_symbols(&context) {
        let symbols_document = crate::parse_surface_completion_symbols_document(source);
        return completion_list_from_context(
            source,
            cursor_offset,
            context,
            symbols_document.completion_symbols,
        );
    }
    completion_list_from_context(
        source,
        cursor_offset,
        context,
        SurfaceCompletionSymbols::default(),
    )
}

pub(crate) fn completion_list_from_context(
    source: &str,
    cursor_offset: usize,
    context: crate::semantic::SemanticCompletionContext,
    mut symbols: SurfaceCompletionSymbols,
) -> CompletionList {
    remove_current_token_symbols(&mut symbols, &context.token_text);

    let mut items = Vec::<CompletionItem>::new();

    if let Some(axis_items) = selector_axis_items(&symbols, &context.token_text) {
        items.extend(axis_items);
    } else {
        for slot in &context.slots {
            add_slot_items(&mut items, &symbols, *slot);
        }
    }

    let replace_start = selector_tag_replace_start(context.replace_start, &context.token_text);
    let replace_end = selector_object_replace_end(
        source,
        cursor_offset,
        context.replace_end,
        &context.token_text,
    );
    let current_typed_replacement = source
        .get(replace_start..cursor_offset.min(replace_end))
        .unwrap_or_default()
        .to_string();

    let prefix = completion_prefix(&context.token_text);
    let mut seen = BTreeSet::<(String, CompletionKind)>::new();
    items.retain(|item| {
        if !prefix.is_empty() && !item.label.starts_with(prefix) {
            return false;
        }
        if context.token_text.contains(':')
            && !current_typed_replacement.is_empty()
            && item.label == current_typed_replacement
            && item.insert_text == current_typed_replacement
        {
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
        replace_start,
        replace_end,
        items,
        warnings: context.warnings,
    }
}

pub(crate) fn completion_context_requires_symbols(
    context: &crate::semantic::SemanticCompletionContext,
) -> bool {
    context.token_text.contains(':') || context.slots.iter().any(slot_requires_symbols)
}

fn slot_requires_symbols(slot: &SemanticCompletionSlot) -> bool {
    match slot {
        SemanticCompletionSlot::Keywords(_)
        | SemanticCompletionSlot::ModelTopLevelKeywords
        | SemanticCompletionSlot::Literals(_)
        | SemanticCompletionSlot::Themes
        | SemanticCompletionSlot::AuthoringRows(_)
        | SemanticCompletionSlot::AuthoringChildren(_)
        | SemanticCompletionSlot::AuthoringContentRows(_)
        | SemanticCompletionSlot::Settings(_) => false,
        SemanticCompletionSlot::Objects
        | SemanticCompletionSlot::Groups
        | SemanticCompletionSlot::States
        | SemanticCompletionSlot::Markes
        | SemanticCompletionSlot::ObjectNameAtoms
        | SemanticCompletionSlot::ValueSets
        | SemanticCompletionSlot::Directions
        | SemanticCompletionSlot::DirectionSets
        | SemanticCompletionSlot::Inputs
        | SemanticCompletionSlot::ModelEffects
        | SemanticCompletionSlot::SceneEffects
        | SemanticCompletionSlot::Emissions
        | SemanticCompletionSlot::Routines
        | SemanticCompletionSlot::Conditions
        | SemanticCompletionSlot::Scenes
        | SemanticCompletionSlot::Puzzles
        | SemanticCompletionSlot::SfxAssets
        | SemanticCompletionSlot::MusicAssets
        | SemanticCompletionSlot::Sprites
        | SemanticCompletionSlot::Assets
        | SemanticCompletionSlot::Shapes
        | SemanticCompletionSlot::Colors => true,
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
    out.push_str("],\"warnings\":[");
    for (index, warning) in list.warnings.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_string(&mut out, "message", warning);
        out.push('}');
    }
    out.push_str("]}");
    out
}

fn selector_axis_items(
    symbols: &SurfaceCompletionSymbols,
    token: &str,
) -> Option<Vec<CompletionItem>> {
    let (base, partial) = token.split_once(':')?;
    let axes = symbols.object_axes.get(base)?;
    let supplied = partial.split(':').count();
    let axis = axes.get(supplied.saturating_sub(1))?;
    let axis_values = symbols.value_sets.get(axis)?;
    let current_tag = partial.rsplit(':').next().unwrap_or_default();
    let mut items = Vec::new();

    for literal in puzzle_authoring::SELECTOR_TAG_SYNTAX_LITERALS {
        let token = literal.token();
        if current_tag == token {
            continue;
        }
        items.push(CompletionItem {
            label: token.to_string(),
            kind: CompletionKind::Literal,
            insert_text: token.to_string(),
            detail: match literal {
                puzzle_authoring::SelectorTagSyntaxLiteral::Wildcard => "wildcard",
            }
            .to_string(),
        });
    }
    for value in axis_values
        .iter()
        .filter(|value| !puzzle_authoring::is_selector_tag_syntax_literal(value))
    {
        items.push(CompletionItem {
            label: value.clone(),
            kind: CompletionKind::Object,
            insert_text: value.clone(),
            detail: "object".to_string(),
        });
    }
    for (name, values) in &symbols.value_sets {
        if name == axis {
            continue;
        }
        if name == current_tag {
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

fn selector_tag_replace_start(token_start: usize, token: &str) -> usize {
    token
        .rsplit_once(':')
        .map_or(token_start, |(head, _)| token_start + head.len() + 1)
}

fn selector_object_replace_end(
    source: &str,
    cursor_offset: usize,
    token_end: usize,
    token: &str,
) -> usize {
    let cursor = cursor_offset.min(source.len());
    let token_end = token_end.min(source.len());
    if token.contains(':') || cursor >= token_end {
        return token_end;
    }
    source[cursor..token_end]
        .find(':')
        .map_or(token_end, |colon| cursor + colon)
}

fn add_slot_items(
    items: &mut Vec<CompletionItem>,
    symbols: &SurfaceCompletionSymbols,
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
        SemanticCompletionSlot::ModelTopLevelKeywords => {
            for keyword in crate::model_top_level_completion_keywords() {
                items.push(CompletionItem {
                    label: keyword.to_string(),
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
        SemanticCompletionSlot::Markes => {
            add_named_items(items, symbols.markes.iter(), CompletionKind::Mark, "mark")
        }
        SemanticCompletionSlot::ObjectNameAtoms => add_named_items(
            items,
            symbols.object_name_atoms.iter(),
            CompletionKind::Object,
            "object",
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
        SemanticCompletionSlot::ModelEffects => add_named_items(
            items,
            symbols.model_effects.iter(),
            CompletionKind::Effect,
            "effect",
        ),
        SemanticCompletionSlot::SceneEffects => add_named_items(
            items,
            symbols.scene_effects.iter(),
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
        SemanticCompletionSlot::Shapes => {
            add_named_items(items, symbols.shapes.iter(), CompletionKind::Shape, "shape")
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
        SemanticCompletionSlot::Colors => {
            for (color, _) in VISUAL_NAMED_COLORS {
                items.push(CompletionItem {
                    label: (*color).to_string(),
                    kind: CompletionKind::Color,
                    insert_text: (*color).to_string(),
                    detail: "color".to_string(),
                });
            }
            add_named_items(items, symbols.colors.iter(), CompletionKind::Color, "color");
        }
        SemanticCompletionSlot::AuthoringRows(kind) => {
            add_keyword_items(
                items,
                crate::authoring_grammar::authoring_row_surfaces(kind),
            );
        }
        SemanticCompletionSlot::AuthoringChildren(kind) => {
            add_keyword_items(
                items,
                crate::authoring_grammar::authoring_child_surfaces(kind),
            );
        }
        SemanticCompletionSlot::AuthoringContentRows(content) => {
            add_authoring_content_row_items(items, content);
        }
        SemanticCompletionSlot::Settings(settings) => {
            let setting_names: Vec<String> = match settings {
                SettingCompletionSet::Static(settings) => settings
                    .iter()
                    .map(|setting| (*setting).to_string())
                    .collect(),
                SettingCompletionSet::AuthoringDefinitions(kind) => {
                    crate::authoring_grammar::authoring_definition_surfaces(kind)
                        .iter()
                        .map(|setting| (*setting).to_string())
                        .collect()
                }
            };
            for setting in setting_names {
                items.push(CompletionItem {
                    label: setting.clone(),
                    kind: CompletionKind::Setting,
                    insert_text: setting,
                    detail: "setting".to_string(),
                });
            }
        }
    }
}

fn add_keyword_items(items: &mut Vec<CompletionItem>, keywords: Vec<&'static str>) {
    for keyword in keywords {
        items.push(CompletionItem {
            label: keyword.to_string(),
            kind: CompletionKind::Keyword,
            insert_text: keyword.to_string(),
            detail: "keyword".to_string(),
        });
    }
}

fn add_authoring_content_row_items(
    items: &mut Vec<CompletionItem>,
    content: crate::authoring_grammar::AuthoringContentKind,
) {
    for row in crate::authoring_grammar::authoring_content_row_surfaces(content) {
        let (kind, detail) = match row.role {
            crate::authoring_grammar::AuthoringSurfaceRole::Setting => {
                (CompletionKind::Setting, "setting")
            }
            _ => (CompletionKind::Keyword, "keyword"),
        };
        items.push(CompletionItem {
            label: row.surface.to_string(),
            kind,
            insert_text: row.surface.to_string(),
            detail: detail.to_string(),
        });
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

fn remove_current_token_symbols(symbols: &mut SurfaceCompletionSymbols, token: &str) {
    let name = clean_spec(token);
    if !is_identifier(name) {
        return;
    }
    symbols.objects.remove(name);
    symbols.groups.remove(name);
    symbols.states.remove(name);
    symbols.markes.remove(name);
    symbols.value_set_names.remove(name);
    symbols.object_name_atoms.remove(name);
    symbols.directions.remove(name);
    symbols.direction_sets.remove(name);
    symbols.inputs.remove(name);
    symbols.commands.remove(name);
    symbols.effects.remove(name);
    symbols.model_effects.remove(name);
    symbols.scene_effects.remove(name);
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
    symbols.shapes.remove(name);
    symbols.colors.remove(name);
    symbols.value_sets.remove(name);
    symbols.object_axes.remove(name);
}

fn keyword_insert_text(keyword: &str) -> &str {
    match keyword {
        "slots"
        | "groups"
        | "marks"
        | "legend"
        | "rules"
        | "levels"
        | "resources"
        | "keys"
        | "tags"
        | "on_level_start"
        | "on_level_clear"
        | "on_last_level_clear" => keyword,
        _ => keyword,
    }
}

fn clean_spec(spec: &str) -> &str {
    let spec = spec.trim_matches(|ch: char| matches!(ch, '[' | ']' | '(' | ')' | '|'));
    spec.split_once('{').map_or(spec, |(head, _)| head)
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
    use crate::syntax::{PUZZLE_COMPLETION_KEYWORDS, PUZZLE_LIFECYCLE_BLOCKS};

    #[test]
    fn suggests_objects_by_prefix() {
        let source = r#"
title = complete_objects
puzzle board {
tags {
kind = A B
}
slots {
__legacy_layer_0 = Player
__legacy_layer_1 = Box:kind
}
rules {
[ Pl
}
}
"#;
        let cursor = source.find("[ Pl").unwrap() + "[ Pl".len();
        let list = suggest_source_completions(&source, cursor);

        assert!(list.items.iter().any(|item| item.label == "Player"));
    }

    #[test]
    fn suggests_selector_rhs_objects_after_assignment() {
        let source = r#"
title = complete_selector_rhs
puzzle board {
slots {
__legacy_layer_0 = Player
__legacy_layer_1 = Box
}
groups {
Actors = Player Box
Movers =
}
legend {
A =
B = Pl
}
}
"#;
        let empty_legend_cursor = source.find("A =").unwrap() + "A =".len();
        let empty_legend_list = suggest_source_completions(source, empty_legend_cursor);
        assert!(
            empty_legend_list
                .items
                .iter()
                .any(|item| item.label == "Player" && item.kind == CompletionKind::Object)
        );
        assert!(
            empty_legend_list
                .items
                .iter()
                .any(|item| item.label == "Actors" && item.kind == CompletionKind::Group)
        );
        assert!(
            empty_legend_list
                .items
                .iter()
                .any(|item| item.label == "empty" && item.kind == CompletionKind::Keyword)
        );

        let prefixed_legend_cursor = source.find("B = Pl").unwrap() + "B = Pl".len();
        let prefixed_legend_list = suggest_source_completions(source, prefixed_legend_cursor);
        assert!(
            prefixed_legend_list
                .items
                .iter()
                .any(|item| item.label == "Player" && item.kind == CompletionKind::Object)
        );

        let group_cursor = source.find("Movers =").unwrap() + "Movers =".len();
        let group_list = suggest_source_completions(source, group_cursor);
        assert!(
            group_list
                .items
                .iter()
                .any(|item| item.label == "Player" && item.kind == CompletionKind::Object)
        );
        assert!(
            group_list
                .items
                .iter()
                .any(|item| item.label == "Actors" && item.kind == CompletionKind::Group)
        );
        assert!(group_list.items.iter().all(|item| item.label != "empty"));
    }

    #[test]
    fn suggests_at_prefixed_objects_from_unparsed_layer_rows() {
        let source = r#"
title = complete_at_prefixed_objects
puzzle board {
slots {
@Count
@Badge
each @Spark @Flash
}
rules {
[ @
"#;
        let cursor = source.rfind("[ @").unwrap() + "[ @".len();
        let list = suggest_source_completions(&source, cursor);

        assert!(list.items.iter().any(|item| {
            item.label == "@Count" && item.kind == CompletionKind::Object && item.detail == "object"
        }));
        assert!(list.items.iter().any(|item| {
            item.label == "@Badge" && item.kind == CompletionKind::Object && item.detail == "object"
        }));
        assert!(list.items.iter().any(|item| {
            item.label == "@Spark" && item.kind == CompletionKind::Object && item.detail == "object"
        }));
        assert!(list.items.iter().any(|item| {
            item.label == "@Flash" && item.kind == CompletionKind::Object && item.detail == "object"
        }));
    }

    #[test]
    fn does_not_suggest_current_group_selector_token_as_object() {
        let source = r#"
title = complete_group_objects
puzzle board {
slots {
__legacy_layer_0 = Player
}
groups {
Actors = Pl
}
}
"#;
        let cursor = source.find("Actors = Pl").unwrap() + "Actors = Pl".len();
        let list = suggest_source_completions(&source, cursor);

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
title = complete_object_definitions
puzzle board {
slots {
__legacy_layer_0 = Player
__legacy_layer_1 = Pl
}
}
"#;
        let cursor = source.rfind("Pl").unwrap() + "Pl".len();
        let list = suggest_source_completions(&source, cursor);

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
title = complete_layer_definitions
puzzle board {
slots {
actor = Player
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
    fn selector_tag_completion_uses_parser_owned_literals() {
        let source = r#"
title = complete_variants
puzzle board {
tags {
kind = A B
}
slots {
actor = Box:kind
}
legend {
. = empty
X = Box:A
}
rules {
once [ Box: ] -> [ Box:A ]
}
level "start" {
X
}
}
"#;
        let cursor = source.find("[ Box: ]").unwrap() + "[ Box:".len();
        let list = suggest_source_completions(source, cursor);
        let wildcard = puzzle_authoring::SELECTOR_WILDCARD;

        assert!(list.items.iter().any(|item| {
            item.label == wildcard
                && item.insert_text == wildcard
                && item.kind == CompletionKind::Literal
        }));
        assert!(list.items.iter().any(|item| item.label == "A"));
        assert!(list.items.iter().any(|item| item.label == "B"));

        let mut completed = source.to_string();
        completed.insert_str(cursor, wildcard);
        crate::parse_game(&completed).expect("parser-owned selector literal must compile");
    }

    #[test]
    fn selector_tag_completion_replaces_only_tag_segment() {
        let source = r#"
title = complete_variants
puzzle board {
tags {
kind = Alpha Beta
}
slots {
__legacy_layer_0 = Box:kind
}
rules {
[ Box:A
}
}
"#;
        let cursor = source.rfind("Box:A").unwrap() + "Box:A".len();
        let list = suggest_source_completions(source, cursor);

        assert_eq!(
            source[list.replace_start..list.replace_end].to_string(),
            "A"
        );
        assert!(list.items.iter().any(|item| item.label == "Alpha"));
    }

    #[test]
    fn selector_object_completion_preserves_existing_tag_suffix() {
        let source = r#"
title = complete_selector_object
puzzle board {
tags {
kind = tag other
}
slots {
base = object:kind
}
rules {
[ obj:tag
}
}
"#;
        let cursor = source.rfind("obj:tag").unwrap() + "obj".len();
        let list = suggest_source_completions(source, cursor);

        assert_eq!(
            source[list.replace_start..list.replace_end].to_string(),
            "obj"
        );
        let item = list
            .items
            .iter()
            .find(|item| item.kind == CompletionKind::Object && item.label == "object")
            .expect("object completion");
        let completed = format!(
            "{}{}{}",
            &source[..list.replace_start],
            item.insert_text,
            &source[list.replace_end..]
        );
        assert!(completed.contains("[ object:tag"));
    }

    #[test]
    fn selector_tag_completion_excludes_current_tag_segment() {
        let source = r#"
title = complete_current_variant
puzzle board {
slots {
actor = Box:state
}
tags {
state = movable stack
}
groups {
solid = Box:*
fixed = Box:stack
}
rules {
[ Box:stack | Box:movable
}
}
"#;
        let empty_tag_cursor = source.find("Box:movable").unwrap() + "Box:".len();
        let empty_tag_list = suggest_source_completions(source, empty_tag_cursor);
        assert_eq!(
            source[empty_tag_list.replace_start..empty_tag_list.replace_end].to_string(),
            "movable"
        );
        assert!(
            empty_tag_list
                .items
                .iter()
                .any(|item| { item.kind == CompletionKind::Object && item.label == "movable" })
        );
        assert!(
            empty_tag_list
                .items
                .iter()
                .any(|item| { item.kind == CompletionKind::Object && item.label == "stack" })
        );

        let stack_cursor = source.find("Box:stack").unwrap() + "Box:stack".len();
        let stack_list = suggest_source_completions(source, stack_cursor);
        assert_eq!(
            source[stack_list.replace_start..stack_list.replace_end].to_string(),
            "stack"
        );
        assert!(
            stack_list
                .items
                .iter()
                .all(|item| { item.label != "stack" })
        );

        let movable_cursor = source.find("Box:movable").unwrap() + "Box:movable".len();
        let movable_list = suggest_source_completions(source, movable_cursor);
        assert_eq!(
            source[movable_list.replace_start..movable_list.replace_end].to_string(),
            "movable"
        );
        assert!(
            movable_list
                .items
                .iter()
                .all(|item| { item.label != "movable" })
        );
    }

    #[test]
    fn selector_tag_completion_does_not_suggest_current_untyped_axis() {
        let source = r#"
title = complete_current_axis
puzzle board {
slots {
actor = obj:dir
}
rules {
[ obj:dir
}
}
"#;
        let cursor = source.rfind("obj:dir").unwrap() + "obj:dir".len();
        let list = suggest_source_completions(source, cursor);

        assert_eq!(
            source[list.replace_start..list.replace_end].to_string(),
            "dir"
        );
        assert!(
            list.items
                .iter()
                .all(|item| { item.label != "dir" && item.insert_text != "dir" })
        );
    }

    #[test]
    fn labels_tag_axes_and_values_without_duplicate_axis_values() {
        let source = r#"
title = complete_tags
puzzle board {
tags {
color = red blue
}
slots {
__legacy_layer_0 = Box:color
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
                .any(|item| { item.label == "color" && item.kind == CompletionKind::Object })
        );

        let tag_cursor = source.find("[ Box:r").unwrap() + "[ Box:r".len();
        let tag_list = suggest_source_completions(source, tag_cursor);
        assert!(tag_list.items.iter().any(|item| {
            item.label == "red" && item.kind == CompletionKind::Object && item.detail == "object"
        }));

        let axis_json = completion_list_json(&axis_list);
        assert!(axis_json.contains(r#""label":"color","kind":"tags""#));
        assert!(!axis_json.contains(r#""label":"color","kind":"object""#));
        let tag_json = completion_list_json(&tag_list);
        assert!(tag_json.contains(r#""label":"red","kind":"object""#));
    }

    #[test]
    fn labels_builtin_axes_by_completion_context() {
        let source = r#"
title = complete_contextual_axes
puzzle board {
slots {
__legacy_layer_0 = Player
__legacy_layer_1 = Box:directions
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

        let mark_cursor = source.find("Player{hori").unwrap() + "Player{hori".len();
        let mark_list = suggest_source_completions(source, mark_cursor);
        assert!(
            mark_list.items.iter().any(|item| {
                item.label == "horizontal" && item.kind == CompletionKind::Direction
            })
        );
        assert!(
            !mark_list.items.iter().any(|item| {
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
title = complete_goto
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
title = complete_level_flow_effects
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
title = complete_line_head
puzzle board {
slots {
__legacy_layer_0 = Player
}
routine refresh once {
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
        assert!(
            list.items
                .iter()
                .any(|item| item.label == "refresh" && item.kind == CompletionKind::Routine)
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

        let move_source = source.replacen("\n\n}", "\nmo\n}", 1);
        let move_cursor = move_source.find("\nmo\n").unwrap() + "\nmo".len();
        let move_list = suggest_source_completions(&move_source, move_cursor);
        assert!(
            !move_list
                .items
                .iter()
                .any(|item| { item.label == "move" && item.detail == "standard rule step" })
        );

        let effect_source = source.replacen("\n\n}", "\nsf\n}", 1);
        let effect_cursor = effect_source.find("\nsf\n").unwrap() + "\nsf".len();
        let effect_list = suggest_source_completions(&effect_source, effect_cursor);
        assert!(
            effect_list
                .items
                .iter()
                .any(|item| item.label == "sfx" && item.kind == CompletionKind::Effect)
        );
    }

    #[test]
    fn line_head_suggests_rule_heads_in_statement_blocks() {
        let source = r#"
title = complete_statement_blocks
puzzle board {
routine setup once {
once_
}
on_level_start {
once_
}
rules {
if true {
once_
}
restart -> {
once_
}
}
render {
once_
}
}
"#;

        let routine_cursor = source.find("\nonce_\n}\non_level_start").unwrap() + "\nonce_".len();
        let lifecycle_cursor = source.find("\nonce_\n}\nrules").unwrap() + "\nonce_".len();
        let nested_cursor = source.find("\nonce_\n}\nrestart").unwrap() + "\nonce_".len();
        let input_effect_cursor = source.find("\nonce_\n}\n}\nrender").unwrap() + "\nonce_".len();
        let render_cursor = source.rfind("\nonce_\n}").unwrap() + "\nonce_".len();

        for cursor in [
            routine_cursor,
            lifecycle_cursor,
            nested_cursor,
            input_effect_cursor,
        ] {
            let list = suggest_source_completions(source, cursor);
            assert!(
                list.items
                    .iter()
                    .any(|item| item.label == "once_all" && item.kind == CompletionKind::Keyword),
                "missing once_all at cursor {cursor}"
            );
        }

        let render_list = suggest_source_completions(source, render_cursor);
        assert!(
            !render_list
                .items
                .iter()
                .any(|item| item.label == "once_all" && item.kind == CompletionKind::Keyword)
        );
    }

    #[test]
    fn arrow_position_suggests_effect_words_only() {
        let source = r#"
title = complete_arrow_position
puzzle board {
slots {
__legacy_layer_0 = Player
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
title = complete_scene_for_source
scene menu {
layout {
items = 0
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
    fn effect_suggestions_follow_owner_scope() {
        let source = r#"
title = complete_effect_scope
puzzle board {
slots {
__legacy_layer_0 = Player
}
keys {
r -> re
}
rules {
[ Player ] -> n
}
}
scene menu {
layout {
button "Play" -> g
}
keys {
Escape -> res
}
routine resume {
goto menu
}
}
"#;
        let model_effect_cursor = source.find("-> n").unwrap() + "-> n".len();
        let model_effect_list = suggest_source_completions(source, model_effect_cursor);
        assert!(
            model_effect_list
                .items
                .iter()
                .any(|item| { item.label == "next_level" && item.kind == CompletionKind::Effect })
        );
        assert!(
            !model_effect_list
                .items
                .iter()
                .any(|item| item.label == "goto" && item.kind == CompletionKind::Effect)
        );

        let scene_effect_cursor = source.find("-> g").unwrap() + "-> g".len();
        let scene_effect_list = suggest_source_completions(source, scene_effect_cursor);
        assert!(
            scene_effect_list
                .items
                .iter()
                .any(|item| item.label == "goto" && item.kind == CompletionKind::Effect)
        );
        assert!(
            !scene_effect_list
                .items
                .iter()
                .any(|item| item.label == "next_level" && item.kind == CompletionKind::Effect)
        );

        let model_keys_cursor = source.find("-> re").unwrap() + "-> re".len();
        let model_keys_list = suggest_source_completions(source, model_keys_cursor);
        assert!(
            !model_keys_list
                .items
                .iter()
                .any(|item| item.label == "restart" && item.kind == CompletionKind::Effect)
        );

        let scene_keys_cursor = source.find("-> res").unwrap() + "-> res".len();
        let scene_keys_list = suggest_source_completions(source, scene_keys_cursor);
        assert!(
            scene_keys_list
                .items
                .iter()
                .any(|item| item.label == "resume" && item.kind == CompletionKind::Routine)
        );
        assert!(
            !scene_keys_list
                .items
                .iter()
                .any(|item| item.label == "restart" && item.kind == CompletionKind::Effect)
        );
    }

    #[test]
    fn suggests_sfx_with_specific_kind_after_sfx() {
        let source = r#"
title = complete_sfx
sounds {
sfx clear { seed = clear01; type = jump }
music music_name { seed = bgm01 }
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
title = complete_sounds_sfx
sounds {
sfx clear { seed = clear01; type = jump }
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
    fn suggests_3d_render_options_from_authoring_schema() {
        let source = r#"
title = complete_3d_render_options
puzzle board {
slots {
actor
}
slots {
__legacy_layer_0 = Player actor
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
                .any(|item| item.label == "camera" && item.kind == CompletionKind::Keyword)
        );
    }

    #[test]
    fn suggests_contextual_option_names_only_in_owned_blocks() {
        let source = r#"
title = complete_contextual_options
sounds {
sfx click se
music bgm he
}
puzzle main {
render {
tween_dur
}
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

        let tween_cursor = source.find("tween_dur").unwrap() + "tween_dur".len();
        let tween_list = suggest_source_completions(source, tween_cursor);
        assert!(
            tween_list
                .items
                .iter()
                .any(|item| item.label == "tween_duration" && item.kind == CompletionKind::Setting)
        );

        let tween_enabled_source = r#"
puzzle main {
render {
tw
}
}
"#;
        let tween_enabled_cursor = tween_enabled_source.find("tw").unwrap() + "tw".len();
        let tween_enabled_list =
            suggest_source_completions(tween_enabled_source, tween_enabled_cursor);
        assert!(
            tween_enabled_list
                .items
                .iter()
                .any(|item| item.label == "tween" && item.kind == CompletionKind::Setting)
        );
        assert!(
            !tween_enabled_list
                .items
                .iter()
                .any(|item| item.label == "tween" && item.kind == CompletionKind::Keyword)
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
    fn does_not_suggest_asset_type_keywords_for_string_asset_rows() {
        let source = r#"
title = complete_asset_keywords
assets {

}
"#;
        let cursor = source.find("assets {\n").unwrap() + "assets {\n".len();
        let list = suggest_source_completions(source, cursor);

        for keyword in ["css", "script", "file"] {
            assert!(
                !list
                    .items
                    .iter()
                    .any(|item| item.label == keyword && item.kind == CompletionKind::Keyword),
                "asset string rows must not expose typed-entry keyword {keyword}"
            );
        }
    }

    #[test]
    fn suggests_theme_presets_after_theme_assignment_value() {
        let source = r#"
title = complete_theme_presets
theme = "p
"#;
        let cursor = source.find("theme = \"p").unwrap() + "theme = \"p".len();
        let list = suggest_source_completions(&source, cursor);

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
    fn reports_missing_authoring_assignment_rhs_schema_in_completion_json() {
        let source = r#"
title = complete_unknown_assignment
theme {
unknown = li
}
"#;
        let cursor = source.find("unknown = li").unwrap() + "unknown = li".len();
        let list = suggest_source_completions(source, cursor);

        assert!(list.items.is_empty());
        assert_eq!(list.warnings.len(), 1);
        let json = completion_list_json(&list);
        assert!(json.contains(r#""warnings":[{"message":"owner schema does not define assignment key `unknown`; no RHS completions are available"}]"#));
    }

    #[test]
    fn suggests_theme_settings_inside_theme_block() {
        let source = r#"
title = complete_theme_settings
theme {
preset = "clean"

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
        assert!(
            list.items
                .iter()
                .any(|item| { item.label == "preset" && item.kind == CompletionKind::Setting })
        );
        assert!(list.items.iter().all(|item| item.label != "ui_font"));
        assert!(list.items.iter().all(|item| item.label != "board_color"));
    }

    #[test]
    fn suggests_visual_color_names_in_sprite_color_rows() {
        let source = r#"
title = complete_sprite_colors
puzzle board {
slots {
__legacy_layer_0 = Box
}
}
sprites {
Box li
}
"#;
        let cursor = source.find("Box li").unwrap() + "Box li".len();
        let list = suggest_source_completions(source, cursor);

        assert!(
            list.items
                .iter()
                .any(|item| { item.label == "lightblue" && item.kind == CompletionKind::Color })
        );
        assert!(
            list.items
                .iter()
                .any(|item| { item.label == "lightgreen" && item.kind == CompletionKind::Color })
        );
    }

    #[test]
    fn suggests_visual_shape_names_in_sprite_entries() {
        let source = r#"
title = complete_visual_resource_refs
puzzle board {
tags {
kind = A B
}
slots {
__legacy_layer_0 = Box:kind
}
}
sprites {
shapes {
box_shape
00
}
Box {
colors red blue
shape box_
}
}
"#;
        let shape_cursor = source.find("shape box_").unwrap() + "shape box_".len();
        let shape_list = suggest_source_completions(source, shape_cursor);
        assert!(
            shape_list
                .items
                .iter()
                .any(|item| { item.label == "box_shape" && item.kind == CompletionKind::Shape })
        );
    }

    #[test]
    fn suggests_declared_assets_after_visual_selector() {
        let source = r#"
title = complete_visual_assets
assets {
"sprites/box.png"
}
puzzle board {
slots {
__legacy_layer_0 = Box
}
}
sprites {
Box spr
}
"#;
        let cursor = source.find("Box spr").unwrap() + "Box spr".len();
        let list = suggest_source_completions(source, cursor);

        assert!(
            list.items.iter().any(|item| {
                item.label == "sprites/box.png" && item.kind == CompletionKind::Asset
            })
        );
    }

    #[test]
    fn suggests_sprite_selector_objects_at_visual_line_head() {
        let source = r#"
title = complete_sprite_selector_line_head
puzzle board {
tags {
kind = Red Blue
}
slots {
__legacy_layer_0 = Player
__legacy_layer_1 = Box:kind
}
groups {
Actors = Player Box:kind
}
}
sprites {

}
"#;
        let cursor = source.find("\n\n}").unwrap() + 1;
        let list = suggest_source_completions(source, cursor);

        assert!(
            list.items
                .iter()
                .any(|item| item.label == "Player" && item.kind == CompletionKind::Object)
        );
        assert!(
            list.items
                .iter()
                .any(|item| item.label == "Box" && item.kind == CompletionKind::Object)
        );
        assert!(
            list.items
                .iter()
                .any(|item| item.label == "Actors" && item.kind == CompletionKind::Group)
        );
        assert!(
            list.items
                .iter()
                .any(|item| item.label == "colors" && item.kind == CompletionKind::Keyword)
        );
        assert!(
            list.items
                .iter()
                .any(|item| item.label == "palette" && item.kind == CompletionKind::Keyword)
        );

        let prefix_source = source.replacen("\n\n}", "\nBo\n}", 1);
        let prefix_cursor = prefix_source.find("\nBo\n").unwrap() + "\nBo".len();
        let prefix_list = suggest_source_completions(&prefix_source, prefix_cursor);
        assert!(
            prefix_list
                .items
                .iter()
                .any(|item| item.label == "Box" && item.kind == CompletionKind::Object)
        );
    }

    #[test]
    fn suggests_objects_after_canonical_sprite_selector_setting() {
        let source = r#"
title = complete_canonical_sprite_selector
puzzle board {
slots {
__legacy_layer_0 = Player
}
}
sprites {
sprite {
selector = Pl
}
}
"#;
        let cursor = source.find("selector = Pl").unwrap() + "selector = Pl".len();
        let list = suggest_source_completions(source, cursor);

        assert!(
            list.items
                .iter()
                .any(|item| item.label == "Player" && item.kind == CompletionKind::Object)
        );
    }

    #[test]
    fn suggests_color_names_for_theme_setting_values() {
        let source = r#"
title = complete_theme_color_values
theme {
background_color = li
}
"#;
        let cursor = source.find("background_color = li").unwrap() + "background_color = li".len();
        let list = suggest_source_completions(source, cursor);

        assert!(
            list.items
                .iter()
                .any(|item| { item.label == "lightblue" && item.kind == CompletionKind::Color })
        );
        assert!(!list.items.iter().any(|item| {
            item.label == "background_color" && item.kind == CompletionKind::Setting
        }));
    }

    #[test]
    fn sounds_authoring_block_suggests_sfx_child() {
        let source = r#"
title = complete_sounds_keyword
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
title = complete_emissions
sounds {
sfx clear { seed = clear01; type = jump }
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
title = complete_effects
puzzle board {
slots {
__legacy_layer_0 = Player
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
title = complete_boolean_literals
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
title = complete_removed_command
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
    fn does_not_suggest_removed_variable_keyword() {
        let top_level = "g";
        let top_level_list = suggest_source_completions(top_level, top_level.len());
        assert!(
            !top_level_list
                .items
                .iter()
                .any(|item| item.label == "variable" && item.kind == CompletionKind::Keyword)
        );

        let puzzle_source = r#"
title = complete_removed_variable
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
                .any(|item| item.label == "variable" && item.kind == CompletionKind::Keyword)
        );
    }

    #[test]
    fn suggests_layer_named_object_as_object() {
        let source = r#"
title = complete_layer_object
puzzle board {
slots {
floor = layer
}
rules {
[ la
}
}
"#;
        let cursor = source.find("[ la").unwrap() + "[ la".len();
        let list = suggest_source_completions(source, cursor);

        assert!(
            list.items
                .iter()
                .any(|item| { item.label == "layer" && item.kind == CompletionKind::Object })
        );
    }

    #[test]
    fn suggests_all_puzzle_lifecycle_blocks() {
        let source = r#"
title = complete_lifecycle
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
    fn suggests_all_puzzle_keywords_from_shared_syntax() {
        for keyword in PUZZLE_COMPLETION_KEYWORDS {
            let source = format!("title complete_{keyword}\npuzzle board {{\n{keyword}\n}}\n");
            let cursor = source.rfind(keyword).unwrap() + keyword.len();
            let list = suggest_source_completions(&source, cursor);

            assert!(
                list.items
                    .iter()
                    .any(|item| item.label == *keyword && item.kind == CompletionKind::Keyword),
                "missing puzzle completion {keyword}"
            );
        }
    }

    #[test]
    fn suggests_model_top_level_keywords_from_parser_surface() {
        for keyword in crate::model_top_level_completion_keywords() {
            let source = keyword.to_string();
            let cursor = source.len();
            let list = suggest_source_completions(&source, cursor);

            assert!(
                list.items
                    .iter()
                    .any(|item| item.label == *keyword && item.kind == CompletionKind::Keyword),
                "missing top-level completion {keyword}"
            );
        }
    }

    #[test]
    fn suggests_top_level_authoring_keywords_from_schema() {
        let source = "";
        let list = suggest_source_completions(source, 0);

        for (label, kind) in [
            ("var", CompletionKind::Keyword),
            ("const", CompletionKind::Keyword),
            ("persistent", CompletionKind::Keyword),
            ("theme", CompletionKind::Keyword),
            ("sounds", CompletionKind::Keyword),
            ("assets", CompletionKind::Keyword),
            ("input_buffer", CompletionKind::Keyword),
            ("title", CompletionKind::Setting),
            ("author", CompletionKind::Setting),
        ] {
            assert!(
                list.items
                    .iter()
                    .any(|item| item.label == label && item.kind == kind),
                "missing schema-owned top-level completion {label}"
            );
        }
    }

    #[test]
    fn model_top_level_completion_keywords_exclude_schema_owned_heads() {
        let keywords = crate::model_top_level_completion_keywords();

        for schema_owned in [
            "var",
            "const",
            "persistent",
            "theme",
            "sounds",
            "assets",
            "input_buffer",
            "title",
            "author",
        ] {
            assert!(
                !keywords.contains(&schema_owned),
                "schema-owned top-level completion must come from authoring schema: {schema_owned}"
            );
        }

        assert!(keywords.contains(&"puzzle"));
        assert!(keywords.contains(&"scene"));
    }

    #[test]
    fn distinguishes_value_sets_and_puzzles() {
        let source = r#"
title = complete_kinds
puzzle sokoban {
tags {
kind = A B
}
slots {
__legacy_layer_0 = Box:kind
}
rules {
for k in ki
}
}
scene playing {
layout {
puzzle board = so
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
title = complete_layer_selectors
puzzle board {
slots {
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

    #[test]
    fn completion_renderer_does_not_reparse_source_for_context() {
        let source = include_str!("completion.rs");
        let forbidden_fragments = [
            ["scan_source", "_context"],
            ["parse_game", "2d"],
            ["SourceScope", "::"],
            ["collect_line", "_symbols"],
            ["collect_completion", "_symbols"],
        ];
        for parts in forbidden_fragments {
            let forbidden = parts.concat();
            assert!(
                !source.contains(&forbidden),
                "completion.rs must consume surface completion products, not rebuild parser context via {forbidden}"
            );
        }
    }

    #[test]
    fn completion_entrypoint_uses_completion_products_not_full_surface_document() {
        let source = include_str!("completion.rs");
        let start = source
            .find("pub fn suggest_source_completions")
            .expect("completion entrypoint");
        let end = source[start..]
            .find("pub(crate) fn completion_list_from_context")
            .map(|offset| start + offset)
            .expect("context completion entrypoint");
        let body = &source[start..end];

        assert!(body.contains("parse_surface_completion_context_document"));
        assert!(body.contains("parse_surface_completion_symbols_document"));
        assert!(
            !body.contains("parse_surface_document"),
            "completion entrypoint must not build the full surface document"
        );
    }
}
