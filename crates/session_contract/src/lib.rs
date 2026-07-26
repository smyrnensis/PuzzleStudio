use std::collections::{BTreeMap, HashMap};

use puzzle_lang::{SceneDef, SceneLayoutDef, SceneTextAlignDef, SceneTextRoleDef, SceneValue};
use puzzle_runtime_contract::{
    RuntimePresentationEvent, RuntimePuzzle3Resources, RuntimePuzzle3Snapshot,
    RuntimeResolvedRenderScene, RuntimeVisualComposition, SolverStateSnapshot,
};
use puzzle_scene::{ComponentPlacement, ComponentVisibility, SceneEffect};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq)]
pub enum RuntimeRendererState {
    TwoD(RuntimePuzzle2Snapshot),
    ThreeD(RuntimePuzzle3Snapshot),
}

pub trait RuntimePresentationBackend {
    type Error;
    type Output;

    fn present(&mut self, snapshot: &RuntimeSessionSnapshot) -> Result<Self::Output, Self::Error>;
}

#[derive(Clone, Debug)]
pub struct RuntimeSessionSnapshot {
    pub revision: u64,
    pub has_progress_save: bool,
    pub sounds: RuntimeSounds,
    pub theme: RuntimeTheme,
    pub default_wait_ms: u64,
    pub input_buffer: RuntimeInputBufferSettings,
    pub animation: RuntimeAnimationSettings,
    pub presentation_events: Vec<RuntimePresentationEvent>,
    pub level_index: Option<usize>,
    pub level_count: usize,
    pub levels: BTreeMap<String, RuntimeLevelRecord>,
    pub scene: Option<RuntimeRendererState>,
    pub accepts_model_input: bool,
    pub game_state: BTreeMap<String, SceneValue>,
    pub scene_state: BTreeMap<String, SceneValue>,
    pub scene_puzzles: Vec<String>,
    pub scene_puzzle_state: BTreeMap<String, RuntimeRendererState>,
    pub puzzle3_authoring_resources: Option<RuntimePuzzle3Resources>,
    pub surface: RuntimeSurface,
    pub solver_state: SolverStateSnapshot,
    pub selected_level_index: usize,
    pub busy: bool,
    pub can_undo: bool,
    pub can_redo: bool,
    pub inputs: Vec<RuntimeInputBinding>,
    pub scenes: Vec<SceneDef>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeSurface {
    pub root: Option<String>,
    pub focus: String,
    pub components: Vec<RuntimeSurfaceComponent>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeSurfaceComponent {
    pub id: String,
    pub definition: String,
    pub placement: ComponentPlacement,
    pub visibility: ComponentVisibility,
    pub modal: bool,
    pub properties: BTreeMap<String, SceneValue>,
    pub await_event: Option<String>,
    pub authored_projection: Option<RuntimeAuthoredComponentProjection>,
    pub presentation: RuntimeComponentPresentation,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeAuthoredComponentProjection {
    pub name: String,
    pub focused: bool,
    pub choice_cursor: Option<usize>,
    pub scene: Option<RuntimeRendererState>,
    pub scene_state: BTreeMap<String, SceneValue>,
    pub scene_puzzles: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RuntimeComponentPresentation {
    Ready(RuntimeResolvedScene),
    Error { error: String },
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeResolvedScene {
    pub name: String,
    pub layout: SceneLayoutDef,
    pub components: Vec<RuntimeResolvedSceneComponent>,
    pub keys: Option<Vec<RuntimeKeyBinding>>,
    pub events: Option<BTreeMap<String, RuntimeResolvedEventBinding>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeResolvedEventBinding {
    pub pointer: bool,
    pub keys: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeKeyBinding {
    pub effect: SceneEffect,
    pub keys: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RuntimeResolvedSceneComponent {
    Viewport {
        dimension: RuntimeViewportDimension,
        source: String,
        layout: SceneLayoutDef,
    },
    Frame {
        kind: String,
        source: String,
        layout: SceneLayoutDef,
    },
    Text {
        role: SceneTextRoleDef,
        value: String,
        text_align: Option<SceneTextAlignDef>,
        layout: SceneLayoutDef,
    },
    Button {
        label: String,
        effect: SceneEffect,
        layout: SceneLayoutDef,
    },
    Choice {
        label: String,
        effect: SceneEffect,
        layout: SceneLayoutDef,
    },
    Row {
        layout: SceneLayoutDef,
        children: Vec<RuntimeResolvedSceneComponent>,
    },
    Column {
        layout: SceneLayoutDef,
        children: Vec<RuntimeResolvedSceneComponent>,
    },
    Box {
        layout: SceneLayoutDef,
        children: Vec<RuntimeResolvedSceneComponent>,
    },
    Conditional {
        condition: bool,
        children: Vec<RuntimeResolvedSceneComponent>,
        else_children: Vec<RuntimeResolvedSceneComponent>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeViewportDimension {
    TwoD,
    ThreeD,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePuzzle2Snapshot {
    pub width: u16,
    pub height: u16,
    pub layer_count: u16,
    pub settings: RuntimePuzzle2Settings,
    pub animation: RuntimeAnimationSettings,
    pub screen: RuntimePuzzle2Screen,
    pub regions: Vec<RuntimeRegion2d>,
    pub resources: RuntimePuzzle2Resources,
    pub cells: Vec<RuntimePuzzle2Cell>,
    pub render_scene: RuntimeResolvedRenderScene,
    pub display_error: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePuzzle2Settings {
    pub render: RuntimeRender2d,
    pub grid: RuntimeGridRender2d,
    pub input_buffer: RuntimeInputBufferSettings,
    pub animation: RuntimeAnimationSettings,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeRender2d {}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeGridRender2d {
    pub visibility: bool,
    pub occupied_cells: bool,
    pub all_cells: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeInputBufferSettings {
    pub queue_during_wait: bool,
    pub fast_forward_wait: bool,
    pub min_wait_ms: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeAnimationSettings {
    pub tween: RuntimeTweenSettings,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeTweenSettings {
    pub enabled: bool,
    pub interval_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePuzzle2Screen {
    pub viewport_size: RuntimeViewportSize2d,
    pub viewport_focus: String,
    pub viewport_focus_objects: Vec<u16>,
    pub viewport_mode: RuntimeViewportMode2d,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeViewportSize2d {
    Full,
    Size { width: u16, height: u16 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeViewportMode2d {
    Paged,
    Centered,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeRegion2d {
    pub index: usize,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePuzzle2Resources {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub levels: Option<RuntimeResourceSelection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visuals: Option<RuntimeResourceSelection>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeResourceSelection {
    pub mode: RuntimeResourceSelectionMode,
    pub names: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeResourceSelectionMode {
    All,
    Named,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePuzzle2Cell {
    pub x: u16,
    pub y: u16,
    pub render_order: u64,
    pub layers: Vec<RuntimePuzzle2Layer>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePuzzle2Layer {
    pub layer: u16,
    pub object_id: u16,
    pub object: String,
    pub visual: String,
    pub render_priority: u16,
    pub render_order: u64,
    pub composition: RuntimeVisualComposition,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeSounds {
    pub sfx: Vec<RuntimeSfxSound>,
    pub music: Vec<RuntimeMusicSound>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeSfxSound {
    pub name: String,
    pub seed: String,
    pub type_target: String,
    pub volume: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeMusicSound {
    pub name: String,
    pub seed: String,
    pub height: f64,
    pub bars: u16,
    pub bpm: u16,
    pub volume: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeTheme {
    pub name: Option<String>,
    pub variables: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeLevelRecord {
    pub id: String,
    pub name: String,
    pub puzzle: String,
    pub pack: Option<String>,
    pub ordinal: usize,
    pub cleared: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeInputBinding {
    pub id: u16,
    pub name: String,
    pub key: Option<String>,
    pub arrow: Option<String>,
    pub keys: Vec<String>,
}

pub fn ordered_scene_values(values: &HashMap<String, SceneValue>) -> BTreeMap<String, SceneValue> {
    values
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}
