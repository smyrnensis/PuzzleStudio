pub type Scene = puzzle_scene::Scene<ScenePuzzle3, SceneComponent, SceneAction, SceneRuleCall>;
pub type SceneAction = puzzle_scene::SceneAction;
pub type SceneComponent = puzzle_scene::SceneComponent<SceneAction, String, String>;
pub type SceneControl = puzzle_scene::SceneControl<SceneAction>;
pub type SceneControlTarget = puzzle_scene::SceneControlTarget<SceneAction>;
pub type SceneInputMap = puzzle_scene::SceneInputMap;
pub type SceneInputBinding = puzzle_scene::SceneInputBinding;
pub type SceneKeyBinding = puzzle_scene::SceneKeyBinding<SceneAction>;
pub type SceneRuleCall = puzzle_scene::SceneRuleCall;

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

pub type SceneLayout3 = puzzle_scene::SceneLayout;
pub type SceneSize3 = puzzle_scene::SceneSize;
pub type SceneAlign3 = puzzle_scene::SceneAlign;
pub type SceneAlignX3 = puzzle_scene::SceneAlignX;
pub type SceneAlignY3 = puzzle_scene::SceneAlignY;
