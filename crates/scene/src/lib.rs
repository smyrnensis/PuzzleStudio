#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SceneLayout {
    pub size: Option<SceneSize>,
    pub gap: Option<u16>,
    pub align: SceneAlign,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SceneSize {
    pub width: u16,
    pub height: u16,
}

impl SceneSize {
    pub fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SceneAlign {
    pub x: SceneAlignX,
    pub y: SceneAlignY,
}

impl Default for SceneAlign {
    fn default() -> Self {
        Self {
            x: SceneAlignX::Center,
            y: SceneAlignY::Center,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneAlignX {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneAlignY {
    Top,
    Center,
    Bottom,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Scene<Effect = SceneCommand, LabelExpr = SceneTextExpr, TextExpr = SceneTextExpr> {
    pub name: String,
    pub layout: SceneLayout,
    pub components: Vec<SceneComponent<Effect, LabelExpr, TextExpr>>,
    pub inputs: Vec<SceneInputBinding>,
    pub transitions: Vec<SceneTransition<Effect>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneInputBinding {
    pub input: String,
    pub keys: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneTransition<Effect = SceneCommand> {
    pub trigger: SceneTransitionTrigger,
    pub effect: Effect,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SceneTransitionTrigger {
    Condition(String),
    SceneStart,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SceneComponent<Effect = SceneCommand, LabelExpr = SceneTextExpr, TextExpr = SceneTextExpr>
{
    Frame(FrameComponent),
    Title(SceneTextComponent<LabelExpr>),
    Subtitle(SceneTextComponent<LabelExpr>),
    Text(SceneTextComponent<TextExpr>),
    Button(SceneButton<Effect, LabelExpr>),
    Row(SceneContainer<Effect, LabelExpr, TextExpr>),
    Column(SceneContainer<Effect, LabelExpr, TextExpr>),
    Box(SceneContainer<Effect, LabelExpr, TextExpr>),
    For(SceneFor<Effect, LabelExpr, TextExpr>),
    LevelMenu(LevelMenuComponent<Effect, LabelExpr>),
    Menu(MenuInstance<LabelExpr>),
}

impl<Effect, LabelExpr, TextExpr> SceneComponent<Effect, LabelExpr, TextExpr> {
    pub fn kind(&self) -> SceneComponentKind {
        match self {
            Self::Frame(_) => SceneComponentKind::Frame,
            Self::Title(_) => SceneComponentKind::Title,
            Self::Subtitle(_) => SceneComponentKind::Subtitle,
            Self::Text(_) => SceneComponentKind::Text,
            Self::Button(_) => SceneComponentKind::Button,
            Self::Row(_) => SceneComponentKind::Row,
            Self::Column(_) => SceneComponentKind::Column,
            Self::Box(_) => SceneComponentKind::Box,
            Self::For(_) => SceneComponentKind::For,
            Self::LevelMenu(_) => SceneComponentKind::LevelMenu,
            Self::Menu(_) => SceneComponentKind::Menu,
        }
    }

    pub fn children(&self) -> &[SceneComponent<Effect, LabelExpr, TextExpr>] {
        match self {
            Self::Row(container) | Self::Column(container) | Self::Box(container) => {
                &container.children
            }
            Self::For(for_component) => &for_component.children,
            _ => &[],
        }
    }

    pub fn children_mut(
        &mut self,
    ) -> Option<&mut Vec<SceneComponent<Effect, LabelExpr, TextExpr>>> {
        match self {
            Self::Row(container) | Self::Column(container) | Self::Box(container) => {
                Some(&mut container.children)
            }
            Self::For(for_component) => Some(&mut for_component.children),
            _ => None,
        }
    }

    pub fn layout(&self) -> Option<&SceneLayout> {
        match self {
            Self::Frame(component) => Some(&component.layout),
            Self::Button(button) => Some(&button.layout),
            Self::Row(container) | Self::Column(container) | Self::Box(container) => {
                Some(&container.layout)
            }
            Self::LevelMenu(menu) => Some(&menu.layout),
            Self::Title(text) | Self::Subtitle(text) => Some(&text.layout),
            Self::Text(text) => Some(&text.layout),
            Self::For(_) | Self::Menu(_) => None,
        }
    }

    pub fn layout_mut(&mut self) -> Option<&mut SceneLayout> {
        match self {
            Self::Frame(component) => Some(&mut component.layout),
            Self::Button(button) => Some(&mut button.layout),
            Self::Row(container) | Self::Column(container) | Self::Box(container) => {
                Some(&mut container.layout)
            }
            Self::LevelMenu(menu) => Some(&mut menu.layout),
            Self::Title(text) | Self::Subtitle(text) => Some(&mut text.layout),
            Self::Text(text) => Some(&mut text.layout),
            Self::For(_) | Self::Menu(_) => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SceneComponentKind {
    Frame,
    Title,
    Subtitle,
    Text,
    Button,
    Row,
    Column,
    Box,
    For,
    LevelMenu,
    Menu,
}

impl SceneComponentKind {
    pub fn keyword(self) -> &'static str {
        match self {
            Self::Frame => "frame",
            Self::Title => "title",
            Self::Subtitle => "subtitle",
            Self::Text => "text",
            Self::Button => "button",
            Self::Row => "row",
            Self::Column => "column",
            Self::Box => "box",
            Self::For => "for",
            Self::LevelMenu => "level_menu",
            Self::Menu => "menu",
        }
    }

    pub fn from_keyword(value: &str) -> Option<Self> {
        Some(match value {
            "frame" | "puzzle" | "puzzle3" => Self::Frame,
            "title" => Self::Title,
            "subtitle" => Self::Subtitle,
            "text" => Self::Text,
            "button" => Self::Button,
            "row" => Self::Row,
            "column" => Self::Column,
            "box" => Self::Box,
            "for" => Self::For,
            "level_menu" => Self::LevelMenu,
            "menu" => Self::Menu,
            _ => return None,
        })
    }

    pub fn is_generic_container(self) -> bool {
        matches!(self, Self::Row | Self::Column | Self::Box)
    }
}

pub const GENERIC_SCENE_COMPONENT_KINDS: &[SceneComponentKind] = &[
    SceneComponentKind::Title,
    SceneComponentKind::Subtitle,
    SceneComponentKind::Text,
    SceneComponentKind::Button,
    SceneComponentKind::Row,
    SceneComponentKind::Column,
    SceneComponentKind::Box,
    SceneComponentKind::For,
    SceneComponentKind::LevelMenu,
    SceneComponentKind::Menu,
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameComponent {
    pub kind: String,
    pub source: String,
    pub layout: SceneLayout,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SceneTextExpr {
    Literal(String),
    Path(Vec<String>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneTextComponent<Expr = SceneTextExpr> {
    pub content: Expr,
    pub layout: SceneLayout,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneButton<Effect = SceneCommand, Expr = SceneTextExpr> {
    pub label: Expr,
    pub effect: Effect,
    pub layout: SceneLayout,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneContainer<
    Effect = SceneCommand,
    LabelExpr = SceneTextExpr,
    TextExpr = SceneTextExpr,
> {
    pub children: Vec<SceneComponent<Effect, LabelExpr, TextExpr>>,
    pub layout: SceneLayout,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneFor<Effect = SceneCommand, LabelExpr = SceneTextExpr, TextExpr = SceneTextExpr> {
    pub binding: String,
    pub source: SceneForSource,
    pub children: Vec<SceneComponent<Effect, LabelExpr, TextExpr>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SceneForSource {
    Levels,
    State(String),
}

impl SceneForSource {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Levels => "levels",
            Self::State(name) => name,
        }
    }

    pub fn is_levels(&self) -> bool {
        matches!(self, Self::Levels)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LevelMenuComponent<Effect = SceneCommand, Expr = SceneTextExpr> {
    pub source: Option<String>,
    pub action: Option<Effect>,
    pub show_index: bool,
    pub show_cleared: bool,
    pub columns: Option<u16>,
    pub wrap: bool,
    pub locked: LevelMenuLocked,
    pub buttons: Vec<SceneButton<Effect, Expr>>,
    pub layout: SceneLayout,
}

impl<Effect, Expr> Default for LevelMenuComponent<Effect, Expr> {
    fn default() -> Self {
        Self {
            source: None,
            action: None,
            show_index: false,
            show_cleared: false,
            columns: None,
            wrap: false,
            locked: LevelMenuLocked::default(),
            buttons: Vec::new(),
            layout: SceneLayout::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LevelMenuLocked {
    #[default]
    Disabled,
    Hidden,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuInstance<Expr = SceneTextExpr> {
    pub name: String,
    pub menu: String,
    pub data: Vec<MenuDataBinding<Expr>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuDataBinding<Expr = SceneTextExpr> {
    pub name: String,
    pub value: Expr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneCommand {
    pub name: String,
    pub args: Vec<SceneCommandArg>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneCommandArg {
    pub name: String,
    pub value: SceneTextExpr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneLayoutParseError {
    pub message: String,
}

impl SceneLayoutParseError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for SceneLayoutParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for SceneLayoutParseError {}

pub fn parse_scene_layout_attrs(tokens: &[&str]) -> Result<SceneLayout, SceneLayoutParseError> {
    let mut index = 0;
    let mut layout = SceneLayout::default();
    while index < tokens.len() {
        match tokens[index] {
            "size" => {
                if index + 2 >= tokens.len() {
                    return Err(SceneLayoutParseError::new(
                        "size must be: size <width> <height>",
                    ));
                }
                layout.size = Some(SceneSize::new(
                    parse_layout_u16(tokens[index + 1], "width")?,
                    parse_layout_u16(tokens[index + 2], "height")?,
                ));
                index += 3;
            }
            "gap" => {
                if index + 1 >= tokens.len() {
                    return Err(SceneLayoutParseError::new("gap must be: gap <amount>"));
                }
                layout.gap = Some(parse_layout_u16(tokens[index + 1], "gap")?);
                index += 2;
            }
            "align" => {
                if index + 1 >= tokens.len() {
                    return Err(SceneLayoutParseError::new(
                        "align must name at least one alignment",
                    ));
                }
                let first = tokens[index + 1];
                let second = tokens.get(index + 2).copied();
                layout.align = parse_scene_align(first, second)?;
                index += if second.is_some_and(is_scene_align_token) {
                    3
                } else {
                    2
                };
            }
            other => {
                return Err(SceneLayoutParseError::new(format!(
                    "unknown scene layout attribute: {other}"
                )));
            }
        }
    }
    Ok(layout)
}

pub fn parse_scene_layout_attr_text(value: &str) -> Result<SceneLayout, SceneLayoutParseError> {
    let tokens = value.split_whitespace().collect::<Vec<_>>();
    parse_scene_layout_attrs(&tokens)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneBlockSyntax {
    End,
    Braces,
}

impl SceneBlockSyntax {
    fn close_token(self) -> &'static str {
        match self {
            Self::End => "end",
            Self::Braces => "}",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneBlockParseError {
    pub line: String,
    pub message: String,
}

impl SceneBlockParseError {
    fn new(line: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            line: line.into(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for SceneBlockParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.line.is_empty() {
            f.write_str(&self.message)
        } else {
            write!(f, "{}: {}", self.message, self.line)
        }
    }
}

impl std::error::Error for SceneBlockParseError {}

pub trait SceneBlockHandler {
    type Error: From<SceneBlockParseError>;

    fn parse_state_block(&mut self, lines: &[String], start: usize) -> Result<usize, Self::Error> {
        let _ = lines;
        Err(SceneBlockParseError::new(&lines[start], "unknown scene directive state").into())
    }

    fn parse_view_block(&mut self, lines: &[String], start: usize) -> Result<usize, Self::Error>;

    fn parse_inputs_block(&mut self, lines: &[String], start: usize) -> Result<usize, Self::Error> {
        let _ = lines;
        Err(SceneBlockParseError::new(&lines[start], "unknown scene directive inputs").into())
    }

    fn parse_keys_block(&mut self, lines: &[String], start: usize) -> Result<usize, Self::Error> {
        let _ = lines;
        Err(SceneBlockParseError::new(&lines[start], "unknown scene directive keys").into())
    }

    fn parse_rules_block(&mut self, lines: &[String], start: usize) -> Result<usize, Self::Error> {
        let _ = lines;
        Err(SceneBlockParseError::new(&lines[start], "unknown scene directive rules").into())
    }

    fn parse_scene_start_block(
        &mut self,
        lines: &[String],
        start: usize,
    ) -> Result<usize, Self::Error> {
        let _ = lines;
        Err(
            SceneBlockParseError::new(&lines[start], "unknown scene directive on_scene_start")
                .into(),
        )
    }

    fn parse_inline_directive(
        &mut self,
        lines: &[String],
        start: usize,
    ) -> Result<usize, Self::Error>;
}

pub fn parse_scene_block_with_handler<Handler>(
    lines: &[String],
    start: usize,
    scene_name: &str,
    syntax: SceneBlockSyntax,
    handler: &mut Handler,
) -> Result<usize, Handler::Error>
where
    Handler: SceneBlockHandler,
{
    let mut index = start;
    while index < lines.len() {
        let line = &lines[index];
        if line == syntax.close_token() {
            return Ok(index + 1);
        }
        if line.is_empty() {
            index += 1;
            continue;
        }
        let keyword = line.split_whitespace().next().unwrap_or("");
        index = match keyword {
            "state" => handler.parse_state_block(lines, index)?,
            "view" => handler.parse_view_block(lines, index)?,
            "inputs" => handler.parse_inputs_block(lines, index)?,
            "keys" => handler.parse_keys_block(lines, index)?,
            "rules" => handler.parse_rules_block(lines, index)?,
            "on_scene_start" => handler.parse_scene_start_block(lines, index)?,
            _ => handler.parse_inline_directive(lines, index)?,
        };
    }
    Err(SceneBlockParseError::new(
        "",
        format!("scene {scene_name} block missing {}", syntax.close_token()),
    )
    .into())
}

pub fn parse_scene_layout_header(
    line: &str,
    keyword: &str,
    syntax: SceneBlockSyntax,
) -> Result<SceneLayout, SceneBlockParseError> {
    let header = match syntax {
        SceneBlockSyntax::End => line.trim(),
        SceneBlockSyntax::Braces => line
            .trim()
            .strip_suffix('{')
            .map(str::trim_end)
            .ok_or_else(|| {
                SceneBlockParseError::new(line, format!("{keyword} block must open with {{"))
            })?,
    };
    let tokens = header.split_whitespace().collect::<Vec<_>>();
    if tokens.first().copied() != Some(keyword) {
        return Err(SceneBlockParseError::new(
            line,
            format!("{keyword} header must start with `{keyword}`"),
        ));
    }
    parse_scene_layout_attrs(&tokens[1..])
        .map_err(|error| SceneBlockParseError::new(line, error.message))
}

pub fn parse_scene_component_block<Component, Error, ParseLeaf, BuildContainer>(
    lines: &[String],
    start: usize,
    block_name: &str,
    syntax: SceneBlockSyntax,
    parse_leaf: &mut ParseLeaf,
    build_container: &BuildContainer,
) -> Result<(usize, Vec<Component>), Error>
where
    Error: From<SceneBlockParseError>,
    ParseLeaf: FnMut(&[String], usize) -> Result<(usize, Component), Error>,
    BuildContainer: Fn(SceneComponentKind, Vec<Component>, SceneLayout) -> Component,
{
    let mut components = Vec::new();
    let mut index = start;
    while index < lines.len() {
        let line = &lines[index];
        if line == syntax.close_token() {
            return Ok((index + 1, components));
        }
        if line.is_empty() {
            index += 1;
            continue;
        }
        let (next, component) =
            parse_scene_component_at(lines, index, syntax, parse_leaf, build_container)?;
        components.push(component);
        index = next;
    }
    Err(SceneBlockParseError::new(
        "",
        format!("scene {block_name} block missing {}", syntax.close_token()),
    )
    .into())
}

pub fn parse_scene_component_at<Component, Error, ParseLeaf, BuildContainer>(
    lines: &[String],
    index: usize,
    syntax: SceneBlockSyntax,
    parse_leaf: &mut ParseLeaf,
    build_container: &BuildContainer,
) -> Result<(usize, Component), Error>
where
    Error: From<SceneBlockParseError>,
    ParseLeaf: FnMut(&[String], usize) -> Result<(usize, Component), Error>,
    BuildContainer: Fn(SceneComponentKind, Vec<Component>, SceneLayout) -> Component,
{
    let line = &lines[index];
    let Some(kind) = scene_container_kind_from_header(line, syntax)? else {
        return parse_leaf(lines, index);
    };
    let layout = parse_scene_layout_header(line, kind.keyword(), syntax)?;
    let (next, children) = parse_scene_component_block(
        lines,
        index + 1,
        kind.keyword(),
        syntax,
        parse_leaf,
        build_container,
    )?;
    Ok((next, build_container(kind, children, layout)))
}

fn scene_container_kind_from_header(
    line: &str,
    syntax: SceneBlockSyntax,
) -> Result<Option<SceneComponentKind>, SceneBlockParseError> {
    let header = match syntax {
        SceneBlockSyntax::End => line.trim(),
        SceneBlockSyntax::Braces => {
            let trimmed = line.trim();
            if !trimmed.ends_with('{') {
                return Ok(None);
            }
            trimmed.trim_end_matches('{').trim_end()
        }
    };
    let Some(keyword) = header.split_whitespace().next() else {
        return Ok(None);
    };
    let Some(kind) = SceneComponentKind::from_keyword(keyword) else {
        return Ok(None);
    };
    if kind.is_generic_container() {
        Ok(Some(kind))
    } else {
        Ok(None)
    }
}

fn parse_layout_u16(value: &str, name: &str) -> Result<u16, SceneLayoutParseError> {
    let parsed = value
        .parse::<u16>()
        .map_err(|_| SceneLayoutParseError::new(format!("{name} must be a positive integer")))?;
    if parsed == 0 {
        return Err(SceneLayoutParseError::new(format!(
            "{name} must be greater than zero"
        )));
    }
    Ok(parsed)
}

fn parse_scene_align(
    first: &str,
    second: Option<&str>,
) -> Result<SceneAlign, SceneLayoutParseError> {
    let mut align = SceneAlign::default();
    apply_scene_align_token(&mut align, first)?;
    if let Some(second) = second.filter(|token| is_scene_align_token(token)) {
        apply_scene_align_token(&mut align, second)?;
    }
    Ok(align)
}

fn apply_scene_align_token(
    align: &mut SceneAlign,
    token: &str,
) -> Result<(), SceneLayoutParseError> {
    match token {
        "left" => align.x = SceneAlignX::Left,
        "center" => {
            align.x = SceneAlignX::Center;
            align.y = SceneAlignY::Center;
        }
        "right" => align.x = SceneAlignX::Right,
        "top" => align.y = SceneAlignY::Top,
        "bottom" => align.y = SceneAlignY::Bottom,
        _ => {
            return Err(SceneLayoutParseError::new(
                "align must use left, center, right, top, or bottom",
            ));
        }
    }
    Ok(())
}

fn is_scene_align_token(token: &str) -> bool {
    matches!(token, "left" | "center" | "right" | "top" | "bottom")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn component_kind_round_trips_keywords() {
        for kind in GENERIC_SCENE_COMPONENT_KINDS {
            assert_eq!(
                SceneComponentKind::from_keyword(kind.keyword()),
                Some(*kind)
            );
        }
        assert_eq!(
            SceneComponentKind::from_keyword("puzzle"),
            Some(SceneComponentKind::Frame)
        );
        assert_eq!(
            SceneComponentKind::from_keyword("puzzle3"),
            Some(SceneComponentKind::Frame)
        );
        assert_eq!(SceneComponentKind::from_keyword("panel"), None);
    }

    #[test]
    fn component_children_helpers_only_expose_tree_children() {
        let child = SceneComponent::<SceneCommand>::Title(SceneTextComponent {
            content: SceneTextExpr::Literal("Title".to_string()),
            layout: SceneLayout::default(),
        });
        let mut row = SceneComponent::<SceneCommand>::Row(SceneContainer {
            children: vec![child],
            layout: SceneLayout::default(),
        });

        assert_eq!(row.kind(), SceneComponentKind::Row);
        assert_eq!(row.children().len(), 1);
        row.children_mut()
            .unwrap()
            .push(SceneComponent::Text(SceneTextComponent {
                content: SceneTextExpr::Literal("Body".to_string()),
                layout: SceneLayout::default(),
            }));
        assert_eq!(row.children().len(), 2);

        let mut button = SceneComponent::<SceneCommand>::Button(SceneButton {
            label: SceneTextExpr::Literal("Go".to_string()),
            effect: SceneCommand {
                name: "confirm".to_string(),
                args: Vec::new(),
            },
            layout: SceneLayout::default(),
        });
        assert!(button.children().is_empty());
        assert!(button.children_mut().is_none());
    }

    #[test]
    fn component_layout_helpers_expose_layout_owned_components() {
        let mut component = SceneComponent::<SceneCommand>::Frame(FrameComponent {
            kind: "puzzle3".to_string(),
            source: "board".to_string(),
            layout: SceneLayout::default(),
        });

        component.layout_mut().unwrap().size = Some(SceneSize::new(4, 3));

        assert_eq!(component.kind(), SceneComponentKind::Frame);
        assert_eq!(
            component.layout().and_then(|layout| layout.size),
            Some(SceneSize::new(4, 3))
        );

        let menu = SceneComponent::<SceneCommand>::Menu(MenuInstance {
            name: "main".to_string(),
            menu: "main_menu".to_string(),
            data: Vec::new(),
        });
        assert!(menu.layout().is_none());
    }

    #[test]
    fn parses_end_delimited_component_containers_with_leaf_callback() {
        let lines = vec![
            "row gap 2".to_string(),
            "leaf A".to_string(),
            "column size 10 20".to_string(),
            "leaf B".to_string(),
            "end".to_string(),
            "end".to_string(),
        ];
        let mut parse_leaf =
            |lines: &[String], index: usize| -> Result<(usize, String), SceneBlockParseError> {
                Ok((index + 1, lines[index].clone()))
            };
        let build_container =
            |kind: SceneComponentKind, children: Vec<String>, layout: SceneLayout| -> String {
                format!(
                    "{}:{}:{}",
                    kind.keyword(),
                    layout.gap.unwrap_or(0),
                    children.join(",")
                )
            };

        let (next, component) = parse_scene_component_at(
            &lines,
            0,
            SceneBlockSyntax::End,
            &mut parse_leaf,
            &build_container,
        )
        .unwrap();

        assert_eq!(next, 6);
        assert_eq!(component, "row:2:leaf A,column:0:leaf B");
    }

    #[test]
    fn parses_brace_delimited_component_blocks_and_layout_headers() {
        let lines = vec![
            "view size 4 3 {".to_string(),
            "box align left top {".to_string(),
            "leaf".to_string(),
            "}".to_string(),
            "}".to_string(),
        ];
        let layout =
            parse_scene_layout_header(&lines[0], "view", SceneBlockSyntax::Braces).unwrap();
        assert_eq!(layout.size, Some(SceneSize::new(4, 3)));

        let mut parse_leaf =
            |lines: &[String], index: usize| -> Result<(usize, String), SceneBlockParseError> {
                Ok((index + 1, lines[index].clone()))
            };
        let build_container =
            |kind: SceneComponentKind, children: Vec<String>, layout: SceneLayout| -> String {
                format!(
                    "{}:{:?}/{:?}:{}",
                    kind.keyword(),
                    layout.align.x,
                    layout.align.y,
                    children.join(",")
                )
            };

        let (next, components) = parse_scene_component_block(
            &lines,
            1,
            "view",
            SceneBlockSyntax::Braces,
            &mut parse_leaf,
            &build_container,
        )
        .unwrap();

        assert_eq!(next, 5);
        assert_eq!(components, vec!["box:Left/Top:leaf".to_string()]);
    }
}
