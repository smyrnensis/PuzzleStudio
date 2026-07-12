use std::collections::HashMap;

use crate::ParsedPuzzle3;
use puzzle_core::{
    ComparisonOp, CompiledGame, ConditionId, ConditionValueKind, InputId, MarkId, ObjectId, RuleId,
    RuleStep, State, VariableId,
};
pub use puzzle_scene::{
    LevelMenuLocked, SceneAlign as SceneAlignDef, SceneAlignX as SceneAlignXDef,
    SceneAlignY as SceneAlignYDef, SceneBinaryOp, SceneButton as SharedSceneButton,
    SceneComponent as SharedSceneComponent, SceneConditional as SharedSceneConditional,
    SceneContainer as SharedSceneContainer, SceneEffect, SceneEffectParam, SceneExpr,
    SceneFor as SharedSceneFor, SceneForSource as ForSource, SceneLayout as SceneLayoutDef,
    SceneLevelKey, SceneSize as SceneSizeDef, SceneTextComponent as SharedSceneTextComponent,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct LoadedDocument {
    pub title: String,
    pub subtitle: Option<String>,
    pub author: Option<String>,
    pub homepage: Option<String>,
    pub default_wait_ms: u64,
    pub default_again_ms: u64,
    pub input_buffer: InputBufferDef,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuleDebugInfo {
    pub source_line: String,
    pub source_line_number: Option<usize>,
    pub routine_stack: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoadedGame {
    pub title: String,
    pub subtitle: Option<String>,
    pub author: Option<String>,
    pub homepage: Option<String>,
    pub game: CompiledGame,
    pub warnings: Vec<String>,
    pub default_wait_ms: u64,
    pub default_again_ms: u64,
    pub input_buffer: InputBufferDef,
    pub animation: AnimationDef,
    pub rule_animations: HashMap<RuleId, Vec<RuleAnimation>>,
    pub rule_effects: HashMap<RuleId, Vec<RuleEffect>>,
    #[serde(default)]
    pub rule_debug_info: HashMap<RuleId, RuleDebugInfo>,
    pub level_start_program: Option<Vec<RuleStep>>,
    pub display_level_start_program: Option<Vec<RuleStep>>,
    pub level_clear_program: Option<Vec<RuleStep>>,
    pub last_level_clear_program: Option<Vec<RuleStep>>,
    pub display_level_clear_program: Option<Vec<RuleStep>>,
    pub display_program: Option<Vec<RuleStep>>,
    #[serde(default)]
    pub display_objects: Vec<ObjectId>,
    #[serde(default)]
    pub display_rules: Vec<RuleId>,
    pub levels: Vec<Level>,
    pub run_rules_on_level_start: bool,
    pub legend: AsciiLegend,
    pub controls: Controls,
    pub variables: Vec<SceneVarDef>,
    pub scenes: Vec<SceneDef>,
    pub object_labels: HashMap<ObjectId, String>,
    pub object_groups: HashMap<String, Vec<ObjectId>>,
    pub input_labels: HashMap<InputId, String>,
    pub variable_labels: HashMap<VariableId, String>,
    #[serde(default)]
    pub mark_labels: HashMap<MarkId, String>,
    pub persistent_vars: Vec<VariableId>,
    pub condition_labels: HashMap<ConditionId, String>,
    #[serde(default)]
    pub queries: HashMap<String, QueryExpr>,
    pub conditions: HashMap<String, GoalCondition>,
    pub goal: Option<GoalCondition>,
    pub lose: Option<GoalCondition>,
    #[serde(default)]
    pub solver_strategy: SolverStrategy,
    pub sounds: SoundsDef,
    #[serde(default)]
    pub model_operation_sounds: Vec<ModelOperationSoundDef>,
    pub theme: ThemeDef,
    pub assets: AssetsDef,
    pub visuals: VisualsDef,
    pub render: PuzzleRenderDef,
    pub screen: PuzzleScreenDef,
}

impl LoadedGame {
    pub fn empty_scene_host(
        title: impl Into<String>,
        puzzle_name: impl Into<String>,
        level_name: impl Into<String>,
    ) -> Self {
        let puzzle_name = puzzle_name.into();
        let initial_state = State::empty(1, 1, 1, 0)
            .expect("empty scene host state uses valid non-zero dimensions");
        Self {
            title: title.into(),
            subtitle: None,
            author: None,
            homepage: None,
            game: CompiledGame::new(1, Vec::new(), Vec::new()),
            warnings: Vec::new(),
            default_wait_ms: 250,
            default_again_ms: 150,
            input_buffer: InputBufferDef::default(),
            animation: AnimationDef::default(),
            rule_animations: HashMap::new(),
            rule_effects: HashMap::new(),
            rule_debug_info: HashMap::new(),
            level_start_program: None,
            display_level_start_program: None,
            level_clear_program: None,
            last_level_clear_program: None,
            display_level_clear_program: None,
            display_program: None,
            display_objects: Vec::new(),
            display_rules: Vec::new(),
            levels: vec![Level {
                name: level_name.into(),
                pack: None,
                puzzle: puzzle_name,
                initial_state,
                regions: Vec::new(),
                level_start_program: None,
                level_clear_program: None,
            }],
            run_rules_on_level_start: false,
            legend: AsciiLegend::new(0, Some('.')),
            controls: Controls::default(),
            variables: Vec::new(),
            scenes: Vec::new(),
            object_labels: HashMap::new(),
            object_groups: HashMap::new(),
            input_labels: HashMap::new(),
            variable_labels: HashMap::new(),
            mark_labels: HashMap::new(),
            persistent_vars: Vec::new(),
            condition_labels: HashMap::new(),
            queries: HashMap::new(),
            conditions: HashMap::new(),
            goal: None,
            lose: None,
            solver_strategy: SolverStrategy::default(),
            sounds: SoundsDef::default(),
            model_operation_sounds: Vec::new(),
            theme: ThemeDef::default(),
            assets: AssetsDef::default(),
            visuals: VisualsDef::default(),
            render: PuzzleRenderDef::default(),
            screen: PuzzleScreenDef::default(),
        }
    }

    pub fn is_display_object(&self, object: ObjectId) -> bool {
        self.display_objects.contains(&object)
    }

    pub fn solver_game(&self) -> CompiledGame {
        self.game.clone()
    }

    pub fn solver_state(&self, state: &State) -> State {
        state.clone()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputBufferDef {
    pub queue_during_wait: bool,
    pub fast_forward_wait: bool,
    pub min_wait_ms: u64,
}

impl Default for InputBufferDef {
    fn default() -> Self {
        Self {
            queue_during_wait: true,
            fast_forward_wait: true,
            min_wait_ms: 50,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleAnimation {
    pub trigger: RuleAnimationTrigger,
    pub name: String,
    pub objects: Vec<ObjectId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleAnimationTrigger {
    Move,
    CantMove,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnimationDef {
    pub tween: TweenAnimationDef,
    #[serde(default)]
    pub default_duration_ms: Option<u64>,
    #[serde(default)]
    pub triggers: Vec<TriggerAnimationDef>,
}

impl Default for AnimationDef {
    fn default() -> Self {
        Self {
            tween: TweenAnimationDef::default(),
            default_duration_ms: None,
            triggers: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriggerAnimationDef {
    pub name: String,
    pub duration_ms: u64,
    pub kind: TriggerAnimationKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriggerAnimationKind {
    Motion {
        motion: String,
    },
    Ascii {
        frames: Vec<Vec<String>>,
        colors: Vec<VisualColorDef>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleEffect {
    Win,
    Restart,
    NextLevel,
    Again,
    Checkpoint,
    ClearCheckpoint,
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
    Wait {
        milliseconds: u64,
    },
    WaitAnimation,
    EmitAnimation {
        name: String,
        component: u16,
        offset: AnimationOffset,
    },
    Message {
        text: String,
        literal: bool,
    },
    Scene(SceneEffect),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnimationOffset {
    pub x: u16,
    pub y: u16,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AssetsDef {
    pub entries: Vec<AssetDef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetDef {
    pub kind: AssetKind,
    pub path: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetKind {
    Css,
    Script,
    File,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SoundsDef {
    pub sfx: Vec<SfxSoundDef>,
    pub music: Vec<MusicSoundDef>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SfxSoundDef {
    pub name: String,
    pub seed: String,
    pub type_target: String,
    pub volume: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MusicSoundDef {
    pub name: String,
    pub seed: String,
    pub height: f64,
    pub bars: u16,
    pub bpm: u16,
    pub volume: f64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelOperationSoundDef {
    pub operation: ModelOperationSound,
    pub sfx_name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelOperationSound {
    Undo,
    Restart,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemeVariableDef {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct VisualsDef {
    pub aliases: Vec<VisualAliasDef>,
    pub sprites: Vec<VisualSpriteDef>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VisualAliasDef {
    pub object: String,
    pub sprite: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VisualSpriteDef {
    pub name: String,
    pub kind: VisualSpriteKind,
    #[serde(default)]
    pub transforms: Vec<VisualSpriteTransform>,
    pub fit: VisualSpriteFit,
    pub sampling: Option<VisualSpriteSampling>,
    #[serde(default)]
    pub loop_animation: Option<VisualSpriteLoopDef>,
    #[serde(default)]
    pub pixels_per_cell: Option<VisualSpritePixelsPerCell>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VisualSpriteLoopDef {
    pub duration_ms: u64,
    pub frames: Vec<Vec<String>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VisualSpriteTransform {
    Rotate {
        degrees: f64,
        space: VisualSpriteSpace,
    },
    Translate {
        x: f64,
        y: f64,
        space: VisualSpriteSpace,
    },
    Flip {
        enabled: bool,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisualSpriteSpace {
    #[default]
    World,
    Local,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualSpriteFit {
    pub mode: VisualSpriteFitMode,
    pub width: u32,
    pub height: u32,
}

impl Default for VisualSpriteFit {
    fn default() -> Self {
        Self {
            mode: VisualSpriteFitMode::Contain,
            width: 1,
            height: 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisualSpriteFitMode {
    Contain,
    Cover,
    Stretch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisualSpriteSampling {
    Pixelated,
    Smooth,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualSpritePixelsPerCell {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualColorDef {
    pub token: char,
    pub color: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PuzzleRenderDef {
    pub grid: PuzzleGridRenderDef,
    pub cell_size: Option<u16>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PuzzleGridRenderDef {
    pub occupied_cells: bool,
    pub all_cells: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Level {
    pub name: String,
    pub pack: Option<String>,
    pub puzzle: String,
    pub initial_state: State,
    pub regions: Vec<LevelRegionDef>,
    pub level_start_program: Option<Vec<RuleStep>>,
    pub level_clear_program: Option<Vec<RuleStep>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LevelRegionDef {
    pub index: usize,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewportSizeDef {
    Full,
    Size { width: u16, height: u16 },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewportModeDef {
    Paged,
    Centered,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GoalCondition {
    pub description: String,
    pub expr: GoalExpr,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GoalExpr {
    All(Vec<GoalExpr>),
    Any(Vec<GoalExpr>),
    Clause(GoalClause),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GoalClause {
    pub value: GoalValue,
    pub op: ComparisonOp,
    pub expected: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GoalValue {
    Variable(VariableId),
    Condition(ConditionId),
    InlineConditionValue(ConditionValueKind),
}

pub type SolverStrategy = SolverStrategyOf<QueryExpr>;
pub type SolverStrategyTerm = SolverStrategyTermOf<QueryExpr>;
pub type QueryExpr = QueryExprOf<ObjectId, ConditionValueKind, VariableId>;
pub type SolverStrategy3 = SolverStrategyOf<QueryExpr3>;
pub type SolverStrategyTerm3 = SolverStrategyTermOf<QueryExpr3>;
pub type QueryExpr3 = QueryExprOf<
    puzzle_grid3d::ObjectId,
    puzzle_grid3d::ConditionValueKind3,
    puzzle_grid3d::VariableId,
>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SolverStrategyOf<Query> {
    pub terms: Vec<SolverStrategyTermOf<Query>>,
    #[serde(default = "empty_solver_deadends")]
    pub deadends: Vec<SolverDeadendOf<Query>>,
}

fn empty_solver_deadends<Query>() -> Vec<SolverDeadendOf<Query>> {
    Vec::new()
}

impl<Query> Default for SolverStrategyOf<Query> {
    fn default() -> Self {
        Self {
            terms: Vec::new(),
            deadends: Vec::new(),
        }
    }
}

impl<Query> SolverStrategyOf<Query> {
    pub fn has_deadend_with(&self, mut evaluate: impl FnMut(&Query) -> bool) -> bool {
        self.deadends
            .iter()
            .any(|deadend| deadend.is_met_with(&mut evaluate))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SolverDeadendOf<Query> {
    All(Vec<Query>),
    Any(Vec<Query>),
}

impl<Query> SolverDeadendOf<Query> {
    pub fn values(&self) -> &[Query] {
        match self {
            Self::All(values) | Self::Any(values) => values,
        }
    }

    fn is_met_with(&self, evaluate: &mut impl FnMut(&Query) -> bool) -> bool {
        match self {
            Self::All(values) => values.iter().all(evaluate),
            Self::Any(values) => values.iter().any(evaluate),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SolverStrategyTermOf<Query> {
    pub direction: SolverStrategyDirection,
    pub value: Query,
    pub weight: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SolverStrategyDirection {
    Maximize,
    Minimize,
    Prefer,
    Avoid,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryExprOf<Object, Value, Variable> {
    Variable(Variable),
    Value(Value),
    Distance {
        from: Vec<Object>,
        to: Vec<Object>,
    },
    AllOnDistance {
        subjects: Vec<Object>,
        covers: Vec<Object>,
    },
    Compare {
        left: Box<QueryExprOf<Object, Value, Variable>>,
        op: ComparisonOp,
        right: i64,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SceneDef {
    pub name: String,
    pub layout: SceneLayoutDef,
    pub resources: SceneResources,
    pub state: SceneStateDef,
    pub components: Vec<SceneComponent>,
    pub key_bindings: Vec<KeyBinding>,
    pub routines: Vec<SceneRoutineDef>,
    pub transitions: Vec<SceneTransition>,
    pub puzzle_rule: Option<ScenePuzzleRule>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneRoutineDef {
    pub name: String,
    pub effect: SceneEffect,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenePuzzleRule {
    pub target: String,
    pub rule: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneResources {
    pub levels: ResourceSelection,
    pub sprites: ResourceSelection,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceSelection {
    #[default]
    All,
    Named(Vec<String>),
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SceneStateDef {
    pub variables: Vec<SceneVarDef>,
    pub puzzles: Vec<ScenePuzzleDef>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SceneVarDef {
    pub name: String,
    #[serde(default)]
    pub kind: SceneVarKind,
    pub default: SceneValue,
    pub lifetime: SceneStateLifetime,
    pub mutable: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneVarKind {
    #[default]
    Value,
    Signal,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScenePuzzleDef {
    pub name: String,
    pub kind: String,
    pub model: String,
    pub initializer: ScenePuzzleInitializer,
    pub lifetime: SceneStateLifetime,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScenePuzzleInitializer {
    CurrentLevel,
    Level(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneValue {
    Bool(bool),
    Int(i64),
    Text(String),
    Symbol(String),
    LevelRef(usize),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneStateLifetime {
    #[default]
    Instance,
    ResetOnStart,
    Persistent,
}

pub type SceneComponent = SharedSceneComponent<SceneEffect, SceneExpr, SceneTextContent, SceneExpr>;

pub type SceneTitleDef = SharedSceneTextComponent<SceneExpr>;

pub type SceneTextDef = SharedSceneTextComponent<SceneTextContent>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneTextContent {
    Literal(String),
    Path(Vec<String>),
    Expr(SceneExpr),
}

pub type SceneButtonDef = SharedSceneButton<SceneEffect, SceneExpr>;

pub type SceneConditionalDef =
    SharedSceneConditional<SceneEffect, SceneExpr, SceneTextContent, SceneExpr>;

pub type SceneContainerDef =
    SharedSceneContainer<SceneEffect, SceneExpr, SceneTextContent, SceneExpr>;

pub type SceneForDef = SharedSceneFor<SceneEffect, SceneExpr, SceneTextContent, SceneExpr>;

pub type LevelMenuDef = puzzle_scene::LevelMenuComponent<SceneEffect, SceneExpr>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyBinding {
    pub keys: Vec<KeyTrigger>,
    pub effect: SceneEffect,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneTransition {
    pub trigger: SceneTransitionTrigger,
    pub effect: SceneEffect,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneTransitionTrigger {
    Condition(SceneExpr),
    Signal(SceneExpr),
    SceneStart,
    LevelStart,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyTrigger {
    Char(char),
    Named(String),
}

impl LoadedGame {
    pub fn object_name(&self, object: ObjectId) -> &str {
        self.object_labels
            .get(&object)
            .map(String::as_str)
            .unwrap_or_else(|| {
                panic!(
                    "compiled object {} is missing its required object label",
                    object.0
                )
            })
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

    pub fn is_variable_truthy(&self, name: &str, state: &State) -> bool {
        let Some(variable) = self.variable_id(name) else {
            return false;
        };

        state.variable_value(variable).unwrap_or(0) != 0
    }

    fn variable_id(&self, name: &str) -> Option<VariableId> {
        self.variable_labels
            .iter()
            .find_map(|(variable, label)| (label == name).then_some(*variable))
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
        GoalValue::Variable(variable) => state.variable_value(*variable).unwrap_or(0),
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AsciiLegend {
    chars: Vec<char>,
    ignored: Vec<ObjectId>,
    empty: char,
    unknown: char,
}

impl AsciiLegend {
    pub(crate) fn new(object_count: usize, empty: Option<char>) -> Self {
        Self {
            chars: vec!['?'; object_count + 1],
            ignored: Vec::new(),
            empty: empty.unwrap_or('?'),
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

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Controls {
    pub keys: HashMap<u8, InputId>,
    pub arrows: HashMap<ArrowKey, InputId>,
    pub named: HashMap<String, InputId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArrowKey {
    Up,
    Down,
    Left,
    Right,
}
