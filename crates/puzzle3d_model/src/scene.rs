pub use puzzle_scene::{
    SceneAlign as SceneAlign3, SceneAlignX as SceneAlignX3, SceneAlignY as SceneAlignY3,
    SceneLayout as SceneLayout3, SceneSize as SceneSize3,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scene3 {
    pub name: String,
    pub layout: SceneLayout3,
    pub puzzles: Vec<ScenePuzzle3>,
    pub keys: Vec<SceneKeyBinding3>,
    pub components: Vec<SceneComponent3>,
}

impl Scene3 {
    pub fn new(
        name: impl Into<String>,
        layout: SceneLayout3,
        puzzles: Vec<ScenePuzzle3>,
        keys: Vec<SceneKeyBinding3>,
        components: Vec<SceneComponent3>,
    ) -> Self {
        Self {
            name: name.into(),
            layout,
            puzzles,
            keys,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SceneAction3 {
    Goto { scene: String },
    StartLevels { levels: String, scene: String },
}
