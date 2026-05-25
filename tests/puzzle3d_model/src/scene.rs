#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scene3 {
    pub name: String,
    pub puzzles: Vec<ScenePuzzle3>,
    pub keys: Vec<SceneKeyBinding3>,
    pub controls: Vec<SceneControl3>,
    pub rules: Vec<SceneRuleCall3>,
    pub components: Vec<SceneComponent3>,
}

impl Scene3 {
    pub fn new(
        name: impl Into<String>,
        puzzles: Vec<ScenePuzzle3>,
        keys: Vec<SceneKeyBinding3>,
        controls: Vec<SceneControl3>,
        rules: Vec<SceneRuleCall3>,
        components: Vec<SceneComponent3>,
    ) -> Self {
        Self {
            name: name.into(),
            puzzles,
            keys,
            controls,
            rules,
            components,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneKeyBinding3 {
    pub key: String,
    pub action: SceneAction3,
}

impl SceneKeyBinding3 {
    pub fn new(key: impl Into<String>, action: SceneAction3) -> Self {
        Self {
            key: key.into(),
            action,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScenePuzzle3 {
    pub slot: String,
    pub model: String,
}

impl ScenePuzzle3 {
    pub fn new(slot: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            slot: slot.into(),
            model: model.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneControl3 {
    pub key: String,
    pub target: SceneControlTarget3,
}

impl SceneControl3 {
    pub fn new(key: impl Into<String>, target: SceneControlTarget3) -> Self {
        Self {
            key: key.into(),
            target,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SceneControlTarget3 {
    Input(String),
    Action(SceneAction3),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneRuleCall3 {
    pub target: String,
    pub rule: String,
    pub input_map: Vec<SceneInputMap3>,
}

impl SceneRuleCall3 {
    pub fn new(
        target: impl Into<String>,
        rule: impl Into<String>,
        input_map: Vec<SceneInputMap3>,
    ) -> Self {
        Self {
            target: target.into(),
            rule: rule.into(),
            input_map,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneInputMap3 {
    pub from: String,
    pub to: String,
}

impl SceneInputMap3 {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentInputBinding3 {
    pub input: String,
    pub keys: Vec<String>,
}

impl ComponentInputBinding3 {
    pub fn new(input: impl Into<String>, keys: Vec<String>) -> Self {
        Self {
            input: input.into(),
            keys,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SceneComponent3 {
    Title {
        text: String,
        layout: SceneLayout3,
    },
    Button {
        label: String,
        action: SceneAction3,
        layout: SceneLayout3,
    },
    LevelMenu {
        levels: String,
        action: SceneAction3,
        layout: SceneLayout3,
    },
    Puzzle3 {
        source: String,
        inputs: Vec<ComponentInputBinding3>,
        layout: SceneLayout3,
    },
    Row {
        children: Vec<SceneComponent3>,
        layout: SceneLayout3,
    },
    Column {
        children: Vec<SceneComponent3>,
        layout: SceneLayout3,
    },
    Box {
        children: Vec<SceneComponent3>,
        layout: SceneLayout3,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SceneLayout3 {
    pub size: Option<SceneSize3>,
    pub gap: Option<u16>,
    pub align: SceneAlign3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SceneSize3 {
    pub width: u16,
    pub height: u16,
}

impl SceneSize3 {
    pub fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SceneAlign3 {
    pub x: SceneAlignX3,
    pub y: SceneAlignY3,
}

impl Default for SceneAlign3 {
    fn default() -> Self {
        Self {
            x: SceneAlignX3::Center,
            y: SceneAlignY3::Center,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneAlignX3 {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneAlignY3 {
    Top,
    Center,
    Bottom,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SceneAction3 {
    Goto { scene: String },
    StartLevels { levels: String, scene: String },
}
