use std::collections::HashMap;

use puzzle_core::{
    ComparisonOp, CompiledGame, ConditionId, ConditionValueKind, GlobalId, InputId, ObjectId,
    RuleId, RuleStep, State,
};
pub use puzzle_scene::{
    LevelMenuLocked, SceneAlign as SceneAlignDef, SceneAlignX as SceneAlignXDef,
    SceneAlignY as SceneAlignYDef, SceneButton as SharedSceneButton,
    SceneComponent as SharedSceneComponent, SceneConditional as SharedSceneConditional,
    SceneContainer as SharedSceneContainer, SceneFor as SharedSceneFor,
    SceneForSource as ForSource, SceneLayout as SceneLayoutDef, SceneSize as SceneSizeDef,
    SceneTextComponent as SharedSceneTextComponent,
};
use puzzle3d_model::ParsedPuzzle3;

#[derive(Clone, Debug)]
pub struct LoadedDocument {
    pub title: String,
    pub subtitle: Option<String>,
    pub author: Option<String>,
    pub homepage: Option<String>,
    pub default_wait_ms: u64,
    pub default_again_ms: u64,
    pub animation: AnimationDef,
    pub sounds: SoundsDef,
    pub theme: ThemeDef,
    pub assets: AssetsDef,
    pub scenes: Vec<SceneDef>,
    pub models: Vec<LoadedDocumentModel>,
}

impl LoadedDocument {
    pub fn single_model(&self) -> Option<&LoadedDocumentModel> {
        let [model] = self.models.as_slice() else {
            return None;
        };
        Some(model)
    }
}

#[derive(Clone, Debug)]
pub enum LoadedDocumentModel {
    Puzzle2d { name: String, game: LoadedGame },
    Puzzle3d { name: String, puzzle: ParsedPuzzle3 },
}

#[derive(Clone, Debug)]
pub struct LoadedGame {
    pub title: String,
    pub subtitle: Option<String>,
    pub author: Option<String>,
    pub homepage: Option<String>,
    pub game: CompiledGame,
    pub warnings: Vec<String>,
    pub default_wait_ms: u64,
    pub default_again_ms: u64,
    pub animation: AnimationDef,
    pub rule_emissions: HashMap<RuleId, Vec<RuleEmission>>,
    pub rule_effects: HashMap<RuleId, Vec<RuleEffect>>,
    pub level_start_program: Option<Vec<RuleStep>>,
    pub display_level_start_program: Option<Vec<RuleStep>>,
    pub level_clear_program: Option<Vec<RuleStep>>,
    pub last_level_clear_program: Option<Vec<RuleStep>>,
    pub display_level_clear_program: Option<Vec<RuleStep>>,
    pub display_program: Option<Vec<RuleStep>>,
    pub levels: Vec<Level>,
    pub run_rules_on_level_start: bool,
    pub legend: AsciiLegend,
    pub controls: Controls,
    pub variables: Vec<SceneVarDef>,
    pub scenes: Vec<SceneDef>,
    pub object_labels: HashMap<ObjectId, String>,
    pub object_groups: HashMap<String, Vec<ObjectId>>,
    pub input_labels: HashMap<InputId, String>,
    pub global_labels: HashMap<GlobalId, String>,
    pub persistent_vars: Vec<GlobalId>,
    pub condition_labels: HashMap<ConditionId, String>,
    pub conditions: HashMap<String, GoalCondition>,
    pub goal: Option<GoalCondition>,
    pub lose: Option<GoalCondition>,
    pub sounds: SoundsDef,
    pub theme: ThemeDef,
    pub assets: AssetsDef,
    pub visuals: VisualsDef,
    pub render: PuzzleRenderDef,
    pub screen: PuzzleScreenDef,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuleEmission {
    PlaySfx {
        name: String,
    },
    Animate {
        trigger: RuleAnimationTrigger,
        name: String,
        objects: Vec<ObjectId>,
    },
    Wait {
        milliseconds: u64,
    },
    Message {
        text: String,
        literal: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleAnimationTrigger {
    Move,
    CantMove,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnimationDef {
    pub tween: TweenAnimationDef,
}

impl Default for AnimationDef {
    fn default() -> Self {
        Self {
            tween: TweenAnimationDef::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TweenAnimationDef {
    pub enabled: bool,
    pub interval_ms: u64,
}

impl Default for TweenAnimationDef {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_ms: 250,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuleEffect {
    Win,
    Restart,
    NextLevel,
    Again,
    Checkpoint,
    ClearCheckpoint,
    PlaySfx { name: String },
    Wait { milliseconds: u64 },
    WaitAnimation,
    Message { text: String, literal: bool },
}

#[derive(Clone, Debug, Default)]
pub struct AssetsDef {
    pub entries: Vec<AssetDef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetDef {
    pub kind: AssetKind,
    pub path: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssetKind {
    Css,
    Script,
}

#[derive(Clone, Debug, Default)]
pub struct SoundsDef {
    pub sfx: Vec<SfxSoundDef>,
    pub music: Vec<MusicSoundDef>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SfxSoundDef {
    pub name: String,
    pub seed: String,
    pub type_target: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MusicSoundDef {
    pub name: String,
    pub seed: String,
    pub height: f64,
    pub bars: u16,
    pub bpm: u16,
    pub volume: f64,
}

#[derive(Clone, Debug)]
pub struct ThemeDef {
    pub name: Option<String>,
    pub variables: Vec<ThemeVariableDef>,
}

impl Default for ThemeDef {
    fn default() -> Self {
        Self {
            name: Some("clean".to_string()),
            variables: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThemeVariableDef {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, Default)]
pub struct VisualsDef {
    pub aliases: Vec<VisualAliasDef>,
    pub sprites: Vec<VisualSpriteDef>,
}

#[derive(Clone, Debug)]
pub struct VisualAliasDef {
    pub object: String,
    pub sprite: String,
}

#[derive(Clone, Debug)]
pub struct VisualSpriteDef {
    pub name: String,
    pub kind: VisualSpriteKind,
    pub offset: VisualSpriteOffset,
    pub pixels_per_cell: Option<VisualSpritePixelsPerCell>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VisualSpriteOffset {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VisualSpritePixelsPerCell {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug)]
pub enum VisualSpriteKind {
    Solid(String),
    Image {
        source: String,
    },
    Ascii {
        pattern: Vec<String>,
        colors: Vec<VisualColorDef>,
    },
}

#[derive(Clone, Debug)]
pub struct VisualColorDef {
    pub token: char,
    pub color: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PuzzleRenderDef {
    pub grid: PuzzleGridRenderDef,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PuzzleGridRenderDef {
    pub occupied_cells: bool,
    pub all_cells: bool,
}

#[derive(Clone, Debug)]
pub struct Level {
    pub name: String,
    pub pack: Option<String>,
    pub puzzle: String,
    pub initial_state: State,
    pub regions: Vec<LevelRegionDef>,
    pub level_start_program: Option<Vec<RuleStep>>,
    pub level_clear_program: Option<Vec<RuleStep>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LevelRegionDef {
    pub index: usize,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PuzzleScreenDef {
    pub viewport_size: ViewportSizeDef,
    pub viewport_focus: String,
    pub viewport_mode: ViewportModeDef,
}

pub type PuzzleViewDef = PuzzleScreenDef;

impl Default for PuzzleScreenDef {
    fn default() -> Self {
        Self {
            viewport_size: ViewportSizeDef::Full,
            viewport_focus: "Player".to_string(),
            viewport_mode: ViewportModeDef::Paged,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ViewportSizeDef {
    Full,
    Size { width: u16, height: u16 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ViewportModeDef {
    Paged,
    Centered,
}

#[derive(Clone, Debug)]
pub struct GoalCondition {
    pub description: String,
    pub expr: GoalExpr,
}

#[derive(Clone, Debug)]
pub enum GoalExpr {
    All(Vec<GoalExpr>),
    Any(Vec<GoalExpr>),
    Clause(GoalClause),
}

#[derive(Clone, Debug)]
pub struct GoalClause {
    pub value: GoalValue,
    pub op: ComparisonOp,
    pub expected: i64,
}

#[derive(Clone, Debug)]
pub enum GoalValue {
    Global(GlobalId),
    Condition(ConditionId),
    InlineConditionValue(ConditionValueKind),
}

#[derive(Clone, Debug)]
pub struct SceneDef {
    pub name: String,
    pub layout: SceneLayoutDef,
    pub resources: SceneResources,
    pub state: SceneStateDef,
    pub components: Vec<SceneComponent>,
    pub key_bindings: Vec<KeyBinding>,
    pub transitions: Vec<SceneTransition>,
    pub puzzle_rule: Option<ScenePuzzleRule>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScenePuzzleRule {
    pub target: String,
    pub rule: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SceneResources {
    pub levels: ResourceSelection,
    pub sprites: ResourceSelection,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ResourceSelection {
    #[default]
    All,
    Named(Vec<String>),
}

#[derive(Clone, Debug, Default)]
pub struct SceneStateDef {
    pub variables: Vec<SceneVarDef>,
    pub puzzles: Vec<ScenePuzzleDef>,
}

#[derive(Clone, Debug)]
pub struct SceneVarDef {
    pub name: String,
    pub default: SceneValue,
    pub lifetime: SceneStateLifetime,
    pub mutable: bool,
}

#[derive(Clone, Debug)]
pub struct ScenePuzzleDef {
    pub name: String,
    pub kind: String,
    pub model: String,
    pub initializer: ScenePuzzleInitializer,
    pub lifetime: SceneStateLifetime,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScenePuzzleInitializer {
    CurrentLevel,
    Level(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SceneValue {
    Bool(bool),
    Int(i64),
    Text(String),
    Symbol(String),
    LevelRef(usize),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SceneStateLifetime {
    #[default]
    Instance,
    ResetOnStart,
    Persistent,
}

pub type SceneComponent = SharedSceneComponent<SceneEffect, SceneExpr, SceneTextContent>;

pub type SceneTitleDef = SharedSceneTextComponent<SceneExpr>;

pub type SceneTextDef = SharedSceneTextComponent<SceneTextContent>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SceneTextContent {
    Literal(String),
    Path(Vec<String>),
}

pub type SceneButtonDef = SharedSceneButton<SceneEffect, SceneExpr>;

pub type SceneConditionalDef = SharedSceneConditional<SceneEffect, SceneExpr, SceneTextContent>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SceneEffect {
    Input(String),
    ComponentEffect(String),
    Message {
        text: SceneExpr,
    },
    Wait {
        milliseconds: Option<u64>,
    },
    Conditional {
        condition: String,
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
    Sequence(Vec<SceneEffect>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SceneEffectParam {
    Level(SceneExpr),
    Named { name: String, value: SceneExpr },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SceneExpr {
    Bool(bool),
    Int(i64),
    Text(String),
    Path(Vec<String>),
    Call { name: String, args: Vec<SceneExpr> },
}

pub type SceneContainerDef = SharedSceneContainer<SceneEffect, SceneExpr, SceneTextContent>;

pub type SceneForDef = SharedSceneFor<SceneEffect, SceneExpr, SceneTextContent>;

pub type LevelMenuDef = puzzle_scene::LevelMenuComponent<SceneEffect, SceneExpr>;

#[derive(Clone, Debug)]
pub struct KeyBinding {
    pub keys: Vec<KeyTrigger>,
    pub effect: SceneEffect,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneTransition {
    pub trigger: SceneTransitionTrigger,
    pub effect: SceneEffect,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SceneTransitionTrigger {
    Condition(String),
    SceneStart,
    LevelStart,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum KeyTrigger {
    Char(char),
    Named(String),
}

impl LoadedGame {
    pub fn object_name(&self, object: ObjectId) -> &str {
        self.object_labels
            .get(&object)
            .map(String::as_str)
            .unwrap_or("?")
    }

    pub fn is_goal_complete(&self, state: &State) -> bool {
        self.goal
            .as_ref()
            .is_some_and(|goal| eval_goal_expr(&self.game, state, &goal.expr))
    }

    pub fn is_lose_complete(&self, state: &State) -> bool {
        self.lose
            .as_ref()
            .is_some_and(|lose| eval_goal_expr(&self.game, state, &lose.expr))
    }

    pub fn is_condition_true(&self, name: &str, state: &State) -> bool {
        let Some(condition) = self.conditions.get(name) else {
            return false;
        };

        eval_goal_expr(&self.game, state, &condition.expr)
    }

    pub fn is_global_truthy(&self, name: &str, state: &State) -> bool {
        let Some(global) = self.global_id(name) else {
            return false;
        };

        state.global_value(global).unwrap_or(0) != 0
    }

    fn global_id(&self, name: &str) -> Option<GlobalId> {
        self.global_labels
            .iter()
            .find_map(|(global, label)| (label == name).then_some(*global))
    }
}

fn eval_goal_expr(game: &CompiledGame, state: &State, expr: &GoalExpr) -> bool {
    match expr {
        GoalExpr::All(exprs) => exprs.iter().all(|expr| eval_goal_expr(game, state, expr)),
        GoalExpr::Any(exprs) => exprs.iter().any(|expr| eval_goal_expr(game, state, expr)),
        GoalExpr::Clause(clause) => compare_i64(
            eval_goal_value(game, state, &clause.value),
            clause.op,
            clause.expected,
        ),
    }
}

fn eval_goal_value(game: &CompiledGame, state: &State, value: &GoalValue) -> i64 {
    match value {
        GoalValue::Global(global) => state.global_value(*global).unwrap_or(0),
        GoalValue::Condition(condition) => game
            .condition_def(*condition)
            .map(|condition| eval_goal_condition_value_kind(game, state, &condition.kind))
            .unwrap_or(0),
        GoalValue::InlineConditionValue(kind) => eval_goal_condition_value_kind(game, state, kind),
    }
}

fn eval_goal_condition_value_kind(
    game: &CompiledGame,
    state: &State,
    kind: &ConditionValueKind,
) -> i64 {
    match kind {
        ConditionValueKind::CountObjects(objects) => objects
            .iter()
            .map(|object| i64::from(state.object_count(*object)))
            .sum(),
        ConditionValueKind::ExistsObjects(objects) => {
            if objects.iter().any(|object| state.object_count(*object) > 0) {
                1
            } else {
                0
            }
        }
        ConditionValueKind::NoneObjects(objects) => {
            if objects.iter().any(|object| state.object_count(*object) > 0) {
                0
            } else {
                1
            }
        }
        ConditionValueKind::CountMatches(patterns) => patterns
            .iter()
            .map(|pattern| i64::from(puzzle_core::count_pattern_matches(game, state, pattern)))
            .sum(),
        ConditionValueKind::ExistsMatches(patterns) => {
            if patterns
                .iter()
                .any(|pattern| puzzle_core::has_pattern_match(game, state, pattern))
            {
                1
            } else {
                0
            }
        }
        ConditionValueKind::NoneMatches(patterns) => {
            if patterns
                .iter()
                .any(|pattern| puzzle_core::has_pattern_match(game, state, pattern))
            {
                0
            } else {
                1
            }
        }
        ConditionValueKind::CountInputMatches(_)
        | ConditionValueKind::ExistsInputMatches(_)
        | ConditionValueKind::NoneInputMatches(_) => 0,
    }
}

fn compare_i64(left: i64, op: ComparisonOp, right: i64) -> bool {
    match op {
        ComparisonOp::Eq => left == right,
        ComparisonOp::NotEq => left != right,
        ComparisonOp::Greater => left > right,
        ComparisonOp::GreaterEq => left >= right,
        ComparisonOp::Less => left < right,
        ComparisonOp::LessEq => left <= right,
    }
}

#[derive(Clone, Debug)]
pub struct AsciiLegend {
    chars: Vec<char>,
    ignored: Vec<ObjectId>,
    empty: char,
    unknown: char,
}

impl AsciiLegend {
    pub(crate) fn new(object_count: usize, empty: char) -> Self {
        Self {
            chars: vec!['?'; object_count + 1],
            ignored: Vec::new(),
            empty,
            unknown: '?',
        }
    }

    pub(crate) fn set(&mut self, object: ObjectId, ch: char) {
        let index = usize::from(object.0);
        if index >= self.chars.len() {
            self.chars.resize(index + 1, self.unknown);
        }
        self.chars[index] = ch;
    }

    pub(crate) fn add_overlay(&mut self, _objects: Vec<ObjectId>, _ch: char) {
        // Multi-object legend entries are still accepted for level input, but
        // runtime ASCII display uses the top layer object's own legend char.
    }

    pub(crate) fn ignore(&mut self, object: ObjectId) {
        if !self.ignored.contains(&object) {
            self.ignored.push(object);
        }
    }

    pub fn char_for_cell(&self, objects: &[ObjectId]) -> char {
        let visible_objects = objects
            .iter()
            .copied()
            .filter(|object| !self.ignored.contains(object))
            .collect::<Vec<_>>();
        self.char_for_cell_with_visible_objects(&visible_objects)
    }

    pub fn legended_objects_for_cell(&self, objects: &[ObjectId]) -> Vec<ObjectId> {
        objects
            .iter()
            .copied()
            .filter(|object| !object.is_empty())
            .filter(|object| !self.ignored.contains(object))
            .filter(|object| self.char_for_object(*object) != self.unknown)
            .collect()
    }

    fn char_for_cell_with_visible_objects(&self, visible_objects: &[ObjectId]) -> char {
        let object = visible_objects.last().copied().unwrap_or(ObjectId::EMPTY);
        if object.is_empty() {
            return self.empty;
        }
        self.char_for_object(object)
    }

    fn char_for_object(&self, object: ObjectId) -> char {
        self.chars
            .get(usize::from(object.0))
            .copied()
            .unwrap_or(self.unknown)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Controls {
    pub keys: HashMap<u8, InputId>,
    pub arrows: HashMap<ArrowKey, InputId>,
    pub named: HashMap<String, InputId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ArrowKey {
    Up,
    Down,
    Left,
    Right,
}
