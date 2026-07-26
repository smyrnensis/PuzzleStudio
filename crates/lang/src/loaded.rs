use std::collections::HashMap;

use crate::SpatialPresentation;
use puzzle_assets::VisualImageAssetManifestEntry;
use puzzle_core::{
    ComparisonOp, ConditionId, ConditionValueKind, GridCompiledGame, GridConditionValueKind,
    GridExecutableProgram, GridGoalCondition, GridInput, GridProgramCatalog, GridProgramRef,
    GridRuleStep, GridSize, GridState, InputId, MarkId, ObjectId, RuleId, Size2, Size3, VariableId,
};
pub use puzzle_core::{GoalClauseOf, GoalConditionOf, GoalExprOf, GoalValueOf};
pub use puzzle_runtime_contract::RuntimeEffect;
pub use puzzle_scene::{
    ComponentOrder, ComponentPlacement, ComponentProperty, ComponentVisibility,
    SceneAlign as SceneAlignDef, SceneAspectRatio as SceneAspectRatioDef, SceneBinaryOp,
    SceneButton as SharedSceneButton, SceneComponent as SharedSceneComponent,
    SceneConditional as SharedSceneConditional, SceneContainer as SharedSceneContainer,
    SceneDistribution as SceneDistributionDef, SceneEffect, SceneEffectParam, SceneExpr,
    SceneLayout as SceneLayoutDef, SceneSpace as SceneSpaceDef,
    SceneTextAlign as SceneTextAlignDef, SceneTextComponent as SharedSceneTextComponent,
    SceneTextRole as SceneTextRoleDef, ViewportProjection as ViewportProjectionDef,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "owner", content = "effect", rename_all = "snake_case")]
/// Ordered effects attached to a lowered rule.
///
/// Runtime effects are source-free commands that may cross the public runtime
/// boundary. Lifecycle effects retain language-owned scene expressions and are
/// executed only by the session owner.
pub enum RuleEffect {
    Runtime(RuntimeEffect),
    Lifecycle(SceneEffect),
}

impl From<RuntimeEffect> for RuleEffect {
    fn from(effect: RuntimeEffect) -> Self {
        Self::Runtime(effect)
    }
}

impl RuleEffect {
    pub fn runtime(&self) -> Option<&RuntimeEffect> {
        match self {
            Self::Runtime(effect) => Some(effect),
            Self::Lifecycle(_) => None,
        }
    }

    pub fn into_runtime(self) -> Result<RuntimeEffect, SceneEffect> {
        match self {
            Self::Runtime(effect) => Ok(effect),
            Self::Lifecycle(effect) => Err(effect),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoadedDocument {
    pub default_wait_ms: u64,
    pub input_buffer: InputBufferDef,
    pub animation: AnimationDef,
    pub variables: Vec<SceneVarDef>,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LoadedDocumentModel {
    Puzzle2d {
        name: String,
        game: LoadedGame,
    },
    Puzzle3d {
        name: String,
        game: LoadedGridGame<3, Size3>,
        presentation: SpatialPresentation,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuleDebugInfo {
    pub source_line: String,
    pub source_line_number: Option<usize>,
    pub routine_stack: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoadedGridGame<const D: usize, Size: GridSize<D>> {
    pub game: GridCompiledGame<D>,
    #[serde(default)]
    pub inputs: Vec<GridInput<D>>,
    #[serde(default, skip_serializing)]
    pub warnings: Vec<String>,
    pub default_wait_ms: u64,
    pub input_buffer: InputBufferDef,
    pub animation: AnimationDef,
    pub rule_animations: HashMap<RuleId, Vec<RuleAnimation>>,
    pub rule_effects: HashMap<RuleId, Vec<RuleEffect>>,
    #[serde(default, skip_serializing)]
    pub rule_debug_info: HashMap<RuleId, RuleDebugInfo>,
    pub level_start_program: Option<GridExecutableProgram<D>>,
    pub level_clear_program: Option<GridExecutableProgram<D>>,
    pub last_level_clear_program: Option<GridExecutableProgram<D>>,
    pub program_catalog: GridProgramCatalog<D>,
    pub levels: Vec<LoadedGridLevel<D, Size>>,
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
    pub queries: HashMap<String, GridQueryExpr<D>>,
    pub conditions: HashMap<String, GridGoalCondition<D>>,
    pub goal: Option<GridGoalCondition<D>>,
    pub lose: Option<GridGoalCondition<D>>,
    #[serde(default, skip_serializing)]
    pub solver_strategy: GridSolverStrategy<D>,
    pub sounds: SoundsDef,
    #[serde(default)]
    pub model_operation_sounds: Vec<ModelOperationSoundDef>,
    pub theme: ThemeDef,
    pub assets: AssetsDef,
    pub visuals: VisualsDef,
    pub render: PuzzleRenderDef,
    pub screen: PuzzleScreenDef,
}

pub type LoadedGame = LoadedGridGame<2, Size2>;

impl<const D: usize, Size: GridSize<D>> LoadedGridGame<D, Size> {
    pub fn programs_for_level(&self, level_index: usize) -> Option<Vec<&GridExecutableProgram<D>>> {
        let level = self.levels.get(level_index)?;
        level
            .program
            .references()
            .iter()
            .map(|reference| self.resolve_program(*reference))
            .collect()
    }

    pub fn program_steps_for_level(&self, level_index: usize) -> Option<Vec<&GridRuleStep<D>>> {
        Some(
            self.programs_for_level(level_index)?
                .into_iter()
                .flat_map(|program| program.as_steps())
                .collect(),
        )
    }

    pub fn level_start_program_for_level(
        &self,
        level_index: usize,
    ) -> Option<&GridExecutableProgram<D>> {
        self.levels
            .get(level_index)
            .and_then(|level| level.level_start_program)
            .and_then(|reference| self.resolve_program(reference))
    }

    pub fn level_clear_program_for_level(
        &self,
        level_index: usize,
    ) -> Option<&GridExecutableProgram<D>> {
        self.levels
            .get(level_index)
            .and_then(|level| level.level_clear_program)
            .and_then(|reference| self.resolve_program(reference))
    }

    pub fn resolve_program(&self, reference: GridProgramRef) -> Option<&GridExecutableProgram<D>> {
        match reference {
            GridProgramRef::Main => Some(self.game.executable_program()),
            GridProgramRef::Catalog(_) => self.program_catalog.get(reference),
        }
    }

    pub fn validate_program_references(&self) -> Result<(), String> {
        for (level_index, level) in self.levels.iter().enumerate() {
            if !level.program.is_valid_level_sequence() {
                return Err(format!(
                    "level {level_index} has an invalid program sequence: {:?}",
                    level.program.references()
                ));
            }
            for (role, reference) in level
                .program
                .references()
                .iter()
                .copied()
                .map(|reference| ("program", Some(reference)))
                .chain([
                    ("level_start_program", level.level_start_program),
                    ("level_clear_program", level.level_clear_program),
                ])
            {
                let Some(reference) = reference else {
                    continue;
                };
                if self.resolve_program(reference).is_none() {
                    return Err(format!(
                        "level {level_index} has an invalid {role} reference: {reference:?}"
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn compiled_game_for_level(&self, level_index: usize) -> Option<GridCompiledGame<D>> {
        let steps = self
            .programs_for_level(level_index)?
            .into_iter()
            .flat_map(|program| program.as_steps().iter().cloned())
            .collect();
        Some(
            self.game
                .clone_with_executable_program(GridExecutableProgram::new(steps)),
        )
    }

    pub fn solver_state(&self, state: &GridState<D, Size>) -> GridState<D, Size> {
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub visual_rewrites: Vec<RuleVisualRewrite>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleVisualRewrite {
    pub remove: ObjectId,
    pub add: ObjectId,
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
    pub entries: Vec<VisualDef>,
    /// Presentation ordering is compiled from the unified layer declarations.
    #[serde(default)]
    pub order: VisualOrderDef,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualOrderDef {
    /// Cell-coordinate comparison directions, most significant first. The
    /// cell on the named direction side is rendered in front.
    pub direction_priority: Vec<String>,
    /// Back-to-front priorities for objects occupying the same cell.
    pub priorities: Vec<VisualOrderPriorityDef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualOrderPriorityDef {
    /// Canonically sorted for merge nodes; authored order for non-merge nodes.
    pub objects: Vec<String>,
    /// Visual resources emitted as transient animations into this priority.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub animations: Vec<String>,
    /// Merge is unordered same-priority composition, never ordered alpha-over.
    pub merge: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VisualAliasDef {
    pub object: String,
    pub visual: String,
    #[serde(skip)]
    pub source_line: Option<String>,
    #[serde(skip)]
    pub source_line_number: Option<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VisualDef {
    pub name: String,
    #[serde(skip)]
    pub source_line: Option<String>,
    #[serde(skip)]
    pub source_line_number: Option<usize>,
    pub kind: VisualKind,
    /// Canonical cell-art frames. Each frame contains one or more parallel planes.
    #[serde(default)]
    pub frames: Vec<VisualFrameDef>,
    #[serde(default)]
    pub transforms: Vec<VisualTransform>,
    pub fit: VisualFit,
    pub sampling: Option<VisualSampling>,
    #[serde(default)]
    pub animation_duration_ms: Option<u64>,
    #[serde(default)]
    pub pixels_per_cell: Option<VisualPixelsPerCell>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualFrameDef {
    pub planes: Vec<Vec<String>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VisualTransform {
    Rotate {
        degrees: f64,
        /// Unit axis in canonical visual space. Planar rotation uses +Z.
        axis: [f64; 3],
        space: VisualSpace,
    },
    Translate {
        /// Canonical visual-space displacement. Planar translation has z = 0.
        value: [f64; 3],
        space: VisualSpace,
    },
    Flip {
        enabled: bool,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisualSpace {
    #[default]
    World,
    Local,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualFit {
    pub mode: VisualFitMode,
    pub width: u32,
    pub height: u32,
}

impl Default for VisualFit {
    fn default() -> Self {
        Self {
            mode: VisualFitMode::Contain,
            width: 1,
            height: 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisualFitMode {
    Contain,
    Cover,
    Stretch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisualSampling {
    Pixelated,
    Smooth,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualPixelsPerCell {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum VisualKind {
    Solid(String),
    Image {
        asset: VisualImageAssetManifestEntry,
    },
    Ascii {
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
    pub grid: PuzzleGridMode,
    pub camera: crate::CameraSettings3,
    pub lighting: crate::LightingSettings3,
    pub visual: crate::VisualRenderSettings3,
    pub shadow: bool,
    pub viewport: crate::ViewportSettings3,
    pub pixelate: crate::PixelateRenderSettings3,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PuzzleGridMode {
    #[default]
    Hidden,
    OccupiedCells,
    AllCells,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoadedGridLevel<const D: usize, Size: GridSize<D>> {
    pub name: String,
    pub pack: Option<String>,
    pub puzzle: String,
    pub initial_state: GridState<D, Size>,
    pub regions: Vec<LevelRegionDef>,
    pub program: puzzle_core::GridProgramSequence,
    pub level_start_program: Option<GridProgramRef>,
    pub level_clear_program: Option<GridProgramRef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LevelId {
    pub puzzle: String,
    pub name: String,
}

impl LevelId {
    pub fn new(puzzle: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            puzzle: puzzle.into(),
            name: name.into(),
        }
    }

    pub fn record_key(&self) -> String {
        scene_level_record_key(&self.puzzle, &self.name)
    }

    pub fn progress_cleared_path(&self) -> Vec<String> {
        vec![
            "levels".to_string(),
            self.record_key(),
            "progress".to_string(),
            "cleared".to_string(),
        ]
    }
}

pub fn scene_level_record_key(puzzle: &str, name: &str) -> String {
    fn append_hex(out: &mut String, value: &str) {
        for byte in value.as_bytes() {
            out.push_str(&format!("{byte:02x}"));
        }
    }

    let mut key = String::from("level_");
    append_hex(&mut key, puzzle);
    key.push('_');
    append_hex(&mut key, name);
    key
}

pub type Level = LoadedGridLevel<2, Size2>;

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

pub type GoalCondition = GoalConditionOf<ConditionValueKind>;
pub type GoalExpr = GoalExprOf<ConditionValueKind>;
pub type GoalClause = GoalClauseOf<ConditionValueKind>;
pub type GoalValue = GoalValueOf<ConditionValueKind>;

pub type GridQueryExpr<const D: usize> =
    QueryExprOf<ObjectId, GridConditionValueKind<D>, VariableId>;
pub type GridSolverStrategy<const D: usize> = SolverStrategyOf<GridQueryExpr<D>>;
pub type SolverStrategy = GridSolverStrategy<2>;
pub type SolverStrategyTerm = SolverStrategyTermOf<QueryExpr>;
pub type QueryExpr = QueryExprOf<ObjectId, ConditionValueKind, VariableId>;

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

    pub(crate) fn try_map_query<Mapped, Error>(
        &self,
        map: &mut impl FnMut(&Query) -> Result<Mapped, Error>,
    ) -> Result<SolverStrategyOf<Mapped>, Error> {
        Ok(SolverStrategyOf {
            terms: self
                .terms
                .iter()
                .map(|term| {
                    Ok(SolverStrategyTermOf {
                        direction: term.direction,
                        value: map(&term.value)?,
                        weight: term.weight,
                    })
                })
                .collect::<Result<_, Error>>()?,
            deadends: self
                .deadends
                .iter()
                .map(|deadend| match deadend {
                    SolverDeadendOf::All(values) => values
                        .iter()
                        .map(&mut *map)
                        .collect::<Result<Vec<_>, _>>()
                        .map(SolverDeadendOf::All),
                    SolverDeadendOf::Any(values) => values
                        .iter()
                        .map(&mut *map)
                        .collect::<Result<Vec<_>, _>>()
                        .map(SolverDeadendOf::Any),
                })
                .collect::<Result<_, _>>()?,
        })
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

impl<Object: Clone, Value, Variable: Clone> QueryExprOf<Object, Value, Variable> {
    pub(crate) fn try_map_value<Mapped, Error>(
        &self,
        map: &mut impl FnMut(&Value) -> Result<Mapped, Error>,
    ) -> Result<QueryExprOf<Object, Mapped, Variable>, Error> {
        Ok(match self {
            Self::Variable(variable) => QueryExprOf::Variable(variable.clone()),
            Self::Value(value) => QueryExprOf::Value(map(value)?),
            Self::Distance { from, to } => QueryExprOf::Distance {
                from: from.clone(),
                to: to.clone(),
            },
            Self::AllOnDistance { subjects, covers } => QueryExprOf::AllOnDistance {
                subjects: subjects.clone(),
                covers: covers.clone(),
            },
            Self::Compare { left, op, right } => QueryExprOf::Compare {
                left: Box::new(left.try_map_value(map)?),
                op: *op,
                right: *right,
            },
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComponentDef {
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

/// `scene` is retained as the authoring term for a component definition that
/// can occupy the surface root. It has no distinct compiled runtime type.
pub type SceneDef = ComponentDef;

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
    pub visuals: ResourceSelection,
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

impl<const D: usize, Size: GridSize<D>> LoadedGridGame<D, Size> {
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

    pub fn is_goal_complete(&self, state: &GridState<D, Size>) -> bool {
        self.goal
            .as_ref()
            .is_some_and(|goal| goal.is_met(&self.game, state))
    }

    pub fn is_lose_complete(&self, state: &GridState<D, Size>) -> bool {
        self.lose
            .as_ref()
            .is_some_and(|lose| lose.is_met(&self.game, state))
    }

    pub fn is_condition_true(&self, name: &str, state: &GridState<D, Size>) -> bool {
        let Some(condition) = self.conditions.get(name) else {
            return false;
        };

        condition.is_met(&self.game, state)
    }

    pub fn is_variable_truthy(&self, name: &str, state: &GridState<D, Size>) -> bool {
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AsciiLegend {
    chars: Vec<char>,
    empty: char,
    unknown: char,
}

impl AsciiLegend {
    pub(crate) fn new(object_count: usize, empty: Option<char>) -> Self {
        Self {
            chars: vec!['?'; object_count + 1],
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

    pub fn char_for_cell(&self, objects: &[ObjectId]) -> char {
        self.char_for_cell_with_visible_objects(objects)
    }

    pub fn legended_objects_for_cell(&self, objects: &[ObjectId]) -> Vec<ObjectId> {
        objects
            .iter()
            .copied()
            .filter(|object| !object.is_empty())
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
