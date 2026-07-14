use crate::LEVEL_MENU_OPTIONS;
use crate::authoring_grammar::AuthoringKind;
use crate::semantic::{SemanticCompletionContext, SemanticCompletionSlot, SettingCompletionSet};
use crate::source::SourceScope;
use crate::surface::{SurfaceDocument, SurfaceLine, SurfaceOptionBlock};
use crate::syntax::{ExpectedCompletionValue, PUZZLE_COMPLETION_KEYWORDS, PUZZLE_LIFECYCLE_BLOCKS};

pub(crate) fn surface_completion_context_for_document(
    source: &str,
    cursor_offset: usize,
    document: &SurfaceDocument,
) -> SemanticCompletionContext {
    let cursor = cursor_offset.min(source.len());
    let token = completion_token_at_cursor(source, cursor);
    let previous = previous_completion_token(source, token.replace_start);
    let scope = scope_at_cursor(document, cursor);
    let option_block = option_block_at_cursor(document, cursor);
    let sounds_definition_scope =
        option_block == Some(SurfaceOptionBlock::Authoring(AuthoringKind::SoundsConfig));
    let warnings =
        authoring_assignment_completion_warnings(source, token.replace_start, option_block, scope);

    let contextual_slots = contextual_completion_slots(source, document, &token, scope);
    let slots = if let Some(slots) = contextual_slots {
        slots
    } else if matches!(
        previous.as_deref(),
        Some("goto" | "resume" | "enter" | "open" | "start")
    ) && scene_effect_scope(scope)
    {
        vec![SemanticCompletionSlot::Scenes]
    } else if previous.as_deref() == Some("input") && scene_effect_scope(scope) {
        vec![SemanticCompletionSlot::Inputs]
    } else if previous.as_deref() == Some("of") {
        vec![SemanticCompletionSlot::Puzzles]
    } else if scope.is_none()
        && let Some(slots) =
            authoring_root_definition_value_completion_slots(source, token.replace_start)
    {
        slots
    } else if previous.as_deref() == Some("sfx")
        && (!sounds_definition_scope
            || sounds_operation_sfx_target_context(source, token.replace_start))
    {
        vec![SemanticCompletionSlot::SfxAssets]
    } else if matches!(
        previous.as_deref(),
        Some("play_music" | "pause_music" | "resume_music" | "stop_music")
    ) && !sounds_definition_scope
    {
        vec![SemanticCompletionSlot::MusicAssets]
    } else if matches!(previous.as_deref(), Some("puzzle")) {
        vec![SemanticCompletionSlot::Puzzles]
    } else {
        default_completion_slots_for_scope(scope)
    };

    SemanticCompletionContext {
        replace_start: token.replace_start,
        replace_end: token.replace_end,
        token_text: token.text,
        slots,
        warnings,
    }
}

fn authoring_assignment_completion_warnings(
    source: &str,
    cursor: usize,
    option_block: Option<SurfaceOptionBlock>,
    scope: Option<SourceScope>,
) -> Vec<String> {
    let Some(kind) = authoring_assignment_owner_kind(option_block, scope) else {
        return Vec::new();
    };
    let line_start = source[..cursor].rfind('\n').map_or(0, |index| index + 1);
    let before = source[line_start..cursor].trim_start();
    let tokens = crate::authoring_grammar::split_authoring_tokens(before);
    let [key, op, ..] = tokens.as_slice() else {
        return Vec::new();
    };
    if op != "=" || crate::authoring_grammar::authoring_definition_spec(kind, key).is_some() {
        return Vec::new();
    }
    vec![format!(
        "owner schema does not define assignment key `{key}`; no RHS completions are available"
    )]
}

fn authoring_assignment_owner_kind(
    option_block: Option<SurfaceOptionBlock>,
    scope: Option<SourceScope>,
) -> Option<AuthoringKind> {
    if let Some(kind) = option_block.and_then(SurfaceOptionBlock::authoring_parent_kind) {
        return Some(kind);
    }
    scope.is_none().then_some(AuthoringKind::Root)
}

fn contextual_completion_slots(
    source: &str,
    document: &SurfaceDocument,
    token: &CompletionTokenAtCursor,
    scope: Option<SourceScope>,
) -> Option<Vec<SemanticCompletionSlot>> {
    if token.text.contains(':') {
        return Some(vec![
            SemanticCompletionSlot::ValueSets,
            SemanticCompletionSlot::ObjectNameAtoms,
        ]);
    }

    let (line_index, line) = line_at_cursor(document, token.replace_start)?;
    let line_end = line.start + line.content.len();
    let before = &source[line.start..token.replace_start.min(line_end)];
    let after = &source[token.replace_end.min(line_end)..line_end];
    let previous = previous_completion_token(source, token.replace_start);

    if cursor_is_after_effect_arrow(before) {
        return Some(arrow_rhs_completion_slots(scope));
    }

    if let Some(options) = option_completion_slots(line.option_block, before) {
        return Some(options);
    }

    if let Some(slots) = visual_completion_slots(line.scope, before) {
        return Some(slots);
    }

    if let Some(slots) = grammar_completion_slots(scope, before) {
        return Some(slots);
    }

    if inside_mark_selector_attrs(before) {
        return Some(vec![
            SemanticCompletionSlot::Directions,
            SemanticCompletionSlot::DirectionSets,
            SemanticCompletionSlot::Markes,
        ]);
    }

    if previous.as_deref() == Some("in") {
        return Some(match scope {
            Some(
                SourceScope::Scene
                | SourceScope::SceneLayout
                | SourceScope::SceneTransitions
                | SourceScope::LevelMenu,
            ) => vec![
                SemanticCompletionSlot::Keywords(SCENE_FOR_SOURCE_COMPLETION_KEYWORDS),
                SemanticCompletionSlot::States,
            ],
            _ => vec![
                SemanticCompletionSlot::ValueSets,
                SemanticCompletionSlot::Groups,
            ],
        });
    }

    if is_rule_like_scope(scope) && next_non_whitespace_starts_pattern(after) {
        return Some(vec![
            SemanticCompletionSlot::Directions,
            SemanticCompletionSlot::DirectionSets,
        ]);
    }

    if before.trim().is_empty() && (token.text.is_empty() || !symbol_definition_scope(scope)) {
        return Some(line_head_completion_slots(document, line_index, scope));
    }

    None
}

fn grammar_completion_slots(
    scope: Option<SourceScope>,
    before: &str,
) -> Option<Vec<SemanticCompletionSlot>> {
    let tokens = split_completion_line_tokens(before);
    let classes = grammar_completion_value_classes(scope, &tokens)?;
    Some(completion_slots_for_value_classes(classes))
}

fn grammar_completion_value_classes(
    scope: Option<SourceScope>,
    tokens: &[&str],
) -> Option<&'static [ExpectedCompletionValue]> {
    let syntax = match scope {
        Some(SourceScope::Legend) => crate::syntax::legend_block_row_syntax(tokens, false),
        Some(SourceScope::Puzzle) => crate::syntax::legend_directive_syntax(tokens, false),
        Some(SourceScope::Level | SourceScope::UnbracedLevel) => {
            crate::syntax::level_legend_directive_syntax(tokens, false)
        }
        Some(SourceScope::Group) => crate::syntax::named_selector_assignment_syntax(tokens, false),
        Some(SourceScope::Slots) => crate::syntax::named_selector_assignment_syntax(tokens, false),
        _ => None,
    }?;
    Some(syntax.expected_completion_values)
}

fn completion_slots_for_value_classes(
    classes: &[ExpectedCompletionValue],
) -> Vec<SemanticCompletionSlot> {
    let mut slots = Vec::new();
    for class in classes {
        match class {
            ExpectedCompletionValue::Selector | ExpectedCompletionValue::SpriteSelector => {
                slots.push(SemanticCompletionSlot::Objects);
                slots.push(SemanticCompletionSlot::Groups);
            }
            ExpectedCompletionValue::LegendEmpty => {
                slots.push(SemanticCompletionSlot::Keywords(LEGEND_COMPLETION_KEYWORDS));
            }
            ExpectedCompletionValue::VisualDirective => {
                slots.push(SemanticCompletionSlot::Keywords(VISUAL_COMPLETION_KEYWORDS));
            }
        }
    }
    slots
}

fn cursor_is_after_effect_arrow(before: &str) -> bool {
    let Some(arrow) = before.rfind("->") else {
        return false;
    };
    let suffix = &before[arrow + 2..];
    let tokens = suffix
        .split_whitespace()
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    tokens.is_empty()
        || (tokens.len() == 1 && suffix.chars().next_back().is_some_and(is_completion_char))
}

fn arrow_rhs_completion_slots(scope: Option<SourceScope>) -> Vec<SemanticCompletionSlot> {
    match scope {
        Some(SourceScope::Keys) => vec![
            SemanticCompletionSlot::Inputs,
            SemanticCompletionSlot::Directions,
        ],
        Some(SourceScope::SceneKeys) => vec![
            SemanticCompletionSlot::SceneEffects,
            SemanticCompletionSlot::Routines,
        ],
        Some(
            SourceScope::Scene
            | SourceScope::SceneLayout
            | SourceScope::SceneTransitions
            | SourceScope::LevelMenu,
        ) => vec![
            SemanticCompletionSlot::SceneEffects,
            SemanticCompletionSlot::Routines,
        ],
        _ => vec![
            SemanticCompletionSlot::ModelEffects,
            SemanticCompletionSlot::Emissions,
        ],
    }
}

fn line_head_completion_slots(
    document: &SurfaceDocument,
    line_index: usize,
    scope: Option<SourceScope>,
) -> Vec<SemanticCompletionSlot> {
    match scope {
        None => authoring_root_line_head_completion_slots(),
        Some(
            SourceScope::Puzzle
            | SourceScope::Scene
            | SourceScope::SceneLayout
            | SourceScope::SceneState
            | SourceScope::SceneKeys
            | SourceScope::SceneTransitions
            | SourceScope::LevelMenu
            | SourceScope::Tags
            | SourceScope::Group
            | SourceScope::Map
            | SourceScope::Slots
            | SourceScope::Mark
            | SourceScope::Keys
            | SourceScope::Legend
            | SourceScope::Levels
            | SourceScope::Level
            | SourceScope::UnbracedLevel
            | SourceScope::Condition
            | SourceScope::VisualShapeTable
            | SourceScope::VisualShapeEntry
            | SourceScope::VisualColorTable,
        ) => vec![SemanticCompletionSlot::Keywords(
            completion_keywords_for_scope(scope),
        )],
        Some(SourceScope::Visuals) => completion_slots_for_value_classes(
            crate::syntax::visual_line_head_expected_completion_values(),
        ),
        Some(SourceScope::Other) if current_statement_block_before_line(document, line_index) => {
            vec![
                SemanticCompletionSlot::Keywords(puzzle_authoring::RULE_STATEMENT_HEAD_KEYWORDS),
                SemanticCompletionSlot::Routines,
                SemanticCompletionSlot::Directions,
                SemanticCompletionSlot::DirectionSets,
                SemanticCompletionSlot::ModelEffects,
                SemanticCompletionSlot::Emissions,
            ]
        }
        Some(SourceScope::Other) => default_completion_slots_for_scope(scope),
    }
}

fn current_statement_block_before_line(document: &SurfaceDocument, line_index: usize) -> bool {
    let mut stack = Vec::<CompletionBlockKind>::new();
    for line in document.lines.iter().take(line_index) {
        update_completion_block_stack(line.content.trim(), &mut stack);
    }
    stack.last() == Some(&CompletionBlockKind::Statement)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CompletionBlockKind {
    Statement,
    Other,
}

fn update_completion_block_stack(line: &str, stack: &mut Vec<CompletionBlockKind>) {
    let mut current = line;
    while let Some(rest) = current.strip_prefix('}') {
        stack.pop();
        current = rest.trim_start();
        if current.is_empty() {
            return;
        }
    }
    if !current.ends_with('{') {
        return;
    }
    let parent_is_statement_block = stack.last() == Some(&CompletionBlockKind::Statement);
    let kind = if puzzle_authoring::rule_statement_block_surface(current, parent_is_statement_block)
        .is_some()
    {
        CompletionBlockKind::Statement
    } else {
        CompletionBlockKind::Other
    };
    stack.push(kind);
}

fn line_at_cursor(document: &SurfaceDocument, cursor: usize) -> Option<(usize, &SurfaceLine)> {
    document.lines.iter().enumerate().find(|(_, line)| {
        let end = line.start + line.content.len();
        cursor >= line.start && cursor <= end
    })
}

fn option_completion_slots(
    block: Option<SurfaceOptionBlock>,
    before: &str,
) -> Option<Vec<SemanticCompletionSlot>> {
    let tokens_before = before
        .split(|ch: char| ch.is_whitespace() || matches!(ch, '{' | '}' | ',' | ';'))
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    let first = tokens_before.first().copied();

    let option_names = match (block, first) {
        (Some(block), _) if block.authoring_parent_kind().is_some() => {
            let kind = block
                .authoring_parent_kind()
                .expect("checked authoring parent kind");
            return authoring_option_completion_slots(kind, before, first);
        }
        (Some(SurfaceOptionBlock::LevelMenu), _) => LEVEL_MENU_OPTIONS,
        _ => return None,
    };

    Some(vec![SemanticCompletionSlot::Settings(
        SettingCompletionSet::Static(option_names),
    )])
}

fn authoring_option_completion_slots(
    kind: AuthoringKind,
    before: &str,
    first: Option<&str>,
) -> Option<Vec<SemanticCompletionSlot>> {
    let mut slots = Vec::<SemanticCompletionSlot>::new();
    if let Some(first) = first
        && let Some(child) = crate::authoring_grammar::placed_authoring_kind(kind, first)
    {
        if let Some(child_slots) = authoring_schema_completion_slots(child, false) {
            slots.extend(child_slots);
        }
    } else if let Some(definition_slots) = authoring_definition_value_completion_slots(before, kind)
    {
        slots.extend(definition_slots);
    } else if let Some(schema_slots) = authoring_schema_completion_slots(kind, true) {
        slots.extend(schema_slots);
    }

    if crate::authoring_grammar::authoring_block_role(kind)
        == Some(crate::authoring_grammar::AuthoringBlockRole::Visuals)
    {
        if let Some(visual_slots) = visual_completion_slots(Some(SourceScope::Visuals), before) {
            slots.extend(visual_slots);
        } else if before.trim().is_empty() {
            slots.extend(completion_slots_for_value_classes(
                crate::syntax::visual_line_head_expected_completion_values(),
            ));
        }
    }

    Some(slots)
}

fn authoring_schema_completion_slots(
    kind: AuthoringKind,
    include_children: bool,
) -> Option<Vec<SemanticCompletionSlot>> {
    let mut slots = Vec::<SemanticCompletionSlot>::new();
    for completion in crate::authoring_grammar::authoring_body_completions(kind, include_children) {
        match completion {
            crate::authoring_grammar::AuthoringBodyCompletion::Rows(kind) => {
                slots.push(SemanticCompletionSlot::AuthoringRows(kind));
            }
            crate::authoring_grammar::AuthoringBodyCompletion::Children(kind) => {
                slots.push(SemanticCompletionSlot::AuthoringChildren(kind));
            }
            crate::authoring_grammar::AuthoringBodyCompletion::Definitions(kind) => {
                slots.push(SemanticCompletionSlot::Settings(
                    SettingCompletionSet::AuthoringDefinitions(kind),
                ));
            }
            crate::authoring_grammar::AuthoringBodyCompletion::ContentRows(content) => {
                slots.push(SemanticCompletionSlot::AuthoringContentRows(content));
            }
        }
    }
    (!slots.is_empty()).then_some(slots)
}

fn authoring_root_line_head_completion_slots() -> Vec<SemanticCompletionSlot> {
    vec![
        SemanticCompletionSlot::AuthoringRows(AuthoringKind::Root),
        SemanticCompletionSlot::AuthoringChildren(AuthoringKind::Root),
        SemanticCompletionSlot::Settings(SettingCompletionSet::AuthoringDefinitions(
            AuthoringKind::Root,
        )),
        SemanticCompletionSlot::ModelTopLevelKeywords,
    ]
}

fn authoring_root_definition_value_completion_slots(
    source: &str,
    cursor: usize,
) -> Option<Vec<SemanticCompletionSlot>> {
    let line_start = source[..cursor].rfind('\n').map_or(0, |index| index + 1);
    let before = source[line_start..cursor].trim_start();
    authoring_definition_value_completion_slots(before, AuthoringKind::Root)
}

fn authoring_definition_value_completion_slots(
    before: &str,
    kind: AuthoringKind,
) -> Option<Vec<SemanticCompletionSlot>> {
    let tokens = crate::authoring_grammar::split_authoring_tokens(before);
    let [key, op, ..] = tokens.as_slice() else {
        return None;
    };
    if op != "=" {
        return None;
    }
    let Some(completion) = crate::authoring_grammar::authoring_definition_completion(kind, key)
    else {
        return Some(Vec::new());
    };
    match completion {
        crate::authoring_grammar::AuthoringDefinitionCompletion::Builtin(
            crate::authoring_grammar::DefinitionBuiltinDomain::ThemePreset,
        ) => Some(vec![SemanticCompletionSlot::Themes]),
        crate::authoring_grammar::AuthoringDefinitionCompletion::Builtin(domain) => {
            Some(vec![SemanticCompletionSlot::Literals(
                crate::authoring_grammar::definition_builtin_domain_values(domain),
            )])
        }
        crate::authoring_grammar::AuthoringDefinitionCompletion::Color => {
            Some(vec![SemanticCompletionSlot::Colors])
        }
        crate::authoring_grammar::AuthoringDefinitionCompletion::Object => Some(vec![
            SemanticCompletionSlot::Objects,
            SemanticCompletionSlot::Groups,
        ]),
    }
}

fn visual_completion_slots(
    scope: Option<SourceScope>,
    before: &str,
) -> Option<Vec<SemanticCompletionSlot>> {
    match scope {
        Some(SourceScope::VisualColorTable) => {
            before.rfind('=')?;
            Some(vec![SemanticCompletionSlot::Colors])
        }
        Some(SourceScope::Visuals) => {
            let tokens = before
                .split(|ch: char| ch.is_whitespace() || matches!(ch, '{' | '}' | ',' | ';'))
                .filter(|token| !token.is_empty())
                .collect::<Vec<_>>();
            match tokens.as_slice() {
                ["colors", ..] => Some(vec![SemanticCompletionSlot::Colors]),
                ["shape", ..] => Some(vec![SemanticCompletionSlot::Shapes]),
                [first, ..] if !matches!(*first, "shape" | "sprite" | "palette" | "colors") => {
                    Some(vec![
                        SemanticCompletionSlot::Colors,
                        SemanticCompletionSlot::Assets,
                    ])
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn inside_mark_selector_attrs(before: &str) -> bool {
    let Some(open) = before.rfind('{') else {
        return false;
    };
    if before[open + 1..].contains('}') {
        return false;
    }
    if open == 0 {
        return true;
    }
    let before_open = &before[..open];
    let Some(previous) = before_open.chars().next_back() else {
        return true;
    };
    if !previous.is_whitespace() {
        return true;
    }
    let trimmed = before_open.trim_end();
    trimmed.ends_with('[') || trimmed.ends_with('|')
}

fn next_non_whitespace_starts_pattern(after: &str) -> bool {
    after.trim_start().starts_with('[')
}

fn is_rule_like_scope(scope: Option<SourceScope>) -> bool {
    matches!(scope, Some(SourceScope::Puzzle | SourceScope::Other))
}

fn symbol_definition_scope(scope: Option<SourceScope>) -> bool {
    matches!(
        scope,
        Some(
            SourceScope::Group
                | SourceScope::Slots
                | SourceScope::Tags
                | SourceScope::Mark
                | SourceScope::Keys
                | SourceScope::SceneKeys
                | SourceScope::SceneState
        )
    )
}

fn scene_effect_scope(scope: Option<SourceScope>) -> bool {
    matches!(
        scope,
        Some(
            SourceScope::Scene
                | SourceScope::SceneLayout
                | SourceScope::SceneKeys
                | SourceScope::SceneTransitions
                | SourceScope::LevelMenu
        )
    )
}

fn default_completion_slots_for_scope(scope: Option<SourceScope>) -> Vec<SemanticCompletionSlot> {
    match scope {
        None => authoring_root_line_head_completion_slots(),
        Some(SourceScope::Tags | SourceScope::Map) => vec![],
        Some(SourceScope::Group | SourceScope::Slots) => vec![
            SemanticCompletionSlot::Keywords(completion_keywords_for_scope(scope)),
            SemanticCompletionSlot::Objects,
            SemanticCompletionSlot::Groups,
        ],
        Some(SourceScope::Mark) => vec![
            SemanticCompletionSlot::Keywords(completion_keywords_for_scope(scope)),
            SemanticCompletionSlot::ObjectNameAtoms,
            SemanticCompletionSlot::Directions,
            SemanticCompletionSlot::DirectionSets,
        ],
        Some(SourceScope::Keys) => vec![
            SemanticCompletionSlot::Keywords(completion_keywords_for_scope(scope)),
            SemanticCompletionSlot::Inputs,
            SemanticCompletionSlot::Directions,
        ],
        Some(SourceScope::SceneKeys) => vec![
            SemanticCompletionSlot::Keywords(completion_keywords_for_scope(scope)),
            SemanticCompletionSlot::SceneEffects,
            SemanticCompletionSlot::Routines,
        ],
        Some(SourceScope::SceneState) => vec![
            SemanticCompletionSlot::Keywords(completion_keywords_for_scope(scope)),
            SemanticCompletionSlot::Literals(COMPLETION_LITERALS),
            SemanticCompletionSlot::States,
            SemanticCompletionSlot::Puzzles,
        ],
        Some(SourceScope::Puzzle) => vec![
            SemanticCompletionSlot::Keywords(completion_keywords_for_scope(scope)),
            SemanticCompletionSlot::Literals(COMPLETION_LITERALS),
            SemanticCompletionSlot::States,
            SemanticCompletionSlot::Conditions,
            SemanticCompletionSlot::Inputs,
            SemanticCompletionSlot::Directions,
            SemanticCompletionSlot::DirectionSets,
        ],
        Some(SourceScope::Condition) => vec![
            SemanticCompletionSlot::Keywords(completion_keywords_for_scope(scope)),
            SemanticCompletionSlot::Objects,
            SemanticCompletionSlot::Groups,
            SemanticCompletionSlot::Conditions,
            SemanticCompletionSlot::States,
        ],
        Some(
            SourceScope::Scene
            | SourceScope::SceneLayout
            | SourceScope::SceneTransitions
            | SourceScope::LevelMenu,
        ) => vec![
            SemanticCompletionSlot::Keywords(completion_keywords_for_scope(scope)),
            SemanticCompletionSlot::Literals(COMPLETION_LITERALS),
            SemanticCompletionSlot::States,
            SemanticCompletionSlot::Routines,
            SemanticCompletionSlot::Conditions,
            SemanticCompletionSlot::Inputs,
            SemanticCompletionSlot::SceneEffects,
        ],
        Some(
            SourceScope::Legend
            | SourceScope::Levels
            | SourceScope::Level
            | SourceScope::UnbracedLevel,
        ) => vec![SemanticCompletionSlot::Keywords(
            completion_keywords_for_scope(scope),
        )],
        Some(
            SourceScope::Visuals
            | SourceScope::VisualShapeTable
            | SourceScope::VisualShapeEntry
            | SourceScope::VisualColorTable,
        ) => vec![
            SemanticCompletionSlot::Keywords(completion_keywords_for_scope(scope)),
            SemanticCompletionSlot::Sprites,
            SemanticCompletionSlot::Assets,
            SemanticCompletionSlot::Shapes,
            SemanticCompletionSlot::Colors,
        ],
        Some(SourceScope::Other) => rule_expression_completion_slots(),
    }
}

fn rule_expression_completion_slots() -> Vec<SemanticCompletionSlot> {
    vec![
        SemanticCompletionSlot::Literals(COMPLETION_LITERALS),
        SemanticCompletionSlot::Objects,
        SemanticCompletionSlot::Groups,
        SemanticCompletionSlot::States,
        SemanticCompletionSlot::Markes,
        SemanticCompletionSlot::ObjectNameAtoms,
        SemanticCompletionSlot::Directions,
        SemanticCompletionSlot::DirectionSets,
        SemanticCompletionSlot::Inputs,
        SemanticCompletionSlot::ModelEffects,
        SemanticCompletionSlot::Emissions,
        SemanticCompletionSlot::Routines,
        SemanticCompletionSlot::Conditions,
    ]
}

fn completion_keywords_for_scope(scope: Option<SourceScope>) -> &'static [&'static str] {
    match scope {
        None => &[],
        Some(SourceScope::Puzzle) => PUZZLE_COMPLETION_KEYWORDS,
        Some(SourceScope::Condition) => &["all", "some", "no", "on"],
        Some(SourceScope::Tags) => TAG_COMPLETION_KEYWORDS,
        Some(SourceScope::Group) => GROUP_COMPLETION_KEYWORDS,
        Some(SourceScope::Map) => &[],
        Some(SourceScope::Slots) => LAYER_COMPLETION_KEYWORDS,
        Some(SourceScope::Mark) => MARK_COMPLETION_KEYWORDS,
        Some(SourceScope::Keys) | Some(SourceScope::SceneKeys) => KEY_COMPLETION_KEYWORDS,
        Some(SourceScope::Legend) => LEGEND_COMPLETION_KEYWORDS,
        Some(SourceScope::Levels) | Some(SourceScope::Level) | Some(SourceScope::UnbracedLevel) => {
            LEVEL_COMPLETION_KEYWORDS
        }
        Some(
            SourceScope::Scene
            | SourceScope::SceneLayout
            | SourceScope::SceneState
            | SourceScope::SceneTransitions
            | SourceScope::LevelMenu,
        ) => SCENE_COMPLETION_KEYWORDS,
        Some(
            SourceScope::Visuals
            | SourceScope::VisualShapeTable
            | SourceScope::VisualShapeEntry
            | SourceScope::VisualColorTable,
        ) => VISUAL_COMPLETION_KEYWORDS,
        Some(SourceScope::Other) => COMPLETION_KEYWORDS,
    }
}

fn scope_at_cursor(document: &SurfaceDocument, cursor: usize) -> Option<SourceScope> {
    let mut previous = None;
    for line in &document.lines {
        let end = line.start + line.content.len();
        if cursor >= line.start && cursor <= end {
            return line.scope;
        }
        if line.start <= cursor {
            previous = line.scope;
        } else {
            break;
        }
    }
    previous
}

fn option_block_at_cursor(document: &SurfaceDocument, cursor: usize) -> Option<SurfaceOptionBlock> {
    let mut previous = None;
    for line in &document.lines {
        let end = line.start + line.content.len();
        if cursor >= line.start && cursor <= end {
            return line.option_block;
        }
        if line.start <= cursor {
            previous = line.option_block;
        } else {
            break;
        }
    }
    previous
}

struct CompletionTokenAtCursor {
    text: String,
    replace_start: usize,
    replace_end: usize,
}

fn completion_token_at_cursor(source: &str, cursor: usize) -> CompletionTokenAtCursor {
    let mut start = cursor;
    while start > 0 {
        let Some(ch) = source[..start].chars().next_back() else {
            break;
        };
        if !is_completion_char(ch) {
            break;
        }
        start -= ch.len_utf8();
    }
    let mut end = cursor;
    while end < source.len() {
        let Some(ch) = source[end..].chars().next() else {
            break;
        };
        if !is_completion_char(ch) {
            break;
        }
        end += ch.len_utf8();
    }
    CompletionTokenAtCursor {
        text: source[start..cursor].to_string(),
        replace_start: start,
        replace_end: end,
    }
}

fn previous_completion_token(source: &str, before: usize) -> Option<String> {
    let mut index = before;
    while index > 0 {
        let ch = source[..index].chars().next_back()?;
        if !ch.is_whitespace() {
            break;
        }
        index -= ch.len_utf8();
    }
    let end = index;
    while index > 0 {
        let ch = source[..index].chars().next_back()?;
        if !is_completion_char(ch) {
            break;
        }
        index -= ch.len_utf8();
    }
    (index < end).then(|| source[index..end].to_string())
}

fn is_completion_char(ch: char) -> bool {
    ch == '@' || ch == '_' || ch == ':' || ch == '.' || ch == '-' || ch.is_ascii_alphanumeric()
}

const TAG_COMPLETION_KEYWORDS: &[&str] = &[];
const GROUP_COMPLETION_KEYWORDS: &[&str] = &["each"];
const LAYER_COMPLETION_KEYWORDS: &[&str] = &["each"];
const MARK_COMPLETION_KEYWORDS: &[&str] = &["const", "persistent", "var"];
const KEY_COMPLETION_KEYWORDS: &[&str] = &["direction", "input"];
const LEGEND_COMPLETION_KEYWORDS: &[&str] = &["empty"];
const LEVEL_COMPLETION_KEYWORDS: &[&str] = &["legend", "level", "of"];
const SCENE_FOR_SOURCE_COMPLETION_KEYWORDS: &[&str] = &["levels"];

const SCENE_COMPLETION_KEYWORDS: &[&str] = &[
    "button",
    "column",
    "const",
    "else",
    "for",
    "if",
    "keys",
    "level_menu",
    "message",
    "on_scene_start",
    "box",
    "puzzle",
    "row",
    "rules",
    "scene",
    "state",
    "text",
    "title",
    "layout",
    "with",
];

const VISUAL_COMPLETION_KEYWORDS: &[&str] = &[
    "contain", "cover", "colors", "image", "offset", "palette", "rotate", "sampling", "shape",
    "shapes", "sprite", "stretch",
];
const COMPLETION_LITERALS: &[&str] = &["false", "true"];

const COMPLETION_KEYWORDS: &[&str] = &[
    "again_interval",
    "assets",
    "author",
    "sounds",
    "button",
    "column",
    "const",
    "colors",
    "collision_layers",
    "component_effect",
    "direction",
    "each",
    "else",
    "for",
    "from",
    "groups",
    "homepage",
    "if",
    "import",
    "input",
    "interactive_look",
    "interactive_zoom",
    "keys",
    "slots",
    "legend",
    "level",
    "level_menu",
    "levels",
    "lose_conditions",
    "map",
    "music",
    "of",
    PUZZLE_LIFECYCLE_BLOCKS[0],
    PUZZLE_LIFECYCLE_BLOCKS[1],
    PUZZLE_LIFECYCLE_BLOCKS[2],
    "once",
    "once_all",
    "once_per_level",
    "box",
    "persistent",
    "puzzle",
    "query",
    "repeat",
    "resources",
    "render",
    "row",
    "routine",
    "rules",
    "scene",
    "marks",
    "sfx",
    "shape",
    "show_index",
    "show_solved",
    "sprite",
    "state",
    "subtitle",
    "text",
    "theme",
    "title",
    "var",
    "layout",
    "win_conditions",
    "with",
];

fn sounds_operation_sfx_target_context(source: &str, cursor: usize) -> bool {
    let line_start = source[..cursor].rfind('\n').map_or(0, |index| index + 1);
    let before = source[line_start..cursor].trim();
    matches!(
        split_completion_line_tokens(before).as_slice(),
        ["undo", "->", "sfx"] | ["restart", "->", "sfx"]
    )
}

fn split_completion_line_tokens(line: &str) -> Vec<&str> {
    line.split_whitespace().collect()
}

#[cfg(test)]
mod tests {
    use crate::authoring_grammar::AuthoringKind;

    use super::{SemanticCompletionSlot, surface_completion_context_for_document};

    #[test]
    fn completion_context_uses_source_scope_for_same_word() {
        let sounds_source = r#"
title = completion_sounds_authoring_block
sounds {
s
}
"#;
        let sounds_cursor = sounds_source.rfind("\ns\n").unwrap() + "\ns".len();
        let sounds_document = crate::parse_surface_document(sounds_source);
        let sounds_context =
            surface_completion_context_for_document(sounds_source, sounds_cursor, &sounds_document);
        assert!(
            sounds_context
                .slots
                .contains(&SemanticCompletionSlot::AuthoringChildren(
                    AuthoringKind::SoundsConfig
                ))
        );
        assert!(
            !sounds_context
                .slots
                .contains(&SemanticCompletionSlot::Emissions)
        );

        let scene_source = r#"
title = completion_scene_scope
scene playing {
rules {
win -> s
}
}
"#;
        let scene_cursor = scene_source.find("win -> s").unwrap() + "win -> s".len();
        let scene_document = crate::parse_surface_document(scene_source);
        let scene_context =
            surface_completion_context_for_document(scene_source, scene_cursor, &scene_document);
        assert!(
            scene_context
                .slots
                .contains(&SemanticCompletionSlot::SceneEffects)
        );
        assert!(!scene_context.slots.iter().any(|slot| {
            matches!(slot, SemanticCompletionSlot::Keywords(keywords) if keywords.contains(&"sfx"))
        }));
    }

    #[test]
    fn assets_completion_does_not_use_typed_entry_keywords() {
        let source = r#"
title = completion_assets_content
assets {

}
"#;
        let cursor = source.find("assets {\n").unwrap() + "assets {\n".len();
        let document = crate::parse_surface_document(source);
        let context = surface_completion_context_for_document(source, cursor, &document);

        assert!(context.slots.is_empty());
        assert!(!context.slots.iter().any(|slot| {
            matches!(slot, SemanticCompletionSlot::Keywords(keywords) if keywords.contains(&"css"))
        }));
    }

    #[test]
    fn authoring_generic_completions_follow_owner_schema() {
        let source = r#"
title = completion_authoring_schema
sounds {
s
}
theme {
background_color = li
unknown = z
}
puzzle board {
render {
g
grid {
type = "o
}
}
}
"#;
        let document = crate::parse_surface_document(source);
        let context_at = |needle: &str| {
            let cursor = source.find(needle).unwrap() + needle.len();
            surface_completion_context_for_document(source, cursor, &document)
        };

        assert!(
            context_at("\ns\n")
                .slots
                .contains(&SemanticCompletionSlot::AuthoringChildren(
                    AuthoringKind::SoundsConfig
                ))
        );
        assert!(
            context_at("\ng\n")
                .slots
                .contains(&SemanticCompletionSlot::AuthoringChildren(
                    AuthoringKind::PuzzleRenderConfig
                ))
        );
        assert!(
            context_at("background_color = li")
                .slots
                .contains(&SemanticCompletionSlot::Colors)
        );
        assert!(
            context_at("type = \"o")
                .slots
                .iter()
                .any(|slot| matches!(slot, SemanticCompletionSlot::Literals(_)))
        );
        let unknown = context_at("unknown = z");
        assert!(unknown.slots.is_empty());
        assert!(unknown.warnings[0].contains("assignment key `unknown`"));
    }

    #[test]
    fn completion_symbols_are_surface_document_product() {
        let source = r#"
title = completion_surface_product
puzzle board {
tags {
facing = left right
}
slots {
actor = Player:facing
}
}
"#;
        let symbols = crate::parse_surface_document(source).completion_symbols;
        assert!(symbols.objects.contains("Player"));
        assert!(symbols.value_set_names.contains("facing"));
        assert_eq!(
            symbols.object_axes.get("Player"),
            Some(&vec!["facing".to_string()])
        );
    }

    #[test]
    fn completion_context_entrypoint_requires_surface_document() {
        let source = include_str!("surface_completion.rs");
        let start = source
            .find("pub(crate) fn surface_completion_context_for_document")
            .unwrap();
        let end = source[start..]
            .find("fn contextual_completion_slots")
            .map(|offset| start + offset)
            .unwrap();
        let body = &source[start..end];
        assert!(body.contains("document: &SurfaceDocument"));

        let forbidden_fragments = [
            ["scan_source", "_context"],
            ["parse_game", "2d"],
            ["parse_surface", "_document"],
            ["for line", " in"],
            ["struct ", "SurfaceCompletionSymbols"],
        ];
        for parts in forbidden_fragments {
            let forbidden = parts.concat();
            assert!(
                !body.contains(&forbidden),
                "completion symbol collection must consume SurfaceDocument, not rebuild parser context via {forbidden}"
            );
        }
    }

    #[test]
    fn completion_context_consumes_surface_document_lines() {
        let source = include_str!("surface_completion.rs");
        let forbidden_fragments = [
            ["scan_source", "_context"],
            ["Source", "Context"],
            ["SourceCon", "textLine"],
            ["surface_option", "_block_before_line"],
            ["fall", "back"],
        ];
        for parts in forbidden_fragments {
            let forbidden = parts.concat();
            assert!(
                !source.contains(&forbidden),
                "surface_completion.rs must consume SurfaceDocument line products, not rebuild source context via {forbidden}"
            );
        }
    }
}
