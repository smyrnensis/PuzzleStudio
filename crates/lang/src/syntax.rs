// Parser-owned authoring vocabulary shared by parsing, completion, and
// highlighting so new surface syntax is registered in one place.

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

pub(crate) fn is_puzzle_lifecycle_block(token: &str) -> bool {
    PUZZLE_LIFECYCLE_BLOCKS.contains(&token)
}

pub(crate) fn is_puzzle_line_head_keyword(token: &str) -> bool {
    PUZZLE_LINE_HEAD_KEYWORDS.contains(&token)
}

pub(crate) fn is_parser_keyword(token: &str) -> bool {
    PARSER_KEYWORDS.contains(&token)
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

pub(crate) const PUZZLE_LINE_HEAD_KEYWORDS: &[&str] = &[
    "collision_layers",
    "condition",
    "const",
    "direction",
    "for",
    "groups",
    "if",
    "input",
    "keys",
    "layers",
    "legend",
    "levels",
    "levels3",
    "map",
    "on_display",
    PUZZLE_LIFECYCLE_BLOCKS[0],
    PUZZLE_LIFECYCLE_BLOCKS[1],
    PUZZLE_LIFECYCLE_BLOCKS[2],
    "persistent",
    "render",
    "resources",
    "routine",
    "rule",
    "rules",
    "scratch",
    "sounds",
    "sprites",
    "sprites3",
    "tags",
    "var",
];

pub(crate) const PUZZLE_COMPLETION_KEYWORDS: &[&str] = &[
    "collision_layers",
    "condition",
    "const",
    "direction",
    "for",
    "groups",
    "if",
    "input",
    "keys",
    "layers",
    "legend",
    "level",
    "levels",
    "levels3",
    "lose_conditions",
    "map",
    "on_display",
    PUZZLE_LIFECYCLE_BLOCKS[0],
    PUZZLE_LIFECYCLE_BLOCKS[1],
    PUZZLE_LIFECYCLE_BLOCKS[2],
    "once",
    "once_all",
    "once_per_level",
    "persistent",
    "repeat",
    "resources",
    "render",
    "routine",
    "rule",
    "rules",
    "scratch",
    "sounds",
    "sprites",
    "sprites3",
    "state",
    "tags",
    "var",
    "win_conditions",
];

pub(crate) const PARSER_KEYWORDS: &[&str] = &[
    "again_interval",
    "assets",
    "align",
    "author",
    "sounds",
    "button",
    "camera",
    "column",
    "component_effect",
    "const",
    "colors",
    "collision_layers",
    "css",
    "direction",
    "default_wait_time",
    "effect",
    "each",
    "else",
    "file",
    "flickscreen",
    "for",
    "gap",
    "screen_focus",
    "from",
    "grid",
    "homepage",
    "puzzle",
    "groups",
    "if",
    "in",
    "import",
    "input",
    "interactive_look",
    "interactive_zoom",
    "keys",
    "layers",
    "legend",
    "level",
    "level_menu",
    "levels",
    "levels3",
    "lose_conditions",
    "map",
    "music",
    "name",
    "occupied_cells",
    "on",
    "on_display",
    "on_scene_start",
    "of",
    "once",
    "once_all",
    "once_per_level",
    "box",
    "persistent",
    "pitch",
    "puzzle3",
    "condition",
    "region",
    "repeat",
    "resources",
    "render",
    "row",
    "routine",
    "rule",
    "rules",
    "scene",
    "script",
    "scratch",
    "sfx",
    "shape",
    "show_index",
    "show_solved",
    "size",
    "sprite",
    "sprites",
    "sprites3",
    "state",
    "tags",
    "subtitle",
    "text",
    "theme",
    "title",
    "var",
    "layout",
    "win_conditions",
    "with",
    "yaw",
    "zoom",
    "zoomscreen",
    PUZZLE_LIFECYCLE_BLOCKS[0],
    PUZZLE_LIFECYCLE_BLOCKS[1],
    PUZZLE_LIFECYCLE_BLOCKS[2],
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
