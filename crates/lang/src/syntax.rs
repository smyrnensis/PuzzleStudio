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

pub(crate) fn puzzle_lifecycle_block(token: &str) -> Option<&'static str> {
    match token {
        "on_level_start" => Some(PUZZLE_LIFECYCLE_BLOCKS[0]),
        "on_level_clear" => Some(PUZZLE_LIFECYCLE_BLOCKS[1]),
        "on_last_level_clear" => Some(PUZZLE_LIFECYCLE_BLOCKS[2]),
        _ => None,
    }
}

pub(crate) fn is_puzzle_lifecycle_block(token: &str) -> bool {
    PUZZLE_LIFECYCLE_BLOCKS.contains(&token)
}

pub(crate) fn is_parser_keyword(token: &str) -> bool {
    PARSER_KEYWORDS.contains(&token)
}

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
    "display",
    "effect",
    "each",
    "else",
    "flickscreen",
    "for",
    "gap",
    "screen_focus",
    "from",
    "grid",
    "homepage",
    "puzzle",
    "group",
    "groups",
    "if",
    "in",
    "import",
    "input",
    "inputs",
    "interactive_look",
    "interactive_zoom",
    "keys",
    "layer",
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
    "objects",
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
