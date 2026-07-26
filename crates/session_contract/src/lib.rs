<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
use std::collections::BTreeMap;

pub use puzzle_runtime_contract::RuntimeTheme;
use puzzle_runtime_contract::{
    RuntimeKeyTrigger, RuntimePresentationEvent, RuntimePuzzle3Snapshot,
    RuntimeResolvedRenderScene, RuntimeResolvedView2d, RuntimeSceneActionToken,
    RuntimeViewportSourceId, RuntimeVisualComposition, SolverStateSnapshot,
};
use puzzle_scene::{
    ComponentPlacement, ComponentVisibility, SceneLayout as SceneLayoutDef,
    SceneTextAlign as SceneTextAlignDef, SceneTextRole as SceneTextRoleDef,
};
=======
use std::collections::{BTreeMap, HashMap};

use puzzle_lang::{SceneDef, SceneLayoutDef, SceneTextAlignDef, SceneTextRoleDef, SceneValue};
use puzzle_runtime_contract::{
    RuntimePresentationEvent, RuntimePuzzle3Resources, RuntimePuzzle3Snapshot,
    RuntimeResolvedRenderScene, RuntimeVisualComposition, SolverStateSnapshot,
};
use puzzle_scene::{ComponentPlacement, ComponentVisibility, SceneEffect};
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
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
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
=======
    pub sounds: RuntimeSounds,
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
    pub theme: RuntimeTheme,
    pub default_wait_ms: u64,
    pub input_buffer: RuntimeInputBufferSettings,
    pub animation: RuntimeAnimationSettings,
    pub presentation_events: Vec<RuntimePresentationEvent>,
    pub level_index: Option<usize>,
    pub level_count: usize,
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
    pub accepts_model_input: bool,
    pub viewport_sources: BTreeMap<RuntimeViewportSourceId, RuntimeRendererState>,
    pub surface: RuntimeSurface,
    pub busy: bool,
    pub can_undo: bool,
    pub can_redo: bool,
}

/// Development-only state paired with a player snapshot for editor inspection.
///
/// Player backends must consume [`RuntimeSessionSnapshot`] directly. This
/// contract contains authored names, solver state, and editable 2D projections
/// that are useful to development tools but are not runtime presentation input.
#[derive(Clone, Debug)]
pub struct RuntimeDevelopmentSessionSnapshot {
    pub player: RuntimeSessionSnapshot,
    pub levels: BTreeMap<String, RuntimeLevelRecord>,
    pub solver_state: SolverStateSnapshot,
    pub selected_level_index: usize,
    pub inputs: Vec<RuntimeInputBinding>,
    pub viewport_sources: BTreeMap<RuntimeViewportSourceId, RuntimeDevelopmentRendererState>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RuntimeDevelopmentRendererState {
    TwoD(RuntimePuzzle2DevelopmentSnapshot),
    ThreeD,
=======
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
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
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
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
    pub placement: ComponentPlacement,
    pub visibility: ComponentVisibility,
    pub modal: bool,
    pub await_event: Option<String>,
=======
    pub definition: String,
    pub placement: ComponentPlacement,
    pub visibility: ComponentVisibility,
    pub modal: bool,
    pub properties: BTreeMap<String, SceneValue>,
    pub await_event: Option<String>,
    pub authored_projection: Option<RuntimeAuthoredComponentProjection>,
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
    pub presentation: RuntimeComponentPresentation,
}

#[derive(Clone, Debug, PartialEq)]
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
=======
pub struct RuntimeAuthoredComponentProjection {
    pub name: String,
    pub focused: bool,
    pub choice_cursor: Option<usize>,
    pub scene: Option<RuntimeRendererState>,
    pub scene_state: BTreeMap<String, SceneValue>,
    pub scene_puzzles: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
pub enum RuntimeComponentPresentation {
    Ready(RuntimeResolvedScene),
    Error { error: String },
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeResolvedScene {
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
=======
    pub name: String,
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
    pub layout: SceneLayoutDef,
    pub components: Vec<RuntimeResolvedSceneComponent>,
    pub keys: Option<Vec<RuntimeKeyBinding>>,
    pub events: Option<BTreeMap<String, RuntimeResolvedEventBinding>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeResolvedEventBinding {
    pub pointer: bool,
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
    pub keys: Vec<RuntimeKeyTrigger>,
    pub action: Option<RuntimeSceneActionToken>,
=======
    pub keys: String,
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeKeyBinding {
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
    pub keys: Vec<RuntimeKeyTrigger>,
    pub action: RuntimeSceneActionToken,
=======
    pub effect: SceneEffect,
    pub keys: Vec<String>,
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
}

#[derive(Clone, Debug, PartialEq)]
pub enum RuntimeResolvedSceneComponent {
    Viewport {
        dimension: RuntimeViewportDimension,
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
        source: RuntimeViewportSourceId,
=======
        source: String,
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
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
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
        action: Option<RuntimeSceneActionToken>,
=======
        effect: SceneEffect,
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
        layout: SceneLayoutDef,
    },
    Choice {
        label: String,
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
        action: Option<RuntimeSceneActionToken>,
        selected: bool,
=======
        effect: SceneEffect,
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
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
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
=======
    Conditional {
        condition: bool,
        children: Vec<RuntimeResolvedSceneComponent>,
        else_children: Vec<RuntimeResolvedSceneComponent>,
    },
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeViewportDimension {
    TwoD,
    ThreeD,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePuzzle2Snapshot {
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
    pub view: RuntimeResolvedView2d,
    pub render_scene: RuntimeResolvedRenderScene,
    pub display_error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePuzzle2DevelopmentSnapshot {
=======
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
    pub width: u16,
    pub height: u16,
    pub layer_count: u16,
    pub settings: RuntimePuzzle2Settings,
    pub animation: RuntimeAnimationSettings,
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
    pub regions: Vec<RuntimeRegion2d>,
    pub resources: RuntimePuzzle2Resources,
    pub cells: Vec<RuntimePuzzle2Cell>,
=======
    pub screen: RuntimePuzzle2Screen,
    pub regions: Vec<RuntimeRegion2d>,
    pub resources: RuntimePuzzle2Resources,
    pub cells: Vec<RuntimePuzzle2Cell>,
    pub render_scene: RuntimeResolvedRenderScene,
    pub display_error: Option<String>,
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePuzzle2Settings {
    pub render: RuntimeRender2d,
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
=======
    pub grid: RuntimeGridRender2d,
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
    pub input_buffer: RuntimeInputBufferSettings,
    pub animation: RuntimeAnimationSettings,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeRender2d {}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
=======
#[serde(deny_unknown_fields)]
pub struct RuntimeGridRender2d {
    pub visibility: bool,
    pub occupied_cells: bool,
    pub all_cells: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
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
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
=======
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
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
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

<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
=======
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

>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
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
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
    pub triggers: Vec<RuntimeKeyTrigger>,
=======
    pub key: Option<String>,
    pub arrow: Option<String>,
    pub keys: Vec<String>,
}

pub fn ordered_scene_values(values: &HashMap<String, SceneValue>) -> BTreeMap<String, SceneValue> {
    values
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
}
