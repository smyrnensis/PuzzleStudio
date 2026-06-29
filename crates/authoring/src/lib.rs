pub const NEW_PUZZLE_DEFAULT_TITLE: &str = "New Puzzle";
pub const NEW_PUZZLE_TEMPLATE: &str = include_str!("../templates/new.puzzle");

pub fn new_puzzle_source(title: &str) -> String {
    let default_title_line = format!("title {NEW_PUZZLE_DEFAULT_TITLE:?}");
    let Some(rest) = NEW_PUZZLE_TEMPLATE.strip_prefix(&default_title_line) else {
        panic!("new puzzle template must start with the default title line");
    };
    format!("title {title:?}{rest}")
}

pub fn is_display_object_token(token: &str) -> bool {
    let Some(rest) = token.strip_prefix('@') else {
        return false;
    };
    let without_scratch = rest.split_once('{').map_or(rest, |(base, _)| base);
    let base = without_scratch
        .split_once(':')
        .map_or(without_scratch, |(base, _)| base);
    is_identifier(base)
}

pub fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return false;
    }
    chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

pub fn is_qualified_identifier(value: &str) -> bool {
    let mut parts = value.split(':');
    let Some(first) = parts.next() else {
        return false;
    };
    is_identifier(first) && parts.all(is_identifier)
}

pub fn split_object_spec(token: &str) -> Option<(&str, impl Iterator<Item = &str> + '_)> {
    let mut parts = token.split(':');
    let base = parts.next()?;
    (!base.is_empty()).then_some((base, parts))
}

pub fn split_header_tokens(line: &str) -> Vec<&str> {
    let mut tokens = line.split_whitespace().collect::<Vec<_>>();
    if tokens.len() > 1 && tokens.last().copied() == Some("{") {
        tokens.pop();
    }
    tokens
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleProgramBlockSurface<'a> {
    Rules { modifier: &'a str },
    OnLevelStart { modifier: &'a str },
    OnLevelClear,
    OnLastLevelClear,
}

pub fn rule_program_block_surface(line: &str) -> Option<RuleProgramBlockSurface<'_>> {
    if let Some(modifier) = named_block_header_modifier(line, "rules") {
        return Some(RuleProgramBlockSurface::Rules { modifier });
    }
    if let Some(modifier) = named_block_header_modifier(line, "on_level_start") {
        return Some(RuleProgramBlockSurface::OnLevelStart { modifier });
    }
    if named_block_header_modifier(line, "on_level_clear").is_some() {
        return Some(RuleProgramBlockSurface::OnLevelClear);
    }
    if named_block_header_modifier(line, "on_last_level_clear").is_some() {
        return Some(RuleProgramBlockSurface::OnLastLevelClear);
    }
    None
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleStatementBlockSurface<'a> {
    Program(RuleProgramBlockSurface<'a>),
    Routine,
    DisplayHook,
    Nested,
}

pub fn rule_statement_block_surface(
    line: &str,
    parent_is_statement_block: bool,
) -> Option<RuleStatementBlockSurface<'_>> {
    let trimmed = line.trim();
    trimmed.strip_suffix('{')?;
    if let Some(program) = rule_program_block_surface(trimmed) {
        return Some(RuleStatementBlockSurface::Program(program));
    }
    let tokens = split_header_tokens(trimmed);
    match tokens.first().copied()? {
        "routine" => Some(RuleStatementBlockSurface::Routine),
        "on_display" => Some(RuleStatementBlockSurface::DisplayHook),
        _ if parent_is_statement_block && nested_rule_statement_block_surface(trimmed, &tokens) => {
            Some(RuleStatementBlockSurface::Nested)
        }
        _ => None,
    }
}

fn nested_rule_statement_block_surface(line: &str, tokens: &[&str]) -> bool {
    if line
        .strip_suffix('{')
        .map(str::trim_end)
        .is_some_and(|head| head.contains("->"))
    {
        return true;
    }
    match tokens.first().copied() {
        Some("display" | "else" | "fix" | "for" | "if") => true,
        Some("repeat") if tokens.get(1).copied() == Some("until") => true,
        Some(first) => rule_application_surface(first).is_some(),
        None => false,
    }
}

fn named_block_header_modifier<'a>(line: &'a str, keyword: &str) -> Option<&'a str> {
    let head = line.trim().strip_suffix('{')?.trim_end();
    let rest = head.strip_prefix(keyword)?;
    if rest.is_empty() {
        return Some("");
    }
    rest.strip_prefix(' ').map(str::trim)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StandardRuleStepSurface {
    Move,
}

pub const STANDARD_RULE_STEP_NAMES: &[&str] = &["move"];

pub fn standard_rule_step_surface(line: &str) -> Option<StandardRuleStepSurface> {
    match line.trim() {
        "move" => Some(StandardRuleStepSurface::Move),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleApplicationSurface {
    Once,
    OnceAll,
    OncePerLevel,
    Repeat,
}

pub const RULE_STATEMENT_HEAD_KEYWORDS: &[&str] = &[
    "display",
    "for",
    "if",
    "input",
    "once",
    "once_all",
    "once_per_level",
    "repeat",
];

pub fn rule_application_surface(token: &str) -> Option<RuleApplicationSurface> {
    match token {
        "once" => Some(RuleApplicationSurface::Once),
        "once_all" => Some(RuleApplicationSurface::OnceAll),
        "once_per_level" => Some(RuleApplicationSurface::OncePerLevel),
        "repeat" => Some(RuleApplicationSurface::Repeat),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleStatementSurface<'a> {
    ApplicationBlock { application: RuleApplicationSurface },
    RuleLine(RuleLineSurface<'a>),
    Call { name: &'a str },
}

pub fn rule_statement_surface(
    line: &str,
) -> Result<RuleStatementSurface<'_>, RuleLineSurfaceError> {
    let line = line.trim();
    let tokens = split_header_tokens(line);
    if let [application] = tokens.as_slice()
        && let Some(application) = rule_application_surface(application)
    {
        return Ok(RuleStatementSurface::ApplicationBlock { application });
    }
    if tokens.len() == 1
        && is_qualified_identifier(tokens[0])
        && standard_rule_step_surface(line).is_none()
    {
        return Ok(RuleStatementSurface::Call { name: tokens[0] });
    }
    rule_line_surface(line).map(RuleStatementSurface::RuleLine)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleLineSurface<'a> {
    StandardStep(StandardRuleStepSurface),
    InputRewrite {
        application: Option<RuleApplicationSurface>,
        surface: InputRewriteSurface<'a>,
    },
    NeutralRewrite {
        application: Option<RuleApplicationSurface>,
        rewrite: &'a str,
    },
    OrientedRewrite {
        application: Option<RuleApplicationSurface>,
        orientation: &'a str,
        rewrite: &'a str,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleLineSurfaceError {
    Input(InputRewriteSurfaceError),
    MissingOrientation,
    RewriteMustStartWithPattern,
}

impl RuleLineSurfaceError {
    pub fn message(self) -> &'static str {
        match self {
            Self::Input(error) => error.message(),
            Self::MissingOrientation | Self::RewriteMustStartWithPattern => {
                "rule must be: <orientation> [ ... ] -> [ ... ]"
            }
        }
    }
}

pub fn rule_line_surface(line: &str) -> Result<RuleLineSurface<'_>, RuleLineSurfaceError> {
    let line = line.trim();
    if let Some(step) = standard_rule_step_surface(line) {
        return Ok(RuleLineSurface::StandardStep(step));
    }
    let (application, line) = split_rule_application_prefix(line)?;
    if let Some(surface) = input_rewrite_surface(line).map_err(RuleLineSurfaceError::Input)? {
        return Ok(RuleLineSurface::InputRewrite {
            application,
            surface,
        });
    }
    if line.starts_with('[') {
        return Ok(RuleLineSurface::NeutralRewrite {
            application,
            rewrite: line,
        });
    }
    let (orientation, rewrite) = line
        .split_once(' ')
        .ok_or(RuleLineSurfaceError::MissingOrientation)?;
    let rewrite = rewrite.trim_start();
    if !rewrite.starts_with('[') {
        return Err(RuleLineSurfaceError::RewriteMustStartWithPattern);
    }
    Ok(RuleLineSurface::OrientedRewrite {
        application,
        orientation,
        rewrite,
    })
}

fn split_rule_application_prefix(
    line: &str,
) -> Result<(Option<RuleApplicationSurface>, &str), RuleLineSurfaceError> {
    let Some((first, rest)) = line.split_once(char::is_whitespace) else {
        return Ok((None, line));
    };
    let Some(application) = rule_application_surface(first) else {
        return Ok((None, line));
    };
    let rest = rest.trim_start();
    if rest.is_empty() {
        return Err(RuleLineSurfaceError::MissingOrientation);
    }
    Ok((Some(application), rest))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputRewriteSurface<'a> {
    pub orientation: Option<&'a str>,
    pub rewrite: &'a str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputRewriteSurfaceError {
    MissingRewrite,
    RewriteMustStartWithPattern,
}

impl InputRewriteSurfaceError {
    pub fn message(self) -> &'static str {
        match self {
            Self::MissingRewrite => "input rule must be: input <orientation> [ ... ] -> [ ... ]",
            Self::RewriteMustStartWithPattern => {
                "input rule must be: input <orientation> [ ... ] -> [ ... ]"
            }
        }
    }
}

pub fn input_rewrite_surface(
    line: &str,
) -> Result<Option<InputRewriteSurface<'_>>, InputRewriteSurfaceError> {
    let Some(rest) = line.trim().strip_prefix("input ").map(str::trim_start) else {
        return Ok(None);
    };
    if rest.starts_with('[') {
        return Ok(Some(InputRewriteSurface {
            orientation: None,
            rewrite: rest,
        }));
    }

    let (orientation, rewrite) = rest
        .split_once(' ')
        .ok_or(InputRewriteSurfaceError::MissingRewrite)?;
    let rewrite = rewrite.trim_start();
    if !rewrite.starts_with('[') {
        return Err(InputRewriteSurfaceError::RewriteMustStartWithPattern);
    }
    Ok(Some(InputRewriteSurface {
        orientation: Some(orientation),
        rewrite,
    }))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScratchSugarKind {
    Movement,
    Bool,
    Int,
}

pub const ANONYMOUS_MOVEMENT_SCRATCH_INDEX: u16 = 0;
pub const MOVEMENT_DIRECTIONS_2D: &[&str] = &["up", "down", "left", "right"];
pub const MOVEMENT_DIRECTIONS_3D: &[&str] = &["up", "down", "left", "right", "front", "back"];

pub fn scratch_sugar_kind(token: &str) -> Option<ScratchSugarKind> {
    if matches!(
        token,
        ">" | "<"
            | "^"
            | "v"
            | "up"
            | "down"
            | "left"
            | "right"
            | "front"
            | "back"
            | "forward"
            | "backward"
            | "directions"
            | "horizontal"
            | "vertical"
            | "parallel"
            | "perpendicular"
    ) {
        Some(ScratchSugarKind::Movement)
    } else if matches!(token, "true" | "false") {
        Some(ScratchSugarKind::Bool)
    } else if token.parse::<i64>().is_ok() {
        Some(ScratchSugarKind::Int)
    } else {
        None
    }
}

pub fn canonical_3d_movement_direction_name(value: &str) -> &str {
    match value {
        "forward" => "front",
        "backward" => "back",
        other => other,
    }
}

pub fn movement_scratch_index(value: &str, directions: &[&str]) -> Option<u16> {
    directions
        .iter()
        .position(|direction| *direction == value)
        .and_then(|index| u16::try_from(index).ok())
}

pub fn movement_scratch_index_3d(value: &str) -> Option<u16> {
    movement_scratch_index(
        canonical_3d_movement_direction_name(value),
        MOVEMENT_DIRECTIONS_3D,
    )
}

pub fn movement_scratch_set_values(value: &str, dimensions: u8) -> Option<&'static [&'static str]> {
    match (value, dimensions) {
        ("directions", 2) => Some(MOVEMENT_DIRECTIONS_2D),
        ("directions", 3) => Some(MOVEMENT_DIRECTIONS_3D),
        ("horizontal", 2) => Some(&["left", "right"]),
        ("horizontal", 3) => Some(&["left", "right", "front", "back"]),
        ("vertical", 2) => Some(&["up", "down"]),
        ("vertical", 3) => Some(&["up", "down"]),
        ("parallel", 2) => Some(&["<", ">"]),
        ("perpendicular", 2) => Some(&["^", "v"]),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StandardMoveObject {
    pub object: u16,
    pub layer: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StandardMoveRulePlan {
    pub object: u16,
    pub direction_index: u16,
    pub layer_objects: Vec<u16>,
}

pub fn standard_move_rule_plans(
    objects: impl IntoIterator<Item = StandardMoveObject>,
    direction_count: u16,
) -> Vec<StandardMoveRulePlan> {
    let objects = objects.into_iter().collect::<Vec<_>>();
    let mut plans = Vec::new();
    for object in &objects {
        let layer_objects = objects
            .iter()
            .filter_map(|candidate| (candidate.layer == object.layer).then_some(candidate.object))
            .collect::<Vec<_>>();
        for direction_index in 0..direction_count {
            plans.push(StandardMoveRulePlan {
                object: object.object,
                direction_index,
                layer_objects: layer_objects.clone(),
            });
        }
    }
    plans
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CellTokenError {
    UnmatchedCloseBrace,
    MissingCloseBrace,
}

pub fn split_cell_tokens(cell: &str) -> Result<Vec<String>, CellTokenError> {
    let mut tokens = Vec::new();
    let mut token = String::new();
    let mut brace_depth = 0_u16;
    for ch in cell.chars() {
        match ch {
            '{' => {
                brace_depth += 1;
                token.push(ch);
            }
            '}' => {
                if brace_depth == 0 {
                    return Err(CellTokenError::UnmatchedCloseBrace);
                }
                brace_depth -= 1;
                token.push(ch);
            }
            ch if ch.is_whitespace() && brace_depth == 0 => {
                if !token.is_empty() {
                    tokens.push(std::mem::take(&mut token));
                }
            }
            _ => token.push(ch),
        }
    }
    if brace_depth != 0 {
        return Err(CellTokenError::MissingCloseBrace);
    }
    if !token.is_empty() {
        tokens.push(token);
    }
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_puzzle_source_replaces_only_template_title() {
        let source = new_puzzle_source("Custom Puzzle");

        assert!(NEW_PUZZLE_TEMPLATE.starts_with("title \"New Puzzle\"\n"));
        assert!(!NEW_PUZZLE_TEMPLATE.contains('\t'));
        assert!(
            !NEW_PUZZLE_TEMPLATE
                .lines()
                .any(|line| line.starts_with(' '))
        );
        assert!(source.starts_with("title \"Custom Puzzle\"\n"));
        assert!(source.contains("puzzle main {"));
        assert!(source.contains("levels main of main {"));
        assert!(!source.contains("keys {"));
        assert!(source.contains("layers 1"));
        assert!(!source.contains("base ="));
        assert!(!source.contains("floor ="));
        assert!(!source.contains("solid ="));
        assert!(!source.contains("scene title {"));
        assert!(!source.contains("scene level_select {"));
        assert!(!source.contains("scene playing {"));
        assert!(!source.contains("<-"));
        assert!(!source.contains("inputs {"));
        assert!(!source.contains("exists("));
        assert!(!source.contains("none("));
        assert!(!source.contains("input directions"));
    }

    #[test]
    fn at_name_marks_display_object_tokens() {
        assert!(is_display_object_token("@Trail"));
        assert!(is_display_object_token("@Trail:kind"));
        assert!(is_display_object_token("@Trail{right}"));
        assert!(!is_display_object_token("Trail"));
        assert!(!is_display_object_token("@"));
        assert!(!is_display_object_token("@:kind"));
    }

    #[test]
    fn shared_scratch_sugar_recognizes_2d_and_3d_direction_words() {
        assert_eq!(scratch_sugar_kind(">"), Some(ScratchSugarKind::Movement));
        assert_eq!(
            scratch_sugar_kind("front"),
            Some(ScratchSugarKind::Movement)
        );
        assert_eq!(scratch_sugar_kind("true"), Some(ScratchSugarKind::Bool));
        assert_eq!(scratch_sugar_kind("7"), Some(ScratchSugarKind::Int));
        assert_eq!(scratch_sugar_kind("Player"), None);
    }

    #[test]
    fn shared_rule_surface_recognizes_common_input_rewrite_and_move_step() {
        assert_eq!(
            split_header_tokens("rules local_frame 3 full {"),
            vec!["rules", "local_frame", "3", "full"]
        );
        assert_eq!(
            rule_program_block_surface("rules local_frame 3 full {"),
            Some(RuleProgramBlockSurface::Rules {
                modifier: "local_frame 3 full"
            })
        );
        assert_eq!(
            rule_program_block_surface("on_level_start {"),
            Some(RuleProgramBlockSurface::OnLevelStart { modifier: "" })
        );
        assert_eq!(
            rule_program_block_surface("on_level_clear {"),
            Some(RuleProgramBlockSurface::OnLevelClear)
        );
        assert_eq!(
            rule_statement_block_surface("rules local_frame 3 full {", false),
            Some(RuleStatementBlockSurface::Program(
                RuleProgramBlockSurface::Rules {
                    modifier: "local_frame 3 full"
                }
            ))
        );
        assert_eq!(
            rule_statement_block_surface("routine slide once {", false),
            Some(RuleStatementBlockSurface::Routine)
        );
        assert_eq!(
            rule_statement_block_surface("if true {", true),
            Some(RuleStatementBlockSurface::Nested)
        );
        assert_eq!(
            rule_statement_block_surface("restart -> {", true),
            Some(RuleStatementBlockSurface::Nested)
        );
        assert_eq!(rule_statement_block_surface("render {", true), None);
        assert_eq!(
            standard_rule_step_surface("move"),
            Some(StandardRuleStepSurface::Move)
        );
        assert_eq!(
            rule_line_surface("move").unwrap(),
            RuleLineSurface::StandardStep(StandardRuleStepSurface::Move)
        );
        assert_eq!(
            rule_statement_surface("once {").unwrap(),
            RuleStatementSurface::ApplicationBlock {
                application: RuleApplicationSurface::Once
            }
        );
        assert_eq!(
            rule_statement_surface("push_boxes").unwrap(),
            RuleStatementSurface::Call { name: "push_boxes" }
        );
        assert_eq!(
            input_rewrite_surface("input [ Player ] -> [ > Player ]").unwrap(),
            Some(InputRewriteSurface {
                orientation: None,
                rewrite: "[ Player ] -> [ > Player ]",
            })
        );
        assert_eq!(
            rule_line_surface("input [ Player ] -> [ > Player ]").unwrap(),
            RuleLineSurface::InputRewrite {
                application: None,
                surface: InputRewriteSurface {
                    orientation: None,
                    rewrite: "[ Player ] -> [ > Player ]",
                },
            }
        );
        assert_eq!(
            rule_line_surface("once input [ Player ] -> [ > Player ]").unwrap(),
            RuleLineSurface::InputRewrite {
                application: Some(RuleApplicationSurface::Once),
                surface: InputRewriteSurface {
                    orientation: None,
                    rewrite: "[ Player ] -> [ > Player ]",
                },
            }
        );
        assert_eq!(
            input_rewrite_surface("input horizontal [ Player ] -> [ > Player ]").unwrap(),
            Some(InputRewriteSurface {
                orientation: Some("horizontal"),
                rewrite: "[ Player ] -> [ > Player ]",
            })
        );
        assert_eq!(
            rule_line_surface("right [ Player ] -> [ > Player ]").unwrap(),
            RuleLineSurface::OrientedRewrite {
                application: None,
                orientation: "right",
                rewrite: "[ Player ] -> [ > Player ]",
            }
        );
        assert_eq!(
            rule_line_surface("repeat right [ Player ] -> [ > Player ]").unwrap(),
            RuleLineSurface::OrientedRewrite {
                application: Some(RuleApplicationSurface::Repeat),
                orientation: "right",
                rewrite: "[ Player ] -> [ > Player ]",
            }
        );
        assert_eq!(
            rule_line_surface("[ > Player | Box ] -> [ > Player | > Box ]").unwrap(),
            RuleLineSurface::NeutralRewrite {
                application: None,
                rewrite: "[ > Player | Box ] -> [ > Player | > Box ]",
            }
        );
        assert_eq!(
            rule_line_surface("once_all [ > Player | Box ] -> [ > Player | > Box ]").unwrap(),
            RuleLineSurface::NeutralRewrite {
                application: Some(RuleApplicationSurface::OnceAll),
                rewrite: "[ > Player | Box ] -> [ > Player | > Box ]",
            }
        );
    }

    #[test]
    fn shared_movement_contract_resolves_direction_aliases_and_sets() {
        assert_eq!(
            movement_scratch_index("right", MOVEMENT_DIRECTIONS_2D),
            Some(3)
        );
        assert_eq!(movement_scratch_index_3d("forward"), Some(4));
        assert_eq!(
            movement_scratch_index("forward", MOVEMENT_DIRECTIONS_2D),
            None,
            "forward/backward aliases are 3D-specific"
        );
        assert_eq!(
            movement_scratch_set_values("horizontal", 3),
            Some(["left", "right", "front", "back"].as_slice())
        );
        assert_eq!(
            movement_scratch_set_values("perpendicular", 3),
            None,
            "relative 2D movement sets are not defined for 3D line space"
        );
    }

    #[test]
    fn shared_cell_tokenizer_keeps_scratch_blocks_together() {
        assert_eq!(
            split_cell_tokens("Player{> no flag} no Wall").unwrap(),
            vec!["Player{> no flag}", "no", "Wall"]
        );
    }

    #[test]
    fn standard_move_plan_expands_objects_by_layer_and_direction() {
        let plans = standard_move_rule_plans(
            [
                StandardMoveObject {
                    object: 1,
                    layer: 0,
                },
                StandardMoveObject {
                    object: 2,
                    layer: 0,
                },
                StandardMoveObject {
                    object: 3,
                    layer: 1,
                },
            ],
            2,
        );

        assert_eq!(
            plans,
            vec![
                StandardMoveRulePlan {
                    object: 1,
                    direction_index: 0,
                    layer_objects: vec![1, 2],
                },
                StandardMoveRulePlan {
                    object: 1,
                    direction_index: 1,
                    layer_objects: vec![1, 2],
                },
                StandardMoveRulePlan {
                    object: 2,
                    direction_index: 0,
                    layer_objects: vec![1, 2],
                },
                StandardMoveRulePlan {
                    object: 2,
                    direction_index: 1,
                    layer_objects: vec![1, 2],
                },
                StandardMoveRulePlan {
                    object: 3,
                    direction_index: 0,
                    layer_objects: vec![3],
                },
                StandardMoveRulePlan {
                    object: 3,
                    direction_index: 1,
                    layer_objects: vec![3],
                },
            ]
        );
    }
}
