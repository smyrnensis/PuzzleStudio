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
    VISUAL_NAMED_COLORS.iter().any(|(name, _)| *name == lower)
}

pub(crate) fn is_visual_color_literal(token: &str) -> bool {
    is_visual_named_color(token) || is_visual_hex_color(token)
}

pub(crate) fn canonical_visual_color_literal(token: &str) -> Option<String> {
    if is_visual_hex_color(token) {
        return Some(token.to_string());
    }
    let lower = token.to_ascii_lowercase();
    VISUAL_NAMED_COLORS
        .iter()
        .find_map(|(name, color)| (*name == lower).then(|| (*color).to_string()))
}

fn is_visual_hex_color(token: &str) -> bool {
    let Some(hex) = token.strip_prefix('#') else {
        return false;
    };
    matches!(hex.len(), 3 | 4 | 6 | 8) && hex.chars().all(|ch| ch.is_ascii_hexdigit())
}

pub(crate) const VISUAL_NAMED_COLORS: &[(&str, &str)] = &[
    ("transparent", "transparent"),
    ("currentcolor", "currentcolor"),
    ("black", "#000000"),
    ("silver", "#c0c0c0"),
    ("gray", "#808080"),
    ("grey", "#808080"),
    ("darkgray", "#404040"),
    ("darkgrey", "#404040"),
    ("lightgray", "#c0c0c0"),
    ("lightgrey", "#c0c0c0"),
    ("white", "#ffffff"),
    ("maroon", "#800000"),
    ("red", "#ff0000"),
    ("darkred", "#800000"),
    ("lightred", "#ff8080"),
    ("purple", "#800080"),
    ("fuchsia", "#ff00ff"),
    ("green", "#008000"),
    ("darkgreen", "#006400"),
    ("lightgreen", "#90ee90"),
    ("lime", "#00ff00"),
    ("olive", "#808000"),
    ("yellow", "#ffff00"),
    ("navy", "#000080"),
    ("blue", "#0000ff"),
    ("darkblue", "#00008b"),
    ("lightblue", "#add8e6"),
    ("teal", "#008080"),
    ("aqua", "#00ffff"),
    ("orange", "#ffa500"),
    ("brown", "#a46322"),
    ("darkbrown", "#493c2b"),
    ("pink", "#ffc0cb"),
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
    "visuals",
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
    VisualSelector,
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
    ExpectedCompletionValue::VisualSelector,
];

pub(crate) const DEFAULT_LEVEL_EMPTY_CHAR: char = '.';

pub(crate) fn level_legend_char_requires_import_remap(ch: char) -> bool {
    ch == DEFAULT_LEVEL_EMPTY_CHAR || matches!(ch, '{' | '}' | '"' | ';')
}

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
