// Parser-owned authoring vocabulary shared by parsing and completion.

pub(crate) const PUZZLE_LIFECYCLE_BLOCKS: &[&str] =
    &["on_level_start", "on_level_clear", "on_last_level_clear"];

pub(crate) fn puzzle_lifecycle_event(block: &str) -> Option<&'static str> {
    match block {
        "on_level_start" => Some("level_start"),
        "on_level_clear" => Some("level_clear"),
        "on_last_level_clear" => Some("last_level_clear"),
        _ => None,
    }
}

pub(crate) fn is_visual_named_color(token: &str) -> bool {
    let lower = token.to_ascii_lowercase();
    VISUAL_COLOR_NAMES.contains(&lower.as_str())
}

pub(crate) const VISUAL_COLOR_NAMES: &[&str] = &[
    "transparent",
    "currentcolor",
    "black",
    "silver",
    "gray",
    "grey",
    "darkgray",
    "darkgrey",
    "lightgray",
    "lightgrey",
    "white",
    "maroon",
    "red",
    "darkred",
    "lightred",
    "purple",
    "fuchsia",
    "green",
    "darkgreen",
    "lightgreen",
    "lime",
    "olive",
    "yellow",
    "navy",
    "blue",
    "darkblue",
    "lightblue",
    "teal",
    "aqua",
    "orange",
    "brown",
    "darkbrown",
    "pink",
];

pub(crate) const PUZZLE_COMPLETION_KEYWORDS: &[&str] = &[
    "collision_layers",
    "const",
    "direction",
    "for",
    "groups",
    "if",
    "input",
    "keys",
    "slots",
    "legend",
    "level",
    "levels",
    "lose_conditions",
    "map",
    PUZZLE_LIFECYCLE_BLOCKS[0],
    PUZZLE_LIFECYCLE_BLOCKS[1],
    PUZZLE_LIFECYCLE_BLOCKS[2],
    "once",
    "once_all",
    "once_per_level",
    "persistent",
    "query",
    "repeat",
    "resources",
    "render",
    "routine",
    "rules",
    "solver",
    "marks",
    "sounds",
    "sprites",
    "state",
    "tags",
    "var",
    "win_conditions",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ExpectedCompletionValue {
    Selector,
    LegendEmpty,
    VisualDirective,
    SpriteSelector,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AssignmentSyntax {
    pub(crate) rhs_start: usize,
    pub(crate) expected_completion_values: &'static [ExpectedCompletionValue],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NamedBlockDeclarationSyntax<'a> {
    pub(crate) name: &'a str,
}

const SELECTOR_EXPECTED: &[ExpectedCompletionValue] = &[ExpectedCompletionValue::Selector];
const LEGEND_EXPECTED: &[ExpectedCompletionValue] = &[
    ExpectedCompletionValue::LegendEmpty,
    ExpectedCompletionValue::Selector,
];
const VISUAL_LINE_HEAD_EXPECTED: &[ExpectedCompletionValue] = &[
    ExpectedCompletionValue::VisualDirective,
    ExpectedCompletionValue::SpriteSelector,
];

pub(crate) fn visual_line_head_expected_completion_values() -> &'static [ExpectedCompletionValue] {
    VISUAL_LINE_HEAD_EXPECTED
}

pub(crate) fn named_block_declaration_syntax<'a>(
    tokens: &'a [&'a str],
    keyword: &str,
) -> Option<NamedBlockDeclarationSyntax<'a>> {
    match tokens {
        [head, name] if *head == keyword => Some(NamedBlockDeclarationSyntax { name }),
        _ => None,
    }
}

pub(crate) fn legend_block_row_syntax(
    tokens: &[&str],
    require_rhs: bool,
) -> Option<AssignmentSyntax> {
    assignment_syntax(tokens, 1, require_rhs, LEGEND_EXPECTED)
}

pub(crate) fn legend_directive_syntax(
    tokens: &[&str],
    require_rhs: bool,
) -> Option<AssignmentSyntax> {
    if tokens.first().copied()? != "legend" {
        return None;
    }
    assignment_syntax(tokens, 2, require_rhs, LEGEND_EXPECTED)
}

pub(crate) fn level_legend_directive_syntax(
    tokens: &[&str],
    require_rhs: bool,
) -> Option<AssignmentSyntax> {
    if tokens.first().copied()? != "legend" {
        return None;
    }
    assignment_syntax(tokens, 2, require_rhs, SELECTOR_EXPECTED)
}

pub(crate) fn named_selector_assignment_syntax(
    tokens: &[&str],
    require_rhs: bool,
) -> Option<AssignmentSyntax> {
    assignment_syntax(tokens, 1, require_rhs, SELECTOR_EXPECTED)
}

fn assignment_syntax(
    tokens: &[&str],
    lhs_token_count: usize,
    require_rhs: bool,
    expected_completion_values: &'static [ExpectedCompletionValue],
) -> Option<AssignmentSyntax> {
    let separator = lhs_token_count;
    if tokens.get(separator).copied()? != "=" {
        return None;
    }
    let rhs_start = separator + 1;
    if require_rhs && tokens.len() <= rhs_start {
        return None;
    }
    Some(AssignmentSyntax {
        rhs_start,
        expected_completion_values,
    })
}
