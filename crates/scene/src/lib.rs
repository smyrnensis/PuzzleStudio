use serde::{Deserialize, Serialize, Serializer};

mod parse;

pub use parse::{
    SceneParseError, parse_scene_effect, parse_scene_effect_at, parse_scene_effect_params,
    parse_scene_expression, parse_scene_expression_args, parse_scene_expression_at,
    parse_seconds_duration_ms, parse_seconds_duration_ms_at, parse_wait_duration_ms,
    parse_wait_duration_ms_at,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneLayout {
    pub space: SceneSpace,
    pub align_self: Option<SceneAlign>,
    pub aspect_ratio: Option<SceneAspectRatio>,
    pub gap: Option<u16>,
    pub align: SceneAlign,
    pub distribute: SceneDistribution,
    pub scroll: bool,
}

impl Default for SceneLayout {
    fn default() -> Self {
        Self {
            space: SceneSpace::Fit,
            align_self: None,
            aspect_ratio: None,
            gap: None,
            align: SceneAlign::Center,
            distribute: SceneDistribution::Center,
            scroll: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneSpace {
    #[default]
    Fit,
    Fill {
        weight: u16,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneAspectRatio {
    pub width: u16,
    pub height: u16,
}

impl SceneAspectRatio {
    pub fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneAlign {
    Start,
    #[default]
    Center,
    End,
    Stretch,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneDistribution {
    Start,
    #[default]
    Center,
    End,
    Between,
}

pub fn scene_layout_is_default(layout: &SceneLayout) -> bool {
    layout.space == SceneSpace::Fit
        && layout.align_self.is_none()
        && layout.aspect_ratio.is_none()
        && layout.gap.is_none()
        && layout.align == SceneLayout::default().align
        && layout.distribute == SceneLayout::default().distribute
        && !layout.scroll
}

pub fn write_scene_layout_json(out: &mut String, layout: &SceneLayout) {
    out.push('{');
    let mut wrote = false;
    if layout.space != SceneSpace::Fit {
        let SceneSpace::Fill { weight } = layout.space else {
            unreachable!()
        };
        out.push_str("\"space\": { \"kind\": \"fill\", \"weight\": ");
        out.push_str(&weight.to_string());
        out.push_str(" }");
        wrote = true;
    }
    if let Some(align_self) = layout.align_self {
        if wrote {
            out.push_str(", ");
        }
        out.push_str("\"alignSelf\": \"");
        out.push_str(scene_align_name(align_self));
        out.push('"');
        wrote = true;
    }
    if let Some(ratio) = layout.aspect_ratio {
        if wrote {
            out.push_str(", ");
        }
        out.push_str("\"aspectRatio\": { \"width\": ");
        out.push_str(&ratio.width.to_string());
        out.push_str(", \"height\": ");
        out.push_str(&ratio.height.to_string());
        out.push_str(" }");
        wrote = true;
    }
    if let Some(gap) = layout.gap {
        if wrote {
            out.push_str(", ");
        }
        out.push_str("\"gap\": ");
        out.push_str(&gap.to_string());
        wrote = true;
    }
    if layout.align != SceneLayout::default().align {
        if wrote {
            out.push_str(", ");
        }
        out.push_str("\"align\": \"");
        out.push_str(scene_align_name(layout.align));
        out.push('"');
        wrote = true;
    }
    if layout.distribute != SceneLayout::default().distribute {
        if wrote {
            out.push_str(", ");
        }
        out.push_str("\"distribute\": \"");
        out.push_str(scene_distribution_name(layout.distribute));
        out.push('"');
        wrote = true;
    }
    if layout.scroll {
        if wrote {
            out.push_str(", ");
        }
        out.push_str("\"scroll\": true");
    }
    out.push('}');
}

fn scene_align_name(value: SceneAlign) -> &'static str {
    match value {
        SceneAlign::Start => "start",
        SceneAlign::Center => "center",
        SceneAlign::End => "end",
        SceneAlign::Stretch => "stretch",
    }
}

fn scene_distribution_name(value: SceneDistribution) -> &'static str {
    match value {
        SceneDistribution::Start => "start",
        SceneDistribution::Center => "center",
        SceneDistribution::End => "end",
        SceneDistribution::Between => "between",
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scene<
    State = (),
    Component = SceneComponent<SceneCommand, SceneTextExpr, SceneTextExpr>,
    Action = SceneCommand,
    Rule = (),
> {
    pub name: String,
    pub layout: SceneLayout,
    pub state: Vec<State>,
    pub components: Vec<Component>,
    pub inputs: Vec<SceneInputBinding>,
    pub keys: Vec<SceneKeyBinding<Action>>,
    pub controls: Vec<SceneControl<Action>>,
    pub rules: Vec<Rule>,
    pub transitions: Vec<SceneTransition<Action>>,
}

impl<State, Component, Action, Rule> Scene<State, Component, Action, Rule> {
    pub fn new(
        name: impl Into<String>,
        state: Vec<State>,
        keys: Vec<SceneKeyBinding<Action>>,
        controls: Vec<SceneControl<Action>>,
        rules: Vec<Rule>,
        components: Vec<Component>,
    ) -> Self {
        Self {
            name: name.into(),
            layout: SceneLayout::default(),
            state,
            components,
            inputs: Vec::new(),
            keys,
            controls,
            rules,
            transitions: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneInputBinding {
    pub input: String,
    pub keys: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneKeyBinding<Action = SceneCommand> {
    pub keys: Vec<String>,
    pub action: Action,
}

impl<Action> SceneKeyBinding<Action> {
    pub fn new(key: impl Into<String>, action: Action) -> Self {
        Self {
            keys: vec![key.into()],
            action,
        }
    }

    pub fn from_keys(keys: Vec<String>, action: Action) -> Self {
        Self { keys, action }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneControl<Action = SceneCommand> {
    pub key: String,
    pub target: SceneControlTarget<Action>,
}

impl<Action> SceneControl<Action> {
    pub fn new(key: impl Into<String>, target: SceneControlTarget<Action>) -> Self {
        Self {
            key: key.into(),
            target,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneControlTarget<Action = SceneCommand> {
    Input(String),
    Action(Action),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneRuleCall {
    pub target: String,
    pub rule: String,
    pub input_map: Vec<SceneInputMap>,
}

impl SceneRuleCall {
    pub fn new(
        target: impl Into<String>,
        rule: impl Into<String>,
        input_map: Vec<SceneInputMap>,
    ) -> Self {
        Self {
            target: target.into(),
            rule: rule.into(),
            input_map,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneInputMap {
    pub from: String,
    pub to: String,
}

impl SceneInputMap {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneTransition<Effect = SceneCommand, ConditionExpr = String> {
    pub trigger: SceneTransitionTrigger<ConditionExpr>,
    pub effect: Effect,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneTransitionTrigger<ConditionExpr = String> {
    Condition(ConditionExpr),
    SceneStart,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneComponent<
    Effect = SceneCommand,
    LabelExpr = SceneTextExpr,
    TextExpr = SceneTextExpr,
    ConditionExpr = String,
> {
    Frame(FrameComponent),
    Text(SceneTextComponent<TextExpr>),
    Button(SceneButton<Effect, LabelExpr>),
    Choice(SceneButton<Effect, LabelExpr>),
    Row(SceneContainer<Effect, LabelExpr, TextExpr, ConditionExpr>),
    Column(SceneContainer<Effect, LabelExpr, TextExpr, ConditionExpr>),
    Box(SceneContainer<Effect, LabelExpr, TextExpr, ConditionExpr>),
    Conditional(SceneConditional<Effect, LabelExpr, TextExpr, ConditionExpr>),
    For(SceneFor<Effect, LabelExpr, TextExpr, ConditionExpr>),
    LevelMenu(LevelMenuComponent<Effect, LabelExpr>),
}

impl<Effect, LabelExpr, TextExpr, ConditionExpr>
    SceneComponent<Effect, LabelExpr, TextExpr, ConditionExpr>
{
    pub fn kind(&self) -> SceneComponentKind {
        match self {
            Self::Frame(_) => SceneComponentKind::Frame,
            Self::Text(_) => SceneComponentKind::Text,
            Self::Button(_) => SceneComponentKind::Button,
            Self::Choice(_) => SceneComponentKind::Choice,
            Self::Row(_) => SceneComponentKind::Row,
            Self::Column(_) => SceneComponentKind::Column,
            Self::Box(_) => SceneComponentKind::Box,
            Self::Conditional(_) => SceneComponentKind::Conditional,
            Self::For(_) => SceneComponentKind::For,
            Self::LevelMenu(_) => SceneComponentKind::LevelMenu,
        }
    }

    pub fn children(&self) -> &[SceneComponent<Effect, LabelExpr, TextExpr, ConditionExpr>] {
        match self {
            Self::Row(container) | Self::Column(container) | Self::Box(container) => {
                &container.children
            }
            Self::Conditional(conditional) => &conditional.children,
            Self::For(for_component) => &for_component.children,
            _ => &[],
        }
    }

    pub fn children_mut(
        &mut self,
    ) -> Option<&mut Vec<SceneComponent<Effect, LabelExpr, TextExpr, ConditionExpr>>> {
        match self {
            Self::Row(container) | Self::Column(container) | Self::Box(container) => {
                Some(&mut container.children)
            }
            Self::Conditional(conditional) => Some(&mut conditional.children),
            Self::For(for_component) => Some(&mut for_component.children),
            _ => None,
        }
    }

    pub fn layout(&self) -> Option<&SceneLayout> {
        match self {
            Self::Frame(component) => Some(&component.layout),
            Self::Button(button) | Self::Choice(button) => Some(&button.layout),
            Self::Row(container) | Self::Column(container) | Self::Box(container) => {
                Some(&container.layout)
            }
            Self::LevelMenu(menu) => Some(&menu.layout),
            Self::Text(text) => Some(&text.layout),
            Self::Conditional(_) | Self::For(_) => None,
        }
    }

    pub fn layout_mut(&mut self) -> Option<&mut SceneLayout> {
        match self {
            Self::Frame(component) => Some(&mut component.layout),
            Self::Button(button) | Self::Choice(button) => Some(&mut button.layout),
            Self::Row(container) | Self::Column(container) | Self::Box(container) => {
                Some(&mut container.layout)
            }
            Self::LevelMenu(menu) => Some(&mut menu.layout),
            Self::Text(text) => Some(&mut text.layout),
            Self::Conditional(_) | Self::For(_) => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SceneComponentKind {
    Frame,
    Text,
    Button,
    Choice,
    Row,
    Column,
    Box,
    Conditional,
    For,
    LevelMenu,
}

impl SceneComponentKind {
    pub fn keyword(self) -> &'static str {
        match self {
            Self::Frame => "frame",
            Self::Text => "text",
            Self::Button => "button",
            Self::Choice => "choice",
            Self::Row => "row",
            Self::Column => "column",
            Self::Box => "box",
            Self::Conditional => "if",
            Self::For => "for",
            Self::LevelMenu => "level_menu",
        }
    }

    pub fn from_keyword(value: &str) -> Option<Self> {
        Some(match value {
            "frame" | "puzzle" | "puzzle3" => Self::Frame,
            "heading" | "subheading" | "text" | "caption" => Self::Text,
            "button" => Self::Button,
            "choice" => Self::Choice,
            "row" => Self::Row,
            "column" => Self::Column,
            "box" => Self::Box,
            "if" => Self::Conditional,
            "for" => Self::For,
            "level_menu" => Self::LevelMenu,
            _ => return None,
        })
    }

    pub fn is_generic_container(self) -> bool {
        matches!(self, Self::Row | Self::Column | Self::Box)
    }
}

pub const GENERIC_SCENE_COMPONENT_KINDS: &[SceneComponentKind] = &[
    SceneComponentKind::Text,
    SceneComponentKind::Button,
    SceneComponentKind::Choice,
    SceneComponentKind::Row,
    SceneComponentKind::Column,
    SceneComponentKind::Box,
    SceneComponentKind::For,
    SceneComponentKind::LevelMenu,
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameComponent {
    pub kind: String,
    pub source: String,
    pub inputs: Vec<SceneInputBinding>,
    pub layout: SceneLayout,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneTextExpr {
    Literal(String),
    Path(Vec<String>),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneTextComponent<Expr = SceneTextExpr> {
    pub role: SceneTextRole,
    pub content: Expr,
    pub text_align: Option<SceneTextAlign>,
    pub layout: SceneLayout,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneTextRole {
    Heading,
    Subheading,
    #[default]
    Body,
    Caption,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneTextAlign {
    Start,
    Center,
    End,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneButton<Effect = SceneCommand, Expr = SceneTextExpr> {
    pub label: Expr,
    pub effect: Effect,
    pub layout: SceneLayout,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneContainer<
    Effect = SceneCommand,
    LabelExpr = SceneTextExpr,
    TextExpr = SceneTextExpr,
    ConditionExpr = String,
> {
    pub children: Vec<SceneComponent<Effect, LabelExpr, TextExpr, ConditionExpr>>,
    pub layout: SceneLayout,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneConditional<
    Effect = SceneCommand,
    LabelExpr = SceneTextExpr,
    TextExpr = SceneTextExpr,
    ConditionExpr = String,
> {
    pub condition: ConditionExpr,
    pub children: Vec<SceneComponent<Effect, LabelExpr, TextExpr, ConditionExpr>>,
    pub else_children: Vec<SceneComponent<Effect, LabelExpr, TextExpr, ConditionExpr>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneFor<
    Effect = SceneCommand,
    LabelExpr = SceneTextExpr,
    TextExpr = SceneTextExpr,
    ConditionExpr = String,
> {
    pub binding: String,
    pub source: SceneForSource,
    pub children: Vec<SceneComponent<Effect, LabelExpr, TextExpr, ConditionExpr>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LevelMenuLocked {
    #[default]
    Disabled,
    Hidden,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneCommand {
    pub name: String,
    pub args: Vec<SceneCommandArg>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneAction {
    Goto { scene: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SceneEffect {
    Input(String),
    ComponentEffect(String),
    RoutineCall(String),
    Message {
        text: SceneExpr,
    },
    Wait {
        milliseconds: Option<u64>,
    },
    Conditional {
        condition: SceneExpr,
        effect: Box<SceneEffect>,
    },
    PlaySfx {
        name: String,
    },
    PlayMusic {
        name: String,
    },
    PauseMusic {
        name: Option<String>,
    },
    ResumeMusic {
        name: Option<String>,
    },
    StopMusic {
        name: Option<String>,
    },
    Goto {
        scene: String,
        params: Vec<SceneEffectParam>,
    },
    Enter {
        scene: String,
        params: Vec<SceneEffectParam>,
    },
    Back,
    Create {
        scene: String,
    },
    Reset {
        scene: String,
    },
    Delete {
        scene: String,
    },
    Show {
        scene: String,
    },
    Hide {
        scene: String,
    },
    Toggle {
        scene: String,
    },
    Focus {
        scene: String,
    },
    PuzzleNextLevel {
        target: String,
    },
    PuzzlePreviousLevel {
        target: String,
    },
    GotoLevel {
        target: String,
        level: SceneExpr,
    },
    ResetPuzzle {
        target: String,
    },
    LoadPuzzle {
        target: String,
        source: String,
    },
    Apply {
        rule: String,
        args: Vec<SceneExpr>,
        target: Option<String>,
    },
    Copy {
        source: String,
        target: String,
    },
    SetVariable {
        name: String,
        value: SceneExpr,
    },
    ClearUndoHistory,
    ClearGameProgress,
    SetCurrentLevel {
        level: SceneExpr,
    },
    ClearCurrentLevel,
    SetLevelCleared {
        level: Option<SceneExpr>,
        cleared: bool,
    },
    ResetPersistentVars,
    Sequence {
        effects: Vec<SceneEffect>,
    },
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SceneEffectSerialize<'a> {
    Input {
        name: &'a str,
    },
    ComponentEffect {
        name: &'a str,
    },
    RoutineCall {
        name: &'a str,
    },
    Message {
        text: &'a SceneExpr,
    },
    Wait {
        milliseconds: &'a Option<u64>,
    },
    Conditional {
        condition: &'a SceneExpr,
        effect: &'a SceneEffect,
    },
    PlaySfx {
        name: &'a str,
    },
    PlayMusic {
        name: &'a str,
    },
    PauseMusic {
        name: &'a Option<String>,
    },
    ResumeMusic {
        name: &'a Option<String>,
    },
    StopMusic {
        name: &'a Option<String>,
    },
    Goto {
        scene: &'a str,
        params: &'a [SceneEffectParam],
    },
    Enter {
        scene: &'a str,
        params: &'a [SceneEffectParam],
    },
    Back,
    Create {
        scene: &'a str,
    },
    Reset {
        scene: &'a str,
    },
    Delete {
        scene: &'a str,
    },
    Show {
        scene: &'a str,
    },
    Hide {
        scene: &'a str,
    },
    Toggle {
        scene: &'a str,
    },
    Focus {
        scene: &'a str,
    },
    PuzzleNextLevel {
        target: &'a str,
    },
    PuzzlePreviousLevel {
        target: &'a str,
    },
    GotoLevel {
        target: &'a str,
        level: &'a SceneExpr,
    },
    ResetPuzzle {
        target: &'a str,
    },
    LoadPuzzle {
        target: &'a str,
        source: &'a str,
    },
    Apply {
        rule: &'a str,
        args: &'a [SceneExpr],
        target: &'a Option<String>,
    },
    Copy {
        source: &'a str,
        target: &'a str,
    },
    SetVariable {
        name: &'a str,
        value: &'a SceneExpr,
    },
    ClearUndoHistory,
    ClearGameProgress,
    SetCurrentLevel {
        level: &'a SceneExpr,
    },
    ClearCurrentLevel,
    SetLevelCleared {
        level: &'a Option<SceneExpr>,
        cleared: bool,
    },
    ResetPersistentVars,
    Sequence {
        effects: &'a [SceneEffect],
    },
}

impl<'a> From<&'a SceneEffect> for SceneEffectSerialize<'a> {
    fn from(effect: &'a SceneEffect) -> Self {
        match effect {
            SceneEffect::Input(name) => Self::Input { name },
            SceneEffect::ComponentEffect(name) => Self::ComponentEffect { name },
            SceneEffect::RoutineCall(name) => Self::RoutineCall { name },
            SceneEffect::Message { text } => Self::Message { text },
            SceneEffect::Wait { milliseconds } => Self::Wait { milliseconds },
            SceneEffect::Conditional { condition, effect } => {
                Self::Conditional { condition, effect }
            }
            SceneEffect::PlaySfx { name } => Self::PlaySfx { name },
            SceneEffect::PlayMusic { name } => Self::PlayMusic { name },
            SceneEffect::PauseMusic { name } => Self::PauseMusic { name },
            SceneEffect::ResumeMusic { name } => Self::ResumeMusic { name },
            SceneEffect::StopMusic { name } => Self::StopMusic { name },
            SceneEffect::Goto { scene, params } => Self::Goto { scene, params },
            SceneEffect::Enter { scene, params } => Self::Enter { scene, params },
            SceneEffect::Back => Self::Back,
            SceneEffect::Create { scene } => Self::Create { scene },
            SceneEffect::Reset { scene } => Self::Reset { scene },
            SceneEffect::Delete { scene } => Self::Delete { scene },
            SceneEffect::Show { scene } => Self::Show { scene },
            SceneEffect::Hide { scene } => Self::Hide { scene },
            SceneEffect::Toggle { scene } => Self::Toggle { scene },
            SceneEffect::Focus { scene } => Self::Focus { scene },
            SceneEffect::PuzzleNextLevel { target } => Self::PuzzleNextLevel { target },
            SceneEffect::PuzzlePreviousLevel { target } => Self::PuzzlePreviousLevel { target },
            SceneEffect::GotoLevel { target, level } => Self::GotoLevel { target, level },
            SceneEffect::ResetPuzzle { target } => Self::ResetPuzzle { target },
            SceneEffect::LoadPuzzle { target, source } => Self::LoadPuzzle { target, source },
            SceneEffect::Apply { rule, args, target } => Self::Apply { rule, args, target },
            SceneEffect::Copy { source, target } => Self::Copy { source, target },
            SceneEffect::SetVariable { name, value } => Self::SetVariable { name, value },
            SceneEffect::ClearUndoHistory => Self::ClearUndoHistory,
            SceneEffect::ClearGameProgress => Self::ClearGameProgress,
            SceneEffect::SetCurrentLevel { level } => Self::SetCurrentLevel { level },
            SceneEffect::ClearCurrentLevel => Self::ClearCurrentLevel,
            SceneEffect::SetLevelCleared { level, cleared } => Self::SetLevelCleared {
                level,
                cleared: *cleared,
            },
            SceneEffect::ResetPersistentVars => Self::ResetPersistentVars,
            SceneEffect::Sequence { effects } => Self::Sequence { effects },
        }
    }
}

impl Serialize for SceneEffect {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        SceneEffectSerialize::from(self).serialize(serializer)
    }
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SceneEffectDeserialize {
    Input {
        name: String,
    },
    ComponentEffect {
        name: String,
    },
    RoutineCall {
        name: String,
    },
    Message {
        text: SceneExpr,
    },
    Wait {
        milliseconds: Option<u64>,
    },
    Conditional {
        condition: SceneExpr,
        effect: Box<SceneEffect>,
    },
    PlaySfx {
        name: String,
    },
    PlayMusic {
        name: String,
    },
    PauseMusic {
        name: Option<String>,
    },
    ResumeMusic {
        name: Option<String>,
    },
    StopMusic {
        name: Option<String>,
    },
    Goto {
        scene: String,
        params: Vec<SceneEffectParam>,
    },
    Enter {
        scene: String,
        params: Vec<SceneEffectParam>,
    },
    Back,
    Create {
        scene: String,
    },
    Reset {
        scene: String,
    },
    Delete {
        scene: String,
    },
    Show {
        scene: String,
    },
    Hide {
        scene: String,
    },
    Toggle {
        scene: String,
    },
    Focus {
        scene: String,
    },
    PuzzleNextLevel {
        target: String,
    },
    PuzzlePreviousLevel {
        target: String,
    },
    GotoLevel {
        target: String,
        level: SceneExpr,
    },
    ResetPuzzle {
        target: String,
    },
    LoadPuzzle {
        target: String,
        source: String,
    },
    Apply {
        rule: String,
        args: Vec<SceneExpr>,
        target: Option<String>,
    },
    Copy {
        source: String,
        target: String,
    },
    SetVariable {
        name: String,
        value: SceneExpr,
    },
    ClearUndoHistory,
    ClearGameProgress,
    SetCurrentLevel {
        level: SceneExpr,
    },
    ClearCurrentLevel,
    SetLevelCleared {
        level: Option<SceneExpr>,
        cleared: bool,
    },
    ResetPersistentVars,
    Sequence {
        effects: Vec<SceneEffect>,
    },
}

impl From<SceneEffectDeserialize> for SceneEffect {
    fn from(effect: SceneEffectDeserialize) -> Self {
        match effect {
            SceneEffectDeserialize::Input { name } => Self::Input(name),
            SceneEffectDeserialize::ComponentEffect { name } => Self::ComponentEffect(name),
            SceneEffectDeserialize::RoutineCall { name } => Self::RoutineCall(name),
            SceneEffectDeserialize::Message { text } => Self::Message { text },
            SceneEffectDeserialize::Wait { milliseconds } => Self::Wait { milliseconds },
            SceneEffectDeserialize::Conditional { condition, effect } => {
                Self::Conditional { condition, effect }
            }
            SceneEffectDeserialize::PlaySfx { name } => Self::PlaySfx { name },
            SceneEffectDeserialize::PlayMusic { name } => Self::PlayMusic { name },
            SceneEffectDeserialize::PauseMusic { name } => Self::PauseMusic { name },
            SceneEffectDeserialize::ResumeMusic { name } => Self::ResumeMusic { name },
            SceneEffectDeserialize::StopMusic { name } => Self::StopMusic { name },
            SceneEffectDeserialize::Goto { scene, params } => Self::Goto { scene, params },
            SceneEffectDeserialize::Enter { scene, params } => Self::Enter { scene, params },
            SceneEffectDeserialize::Back => Self::Back,
            SceneEffectDeserialize::Create { scene } => Self::Create { scene },
            SceneEffectDeserialize::Reset { scene } => Self::Reset { scene },
            SceneEffectDeserialize::Delete { scene } => Self::Delete { scene },
            SceneEffectDeserialize::Show { scene } => Self::Show { scene },
            SceneEffectDeserialize::Hide { scene } => Self::Hide { scene },
            SceneEffectDeserialize::Toggle { scene } => Self::Toggle { scene },
            SceneEffectDeserialize::Focus { scene } => Self::Focus { scene },
            SceneEffectDeserialize::PuzzleNextLevel { target } => Self::PuzzleNextLevel { target },
            SceneEffectDeserialize::PuzzlePreviousLevel { target } => {
                Self::PuzzlePreviousLevel { target }
            }
            SceneEffectDeserialize::GotoLevel { target, level } => {
                Self::GotoLevel { target, level }
            }
            SceneEffectDeserialize::ResetPuzzle { target } => Self::ResetPuzzle { target },
            SceneEffectDeserialize::LoadPuzzle { target, source } => {
                Self::LoadPuzzle { target, source }
            }
            SceneEffectDeserialize::Apply { rule, args, target } => {
                Self::Apply { rule, args, target }
            }
            SceneEffectDeserialize::Copy { source, target } => Self::Copy { source, target },
            SceneEffectDeserialize::SetVariable { name, value } => {
                Self::SetVariable { name, value }
            }
            SceneEffectDeserialize::ClearUndoHistory => Self::ClearUndoHistory,
            SceneEffectDeserialize::ClearGameProgress => Self::ClearGameProgress,
            SceneEffectDeserialize::SetCurrentLevel { level } => Self::SetCurrentLevel { level },
            SceneEffectDeserialize::ClearCurrentLevel => Self::ClearCurrentLevel,
            SceneEffectDeserialize::SetLevelCleared { level, cleared } => {
                Self::SetLevelCleared { level, cleared }
            }
            SceneEffectDeserialize::ResetPersistentVars => Self::ResetPersistentVars,
            SceneEffectDeserialize::Sequence { effects } => Self::Sequence { effects },
        }
    }
}

impl<'de> Deserialize<'de> for SceneEffect {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        SceneEffectDeserialize::deserialize(deserializer).map(Self::from)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum SceneEffectParam {
    Level(SceneExpr),
    Named { name: String, value: SceneExpr },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum SceneExpr {
    Bool(bool),
    Int(i64),
    Text(String),
    Path(Vec<String>),
    LevelSelector {
        collection: String,
        key: SceneLevelKey,
        property: Option<String>,
    },
    Call {
        name: String,
        args: Vec<SceneExpr>,
    },
    Binary {
        op: SceneBinaryOp,
        left: Box<SceneExpr>,
        right: Box<SceneExpr>,
    },
    If {
        condition: Box<SceneExpr>,
        then_branch: Box<SceneExpr>,
        else_branch: Box<SceneExpr>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum SceneLevelKey {
    Index(i64),
    Id(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneBinaryOp {
    And,
    Eq,
    In,
    NotEq,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SceneFixtureJsonOptions<'a> {
    pub frame_kind: Option<&'a str>,
    pub default_level_menu_action: Option<&'a SceneEffect>,
}

pub fn write_scene_component_fixture_json<TextExpr, TextWriter, LevelSource>(
    out: &mut String,
    component: &SceneComponent<SceneEffect, SceneExpr, TextExpr, SceneExpr>,
    options: SceneFixtureJsonOptions<'_>,
    write_text_fields: TextWriter,
    note_level_source: &mut LevelSource,
) -> bool
where
    TextWriter: Fn(&mut String, &TextExpr) + Copy,
    LevelSource: FnMut(&str),
{
    match component {
        SceneComponent::Frame(frame) => {
            if options
                .frame_kind
                .is_some_and(|frame_kind| frame.kind != frame_kind)
            {
                return false;
            }
            out.push_str("{ \"kind\": ");
            write_json_string(out, &frame.kind);
            out.push_str(", \"source\": ");
            write_json_string(out, &frame.source);
            push_inline_layout_json(out, &frame.layout);
            out.push_str(" }");
        }
        SceneComponent::Text(text) => {
            out.push_str("{ \"kind\": \"text\", \"role\": \"");
            out.push_str(match text.role {
                SceneTextRole::Heading => "heading",
                SceneTextRole::Subheading => "subheading",
                SceneTextRole::Body => "body",
                SceneTextRole::Caption => "caption",
            });
            out.push_str("\", ");
            write_text_fields(out, &text.content);
            if let Some(align) = text.text_align {
                out.push_str(", \"textAlign\": \"");
                out.push_str(match align {
                    SceneTextAlign::Start => "start",
                    SceneTextAlign::Center => "center",
                    SceneTextAlign::End => "end",
                });
                out.push('"');
            }
            push_inline_layout_json(out, &text.layout);
            out.push_str(" }");
        }
        SceneComponent::Button(button) | SceneComponent::Choice(button) => {
            let kind = match component {
                SceneComponent::Choice(_) => "choice",
                _ => "button",
            };
            out.push_str("{ \"kind\": ");
            write_json_string(out, kind);
            out.push_str(", \"label\": ");
            write_scene_expr_json(out, &button.label);
            out.push_str(", \"effect\": ");
            write_scene_effect_json(out, &button.effect);
            push_inline_layout_json(out, &button.layout);
            out.push_str(" }");
        }
        SceneComponent::Row(container) => {
            write_container_fixture_json(
                out,
                "row",
                &container.children,
                &container.layout,
                options,
                write_text_fields,
                note_level_source,
            );
        }
        SceneComponent::Column(container) => {
            write_container_fixture_json(
                out,
                "column",
                &container.children,
                &container.layout,
                options,
                write_text_fields,
                note_level_source,
            );
        }
        SceneComponent::Box(container) => {
            write_container_fixture_json(
                out,
                "box",
                &container.children,
                &container.layout,
                options,
                write_text_fields,
                note_level_source,
            );
        }
        SceneComponent::Conditional(conditional) => {
            out.push_str("{ \"kind\": \"conditional\", \"condition\": ");
            write_scene_expr_json(out, &conditional.condition);
            out.push_str(", \"children\": [");
            write_scene_component_list_fixture_json(
                out,
                &conditional.children,
                options,
                write_text_fields,
                note_level_source,
            );
            out.push_str("], \"elseChildren\": [");
            write_scene_component_list_fixture_json(
                out,
                &conditional.else_children,
                options,
                write_text_fields,
                note_level_source,
            );
            out.push_str("] }");
        }
        SceneComponent::For(for_view) => {
            out.push_str("{ \"kind\": \"for\", \"binding\": ");
            write_json_string(out, &for_view.binding);
            out.push_str(", \"source\": ");
            write_json_string(out, for_view.source.as_str());
            out.push_str(", \"children\": [");
            write_scene_component_list_fixture_json(
                out,
                &for_view.children,
                options,
                write_text_fields,
                note_level_source,
            );
            out.push_str("] }");
        }
        SceneComponent::LevelMenu(menu) => {
            let source = menu.source.as_deref().unwrap_or("levels");
            note_level_source(source);
            let action = menu.action.as_ref().or(options.default_level_menu_action);
            out.push_str("{ \"kind\": \"level_menu\", \"source\": ");
            write_json_string(out, source);
            out.push_str(", \"levels\": ");
            write_json_string(out, source);
            out.push_str(", \"showIndex\": ");
            out.push_str(if menu.show_index { "true" } else { "false" });
            out.push_str(", \"showCleared\": ");
            out.push_str(if menu.show_cleared { "true" } else { "false" });
            out.push_str(", \"columns\": ");
            if let Some(columns) = menu.columns {
                out.push_str(&columns.to_string());
            } else {
                out.push_str("null");
            }
            out.push_str(", \"wrap\": ");
            out.push_str(if menu.wrap { "true" } else { "false" });
            out.push_str(", \"action\": ");
            if let Some(action) = action {
                write_scene_effect_json(out, action);
            } else {
                out.push_str("null");
            }
            out.push_str(", \"buttons\": [");
            for (index, button) in menu.buttons.iter().enumerate() {
                if index > 0 {
                    out.push_str(", ");
                }
                out.push_str("{ \"label\": ");
                write_scene_expr_json(out, &button.label);
                out.push_str(", \"effect\": ");
                write_scene_effect_json(out, &button.effect);
                out.push_str(" }");
            }
            out.push(']');
            push_inline_layout_json(out, &menu.layout);
            out.push_str(" }");
        }
    }
    true
}

pub fn write_scene_effect_json(out: &mut String, effect: &SceneEffect) {
    out.push('{');
    write_scene_effect_json_fields(out, effect);
    out.push('}');
}

pub fn write_scene_effect_json_fields(out: &mut String, effect: &SceneEffect) {
    match effect {
        SceneEffect::Input(input) => {
            write_json_pair(out, "kind", "input");
            out.push_str(", ");
            write_json_pair(out, "name", input);
        }
        SceneEffect::ComponentEffect(effect) => {
            write_json_pair(out, "kind", "component_effect");
            out.push_str(", ");
            write_json_pair(out, "name", effect);
        }
        SceneEffect::RoutineCall(name) => {
            write_json_pair(out, "kind", "routine_call");
            out.push_str(", ");
            write_json_pair(out, "name", name);
        }
        SceneEffect::Message { text } => {
            write_json_pair(out, "kind", "message");
            out.push_str(", \"text\": ");
            write_scene_expr_json(out, text);
        }
        SceneEffect::Wait { milliseconds } => {
            write_json_pair(out, "kind", "wait");
            out.push_str(", \"milliseconds\": ");
            out.push_str(&milliseconds.unwrap_or(200).to_string());
        }
        SceneEffect::Conditional { condition, effect } => {
            write_json_pair(out, "kind", "conditional");
            out.push_str(", \"condition\": ");
            write_scene_expr_json(out, condition);
            out.push_str(", \"effect\": ");
            write_scene_effect_json(out, effect);
        }
        SceneEffect::PlaySfx { name } => {
            write_json_pair(out, "kind", "play_sfx");
            out.push_str(", ");
            write_json_pair(out, "name", name);
        }
        SceneEffect::PlayMusic { name } => {
            write_json_pair(out, "kind", "play_music");
            out.push_str(", ");
            write_json_pair(out, "name", name);
        }
        SceneEffect::PauseMusic { name } => {
            write_optional_music_effect_json(out, "pause_music", name)
        }
        SceneEffect::ResumeMusic { name } => {
            write_optional_music_effect_json(out, "resume_music", name)
        }
        SceneEffect::StopMusic { name } => {
            write_optional_music_effect_json(out, "stop_music", name)
        }
        SceneEffect::Goto { scene, params } => {
            write_scene_target_effect_json(out, "goto", scene, params)
        }
        SceneEffect::Enter { scene, params } => {
            write_scene_target_effect_json(out, "enter", scene, params)
        }
        SceneEffect::Back => write_json_pair(out, "kind", "back"),
        SceneEffect::Create { scene } => write_scene_target_effect_json(out, "create", scene, &[]),
        SceneEffect::Reset { scene } => write_scene_target_effect_json(out, "reset", scene, &[]),
        SceneEffect::Delete { scene } => write_scene_target_effect_json(out, "delete", scene, &[]),
        SceneEffect::Show { scene } => write_scene_target_effect_json(out, "show", scene, &[]),
        SceneEffect::Hide { scene } => write_scene_target_effect_json(out, "hide", scene, &[]),
        SceneEffect::Toggle { scene } => write_scene_target_effect_json(out, "toggle", scene, &[]),
        SceneEffect::Focus { scene } => write_scene_target_effect_json(out, "focus", scene, &[]),
        SceneEffect::PuzzleNextLevel { target } => {
            write_json_pair(out, "kind", "puzzle_next_level");
            out.push_str(", ");
            write_json_pair(out, "target", target);
        }
        SceneEffect::PuzzlePreviousLevel { target } => {
            write_json_pair(out, "kind", "puzzle_previous_level");
            out.push_str(", ");
            write_json_pair(out, "target", target);
        }
        SceneEffect::GotoLevel { target, level } => {
            write_json_pair(out, "kind", "puzzle_goto_level");
            out.push_str(", ");
            write_json_pair(out, "target", target);
            out.push_str(", \"level\": ");
            write_scene_expr_json(out, level);
        }
        SceneEffect::ResetPuzzle { target } => {
            write_json_pair(out, "kind", "puzzle_reset");
            out.push_str(", ");
            write_json_pair(out, "target", target);
        }
        SceneEffect::LoadPuzzle { target, source } => {
            write_json_pair(out, "kind", "puzzle_load");
            out.push_str(", ");
            write_json_pair(out, "target", target);
            out.push_str(", ");
            write_json_pair(out, "source", source);
        }
        SceneEffect::Apply { rule, args, target } => {
            write_json_pair(out, "kind", "apply");
            out.push_str(", ");
            write_json_pair(out, "rule", rule);
            out.push_str(", \"args\": [");
            for (index, arg) in args.iter().enumerate() {
                if index > 0 {
                    out.push_str(", ");
                }
                write_scene_expr_json(out, arg);
            }
            out.push(']');
            if let Some(target) = target {
                out.push_str(", ");
                write_json_pair(out, "target", target);
            }
        }
        SceneEffect::Copy { source, target } => {
            write_json_pair(out, "kind", "copy");
            out.push_str(", ");
            write_json_pair(out, "source", source);
            out.push_str(", ");
            write_json_pair(out, "target", target);
        }
        SceneEffect::SetVariable { name, value } => {
            write_json_pair(out, "kind", "set_variable");
            out.push_str(", ");
            write_json_pair(out, "name", name);
            out.push_str(", \"value\": ");
            write_scene_expr_json(out, value);
        }
        SceneEffect::ClearUndoHistory => write_json_pair(out, "kind", "clear_undo_history"),
        SceneEffect::ClearGameProgress => write_json_pair(out, "kind", "clear_game_progress"),
        SceneEffect::SetCurrentLevel { level } => {
            write_json_pair(out, "kind", "set_current_level");
            out.push_str(", \"level\": ");
            write_scene_expr_json(out, level);
        }
        SceneEffect::ClearCurrentLevel => write_json_pair(out, "kind", "clear_current_level"),
        SceneEffect::SetLevelCleared { level, cleared } => {
            write_json_pair(out, "kind", "set_level_cleared");
            out.push_str(", \"cleared\": ");
            out.push_str(if *cleared { "true" } else { "false" });
            if let Some(level) = level {
                out.push_str(", \"level\": ");
                write_scene_expr_json(out, level);
            }
        }
        SceneEffect::ResetPersistentVars => write_json_pair(out, "kind", "reset_persistent_vars"),
        SceneEffect::Sequence { effects } => {
            write_json_pair(out, "kind", "sequence");
            out.push_str(", \"effects\": [");
            for (index, effect) in effects.iter().enumerate() {
                if index > 0 {
                    out.push_str(", ");
                }
                write_scene_effect_json(out, effect);
            }
            out.push(']');
        }
    }
}

pub fn write_scene_expr_json(out: &mut String, expr: &SceneExpr) {
    match expr {
        SceneExpr::Bool(value) => {
            out.push_str("{ \"kind\": \"bool\", \"value\": ");
            out.push_str(if *value { "true" } else { "false" });
            out.push_str(" }");
        }
        SceneExpr::Int(value) => {
            out.push_str("{ \"kind\": \"int\", \"value\": ");
            out.push_str(&value.to_string());
            out.push_str(" }");
        }
        SceneExpr::Text(value) => {
            out.push_str("{ \"kind\": \"text\", \"value\": ");
            write_json_string(out, value);
            out.push_str(" }");
        }
        SceneExpr::Path(path) => {
            out.push_str("{ \"kind\": \"path\", \"path\": ");
            write_json_string(out, &path.join("."));
            out.push_str(" }");
        }
        SceneExpr::LevelSelector {
            collection,
            key,
            property,
        } => {
            out.push_str("{ \"kind\": \"level_selector\", \"collection\": ");
            write_json_string(out, collection);
            out.push_str(", \"key\": ");
            match key {
                SceneLevelKey::Index(index) => {
                    out.push_str("{ \"kind\": \"index\", \"value\": ");
                    out.push_str(&index.to_string());
                    out.push_str(" }");
                }
                SceneLevelKey::Id(id) => {
                    out.push_str("{ \"kind\": \"id\", \"value\": ");
                    write_json_string(out, id);
                    out.push_str(" }");
                }
            }
            if let Some(property) = property {
                out.push_str(", \"property\": ");
                write_json_string(out, property);
            }
            out.push_str(" }");
        }
        SceneExpr::Call { name, args } => {
            out.push_str("{ \"kind\": \"call\", \"name\": ");
            write_json_string(out, name);
            out.push_str(", \"args\": [");
            for (index, arg) in args.iter().enumerate() {
                if index > 0 {
                    out.push_str(", ");
                }
                write_scene_expr_json(out, arg);
            }
            out.push_str("] }");
        }
        SceneExpr::Binary { op, left, right } => {
            let op = match op {
                SceneBinaryOp::And => "and",
                SceneBinaryOp::Eq => "eq",
                SceneBinaryOp::In => "in",
                SceneBinaryOp::NotEq => "neq",
            };
            out.push_str("{ \"kind\": \"binary\", \"op\": ");
            write_json_string(out, op);
            out.push_str(", \"left\": ");
            write_scene_expr_json(out, left);
            out.push_str(", \"right\": ");
            write_scene_expr_json(out, right);
            out.push_str(" }");
        }
        SceneExpr::If {
            condition,
            then_branch,
            else_branch,
        } => {
            out.push_str("{ \"kind\": \"if\", \"condition\": ");
            write_scene_expr_json(out, condition);
            out.push_str(", \"then\": ");
            write_scene_expr_json(out, then_branch);
            out.push_str(", \"else\": ");
            write_scene_expr_json(out, else_branch);
            out.push_str(" }");
        }
    }
}

fn write_container_fixture_json<TextExpr, TextWriter, LevelSource>(
    out: &mut String,
    kind: &str,
    children: &[SceneComponent<SceneEffect, SceneExpr, TextExpr, SceneExpr>],
    layout: &SceneLayout,
    options: SceneFixtureJsonOptions<'_>,
    write_text_fields: TextWriter,
    note_level_source: &mut LevelSource,
) where
    TextWriter: Fn(&mut String, &TextExpr) + Copy,
    LevelSource: FnMut(&str),
{
    out.push_str("{ \"kind\": ");
    write_json_string(out, kind);
    out.push_str(", \"children\": [");
    write_scene_component_list_fixture_json(
        out,
        children,
        options,
        write_text_fields,
        note_level_source,
    );
    out.push(']');
    push_inline_layout_json(out, layout);
    out.push_str(" }");
}

fn write_scene_component_list_fixture_json<TextExpr, TextWriter, LevelSource>(
    out: &mut String,
    components: &[SceneComponent<SceneEffect, SceneExpr, TextExpr, SceneExpr>],
    options: SceneFixtureJsonOptions<'_>,
    write_text_fields: TextWriter,
    note_level_source: &mut LevelSource,
) where
    TextWriter: Fn(&mut String, &TextExpr) + Copy,
    LevelSource: FnMut(&str),
{
    let mut wrote = false;
    for component in components {
        let mut component_json = String::new();
        if !write_scene_component_fixture_json(
            &mut component_json,
            component,
            options,
            write_text_fields,
            note_level_source,
        ) {
            continue;
        }
        if wrote {
            out.push_str(", ");
        }
        wrote = true;
        out.push_str(&component_json);
    }
}

fn write_optional_music_effect_json(out: &mut String, kind: &str, name: &Option<String>) {
    write_json_pair(out, "kind", kind);
    out.push_str(", \"name\": ");
    if let Some(name) = name {
        write_json_string(out, name);
    } else {
        out.push_str("null");
    }
}

fn write_scene_target_effect_json(
    out: &mut String,
    kind: &str,
    scene: &str,
    params: &[SceneEffectParam],
) {
    write_json_pair(out, "kind", kind);
    out.push_str(", ");
    write_json_pair(out, "screen", scene);
    out.push_str(", ");
    write_json_pair(out, "scene", scene);
    out.push_str(", \"params\": [");
    for (index, param) in params.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        write_scene_effect_param_json(out, param);
    }
    out.push(']');
}

fn write_scene_effect_param_json(out: &mut String, param: &SceneEffectParam) {
    match param {
        SceneEffectParam::Level(value) => {
            out.push_str("{ \"kind\": \"level\", \"value\": ");
            write_scene_expr_json(out, value);
            out.push_str(" }");
        }
        SceneEffectParam::Named { name, value } => {
            out.push_str("{ \"kind\": \"named\", \"name\": ");
            write_json_string(out, name);
            out.push_str(", \"value\": ");
            write_scene_expr_json(out, value);
            out.push_str(" }");
        }
    }
}

fn push_inline_layout_json(out: &mut String, layout: &SceneLayout) {
    if scene_layout_is_default(layout) {
        return;
    }
    out.push_str(", \"layout\": ");
    write_scene_layout_json(out, layout);
}

fn write_json_pair(out: &mut String, key: &str, value: &str) {
    write_json_string(out, key);
    out.push_str(": ");
    write_json_string(out, value);
}

pub fn write_json_string(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out.push('"');
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
            "space" => {
                let Some(kind) = tokens.get(index + 1) else {
                    return Err(SceneLayoutParseError::new(
                        "space must be: space fit | space fill [weight]",
                    ));
                };
                match *kind {
                    "fit" => {
                        layout.space = SceneSpace::Fit;
                        index += 2;
                    }
                    "fill" => {
                        let weight = tokens
                            .get(index + 2)
                            .filter(|value| value.chars().all(|ch| ch.is_ascii_digit()))
                            .map(|value| parse_layout_u16(value, "space weight"))
                            .transpose()?
                            .unwrap_or(1);
                        layout.space = SceneSpace::Fill { weight };
                        index += if tokens
                            .get(index + 2)
                            .is_some_and(|value| value.chars().all(|ch| ch.is_ascii_digit()))
                        {
                            3
                        } else {
                            2
                        };
                    }
                    _ => {
                        return Err(SceneLayoutParseError::new(
                            "space must be: space fit | space fill [weight]",
                        ));
                    }
                }
            }
            "aspect" => {
                if index + 2 >= tokens.len() {
                    return Err(SceneLayoutParseError::new(
                        "aspect must be: aspect <width> <height>",
                    ));
                }
                layout.aspect_ratio = Some(SceneAspectRatio::new(
                    parse_layout_u16(tokens[index + 1], "aspect width")?,
                    parse_layout_u16(tokens[index + 2], "aspect height")?,
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
                layout.align = parse_scene_align(tokens[index + 1])?;
                index += 2;
            }
            "align_self" => {
                let Some(value) = tokens.get(index + 1) else {
                    return Err(SceneLayoutParseError::new(
                        "align_self must name start, center, end, or stretch",
                    ));
                };
                layout.align_self = Some(parse_scene_align(value)?);
                index += 2;
            }
            "distribute" => {
                let Some(value) = tokens.get(index + 1) else {
                    return Err(SceneLayoutParseError::new(
                        "distribute must name start, center, end, or between",
                    ));
                };
                layout.distribute = parse_scene_distribution(value)?;
                index += 2;
            }
            "scroll" => {
                layout.scroll = true;
                index += 1;
            }
            "scroll=true" => {
                layout.scroll = true;
                index += 1;
            }
            "scroll=false" => {
                layout.scroll = false;
                index += 1;
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

    fn parse_layout_block(&mut self, lines: &[String], start: usize) -> Result<usize, Self::Error>;

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
            "layout" => handler.parse_layout_block(lines, index)?,
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

fn parse_scene_align(token: &str) -> Result<SceneAlign, SceneLayoutParseError> {
    match token {
        "start" => Ok(SceneAlign::Start),
        "center" => Ok(SceneAlign::Center),
        "end" => Ok(SceneAlign::End),
        "stretch" => Ok(SceneAlign::Stretch),
        _ => Err(SceneLayoutParseError::new(
            "align must use start, center, end, or stretch",
        )),
    }
}

fn parse_scene_distribution(token: &str) -> Result<SceneDistribution, SceneLayoutParseError> {
    match token {
        "start" => Ok(SceneDistribution::Start),
        "center" => Ok(SceneDistribution::Center),
        "end" => Ok(SceneDistribution::End),
        "between" => Ok(SceneDistribution::Between),
        _ => Err(SceneLayoutParseError::new(
            "distribute must use start, center, end, or between",
        )),
    }
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
        let child = SceneComponent::<SceneCommand>::Text(SceneTextComponent {
            role: SceneTextRole::Heading,
            content: SceneTextExpr::Literal("Title".to_string()),
            text_align: None,
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
                role: SceneTextRole::Body,
                content: SceneTextExpr::Literal("Body".to_string()),
                text_align: None,
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
            inputs: Vec::new(),
            layout: SceneLayout::default(),
        });

        component.layout_mut().unwrap().space = SceneSpace::Fill { weight: 2 };
        component.layout_mut().unwrap().scroll = true;

        assert_eq!(component.kind(), SceneComponentKind::Frame);
        assert_eq!(
            component.layout().map(|layout| layout.space),
            Some(SceneSpace::Fill { weight: 2 })
        );
        assert!(component.layout().is_some_and(|layout| layout.scroll));
    }

    #[test]
    fn scene_effect_input_serde_uses_named_payload() {
        let effect = SceneEffect::Sequence {
            effects: vec![
                SceneEffect::Input("continue_game".to_string()),
                SceneEffect::ComponentEffect("down".to_string()),
                SceneEffect::RoutineCall("open_menu".to_string()),
            ],
        };

        let json = serde_json::to_string(&effect).unwrap();

        assert!(json.contains(r#""kind":"input","name":"continue_game""#));
        assert!(json.contains(r#""kind":"component_effect","name":"down""#));
        assert!(json.contains(r#""kind":"routine_call","name":"open_menu""#));
        let roundtrip: SceneEffect = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip, effect);
    }

    #[test]
    fn parses_end_delimited_component_containers_with_leaf_callback() {
        let lines = vec![
            "row gap 2".to_string(),
            "leaf A".to_string(),
            "column space fill 2 aspect 10 20".to_string(),
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
            "layout space fill 2 {".to_string(),
            "box align start distribute end {".to_string(),
            "leaf".to_string(),
            "}".to_string(),
            "}".to_string(),
        ];
        let layout =
            parse_scene_layout_header(&lines[0], "layout", SceneBlockSyntax::Braces).unwrap();
        assert_eq!(layout.space, SceneSpace::Fill { weight: 2 });

        let mut parse_leaf =
            |lines: &[String], index: usize| -> Result<(usize, String), SceneBlockParseError> {
                Ok((index + 1, lines[index].clone()))
            };
        let build_container =
            |kind: SceneComponentKind, children: Vec<String>, layout: SceneLayout| -> String {
                format!(
                    "{}:{:?}/{:?}:{}",
                    kind.keyword(),
                    layout.align,
                    layout.distribute,
                    children.join(",")
                )
            };

        let (next, components) = parse_scene_component_block(
            &lines,
            1,
            "layout",
            SceneBlockSyntax::Braces,
            &mut parse_leaf,
            &build_container,
        )
        .unwrap();

        assert_eq!(next, 5);
        assert_eq!(components, vec!["box:Start/End:leaf".to_string()]);
    }
}
