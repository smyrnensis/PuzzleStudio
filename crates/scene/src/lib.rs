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

/// Layout metadata shared by every authored component. `SceneLayout` remains
/// an authoring spelling; the runtime contract is component-oriented.
pub type ComponentLayout = SceneLayout;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentPlacement {
    Root,
    #[default]
    Content,
    Overlay,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentVisibility {
    #[default]
    Visible,
    Hidden,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "component", rename_all = "snake_case")]
pub enum ComponentOrder {
    First,
    Last,
    Before(String),
    After(String),
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
pub struct ComponentDefinition<
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

pub type Scene<
    State = (),
    Component = SceneComponent<SceneCommand, SceneTextExpr, SceneTextExpr>,
    Action = SceneCommand,
    Rule = (),
> = ComponentDefinition<State, Component, Action, Rule>;

impl<State, Component, Action, Rule> ComponentDefinition<State, Component, Action, Rule> {
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
pub enum ComponentNode<
    Effect = SceneCommand,
    LabelExpr = SceneTextExpr,
    TextExpr = SceneTextExpr,
    ConditionExpr = String,
> {
    Viewport(ViewportComponent),
    Frame(FrameComponent),
    Text(SceneTextComponent<TextExpr>),
    Button(SceneButton<Effect, LabelExpr>),
    Choice(SceneButton<Effect, LabelExpr>),
    Row(SceneContainer<Effect, LabelExpr, TextExpr, ConditionExpr>),
    Column(SceneContainer<Effect, LabelExpr, TextExpr, ConditionExpr>),
    Box(SceneContainer<Effect, LabelExpr, TextExpr, ConditionExpr>),
    Conditional(SceneConditional<Effect, LabelExpr, TextExpr, ConditionExpr>),
}

pub type SceneComponent<
    Effect = SceneCommand,
    LabelExpr = SceneTextExpr,
    TextExpr = SceneTextExpr,
    ConditionExpr = String,
> = ComponentNode<Effect, LabelExpr, TextExpr, ConditionExpr>;

impl<Effect, LabelExpr, TextExpr, ConditionExpr>
    ComponentNode<Effect, LabelExpr, TextExpr, ConditionExpr>
{
    pub fn kind(&self) -> SceneComponentKind {
        match self {
            Self::Viewport(_) => SceneComponentKind::Viewport,
            Self::Frame(_) => SceneComponentKind::Frame,
            Self::Text(_) => SceneComponentKind::Text,
            Self::Button(_) => SceneComponentKind::Button,
            Self::Choice(_) => SceneComponentKind::Choice,
            Self::Row(_) => SceneComponentKind::Row,
            Self::Column(_) => SceneComponentKind::Column,
            Self::Box(_) => SceneComponentKind::Box,
            Self::Conditional(_) => SceneComponentKind::Conditional,
        }
    }

    pub fn children(&self) -> &[SceneComponent<Effect, LabelExpr, TextExpr, ConditionExpr>] {
        match self {
            Self::Row(container) | Self::Column(container) | Self::Box(container) => {
                &container.children
            }
            Self::Conditional(conditional) => &conditional.children,
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
            _ => None,
        }
    }

    pub fn layout(&self) -> Option<&SceneLayout> {
        match self {
            Self::Viewport(component) => Some(&component.layout),
            Self::Frame(component) => Some(&component.layout),
            Self::Button(button) | Self::Choice(button) => Some(&button.layout),
            Self::Row(container) | Self::Column(container) | Self::Box(container) => {
                Some(&container.layout)
            }
            Self::Text(text) => Some(&text.layout),
            Self::Conditional(_) => None,
        }
    }

    pub fn layout_mut(&mut self) -> Option<&mut SceneLayout> {
        match self {
            Self::Viewport(component) => Some(&mut component.layout),
            Self::Frame(component) => Some(&mut component.layout),
            Self::Button(button) | Self::Choice(button) => Some(&mut button.layout),
            Self::Row(container) | Self::Column(container) | Self::Box(container) => {
                Some(&mut container.layout)
            }
            Self::Text(text) => Some(&mut text.layout),
            Self::Conditional(_) => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentChoiceCell<'a, Effect> {
    pub x: usize,
    pub y: usize,
    pub effect: &'a Effect,
}

pub fn component_choice_cells<'a, Effect, LabelExpr, TextExpr, ConditionExpr>(
    components: &'a [SceneComponent<Effect, LabelExpr, TextExpr, ConditionExpr>],
    mut condition_is_true: impl FnMut(&ConditionExpr) -> bool,
) -> Vec<ComponentChoiceCell<'a, Effect>> {
    component_column_footprint(components, &mut condition_is_true).cells
}

struct ComponentChoiceFootprint<'a, Effect> {
    width: usize,
    height: usize,
    cells: Vec<ComponentChoiceCell<'a, Effect>>,
}

fn component_choice_footprint<'a, Effect, LabelExpr, TextExpr, ConditionExpr>(
    component: &'a SceneComponent<Effect, LabelExpr, TextExpr, ConditionExpr>,
    condition_is_true: &mut impl FnMut(&ConditionExpr) -> bool,
) -> ComponentChoiceFootprint<'a, Effect> {
    match component {
        ComponentNode::Choice(choice) => ComponentChoiceFootprint {
            width: 1,
            height: 1,
            cells: vec![ComponentChoiceCell {
                x: 0,
                y: 0,
                effect: &choice.effect,
            }],
        },
        ComponentNode::Row(container) => {
            component_row_footprint(&container.children, condition_is_true)
        }
        ComponentNode::Column(container) | ComponentNode::Box(container) => {
            component_column_footprint(&container.children, condition_is_true)
        }
        ComponentNode::Conditional(conditional) => component_column_footprint(
            if condition_is_true(&conditional.condition) {
                &conditional.children
            } else {
                &conditional.else_children
            },
            condition_is_true,
        ),
        ComponentNode::Viewport(_)
        | ComponentNode::Frame(_)
        | ComponentNode::Text(_)
        | ComponentNode::Button(_) => ComponentChoiceFootprint {
            width: 1,
            height: 1,
            cells: Vec::new(),
        },
    }
}

fn component_row_footprint<'a, Effect, LabelExpr, TextExpr, ConditionExpr>(
    components: &'a [SceneComponent<Effect, LabelExpr, TextExpr, ConditionExpr>],
    condition_is_true: &mut impl FnMut(&ConditionExpr) -> bool,
) -> ComponentChoiceFootprint<'a, Effect> {
    let mut width = 0;
    let mut height = 0;
    let mut cells = Vec::new();
    for component in components {
        let child = component_choice_footprint(component, condition_is_true);
        cells.extend(child.cells.into_iter().map(|cell| ComponentChoiceCell {
            x: cell.x + width,
            ..cell
        }));
        width += child.width;
        height = height.max(child.height);
    }
    ComponentChoiceFootprint {
        width: width.max(1),
        height: height.max(1),
        cells,
    }
}

fn component_column_footprint<'a, Effect, LabelExpr, TextExpr, ConditionExpr>(
    components: &'a [SceneComponent<Effect, LabelExpr, TextExpr, ConditionExpr>],
    condition_is_true: &mut impl FnMut(&ConditionExpr) -> bool,
) -> ComponentChoiceFootprint<'a, Effect> {
    let mut width = 0;
    let mut height = 0;
    let mut cells = Vec::new();
    for component in components {
        let child = component_choice_footprint(component, condition_is_true);
        cells.extend(child.cells.into_iter().map(|cell| ComponentChoiceCell {
            y: cell.y + height,
            ..cell
        }));
        width = width.max(child.width);
        height += child.height;
    }
    ComponentChoiceFootprint {
        width: width.max(1),
        height: height.max(1),
        cells,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SceneComponentKind {
    Viewport,
    Frame,
    Text,
    Button,
    Choice,
    Row,
    Column,
    Box,
    Conditional,
}

impl SceneComponentKind {
    pub fn keyword(self) -> &'static str {
        match self {
            Self::Viewport => "puzzle",
            Self::Frame => "frame",
            Self::Text => "text",
            Self::Button => "button",
            Self::Choice => "choice",
            Self::Row => "row",
            Self::Column => "column",
            Self::Box => "box",
            Self::Conditional => "if",
        }
    }

    pub fn from_keyword(value: &str) -> Option<Self> {
        Some(match value {
            "puzzle" | "puzzle3" => Self::Viewport,
            "frame" => Self::Frame,
            "heading" | "subheading" | "text" | "caption" => Self::Text,
            "button" => Self::Button,
            "choice" => Self::Choice,
            "row" => Self::Row,
            "column" => Self::Column,
            "box" => Self::Box,
            "if" => Self::Conditional,
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
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameComponent {
    pub kind: String,
    pub source: String,
    pub inputs: Vec<SceneInputBinding>,
    pub layout: SceneLayout,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewportProjection {
    #[default]
    TwoD,
    ThreeD,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewportComponent {
    pub source: String,
    pub projection: ViewportProjection,
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
pub struct SceneCommand {
    pub name: String,
    pub args: Vec<SceneCommandArg>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneAction {
    Goto { scene: String },
}

pub const STANDARD_MESSAGE_COMPONENT: &str = "standard.message";
pub const STANDARD_MESSAGE_TEXT_PROPERTY: &str = "text";
pub const STANDARD_MESSAGE_DISMISS_EVENT: &str = "dismiss";

pub fn standard_message_effect(text: SceneExpr) -> SceneEffect {
    SceneEffect::PresentComponent {
        definition: STANDARD_MESSAGE_COMPONENT.to_string(),
        properties: vec![ComponentProperty {
            name: STANDARD_MESSAGE_TEXT_PROPERTY.to_string(),
            value: text,
        }],
        placement: ComponentPlacement::Overlay,
        await_event: Some(STANDARD_MESSAGE_DISMISS_EVENT.to_string()),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SceneEffect {
    Input(String),
    ComponentEffect(String),
    RoutineCall(String),
    PresentComponent {
        definition: String,
        properties: Vec<ComponentProperty>,
        placement: ComponentPlacement,
        await_event: Option<String>,
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
    Move {
        component: String,
        order: ComponentOrder,
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

impl SceneEffect {
    pub fn try_map_scene_references<Error>(
        &mut self,
        map: &mut impl FnMut(&str) -> Result<String, Error>,
    ) -> Result<(), Error> {
        match self {
            Self::Goto { scene, .. }
            | Self::Enter { scene, .. }
            | Self::Create { scene }
            | Self::Reset { scene }
            | Self::Delete { scene }
            | Self::Show { scene }
            | Self::Hide { scene }
            | Self::Toggle { scene }
            | Self::Focus { scene } => {
                *scene = map(scene)?;
            }
            Self::Conditional { effect, .. } => {
                effect.try_map_scene_references(map)?;
            }
            Self::Sequence { effects } => {
                for effect in effects {
                    effect.try_map_scene_references(map)?;
                }
            }
            Self::Input(_)
            | Self::ComponentEffect(_)
            | Self::RoutineCall(_)
            | Self::PresentComponent { .. }
            | Self::Wait { .. }
            | Self::PlaySfx { .. }
            | Self::PlayMusic { .. }
            | Self::PauseMusic { .. }
            | Self::ResumeMusic { .. }
            | Self::StopMusic { .. }
            | Self::Back
            | Self::Move { .. }
            | Self::PuzzleNextLevel { .. }
            | Self::PuzzlePreviousLevel { .. }
            | Self::GotoLevel { .. }
            | Self::ResetPuzzle { .. }
            | Self::LoadPuzzle { .. }
            | Self::Apply { .. }
            | Self::Copy { .. }
            | Self::SetVariable { .. }
            | Self::ClearUndoHistory
            | Self::ClearGameProgress
            | Self::SetCurrentLevel { .. }
            | Self::ClearCurrentLevel
            | Self::SetLevelCleared { .. }
            | Self::ResetPersistentVars => {}
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentProperty {
    pub name: String,
    pub value: SceneExpr,
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
    PresentComponent {
        definition: &'a str,
        properties: &'a [ComponentProperty],
        placement: ComponentPlacement,
        await_event: &'a Option<String>,
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
    Move {
        component: &'a str,
        order: &'a ComponentOrder,
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
            SceneEffect::PresentComponent {
                definition,
                properties,
                placement,
                await_event,
            } => Self::PresentComponent {
                definition,
                properties,
                placement: *placement,
                await_event,
            },
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
            SceneEffect::Move { component, order } => Self::Move { component, order },
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
    PresentComponent {
        definition: String,
        properties: Vec<ComponentProperty>,
        placement: ComponentPlacement,
        await_event: Option<String>,
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
    Move {
        component: String,
        order: ComponentOrder,
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
            SceneEffectDeserialize::PresentComponent {
                definition,
                properties,
                placement,
                await_event,
            } => Self::PresentComponent {
                definition,
                properties,
                placement,
                await_event,
            },
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
            SceneEffectDeserialize::Move { component, order } => Self::Move { component, order },
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneBinaryOp {
    And,
    Eq,
    In,
    NotEq,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SceneFixtureJsonOptions {
    pub viewport_projection: Option<ViewportProjection>,
}

pub fn write_scene_component_fixture_json<TextExpr, TextWriter, LevelSource>(
    out: &mut String,
    component: &SceneComponent<SceneEffect, SceneExpr, TextExpr, SceneExpr>,
    options: SceneFixtureJsonOptions,
    write_text_fields: TextWriter,
    note_level_source: &mut LevelSource,
) -> bool
where
    TextWriter: Fn(&mut String, &TextExpr) + Copy,
    LevelSource: FnMut(&str),
{
    match component {
        SceneComponent::Viewport(viewport) => {
            if options
                .viewport_projection
                .is_some_and(|projection| viewport.projection != projection)
            {
                return false;
            }
            out.push_str("{ \"kind\": ");
            write_json_string(
                out,
                match viewport.projection {
                    ViewportProjection::TwoD => "puzzle",
                    ViewportProjection::ThreeD => "puzzle3",
                },
            );
            out.push_str(", \"source\": ");
            write_json_string(out, &viewport.source);
            push_inline_layout_json(out, &viewport.layout);
            out.push_str(" }");
        }
        SceneComponent::Frame(frame) => {
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
    }
    true
}

pub fn write_scene_effect_json(out: &mut String, effect: &SceneEffect) {
    out.push_str(
        &serde_json::to_string(effect).expect("validated scene effect should serialize to JSON"),
    );
}

pub fn write_scene_expr_json(out: &mut String, expr: &SceneExpr) {
    out.push_str(
        &serde_json::to_string(&scene_expr_json_value(expr))
            .expect("validated scene expression should serialize to JSON"),
    );
}

pub fn scene_expr_json_value(expr: &SceneExpr) -> serde_json::Value {
    match expr {
        SceneExpr::Bool(value) => serde_json::json!({ "kind": "bool", "value": value }),
        SceneExpr::Int(value) => serde_json::json!({ "kind": "int", "value": value }),
        SceneExpr::Text(value) => serde_json::json!({ "kind": "text", "value": value }),
        SceneExpr::Path(path) => {
            serde_json::json!({ "kind": "path", "path": path.join(".") })
        }
        SceneExpr::Call { name, args } => serde_json::json!({
            "kind": "call",
            "name": name,
            "args": args.iter().map(scene_expr_json_value).collect::<Vec<_>>(),
        }),
        SceneExpr::Binary { op, left, right } => {
            let operator = match op {
                SceneBinaryOp::And => "and",
                SceneBinaryOp::Eq => "eq",
                SceneBinaryOp::In => "in",
                SceneBinaryOp::NotEq => "neq",
            };
            serde_json::json!({
                "kind": "binary",
                "op": operator,
                "left": scene_expr_json_value(left),
                "right": scene_expr_json_value(right),
            })
        }
        SceneExpr::If {
            condition,
            then_branch,
            else_branch,
        } => serde_json::json!({
            "kind": "if",
            "condition": scene_expr_json_value(condition),
            "then": scene_expr_json_value(then_branch),
            "else": scene_expr_json_value(else_branch),
        }),
    }
}

fn write_container_fixture_json<TextExpr, TextWriter, LevelSource>(
    out: &mut String,
    kind: &str,
    children: &[SceneComponent<SceneEffect, SceneExpr, TextExpr, SceneExpr>],
    layout: &SceneLayout,
    options: SceneFixtureJsonOptions,
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
    options: SceneFixtureJsonOptions,
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

fn push_inline_layout_json(out: &mut String, layout: &SceneLayout) {
    if scene_layout_is_default(layout) {
        return;
    }
    out.push_str(", \"layout\": ");
    write_scene_layout_json(out, layout);
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

pub trait SceneBlockHandler<Line>
where
    Line: AsRef<str>,
{
    type Error: From<SceneBlockParseError>;

    fn parse_state_block(&mut self, lines: &[Line], start: usize) -> Result<usize, Self::Error> {
        Err(
            SceneBlockParseError::new(lines[start].as_ref(), "unknown scene directive state")
                .into(),
        )
    }

    fn parse_layout_block(&mut self, lines: &[Line], start: usize) -> Result<usize, Self::Error>;

    fn parse_inputs_block(&mut self, lines: &[Line], start: usize) -> Result<usize, Self::Error> {
        Err(
            SceneBlockParseError::new(lines[start].as_ref(), "unknown scene directive inputs")
                .into(),
        )
    }

    fn parse_keys_block(&mut self, lines: &[Line], start: usize) -> Result<usize, Self::Error> {
        Err(SceneBlockParseError::new(lines[start].as_ref(), "unknown scene directive keys").into())
    }

    fn parse_rules_block(&mut self, lines: &[Line], start: usize) -> Result<usize, Self::Error> {
        Err(
            SceneBlockParseError::new(lines[start].as_ref(), "unknown scene directive rules")
                .into(),
        )
    }

    fn parse_scene_start_block(
        &mut self,
        lines: &[Line],
        start: usize,
    ) -> Result<usize, Self::Error> {
        let _ = lines;
        Err(SceneBlockParseError::new(
            lines[start].as_ref(),
            "unknown scene directive on_scene_start",
        )
        .into())
    }

    fn parse_inline_directive(
        &mut self,
        lines: &[Line],
        start: usize,
    ) -> Result<usize, Self::Error>;
}

pub fn parse_scene_block_with_handler<Line, Handler>(
    lines: &[Line],
    start: usize,
    scene_name: &str,
    syntax: SceneBlockSyntax,
    handler: &mut Handler,
) -> Result<usize, Handler::Error>
where
    Line: AsRef<str>,
    Handler: SceneBlockHandler<Line>,
{
    let mut index = start;
    while index < lines.len() {
        let line = lines[index].as_ref();
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

pub fn parse_scene_component_block<Line, Component, Error, ParseLeaf, BuildContainer>(
    lines: &[Line],
    start: usize,
    block_name: &str,
    syntax: SceneBlockSyntax,
    parse_leaf: &mut ParseLeaf,
    build_container: &BuildContainer,
) -> Result<(usize, Vec<Component>), Error>
where
    Error: From<SceneBlockParseError>,
    Line: AsRef<str>,
    ParseLeaf: FnMut(&[Line], usize) -> Result<(usize, Component), Error>,
    BuildContainer: Fn(SceneComponentKind, Vec<Component>, SceneLayout) -> Component,
{
    let mut components = Vec::new();
    let mut index = start;
    while index < lines.len() {
        let line = lines[index].as_ref();
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

pub fn parse_scene_component_at<Line, Component, Error, ParseLeaf, BuildContainer>(
    lines: &[Line],
    index: usize,
    syntax: SceneBlockSyntax,
    parse_leaf: &mut ParseLeaf,
    build_container: &BuildContainer,
) -> Result<(usize, Component), Error>
where
    Error: From<SceneBlockParseError>,
    Line: AsRef<str>,
    ParseLeaf: FnMut(&[Line], usize) -> Result<(usize, Component), Error>,
    BuildContainer: Fn(SceneComponentKind, Vec<Component>, SceneLayout) -> Component,
{
    let line = lines[index].as_ref();
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
    fn scene_expression_json_value_uses_public_path_shape() {
        let expr = SceneExpr::Path(vec![
            "level".to_string(),
            "progress".to_string(),
            "cleared".to_string(),
        ]);

        assert_eq!(
            scene_expr_json_value(&expr),
            serde_json::json!({
                "kind": "path",
                "path": "level.progress.cleared",
            })
        );
    }

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
            Some(SceneComponentKind::Viewport)
        );
        assert_eq!(
            SceneComponentKind::from_keyword("puzzle3"),
            Some(SceneComponentKind::Viewport)
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
    fn choice_cells_preserve_container_geometry_and_resolve_conditional_branch() {
        fn choice(name: &str) -> SceneComponent {
            SceneComponent::Choice(SceneButton {
                label: SceneTextExpr::Literal(name.to_string()),
                effect: SceneCommand {
                    name: name.to_string(),
                    args: Vec::new(),
                },
                layout: SceneLayout::default(),
            })
        }

        let components = vec![
            SceneComponent::Row(SceneContainer {
                children: vec![choice("top_left"), choice("top_right")],
                layout: SceneLayout::default(),
            }),
            SceneComponent::Conditional(SceneConditional {
                condition: "visible".to_string(),
                children: vec![SceneComponent::Row(SceneContainer {
                    children: vec![choice("bottom_left"), choice("bottom_right")],
                    layout: SceneLayout::default(),
                })],
                else_children: vec![choice("hidden")],
            }),
        ];

        let cells = component_choice_cells(&components, |condition| condition == "visible");
        assert_eq!(
            cells
                .iter()
                .map(|cell| { (cell.x, cell.y, cell.effect.name.as_str()) })
                .collect::<Vec<_>>(),
            vec![
                (0, 0, "top_left"),
                (1, 0, "top_right"),
                (0, 1, "bottom_left"),
                (1, 1, "bottom_right"),
            ]
        );
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
